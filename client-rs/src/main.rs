mod display;
mod input;
mod protocol;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use base64::Engine;
use clap::{Parser, Subcommand};
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

use display::{DisplayState, Mode, TabInfo, PageElement, BROWSER_MENU};
use protocol::{to_payload, Message, types};

/// JavaScript to extract page text, links, and interactive elements with selectors
const PAGE_EXTRACT_JS: &str = r#"(function(){
  function qs(el){if(el.id)return '#'+CSS.escape(el.id);var p=[],c=el;
    while(c&&c!==document.body&&c!==document.documentElement){
      var t=c.tagName.toLowerCase();if(c.id){p.unshift('#'+CSS.escape(c.id));break}
      var pa=c.parentElement;if(pa){var sib=Array.from(pa.children).filter(function(x){return x.tagName===c.tagName});var idx=sib.indexOf(c)+1;t+=':nth-of-type('+idx+')'}
      p.unshift(t);c=pa}return p.join(' > ')}
  var els=document.querySelectorAll('a[href],button,input,textarea,select,[role="button"],[onclick],[tabindex]:not([tabindex="-1"]),[contenteditable]');
  var seen=new Set();var elems=Array.from(els).filter(function(e){
    var r=e.getBoundingClientRect();return r.w>0&&r.h>0&&!seen.has(e)&&(seen.add(e),true)
  }).map(function(e,i){var r=e.getBoundingClientRect();return{
    id:i,tag:e.tagName.toLowerCase(),type:e.type||null,
    text:(e.textContent||'').trim().replace(/\s+/g,' ').slice(0,80),
    href:e.href||null,role:e.getAttribute('role'),selector:qs(e),
    rect:{x:r.x,y:r.y,w:r.width,h:r.height},
    input_type:(e.tagName==='INPUT'||e.tagName==='TEXTAREA')?(e.type||'text'):null
  }});
  return JSON.stringify({
    text:document.body.innerText.replace(/\s+/g,' ').trim().slice(0,50000),
    links:Array.from(document.querySelectorAll('a[href]')).map(function(a){return{text:(a.textContent||'').trim().slice(0,80),href:a.href}}).filter(function(l){return l.text&&l.href}),
    elements:elems
  });
})()"#;

#[derive(Parser, Debug)]
#[command(
    name = "bcli",
    about = "Terminal Browser — interactive TUI or direct CLI commands",
    long_about = "BCLI: Terminal browser with headless Chrome.\n\
        \x20 • Interactive mode (default): full TUI with mouse/keyboard\n\
        \x20 • Command mode: run one-off commands for automation/scripts\n\
        \nExamples:\n\
        \x20 bcli                        Interactive TUI (text mode)\n\
        \x20 bcli --graphical            Interactive TUI (graphical mode)\n\
        \x20 bcli nav https://x.com      Navigate to URL\n\
        \x20 bcli text                   Get page text\n\
        \x20 bcli links                  Get all links\n\
        \x20 bcli click '#btn'           Click element\n\
        \x20 bcli eval 'document.title'  Run JavaScript\n\
        \x20 bcli screenshot page.jpg    Take screenshot\n\
        \x20 bcli status                 Session status"
)]
struct Args {
    #[arg(short = 'c', long = "connect", default_value = "ws://127.0.0.1:9222/browser")]
    connect: String,

