use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    UrlInput { cursor: usize },
    FindActive { cursor: usize },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TabInfo {
    pub id: String,
    pub url: String,
    pub title: String,
    pub active: bool,
}

pub struct DisplayState {
    pub image: Option<Arc<DynamicImage>>,
    pub picker: Option<Picker>,
    pub image_state: Option<Box<dyn StatefulProtocol>>,
    pub url: String,
    pub url_buffer: String,
    pub find_buffer: String,
    pub status: String,
    pub error_log: Vec<String>,
    pub cols: u16,
    pub rows: u16,
    pub connected: bool,
    pub loading: bool,
    pub mode: Mode,
    pub tabs: Vec<TabInfo>,
    pub dotted: bool,
    pub page_text: String,
    pub page_links: Vec<(String, String)>,
    pub scroll_offset: usize,
}

impl DisplayState {
    pub fn push_error(&mut self, msg: &str) {
        self.error_log.push(msg.to_string());
        if self.error_log.len() > 10 {
            self.error_log.remove(0);
        }
    }
}

impl DisplayState {
    pub fn new(cols: u16, rows: u16, dotted: bool) -> Self {
        let picker = if dotted {
            None
        } else {
            let mut p = Picker::new((8, 12));
            let _ = p.guess_protocol();
            Some(p)
        };
        Self {
            image: None,
            picker,
            image_state: None,
            url: String::new(),
            url_buffer: String::new(),
            find_buffer: String::new(),
            status: String::new(),
            error_log: Vec::new(),
            cols,
            rows,
            connected: false,
            loading: false,
            mode: Mode::Normal,
            tabs: Vec::new(),
            dotted,
            page_text: String::new(),
            page_links: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn set_image_from_jpeg(&mut self, jpeg_bytes: &[u8]) -> Result<()> {
        let img = image::load_from_memory_with_format(jpeg_bytes, ImageFormat::Jpeg)
            .context("Failed to decode JPEG frame")?;
        self.image = Some(Arc::new(img));
        self.image_state = None;
        Ok(())
    }
}

/// Build layout: [url_bar], [image_area], [status_bar]
pub fn build_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

/// Render URL bar styled like a browser address bar
pub fn render_url_bar(frame: &mut Frame, area: Rect, state: &DisplayState) {
    let is_url_mode = matches!(state.mode, Mode::UrlInput { .. });

    let display_url = match &state.mode {
        Mode::UrlInput { cursor } => {
            let mut s = state.url_buffer.clone();
            let pos = (*cursor).min(s.len());
            s.insert(pos, '█');
            s
        }
        _ => {
            if state.url.is_empty() {
                "about:blank".to_string()
            } else {
                state.url.clone()
            }
        }
    };

    let prefix = if state.url.starts_with("https://") {
        " 🔒 "
    } else {
        "  "  // or " ○ " for non-HTTPS
    };

    let bar_color = if is_url_mode {
        Color::Cyan
    } else {
        Color::Green
    };

    let bar_style = Style::default()
        .fg(bar_color)
        .add_modifier(Modifier::BOLD);

    // Build the address bar text
    let url_span = Span::styled(
        format!("{}{}", prefix, display_url),
        bar_style,
    );

    // Right side: tab indicator + loading
    let mut right_spans = Vec::new();
    if state.loading {
        right_spans.push(Span::styled(
            " ◌",
            Style::default().fg(Color::Yellow),
        ));
    }
    if !state.tabs.is_empty() {
        let active_idx = state.tabs.iter().position(|t| t.active).unwrap_or(0);
        right_spans.push(Span::styled(
            format!(" [{}]", active_idx + 1),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Use a block to frame the address bar like Chrome omnibox
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Left: URL with icon + right: tab/loading info
    let right_width: u16 = right_spans.iter().map(|s| s.content.len() as u16).sum();

    let left_area = Rect { width: inner.width.saturating_sub(right_width + 1), ..inner };
    let right_area = Rect { x: inner.x + inner.width.saturating_sub(right_width), width: right_width, ..inner };

    let left_line = Line::from(url_span);
    frame.render_widget(Paragraph::new(left_line).style(Style::default().fg(bar_color)), left_area);

    if !right_spans.is_empty() {
        let right_line = Line::from(right_spans);
        frame.render_widget(Paragraph::new(right_line), right_area);
    }
}

/// Render the page screenshot as an image
pub fn render_image(frame: &mut Frame, area: Rect, state: &mut DisplayState) -> Result<()> {
    let Some(img) = state.image.as_ref().cloned() else {
        let msg = if state.loading {
            "Loading..."
        } else if state.connected {
            "No page loaded — navigate to a URL (Ctrl+L)"
        } else {
            "Connecting to server..."
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(p, area);
        return Ok(());
    };

    if state.image_state.is_none() {
        if let Some(ref mut picker) = state.picker {
            state.image_state = Some(picker.new_resize_protocol((*img).clone()));
        }
    }

    if let Some(ref mut image_state) = state.image_state {
        let widget = StatefulImage::new(None);
        frame.render_stateful_widget(widget, area, image_state);
    }

    Ok(())
}

/// Render minimal status bar: status text on left, mode tag on right
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &DisplayState) {
    let mode_tag = match state.mode {
        Mode::Normal => "NORMAL",
        Mode::UrlInput { .. } => "URL",
        Mode::FindActive { .. } => "FIND",
    };

    let find_text = match &state.mode {
        Mode::FindActive { cursor } => {
            let mut s = state.find_buffer.clone();
            let pos = (*cursor).min(s.len());
            s.insert(pos, '█');
            format!("Find: {}", s)
        }
        _ => String::new(),
    };

    let right_text = format!(" {}×{}  [{}]", state.cols, state.rows, mode_tag);

    let left_text = if !find_text.is_empty() {
        find_text
    } else if !state.status.is_empty() {
        state.status.clone()
    } else {
        String::new()
    };

    let min_right = right_text.len() + 1;
    let full = if left_text.is_empty() {
        right_text
    } else if area.width as usize <= min_right {
        right_text
    } else if area.width as usize <= min_right + 3 {
        format!("{}", right_text)
    } else {
        let avail = (area.width as usize).saturating_sub(min_right);
        if left_text.len() > avail {
            format!("{}… {}", &left_text[..avail.saturating_sub(1)], right_text)
        } else {
            format!("{} {}", left_text, right_text)
        }
    };

    let style = Style::default().fg(Color::DarkGray);
    let p = Paragraph::new(Text::styled(full, style));
    frame.render_widget(p, area);
}

/// Render page as text with numbered links (dotted mode)
pub fn render_dotted(frame: &mut Frame, area: Rect, state: &mut DisplayState) {
    if state.page_text.is_empty() && !state.loading {
        let msg = if state.connected {
            "No page loaded — press Ctrl+L to enter a URL"
        } else {
            "Connecting to server..."
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(p, area);
        return;
    }

    if state.loading && state.page_text.is_empty() {
        let p = Paragraph::new("Loading...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        frame.render_widget(p, area);
        return;
    }

    // Build text lines with numbered links
    let mut lines: Vec<Line> = Vec::new();
    let text_lines: Vec<&str> = state.page_text.lines().collect();
    let available = area.height as usize;

    for line_idx in state.scroll_offset..text_lines.len().min(state.scroll_offset + available.saturating_sub(2)) {
        let line = text_lines[line_idx];
        if line.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::raw(line)));
        }
    }

    // Link legend at bottom
    if !state.page_links.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("─── Links (press number to navigate) ─── {} total ───", state.page_links.len()),
            Style::default().fg(Color::DarkGray),
        )));
        let max_show = state.page_links.len().min(20);
        for i in 0..max_show {
            let (text, href) = &state.page_links[i];
            let label = format!("[{}] {} → {}", i + 1, text, href);
            lines.push(Line::from(Span::styled(
                label,
                Style::default().fg(Color::Cyan),
            )));
        }
        if state.page_links.len() > 20 {
            lines.push(Line::from(Span::styled(
                format!("... and {} more links", state.page_links.len() - 20),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let p = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(Color::White));
    frame.render_widget(p, area);
}

/// Render all UI elements
pub fn render_all(frame: &mut Frame, state: &mut DisplayState) -> Result<()> {
    let area = frame.area();
    let (url_area, img_area, status_area) = build_layout(area);

    render_url_bar(frame, url_area, state);
    if state.dotted {
        render_dotted(frame, img_area, state);
    } else {
        render_image(frame, img_area, state)?;
    }
    render_status_bar(frame, status_area, state);

    Ok(())
}
