mod display;
mod input;
mod protocol;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use base64::Engine;
use clap::Parser;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use futures_util::SinkExt;
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use display::{DisplayState, Mode, TabInfo};
use protocol::{to_payload, Message, types};

#[derive(Parser, Debug)]
#[command(name = "termweb-client", about = "Terminal Browser Viewer (Rust)")]
struct Args {
    #[arg(short = 'c', long = "connect", default_value = "ws://127.0.0.1:9222/browser")]
    connect: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    stdout.execute(Hide)?;
    let _ = stdout.execute(PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
    ));

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let (frame_tx, mut frame_rx) = mpsc::channel::<Vec<u8>>(16);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(256);
    let (ws_tx, ws_rx) = mpsc::channel::<String>(64);

    let running = Arc::new(AtomicBool::new(true));

    let (cols, rows) = {
        let size = terminal.size()?;
        (size.width, size.height)
    };
    let mut state = DisplayState::new(cols, rows);
    state.connected = true;

    let ws_url = args.connect.clone();
    let ws_run = running.clone();
    let ws_cmd_tx = cmd_tx.clone();
    tokio::spawn(async move {
        websocket_task(&ws_url, frame_tx, ws_rx, ws_cmd_tx, ws_run).await;
    });

    let input_run = running.clone();
    let input_tx = cmd_tx.clone();
    tokio::spawn(async move {
        input_task(input_tx, input_run).await;
    });

    let mut frame_interval = tokio::time::interval(Duration::from_millis(33));

    'main: while running.load(Ordering::Relaxed) {
        frame_interval.tick().await;

        while let Ok(bytes) = frame_rx.try_recv() {
            if let Err(e) = state.set_image_from_jpeg(&bytes) {
                log::warn!("Image decode error: {e}");
            }
        }

        while let Ok(cmd) = cmd_rx.try_recv() {
            if let Some(ws_msg) = handle_command(&mut state, &cmd) {
                if ws_tx.send(ws_msg).await.is_err() {
                    break 'main;
                }
            }
        }

        let _ = terminal.draw(|f| {
            let _ = display::render_all(f, &mut state);
        });
    }

    let _ = terminal.backend_mut().execute(Show);
    let _ = terminal.backend_mut().execute(DisableMouseCapture);
    let _ = terminal.backend_mut().execute(PopKeyboardEnhancementFlags);
    let _ = terminal.backend_mut().execute(LeaveAlternateScreen);
    disable_raw_mode()?;
    terminal.clear()?;

    Ok(())
}