    #[arg(
        long = "graphical",
        help = "Interactive mode: render page screenshots (requires Kitty/Sixel)"
    )]
    graphical: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Navigate to a URL
    Nav {
        /// URL to navigate to (e.g. https://example.com)
        url: String,
    },
    /// Click element by CSS selector or pixel coordinates
    Click {
        /// CSS selector (e.g. '#btn', 'a[href*="login"]')
        selector: Option<String>,
        /// X pixel coordinate (used with --y, ignored if --selector given)
        #[arg(short = 'x')]
        x: Option<u32>,
        /// Y pixel coordinate
        #[arg(short = 'y')]
        y: Option<u32>,
    },
    /// Type text into an element (optionally by selector)
    Type {
        /// CSS selector (types into focused element if omitted)
        selector: Option<String>,
        /// Text to type
        text: String,
    },
    /// Scroll the page
    Scroll {
        /// Pixels to scroll (negative=up, positive=down)
        #[arg(short = 'y', default_value = "300")]
        delta_y: i32,
    },
    /// Execute JavaScript in the page and print result
    Eval {
        /// JavaScript code to execute
        code: String,
    },
    /// Save a screenshot to file
    Screenshot {
        /// Output file path (default: screenshot-{timestamp}.jpg)
        path: Option<String>,
    },
    /// Extract clean page text
    Text,
    /// Extract all links with hrefs
    Links,
    /// Extract interactive elements with CSS selectors
    Elements,
    /// Show session and page status
    Status,
    /// Go back in history
    Back,
    /// Go forward in history
    Forward,
    /// Wait for N milliseconds
    Wait {
        /// Milliseconds to wait
        #[arg(default_value = "2000")]
        ms: u64,
    },
    /// Find text in page
    Find {
        /// Text to search for
        text: String,
    },
    /// Manage browser session (create, destroy, status)
    Session {
        /// Action: create, destroy, status
        action: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // ─── Command mode (non-interactive) ─────────────────────────────────
    if let Some(cmd) = &args.command {
        return run_command(&args.connect, cmd).await;
    }

    // ─── Interactive TUI mode ───────────────────────────────────────────
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

    let dotted = !args.graphical;

    if dotted {
        let req = Message::new(types::EVALUATE, serde_json::json!({
            "code": PAGE_EXTRACT_JS
        }));
        let _ = ws_tx.try_send(req.to_json());
    }

    let running = Arc::new(AtomicBool::new(true));

    let (cols, rows) = {
        let size = terminal.size()?;
        (size.width, size.height)
    };
    let mut state = DisplayState::new(cols, rows, dotted);
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
            if state.dotted {
            } else if let Err(e) = state.set_image_from_jpeg(&bytes) {
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

        if state.dotted && state.page_text.is_empty() {
            let req = Message::new(types::EVALUATE, serde_json::json!({
                "code": PAGE_EXTRACT_JS
            }));
            let _ = ws_tx.try_send(req.to_json()).ok();
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

// ─── Command mode runner ─────────────────────────────────────────────────

async fn run_command(ws_url: &str, cmd: &Command) -> Result<()> {
    let (ws_stream, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_async(ws_url),
    ).await
        .map_err(|_| anyhow::anyhow!("Connection timed out (10s)"))?
        .map_err(|e| anyhow::anyhow!("Connection failed: {e}"))?;

    let (mut write, mut read) = ws_stream.split();

    match cmd {
        Command::Nav { url } => {
            let msg = Message::new(types::NAVIGATE, serde_json::json!({"url": url}));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            println!("Navigating to: {url}");
        }

        Command::Click { selector, x, y } => {
            let msg = if let Some(sel) = selector {
                Message::new(types::CLICK, serde_json::json!({"selector": sel, "x": 0, "y": 0}))
            } else {
                let cx = x.unwrap_or(0);
                let cy = y.unwrap_or(0);
                Message::new(types::CLICK, serde_json::json!({"x": cx, "y": cy, "button": "left"}))
            };
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            let loc_str = format!("({}, {})", x.unwrap_or(0), y.unwrap_or(0));
            let loc = selector.as_deref().unwrap_or(&loc_str);
            println!("Clicked: {loc}");
        }

        Command::Type { selector, text } => {
            if let Some(sel) = selector {
                let click = Message::new(types::CLICK, serde_json::json!({"selector": sel, "x": 0, "y": 0}));
                write.send(WsMessage::Text(click.to_json().into())).await?;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let msg = Message::new(types::TYPE, serde_json::json!({"text": text}));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            println!("Typed {} chars: {}", text.len(), text.chars().take(60).collect::<String>());
        }

        Command::Scroll { delta_y } => {
            let msg = Message::new(types::SCROLL, serde_json::json!({"delta_y": delta_y}));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            println!("Scrolled: {delta_y}px");
        }

        Command::Back => {
            let msg = Message::new(types::GO_BACK, serde_json::Value::Null);
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            println!("Going back");
        }

        Command::Forward => {
            let msg = Message::new(types::GO_FORWARD, serde_json::Value::Null);
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            println!("Going forward");
        }

        Command::Wait { ms } => {
            tokio::time::sleep(Duration::from_millis(*ms)).await;
            println!("Waited {ms}ms");
        }

        Command::Find { text } => {
            let msg = Message::new(types::FIND_IN_PAGE, serde_json::json!({"text": text}));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            println!("Searching for: {text}");
        }

        // ─── Query commands (wait for response) ────────────────────────

        Command::Eval { code } => {
            let msg = Message::new(types::EVALUATE, serde_json::json!({"code": code}));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            let result = wait_for_response(&mut read, &[types::EVALUATE_RESULT], Duration::from_secs(15)).await;
            match result {
                Some(json) => {
                    if let Some(r) = json.get("result") {
                        println!("{}", serde_json::to_string_pretty(r).unwrap_or_else(|_| r.to_string()));
                    }
                }
                None => eprintln!("No response (timeout)"),
            }
        }

        Command::Text => {
            let msg = Message::new(types::EVALUATE, serde_json::json!({
                "code": "document.body.innerText.replace(/\\s+/g,' ').trim().slice(0,100000)"
            }));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            let result = wait_for_response(&mut read, &[types::EVALUATE_RESULT], Duration::from_secs(15)).await;
            match result {
                Some(json) => {
                    if let Some(r) = json.get("result").and_then(|v| v.as_str()) {
                        println!("{r}");
                    } else {
                        println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                    }
                }
                None => eprintln!("No response (timeout)"),
            }
        }

        Command::Links => {
            let msg = Message::new(types::EVALUATE, serde_json::json!({
                "code": "JSON.stringify(Array.from(document.querySelectorAll('a[href]')).map(a=>({text:(a.textContent||'').trim().slice(0,100),href:a.href})).filter(l=>l.text&&l.href))"
            }));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            let result = wait_for_response(&mut read, &[types::EVALUATE_RESULT], Duration::from_secs(15)).await;
            match result {
                Some(json) => {
                    if let Some(links) = json.get("result").and_then(|v| v.as_str()).and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok()) {
                        for (i, link) in links.iter().enumerate() {
                            let text = link.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            let href = link.get("href").and_then(|v| v.as_str()).unwrap_or("");
                            println!("{:>4}. {} → {}", i + 1, text, href);
                        }
                        println!("── {} links total ──", links.len());
                    }
                }
                None => eprintln!("No response (timeout)"),
            }
        }

        Command::Elements => {
            let msg = Message::new(types::EVALUATE, serde_json::json!({"code": PAGE_EXTRACT_JS}));
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            let result = wait_for_response(&mut read, &[types::EVALUATE_RESULT], Duration::from_secs(15)).await;
            match result {
                Some(json) => {
                    if let Some(elements) = json.get("elements").and_then(|v| v.as_array()) {
                        for (i, el) in elements.iter().enumerate() {
                            let tag = el.get("tag").and_then(|v| v.as_str()).unwrap_or("?");
                            let text = el.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            let sel = el.get("selector").and_then(|v| v.as_str()).unwrap_or("");
                            let it = el.get("input_type").and_then(|v| v.as_str()).unwrap_or("");
                            if !text.is_empty() || !sel.is_empty() {
                                println!("{:>4}. <{}> {} | sel={} {}", i + 1, tag, text, sel,
                                    if !it.is_empty() { format!("type={}", it) } else { String::new() });
                            }
                        }
                        if let Some(count) = json.get("elements").and_then(|v| v.as_array()).map(|a| a.len()) {
                            println!("── {count} elements total ──");
                        }
                    } else {
                        println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                    }
                }
                None => eprintln!("No response (timeout)"),
            }
        }

        Command::Screenshot { path } => {
            let msg = Message::new(types::REQUEST_SCREENSHOT, serde_json::Value::Null);
            write.send(WsMessage::Text(msg.to_json().into())).await?;
            let result = wait_for_response(&mut read, &[types::FRAME], Duration::from_secs(15)).await;
            match result {
                Some(json) => {
                    if let Some(data) = json.get("data").and_then(|v| v.as_str()) {
                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) {
                            let filename = path.clone().unwrap_or_else(|| {
                                let ts = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default().as_secs();
                                format!("screenshot-{ts}.jpg")
                            });
                            std::fs::write(&filename, &bytes)?;
                            let w = json.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                            let h = json.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                            println!("Screenshot saved: {filename} ({w}x{h}, {} bytes)", bytes.len());
                        }
                    }
                }
                None => eprintln!("No screenshot received (timeout)"),
            }
        }

        Command::Status => {
            let result = wait_for_response(&mut read, &[types::SESSION_INFO, types::URL_CHANGED], Duration::from_secs(5)).await;
            match result {
                Some(json) => {
                    let url = json.get("url").and_then(|v| v.as_str()).unwrap_or("?");
                    let title = json.get("title").and_then(|v| v.as_str()).unwrap_or("?");
                    let sid = json.get("sessionId").and_then(|v| v.as_str()).unwrap_or("?");
                    let vp_w = json.get("viewport").and_then(|v| v.get("width")).and_then(|v| v.as_u64()).unwrap_or(0);
                    let vp_h = json.get("viewport").and_then(|v| v.get("height")).and_then(|v| v.as_u64()).unwrap_or(0);
                    let tabs = json.get("tabs").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                    println!("Session:     {sid}");
                    println!("URL:         {url}");
                    println!("Title:       {title}");
                    println!("Viewport:    {vp_w}x{vp_h}");
                    println!("Tabs open:   {tabs}");
                    println!("Connected:   ws://…");
                }
                None => {
                    println!("Session:     active");
                    println!("Connected:   {ws_url}");
                }
            }
        }

        Command::Session { action } => {
            match action.as_str() {
                "status" | "info" => {
                    let result = wait_for_response(&mut read, &[types::SESSION_INFO], Duration::from_secs(5)).await;
                    match result {
                        Some(json) => {
                            let sid = json.get("sessionId").and_then(|v| v.as_str()).unwrap_or("?");
                            let url = json.get("url").and_then(|v| v.as_str()).unwrap_or("?");
                            let tabs = json.get("tabs").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                            println!("Session: {sid} | URL: {url} | Tabs: {tabs}");
                        }
                        None => println!("Session active but no info received yet"),
                    }
                }
                "create" | "new" => {
                    // Session is auto-created on WS connect
                    let result = wait_for_response(&mut read, &[types::SESSION_INFO], Duration::from_secs(5)).await;
                    match result {
                        Some(json) => {
                            let sid = json.get("sessionId").and_then(|v| v.as_str()).unwrap_or("?");
                            println!("Session created: {sid}");
                        }
                        None => println!("Session created (no info)"),
                    }
                }
                "destroy" | "close" | "kill" => {
                    // Close WS connection — server will cleanup idle session
                    println!("Session will close (idle cleanup in 30s)");
                }
                _ => {
                    eprintln!("Unknown session action: {action}. Use: status, create, destroy");
                }
            }
        }

    }

    // Brief wait for server to process, then disconnect
    write.send(WsMessage::Close(None)).await.ok();
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

/// Wait for a server message matching one of the expected types
async fn wait_for_response(
    read: &mut (impl futures_util::Stream<Item = Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
    expected_types: &[&str],
    timeout: Duration,
) -> Option<serde_json::Value> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                        if let Some(msg) = Message::from_json(&text) {
                            if expected_types.contains(&msg.msg_type.as_str()) {
                                return Some(msg.payload);
                            }
                            // Also capture URL_CHANGED after NAVIGATE
                            if expected_types.contains(&types::URL_CHANGED) && msg.msg_type == types::URL_CHANGED {
                                return Some(msg.payload);
                            }
                            // Capture SESSION_INFO
                            if expected_types.contains(&types::SESSION_INFO) && msg.msg_type == types::SESSION_INFO {
                                return Some(msg.payload);
                            }
                        }
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => {
                        return None;
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
}

fn handle_command(state: &mut DisplayState, cmd: &str) -> Option<String> {
    if let Some(url) = cmd.strip_prefix("url_navigate:") {
        state.url = url.to_string();
        state.mode = Mode::Normal;
        state.url_buffer.clear();
    } else if let Some(url) = cmd.strip_prefix("url_set:") {
        state.url = url.to_string();
        state.url_buffer = url.to_string();
        if state.dotted {
            state.page_text.clear();
            state.page_links.clear();
            state.page_elements.clear();
            state.focus_idx = None;
            state.loading = true;
        }
    } else if cmd == "mode:normal" {
        state.mode = Mode::Normal;
    } else if cmd == "mode:browser" {
        state.mode = Mode::Browser { menu_idx: 0 };
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
    } else if let Some(val) = cmd.strip_prefix("evaluate_result:") {
        // Parse JSON result from evaluate command
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(val) {
            if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                state.page_text = text.to_string();
            }
            if let Some(links) = data.get("links").and_then(|v| v.as_array()) {
                state.page_links.clear();
                for link in links {
                    if let (Some(t), Some(h)) = (link.get("text").and_then(|v| v.as_str()), link.get("href").and_then(|v| v.as_str())) {
                        state.page_links.push((t.to_string(), h.to_string()));
                    }
                }
            }
            // Parse interactive elements
            if let Some(elements) = data.get("elements").and_then(|v| v.as_array()) {
                state.page_elements = elements.iter()
                    .filter_map(|e| serde_json::from_value::<PageElement>(e.clone()).ok())
                    .collect();
                // Sort by vertical position (top-to-bottom) then horizontal (left-to-right)
                state.page_elements.sort_by(|a, b| {
                    a.center_y().partial_cmp(&b.center_y())
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.center_x().partial_cmp(&b.center_x())
                            .unwrap_or(std::cmp::Ordering::Equal))
                });
                // Re-assign IDs after sort
                for (i, el) in state.page_elements.iter_mut().enumerate() {
                    el.id = i;
                }
                // Auto-focus first element if none focused
                if state.focus_idx.is_none() && !state.page_elements.is_empty() {
                    state.focus_idx = Some(0);
                }
            }
            state.loading = false;
            let el_count = state.page_elements.len();
            let link_count = state.page_links.len();
            state.status = format!("{} elements, {} links", el_count, link_count);
        }
    } else if let Some(val) = cmd.strip_prefix("ws:") {
        // Intercept outgoing WS messages for dotted mode
        if state.dotted {
            if let Some(msg) = Message::from_json(val) {
                match msg.msg_type.as_str() {
                    types::TYPE | types::KEY_PRESS => {
                        let text = msg.payload.get("text").or_else(|| msg.payload.get("key")).and_then(|v| v.as_str()).unwrap_or("");
                        // Check if it's a digit for link clicking
                        if let Ok(n) = text.parse::<usize>() {
                            if n > 0 && n <= 9 && !state.page_links.is_empty() && state.focus_idx.is_none() {
                                return handle_dotted_link_click(state, n);
                            }
                        }
                        // Element navigation in Normal mode with elements loaded
                        if !state.page_elements.is_empty() && matches!(state.mode, Mode::Normal) {
                            match text {
                                "Tab" | "ArrowDown" | "Down" => {
                                    let next = state.focus_idx.map(|i| i + 1).unwrap_or(0);
                                    if next < state.page_elements.len() {
                                        state.focus_idx = Some(next);
                                    } else {
                                        state.focus_idx = Some(0); // wrap
                                    }
                                    state.scroll_offset = 0;
                                    return None;
                                }
                                "BackTab" | "ArrowUp" | "Up" => {
                                    let prev = state.focus_idx.map(|i| i.wrapping_sub(1)).unwrap_or(0);
                                    state.focus_idx = Some(if prev >= state.page_elements.len() { state.page_elements.len() - 1 } else { prev });
                                    state.scroll_offset = 0;
                                    return None;
                                }
                                "ArrowRight" | "Right" => {
                                    if let Some(idx) = state.focus_idx {
                                        let cur_y = state.page_elements[idx].center_y();
                                        // Find next element on similar Y
                                        let next = (idx + 1..state.page_elements.len())
                                            .find(|&i| (state.page_elements[i].center_y() - cur_y).abs() < 30.0)
                                            .unwrap_or(idx);
                                        state.focus_idx = Some(next);
                                    }
                                    return None;
                                }
                                "ArrowLeft" | "Left" => {
                                    if let Some(idx) = state.focus_idx {
                                        let cur_y = state.page_elements[idx].center_y();
                                        let prev = (0..idx).rev()
                                            .find(|&i| (state.page_elements[i].center_y() - cur_y).abs() < 30.0)
                                            .unwrap_or(idx);
                                        state.focus_idx = Some(prev);
                                    }
                                    return None;
                                }
                                "Enter" => {
                                    if let Some(idx) = state.focus_idx {
                                        let el = state.page_elements.get(idx).cloned();
                                        if let Some(el) = el {
                                            // Input/text areas → enter typing mode
                                            if el.input_type.is_some() {
                                                state.mode = Mode::ElementInput { element_id: idx };
                                                state.status = format!("Typing into {}… press Esc when done", el.tag);
                                                return None;
                                            }
                                            // Otherwise click the element
                                            return handle_element_click(state, &el);
                                        }
                                    }
                                }
                                "Escape" => {
                                    state.focus_idx = None;
                                    return None;
                                }
                                _ => {}
                            }
                        }

                        // Arrow/scroll keys for text scrolling (when no elements or elements but no focus)
                        if state.page_elements.is_empty() || state.focus_idx.is_none() {
                            match text {
                                "ArrowUp" | "Up" => {
                                    state.scroll_offset = state.scroll_offset.saturating_sub(1);
                                    return None;
                                }
                                "ArrowDown" | "Down" => {
                                    let max = state.page_text.lines().count().saturating_sub(5);
                                    state.scroll_offset = (state.scroll_offset + 1).min(max);
                                    return None;
                                }
                                "PageUp" => {
                                    state.scroll_offset = state.scroll_offset.saturating_sub(state.rows as usize).saturating_sub(3);
                                    return None;
                                }
                                "PageDown" => {
                                    let max = state.page_text.lines().count().saturating_sub(5);
                                    state.scroll_offset = (state.scroll_offset + state.rows as usize).min(max);
                                    return None;
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // Pass through to WebSocket
        return Some(val.to_string());
    } else if let Some(val) = cmd.strip_prefix("dotted_scroll:") {
        let delta: i32 = val.parse().unwrap_or(0);
        let max_scroll = state.page_text.lines().count().saturating_sub(1);
        let new_offset = (state.scroll_offset as i32 + delta).max(0) as usize;
        state.scroll_offset = new_offset.min(max_scroll);
    } else if let Some(val) = cmd.strip_prefix("dotted_click:") {
        let idx: usize = val.parse().unwrap_or(0);
        if idx > 0 && idx <= state.page_links.len() {
            let (_, href) = state.page_links[idx - 1].clone();
            state.url = href.clone();
            state.loading = true;
            state.page_text.clear();
            state.page_links.clear();
            let msg = Message::new(types::NAVIGATE, to_payload(&protocol::NavigatePayload {
                url: href,
                tab_id: None,
            }));
            return Some(msg.to_json());
        }
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

fn handle_element_click(state: &mut DisplayState, el: &PageElement) -> Option<String> {
    // Click element by selector (preferred) or by center coordinates
    if let Some(selector) = &el.selector {
        state.status = format!("Clicking {}: {}", el.tag, el.text);
        let msg = Message::new(types::CLICK, serde_json::json!({
            "selector": selector,
            "x": el.rect.x + el.rect.w / 2.0,
            "y": el.rect.y + el.rect.h / 2.0,
            "tab_id": null
        }));
        return Some(msg.to_json());
    }
    None
}

fn handle_dotted_link_click(state: &mut DisplayState, n: usize) -> Option<String> {
    if n > 0 && n <= state.page_links.len() {
        let (_, href) = state.page_links[n - 1].clone();
        state.url = href.clone();
        state.loading = true;
        state.page_text.clear();
        state.page_links.clear();
        let msg = Message::new(types::NAVIGATE, to_payload(&protocol::NavigatePayload {
            url: href,
            tab_id: None,
        }));
        return Some(msg.to_json());
    }
    None
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
        types::EVALUATE_RESULT => {
            if let Some(result) = msg.payload.get("result") {
                let _ = cmd_tx.try_send(format!("evaluate_result:{}", result));
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
                                Mode::ElementInput { element_id } => {
                                    handle_element_input(&k, element_id, &mut mode, &tx).await;
                                }
                                Mode::Browser { .. } => {
                                    handle_browser_input(&k, &mut mode, &tx).await;
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
            "0" => {
                // Toggle to browser mode
                *mode = Mode::Browser { menu_idx: 0 };
                let _ = tx.send("mode:browser".to_string()).await;
                return;
            }
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

async fn handle_element_input(
    k: &input::KeyInput,
    _element_id: usize,
    mode: &mut Mode,
    tx: &mpsc::Sender<String>,
) {
    match k.key.as_str() {
        "Escape" => {
            *mode = Mode::Normal;
            let _ = tx.send("mode:normal".to_string()).await;
        }
        "Enter" => {
            // Forward Enter as TYPE newline then exit
            let msg = Message::new(types::TYPE, to_payload(&protocol::TypePayload {
                text: "\n".to_string(),
                tab_id: None,
            }));
            let _ = tx.send(format!("ws:{}", msg.to_json())).await;
        }
        "Backspace" => {
            let msg = Message::new(types::KEY_PRESS, to_payload(&protocol::KeyPressPayload {
                key: "Backspace".to_string(),
                modifiers: None,
                tab_id: None,
            }));
            let _ = tx.send(format!("ws:{}", msg.to_json())).await;
        }
        "Tab" => {
            *mode = Mode::Normal;
            let _ = tx.send("mode:normal".to_string()).await;
        }
        _ => {
            if k.key.len() == 1 && !k.is_special {
                let msg = Message::new(types::TYPE, to_payload(&protocol::TypePayload {
                    text: k.key.clone(),
                    tab_id: None,
                }));
                let _ = tx.send(format!("ws:{}", msg.to_json())).await;
            }
        }
    }
}

async fn handle_browser_input(
    k: &input::KeyInput,
    mode: &mut Mode,
    tx: &mpsc::Sender<String>,
) {
    let menu_idx = match mode {
        Mode::Browser { menu_idx } => *menu_idx,
        _ => 0,
    };
    let max_idx = BROWSER_MENU.len().saturating_sub(1);

    match k.key.as_str() {
        "0" if k.ctrl => {
            // Toggle back to page mode
            *mode = Mode::Normal;
            let _ = tx.send("mode:normal".to_string()).await;
        }
        "q" if k.ctrl => {
            // Quit browser mode
            *mode = Mode::Normal;
            let _ = tx.send("mode:normal".to_string()).await;
        }
        "Escape" => {
            *mode = Mode::Normal;
            let _ = tx.send("mode:normal".to_string()).await;
        }
        "ArrowDown" | "Down" | "Tab" | "j" => {
            let next = (menu_idx + 1).min(max_idx);
            *mode = Mode::Browser { menu_idx: next };
        }
        "ArrowUp" | "Up" | "BackTab" | "k" => {
            let prev = menu_idx.wrapping_sub(1).min(max_idx);
            *mode = Mode::Browser { menu_idx: prev };
        }
        "Enter" | " " => {
            if let Some(item) = BROWSER_MENU.get(menu_idx) {
                let action = item.id;
                match action {
                    "back" => {
                        *mode = Mode::Normal;
                        let _ = tx.send("mode:normal".to_string()).await;
                    }
                    "search" => {
                        *mode = Mode::Normal;
                        let msg = Message::new(types::NAVIGATE, to_payload(&protocol::NavigatePayload {
                            url: "chrome://settings/searchEngines".to_string(),
                            tab_id: None,
                        }));
                        let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                    }
                    "tabs" => {
                        // Just show tab info in the info panel (already visible)
                    }
                    "settings" => {
                        *mode = Mode::Normal;
                        let msg = Message::new(types::NAVIGATE, to_payload(&protocol::NavigatePayload {
                            url: "chrome://settings".to_string(),
                            tab_id: None,
                        }));
                        let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                    }
                    "bookmarks" => {
                        *mode = Mode::Normal;
                        let msg = Message::new(types::NAVIGATE, to_payload(&protocol::NavigatePayload {
                            url: "chrome://bookmarks".to_string(),
                            tab_id: None,
                        }));
                        let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                    }
                    "clear" => {
                        *mode = Mode::Normal;
                        let msg = Message::new(types::NAVIGATE, to_payload(&protocol::NavigatePayload {
                            url: "chrome://settings/clearBrowserData".to_string(),
                            tab_id: None,
                        }));
                        let _ = tx.send(format!("ws:{}", msg.to_json())).await;
                    }
                    "about" => {
                        // Stay in browser mode, info shown in panel
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
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