fn handle_command(state: &mut DisplayState, cmd: &str) -> Option<String> {
    if let Some(url) = cmd.strip_prefix("url_navigate:") {
        state.url = url.to_string();
        state.mode = Mode::Normal;
        state.url_buffer.clear();
    } else if let Some(url) = cmd.strip_prefix("url_set:") {
        state.url = url.to_string();
        state.url_buffer = url.to_string();
    } else if cmd == "mode:normal" {
        state.mode = Mode::Normal;
    } else if cmd == "mode:url" {
        state.url_buffer = state.url.clone();
        state.mode = Mode::UrlInput { cursor: state.url.len() };
    } else if let Some(key) = cmd.strip_prefix("url_key:") {
        match key {
            "Enter" => {
                let url = std::mem::take(&mut state.url_buffer);
                let nav_url = if url.contains("://") { url } else { format!("https://{}", url) };
                state.url = nav_url.clone();
                state.mode = Mode::Normal;
                let msg = Message::new(types::NAVIGATE, to_payload(&protocol::NavigatePayload {
                    url: nav_url,
                    tab_id: None,
                }));
                return Some(msg.to_json());
            }
            "Escape" => {
                state.mode = Mode::Normal;
                state.url_buffer.clear();
            }
            "Backspace" => {
                if let Mode::UrlInput { cursor } = &state.mode {
                    let mut c = *cursor;
                    if c > 0 {
                        c -= 1;
                        state.url_buffer.remove(c);
                        state.mode = Mode::UrlInput { cursor: c };
                    }
                }
            }
            "Delete" => {
                if let Mode::UrlInput { cursor } = &state.mode {
                    let c = *cursor;
                    if c < state.url_buffer.len() {
                        state.url_buffer.remove(c);
                        state.mode = Mode::UrlInput { cursor: c };
                    }
                }
            }
            "Left" => {
                if let Mode::UrlInput { cursor } = &state.mode {
                    if *cursor > 0 {
                        state.mode = Mode::UrlInput { cursor: cursor - 1 };
                    }
                }
            }
            "Right" => {
                if let Mode::UrlInput { cursor } = &state.mode {
                    if *cursor < state.url_buffer.len() {
                        state.mode = Mode::UrlInput { cursor: cursor + 1 };
                    }
                }
            }
            "Home" => {
                if let Mode::UrlInput { .. } = &state.mode {
                    state.mode = Mode::UrlInput { cursor: 0 };
                }
            }
            "End" => {
                if let Mode::UrlInput { .. } = &state.mode {
                    state.mode = Mode::UrlInput { cursor: state.url_buffer.len() };
                }
            }
            _ => {
                let special = ["Enter","Escape","Backspace","Delete","Left","Right","Home","End","Tab","Up","Down","PageUp","PageDown","Insert"];
                if key.len() == 1 && !special.contains(&key) {
                    if let Mode::UrlInput { cursor } = &state.mode {
                        let c = *cursor;
                        state.url_buffer.insert(c, key.chars().next().unwrap());
                        state.mode = Mode::UrlInput { cursor: c + 1 };
                    }
                }
            }
        }
    } else if let Some(key) = cmd.strip_prefix("find_key:") {
        match key {
            "Enter" => {
                let text = std::mem::take(&mut state.find_buffer);
                if !text.is_empty() {
                    let msg = Message::new(types::FIND_IN_PAGE, to_payload(&protocol::FindInPagePayload {
                        text,
                        tab_id: None,
                    }));
                    state.mode = Mode::Normal;
                    return Some(msg.to_json());
                } else {
                    state.mode = Mode::Normal;
                }
            }
            "Backspace" => {
                if let Mode::FindActive { cursor } = &state.mode {
                    let mut c = *cursor;
                    if c > 0 {
                        c -= 1;
                        state.find_buffer.remove(c);
                        state.mode = Mode::FindActive { cursor: c };
                    }
                }
            }
            "Delete" => {
                if let Mode::FindActive { cursor } = &state.mode {
                    let c = *cursor;
                    if c < state.find_buffer.len() {
                        state.find_buffer.remove(c);
                        state.mode = Mode::FindActive { cursor: c };
                    }
                }
            }
            "clear" => {
                state.find_buffer.clear();
            }
            _ => {
                if key.len() == 1 && !["Enter","Escape","Backspace","Delete"].contains(&key) {
                    if let Mode::FindActive { cursor } = &state.mode {
                        let c = *cursor;
                        state.find_buffer.insert(c, key.chars().next().unwrap());
                        state.mode = Mode::FindActive { cursor: c + 1 };
                    }
                }
            }
        }
    } else if cmd.starts_with("mode:find") {
        state.mode = Mode::FindActive { cursor: 0 };
        state.find_buffer.clear();
    } else if let Some(idx_str) = cmd.strip_prefix("switch_to_tab_idx:") {
        if let Ok(idx) = idx_str.parse::<usize>() {
            if idx > 0 && idx <= state.tabs.len() {
                let tab_id = state.tabs[idx - 1].id.clone();
                let msg = Message::new(types::SWITCH_TAB, to_payload(&protocol::SwitchTabPayload {
                    tab_id,
                }));
                return Some(msg.to_json());
            }
        }
    } else if let Some(val) = cmd.strip_prefix("loading:") {
        state.loading = val == "true";
    } else if let Some(val) = cmd.strip_prefix("status:") {
        state.status = val.to_string();
    } else if let Some(val) = cmd.strip_prefix("error_msg:") {
        state.push_error(val);
        state.status = format!("⚠ {}", val);
    } else if let Some(val) = cmd.strip_prefix("tabs:") {
        if let Ok(tabs) = serde_json::from_str::<Vec<TabInfo>>(val) {
            state.tabs = tabs;
        }
    }
    None
}

async fn websocket_task(
    url: &str,
    frame_tx: mpsc::Sender<Vec<u8>>,
    mut ws_rx: mpsc::Receiver<String>,
    cmd_tx: mpsc::Sender<String>,
    running: Arc<AtomicBool>,
) {
    let mut reconnect_delay = Duration::from_secs(1);

    while running.load(Ordering::Relaxed) {
        log::info!("Connecting to {url}");
        let connect = connect_async(url);
        let connect = tokio::time::timeout(Duration::from_secs(5), connect).await;
        match connect {
            Ok(Ok((ws_stream, _))) => {
                reconnect_delay = Duration::from_secs(1);
                let (mut write, mut read) = ws_stream.split();

                loop {
                    tokio::select! {
                        msg = read.next() => {
                            match msg {
                                Some(Ok(WsMessage::Text(text))) => {
                                    handle_server_message(&text, &frame_tx, &cmd_tx);
                                }
                                Some(Ok(WsMessage::Close(_))) | None => {
                                    log::warn!("Server closed connection");
                                    break;
                                }
                                Some(Err(e)) => {
                                    log::error!("WS read error: {e}");
                                    break;
                                }
                                _ => {}
                            }
                        }

                        msg = ws_rx.recv() => {
                            let json = match msg {
                                Some(m) => m,
                                None => {
                                    running.store(false, Ordering::Relaxed);
                                    return;
                                }
                            };
                            if let Err(e) = write.send(WsMessage::Text(json.into())).await {
                                log::error!("WS write error: {e}");
                                break;
                            }
                        }

                        _ = tokio::time::sleep(Duration::from_secs(15)) => {
                            if let Err(e) = write.send(WsMessage::Ping(vec![])).await {
                                log::error!("WS ping error: {e}");
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                log::error!("Connection failed: {e}");
            }
            Err(_) => {
                log::error!("Connection timed out after 5s");
            }
        }

        if running.load(Ordering::Relaxed) {
            log::info!("Reconnecting in {}s", reconnect_delay.as_secs());
            tokio::time::sleep(reconnect_delay).await;
            reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(30));
        }
    }
}

fn handle_server_message(text: &str, frame_tx: &mpsc::Sender<Vec<u8>>, cmd_tx: &mpsc::Sender<String>) {
    let Some(msg) = Message::from_json(text) else {
        return;
    };

    match msg.msg_type.as_str() {
        types::FRAME => {
            let data = msg.payload
                .get("data")
                .and_then(|v| v.as_str())
                .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok());
            if let Some(bytes) = data {
                let _ = frame_tx.try_send(bytes);
            }
        }
        types::SESSION_INFO => {
            if let Some(url) = msg.payload.get("url").and_then(|v| v.as_str()) {
                let _ = cmd_tx.try_send(format!("url_set:{url}"));
            }
            if let Some(tabs) = msg.payload.get("tabs").and_then(|v| v.as_array()) {
                let tab_infos: Vec<TabInfo> = tabs.iter().filter_map(|t| {
                    Some(TabInfo {
                        id: t.get("tabId")?.as_str()?.to_string(),
                        url: t.get("url")?.as_str()?.to_string(),
                        title: t.get("title")?.as_str()?.to_string(),
                        active: t.get("active")?.as_bool().unwrap_or(false),
                    })
                }).collect();
                if !tab_infos.is_empty() {
                    if let Ok(json) = serde_json::to_string(&tab_infos) {
                        let _ = cmd_tx.try_send(format!("tabs:{json}"));
                    }
                }
            }
        }
        types::URL_CHANGED => {
            if let Some(url) = msg.payload.get("url").and_then(|v| v.as_str()) {
                let _ = cmd_tx.try_send(format!("url_set:{url}"));
            }
        }
        types::LOADING_STATE => {
            if let Some(loading) = msg.payload.get("loading").and_then(|v| v.as_bool()) {
                let _ = cmd_tx.try_send(format!("loading:{loading}"));
            }
        }
        types::TAB_LIST => {
            if let Some(tabs) = msg.payload.get("tabs").and_then(|v| v.as_array()) {
                let tab_infos: Vec<TabInfo> = tabs.iter().filter_map(|t| {
                    Some(TabInfo {
                        id: t.get("tabId")?.as_str()?.to_string(),
                        url: t.get("url")?.as_str()?.to_string(),
                        title: t.get("title")?.as_str()?.to_string(),
                        active: t.get("active")?.as_bool().unwrap_or(false),
                    })
                }).collect();
                if !tab_infos.is_empty() {
                    if let Ok(json) = serde_json::to_string(&tab_infos) {
                        let _ = cmd_tx.try_send(format!("tabs:{json}"));
                    }
                }
            }
        }
        types::FIND_RESULTS => {
            let found = msg.payload.get("found").and_then(|v| v.as_bool()).unwrap_or(false);
            let text = msg.payload.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let _ = cmd_tx.try_send(format!("status:Find '{}' {}", text, if found { "found" } else { "not found" }));
        }
        types::ERROR => {
            if let Some(err) = msg.payload.get("message").and_then(|v| v.as_str()) {
                let _ = cmd_tx.try_send(format!("error_msg:{err}"));
            }
        }
        _ => {}
    }
}

async fn input_task(tx: mpsc::Sender<String>, running: Arc<AtomicBool>) {
    use crossterm::event::EventStream;
    let mut reader = EventStream::new();

    let mut mode = Mode::Normal;

    while running.load(Ordering::Relaxed) {
        tokio::select! {
            event = reader.next() => {
                match event {
                    Some(Ok(crossterm::event::Event::Key(ke))) => {
                        if ke.kind != crossterm::event::KeyEventKind::Press {
                            continue;
                        }
                        let input = input::parse_key(&ke);
                        if let input::InputEvent::Key(k) = input {
                            if k.ctrl && k.key == "c" {
                                running.store(false, Ordering::Relaxed);
                                return;
                            }

                            match mode {
                                Mode::UrlInput { .. } => {
                                    // URL input keys are forwarded as url_key commands
                                    let cmd = url_key_cmd(&k);
                                    let _ = tx.send(cmd).await;
                                    if k.key == "Enter" || k.key == "Escape" {
                                        mode = Mode::Normal;
                                        let _ = tx.send("mode:normal".to_string()).await;
                                    }
                                }
                                Mode::FindActive { .. } => {
                                    handle_find_input(&k, &mut mode, &tx).await;
                                }
                                Mode::Normal => {
                                    handle_normal_input(k, &mut mode, &tx).await;
                                }
                            }
                        }
                    }
                    Some(Ok(crossterm::event::Event::Mouse(me))) => {
                        if let Some(input::InputEvent::Mouse(m)) = input::parse_mouse(&me) {
                            forward_mouse(m, &tx).await;
                        }
                    }
                    Some(Ok(crossterm::event::Event::Resize(w, h))) => {
                        let msg = Message::new(types::RESIZE, to_payload(&protocol::ResizePayload {
                            width: w as u32,
                            height: h as u32,
                        }));
                        let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                    }
                    Some(Err(e)) => log::error!("Input error: {e}"),
                    None => break,
                    _ => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
}

fn url_key_cmd(k: &input::KeyInput) -> String {
    if k.is_special {
        format!("url_key:{}", k.key)
    } else if k.ctrl || k.alt {
        String::new() // ignore ctrl/alt in URL mode
    } else {
        format!("url_key:{}", k.key)
    }
}

async fn handle_find_input(
    k: &input::KeyInput,
    mode: &mut Mode,
    tx: &mpsc::Sender<String>,
) {
    match k.key.as_str() {
        "Enter" => {
            let _ = tx.send("find_key:Enter".to_string()).await;
        }
        "Escape" => {
            *mode = Mode::Normal;
            let _ = tx.send("mode:normal".to_string()).await;
            let _ = tx.send("find_key:clear".to_string()).await;
        }
        "Backspace" => {
            let _ = tx.send("find_key:Backspace".to_string()).await;
        }
        "Delete" => {
            let _ = tx.send("find_key:Delete".to_string()).await;
        }
        _ => {
            if !k.ctrl && !k.alt && k.key.len() == 1 && !k.is_special {
                let _ = tx.send(format!("find_key:{}", k.key)).await;
            }
        }
    }
}

async fn handle_normal_input(
    k: input::KeyInput,
    mode: &mut Mode,
    tx: &mpsc::Sender<String>,
) {
    if k.ctrl {
        match k.key.as_str() {
            "l" => {
                *mode = Mode::UrlInput { cursor: 0 };
                let _ = tx.send("mode:url".to_string()).await;
                return;
            }
            "r" => {
                let msg = Message::new(types::REQUEST_SCREENSHOT, serde_json::Value::Null);
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                return;
            }
            "t" => {
                let msg = Message::new(types::CREATE_TAB, serde_json::json!({"url": "about:blank"}));
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                return;
            }
            "w" => {
                let msg = Message::new(types::CLOSE_TAB, serde_json::Value::Null);
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                return;
            }
            "f" => {
                *mode = Mode::FindActive { cursor: 0 };
                let _ = tx.send("mode:find".to_string()).await;
                return;
            }
            "b" => {
                let msg = Message::new(types::GO_BACK, serde_json::Value::Null);
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                return;
            }
            "n" => {
                let msg = Message::new(types::GO_FORWARD, serde_json::Value::Null);
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                return;
            }
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                let _ = tx.send(format!("switch_to_tab_idx:{}", k.key)).await;
                return;
            }
            _ => {}
        }
    }

    if k.alt {
        match k.key.as_str() {
            "Left" | "h" => {
                let msg = Message::new(types::GO_BACK, serde_json::Value::Null);
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                return;
            }
            "Right" | "l" => {
                let msg = Message::new(types::GO_FORWARD, serde_json::Value::Null);
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                return;
            }
            _ => {}
        }
    }

    let msg = if k.is_special {
        Message::new(types::KEY_PRESS, to_payload(&protocol::KeyPressPayload {
            key: k.key.clone(),
            modifiers: Some(input::key_to_modifiers(&k)),
            tab_id: None,
        }))
    } else if k.ctrl || k.alt {
        Message::new(types::KEY_PRESS, to_payload(&protocol::KeyPressPayload {
            key: k.key.clone(),
            modifiers: Some(input::key_to_modifiers(&k)),
            tab_id: None,
        }))
    } else if k.key == "Enter" || k.key == "Backspace" || k.key == "Tab" || k.key == "Escape" {
        Message::new(types::KEY_PRESS, to_payload(&protocol::KeyPressPayload {
            key: k.key.clone(),
            modifiers: None,
            tab_id: None,
        }))
    } else {
        Message::new(types::TYPE, to_payload(&protocol::TypePayload {
            text: k.key.clone(),
            tab_id: None,
        }))
    };

    let _ = tx.send(format!("ws:{}", msg.to_json())).await;
}

async fn forward_mouse(input: input::MouseInput, tx: &mpsc::Sender<String>) {
    let (col, row) = (input.col, input.row);

    let msg = match input.action {
        input::MouseAction::Click => {
            Message::new(types::CLICK, to_payload(&protocol::ClickPayload {
                x: col as u32,
                y: row as u32,
                button: input.button.to_string(),
                tab_id: None,
            }))
        }
        input::MouseAction::Drag => {
            Message::new(types::MOUSE_MOVE, to_payload(&protocol::MouseMovePayload {
                x: col as u32,
                y: row as u32,
                tab_id: None,
            }))
        }
        input::MouseAction::Release => {
            Message::new(types::MOUSE_UP, to_payload(&protocol::MouseUpPayload {
                x: col as u32,
                y: row as u32,
                button: input.button.to_string(),
                tab_id: None,
            }))
        }
        input::MouseAction::ScrollUp => {
            Message::new(types::SCROLL, to_payload(&protocol::ScrollPayload {
                delta_x: 0,
                delta_y: -3,
                tab_id: None,
            }))
        }
        input::MouseAction::ScrollDown => {
            Message::new(types::SCROLL, to_payload(&protocol::ScrollPayload {
                delta_x: 0,
                delta_y: 3,
                tab_id: None,
            }))
        }
        input::MouseAction::ScrollLeft => {
            Message::new(types::SCROLL, to_payload(&protocol::ScrollPayload {
                delta_x: -3,
                delta_y: 0,
                tab_id: None,
            }))
        }
        input::MouseAction::ScrollRight => {
            Message::new(types::SCROLL, to_payload(&protocol::ScrollPayload {
                delta_x: 3,
                delta_y: 0,
                tab_id: None,
            }))
        }
    };

    let _ = tx.send(format!("ws:{}", msg.to_json())).await;
}
