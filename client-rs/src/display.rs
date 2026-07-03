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
    Browser { menu_idx: usize },
    UrlInput { cursor: usize },
    FindActive { cursor: usize },
    ElementInput { element_id: usize },
}

#[derive(Debug, Clone)]
pub struct BrowserMenuItem {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub desc: &'static str,
}

pub const BROWSER_MENU: &[BrowserMenuItem] = &[
    BrowserMenuItem { id: "search", icon: "🔍", label: "Search Engine", desc: "Configure default search engine" },
    BrowserMenuItem { id: "tabs",   icon: "📑", label: "Tabs",         desc: "Manage open tabs" },
    BrowserMenuItem { id: "settings", icon: "⚙", label: "Settings",   desc: "Browser preferences" },
    BrowserMenuItem { id: "bookmarks", icon: "⭐", label: "Bookmarks", desc: "View bookmarks" },
    BrowserMenuItem { id: "clear",  icon: "🗑",  label: "Clear Data",  desc: "Clear browsing data" },
    BrowserMenuItem { id: "about",  icon: "ℹ",  label: "About",       desc: "Version info" },
    BrowserMenuItem { id: "back",   icon: "◀",  label: "Back to Page",desc: "Return to page (Ctrl+0)" },
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TabInfo {
    pub id: String,
    pub url: String,
    pub title: String,
    pub active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ElementRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageElement {
    pub id: usize,
    pub tag: String,
    #[serde(rename = "type")]
    pub el_type: Option<String>,
    pub text: String,
    pub href: Option<String>,
    pub role: Option<String>,
    pub selector: Option<String>,
    pub rect: ElementRect,
    pub input_type: Option<String>,
}

impl PageElement {
    pub fn center_y(&self) -> f64 {
        self.rect.y + self.rect.h / 2.0
    }

    pub fn center_x(&self) -> f64 {
        self.rect.x + self.rect.w / 2.0
    }

    pub fn label(&self) -> &'static str {
        if self.tag == "a" || self.tag == "link" { "L" }
        else if self.tag == "button" || self.role.as_deref() == Some("button") { "B" }
        else if self.tag == "input" { "I" }
        else if self.tag == "textarea" { "T" }
        else if self.tag == "select" { "S" }
        else { "E" }
    }

    pub fn color(&self) -> Color {
        if self.tag == "a" || self.tag == "link" { Color::Cyan }
        else if self.tag == "button" || self.role.as_deref() == Some("button") { Color::Yellow }
        else if self.tag == "input" || self.tag == "textarea" { Color::Magenta }
        else if self.tag == "select" { Color::LightBlue }
        else { Color::Gray }
    }
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
    pub page_elements: Vec<PageElement>,
    pub focus_idx: Option<usize>,
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
            page_elements: Vec::new(),
            focus_idx: None,
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
        " ◉ "
    } else if state.url.is_empty() {
        " ◎ "
    } else {
        " ○ "
    };

    let bar_color = if is_url_mode {
        Color::Cyan
    } else if state.url.starts_with("https://") {
        Color::Green
    } else {
        Color::Yellow
    };

    let bg = if is_url_mode {
        Color::Rgb(0x0a, 0x1a, 0x2a)
    } else {
        Color::Rgb(0x0d, 0x0d, 0x0d)
    };

    let bar_style = Style::default()
        .fg(bar_color)
        .bg(bg)
        .add_modifier(Modifier::BOLD);

    let url_span = Span::styled(
        format!("{}{}", prefix, display_url),
        bar_style,
    );

    let mut right_spans = Vec::new();
    if state.loading {
        right_spans.push(Span::styled(
            " ◌ ",
            Style::default().fg(Color::Yellow).bg(bg),
        ));
    }
    if !state.tabs.is_empty() {
        let active_idx = state.tabs.iter().position(|t| t.active).unwrap_or(0);
        right_spans.push(Span::styled(
            format!(" [{}]", active_idx + 1),
            Style::default().fg(Color::Rgb(0x66, 0x66, 0x66)).bg(bg),
        ));
    }

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(0x33, 0x33, 0x33)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let right_width: u16 = right_spans.iter().map(|s| s.content.len() as u16).sum();

    let left_area = Rect { width: inner.width.saturating_sub(right_width + 1), ..inner };
    let right_area = Rect { x: inner.x + inner.width.saturating_sub(right_width), width: right_width, ..inner };

    let left_line = Line::from(url_span);
    frame.render_widget(Paragraph::new(left_line).style(Style::default().bg(bg)), left_area);

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
        Mode::Normal => "NORM",
        Mode::Browser { .. } => "BROWS",
        Mode::UrlInput { .. } => "URL",
        Mode::FindActive { .. } => "FIND",
        Mode::ElementInput { .. } => " EDIT",
    };

    let mode_color = match state.mode {
        Mode::Normal => Color::Green,
        Mode::Browser { .. } => Color::Rgb(0xff, 0x99, 0x00),
        Mode::UrlInput { .. } => Color::Cyan,
        Mode::FindActive { .. } => Color::Yellow,
        Mode::ElementInput { .. } => Color::Magenta,
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

    let focus_text = match state.mode {
        Mode::Normal => {
            if let Some(idx) = state.focus_idx {
                if let Some(el) = state.page_elements.get(idx) {
                    let tag = el.tag.to_uppercase();
                    let text = if el.text.len() > 25 { format!("{}…", &el.text[..24]) } else { el.text.clone() };
                    format!("▸{} {} ", tag, text)
                } else { String::new() }
            } else { String::new() }
        }
        Mode::Browser { menu_idx } => {
            if let Some(item) = BROWSER_MENU.get(*menu_idx) {
                format!("{} {} ", item.icon, item.label)
            } else { String::new() }
        }
        _ => String::new(),
    };

    let right_text = format!(" {}×{} ", state.cols, state.rows);

    let left_text = if !find_text.is_empty() {
        find_text
    } else if !focus_text.is_empty() {
        focus_text
    } else if !state.status.is_empty() {
        state.status.clone()
    } else {
        String::new()
    };

    let bg = Color::Rgb(0x0a, 0x0a, 0x0a);
    let dim = Color::Rgb(0x66, 0x66, 0x66);

    let mode_span = Span::styled(mode_tag, Style::default().fg(mode_color).bg(bg));
    let dim_style = Style::default().fg(dim).bg(bg);

    let full = if left_text.is_empty() {
        Line::from(vec![
            Span::styled(right_text, dim_style),
            Span::styled("│ ", dim_style),
            mode_span,
        ])
    } else {
        let avail = (area.width as usize).saturating_sub(right_text.len() + 7);
        let clipped = if left_text.len() > avail && avail > 3 {
            format!("{}…", &left_text[..avail.saturating_sub(1)])
        } else {
            left_text
        };
        Line::from(vec![
            Span::styled(clipped, dim_style),
            Span::styled(format!(" {} ", right_text), dim_style),
            Span::styled("│ ", dim_style),
            mode_span,
        ])
    };

    let block = Block::default().style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let p = Paragraph::new(Text::from(vec![full]));
    frame.render_widget(p, inner);
}

/// Render page text with interactive element focus and styled links (dotted mode)
pub fn render_dotted(frame: &mut Frame, area: Rect, state: &mut DisplayState) {
    if state.page_text.is_empty() && !state.loading {
        let msg = if state.connected {
            Span::styled(" ◇ No page loaded — press Ctrl+L to enter a URL", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(" ◇ Connecting to server...", Style::default().fg(Color::DarkGray))
        };
        let p = Paragraph::new(Line::from(msg)).alignment(Alignment::Center);
        frame.render_widget(p, area);
        return;
    }

    if state.loading && state.page_text.is_empty() {
        let p = Paragraph::new(Line::from(Span::styled(
            " ⟳ Loading...",
            Style::default().fg(Color::Yellow),
        ))).alignment(Alignment::Center);
        frame.render_widget(p, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let text_lines: Vec<&str> = state.page_text.lines().collect();
    let available = area.height as usize;
    let el_count = state.page_elements.len();

    // Reserve space for element list (compact: 1 header + up to 8 elements)
    let element_lines = if el_count > 0 { 1 + el_count.min(8) + if el_count > 8 { 1 } else { 0 } } else { 0 };
    let max_body = available.saturating_sub(element_lines);
    let end = text_lines.len().min(state.scroll_offset + max_body);

    for line_idx in state.scroll_offset..end {
        let raw = text_lines[line_idx];
        if raw.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::raw(raw)));
        }
    }

    // Scroll indicator
    if end < text_lines.len() {
        let remaining = text_lines.len() - end;
        lines.push(Line::from(Span::styled(
            format!(" ── {} more lines ──", remaining),
            Style::default().fg(Color::Rgb(0x55, 0x55, 0x55)),
        )));
    }

    // Element focus list
    if el_count > 0 {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }

        let focused = state.focus_idx;
        let max_show = el_count.min(8);
        for i in 0..max_show {
            let el = &state.page_elements[i];
            let is_focused = focused == Some(i);
            let label = el.label();
            let color = el.color();

            let marker = if is_focused { "▸" } else { " " };
            let badge = format!(" {} ", label);
            let text = if el.text.len() > 50 { format!("{}…", &el.text[..49]) } else { el.text.clone() };

            let badge_bg = if is_focused { color } else { Color::Rgb(0x22, 0x22, 0x22) };
            let badge_fg = if is_focused { Color::Black } else { Color::Rgb(0xaa, 0xaa, 0xaa) };
            let text_fg = if is_focused { Color::Rgb(0xff, 0xff, 0xff) } else { Color::Rgb(0xcc, 0xcc, 0xcc) };

            let mut spans = vec![
                Span::styled(marker, Style::default().fg(color)),
                Span::styled(badge, Style::default().fg(badge_fg).bg(badge_bg)),
                Span::styled(" ", Style::default()),
            ];

            if is_focused {
                spans.push(Span::styled(text, Style::default().fg(text_fg).bg(Color::Rgb(0x1a, 0x1a, 0x3a))));
            } else {
                spans.push(Span::styled(text, Style::default().fg(text_fg)));
            }

            if let Some(href) = &el.href {
                if is_focused || href.len() < 60 {
                    let display_href = if href.len() > 50 { format!("{}…", &href[..49]) } else { href.clone() };
                    spans.push(Span::styled(
                        format!(" → {}", display_href),
                        Style::default().fg(Color::Rgb(0x66, 0x88, 0xaa)).add_modifier(Modifier::DIM),
                    ));
                }
            }

            lines.push(Line::from(spans));
        }

        if el_count > 8 {
            let remaining = el_count - 8;
            lines.push(Line::from(Span::styled(
                format!("   … {} more elements (scroll with ↑↓)", remaining),
                Style::default().fg(Color::Rgb(0x55, 0x55, 0x55)),
            )));
        }

        // Show hint on first render
        if focused.is_none() {
            lines.push(Line::from(Span::styled(
                "   ↑↓ navigate · Enter click · Tab cycle · Esc clear",
                Style::default().fg(Color::Rgb(0x55, 0x55, 0x55)),
            )));
        }
    }

    let block = Block::default()
        .style(Style::default().bg(Color::Rgb(0x0d, 0x0d, 0x0d)))
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::Rgb(0x33, 0x33, 0x33)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let p = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(Color::Rgb(0xd0, 0xd0, 0xd0)));
    frame.render_widget(p, inner);
}

/// Render the browser settings/command panel (Ctrl+0 mode)
pub fn render_browser_mode(frame: &mut Frame, area: Rect, state: &mut DisplayState) {
    let bg = Color::Rgb(0x0d, 0x0d, 0x0d);
    let border_color = Color::Rgb(0xff, 0x99, 0x00);

    let block = Block::default()
        .title(" Browser ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    let current = match state.mode {
        Mode::Browser { menu_idx } => menu_idx,
        _ => 0,
    };

    // Build menu items
    for (i, item) in BROWSER_MENU.iter().enumerate() {
        let is_selected = i == current;
        let marker = if is_selected { "▸" } else { " " };
        let item_style = if is_selected {
            Style::default()
                .fg(Color::Rgb(0xff, 0xcc, 0x44))
                .bg(Color::Rgb(0x22, 0x18, 0x00))
        } else {
            Style::default().fg(Color::Rgb(0xcc, 0xcc, 0xcc)).bg(bg)
        };

        let icon = Span::styled(
            format!(" {}  {}", marker, item.icon),
            item_style,
        );
        let label = Span::styled(
            format!(" {} ", item.label),
            item_style.add_modifier(Modifier::BOLD),
        );
        let desc = Span::styled(
            item.desc,
            if is_selected {
                Style::default().fg(Color::Rgb(0xaa, 0x88, 0x44)).bg(Color::Rgb(0x22, 0x18, 0x00))
            } else {
                Style::default().fg(Color::Rgb(0x66, 0x66, 0x66)).bg(bg)
            },
        );
        lines.push(Line::from(vec![icon, label, desc]));
    }

    // Spacer + hint
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑↓ navigate · Enter select · Ctrl+0 toggle",
        Style::default().fg(Color::Rgb(0x55, 0x55, 0x55)),
    )));
    lines.push(Line::from(Span::styled(
        "  Ctrl+Q quit browser mode",
        Style::default().fg(Color::Rgb(0x55, 0x55, 0x55)),
    )));

    // Right side: info panel for selected item
    let info_lines = match current {
        idx if idx < BROWSER_MENU.len() => {
            match BROWSER_MENU[idx].id {
                "search" => vec![
                    Line::from(Span::styled("Configure the default search engine used", Style::default().fg(Color::DarkGray))),
                    Line::from(Span::styled("when typing in the URL bar.", Style::default().fg(Color::DarkGray))),
                    Line::from(Span::styled("", Style::default())),
                    Line::from(Span::styled("Sets: google, duckduckgo, bing, or custom", Style::default().fg(Color::DarkGray))),
                ],
                "tabs" => {
                    let mut v = vec![Line::from(Span::styled(format!("{} tab(s) open", state.tabs.len()), Style::default().fg(Color::DarkGray)))];
                    for tab in &state.tabs {
                        let active = if tab.active { " ◉" } else { " ○" };
                        let title = if tab.title.len() > 30 { format!("{}…", &tab.title[..29]) } else { tab.title.clone() };
                        v.push(Line::from(Span::styled(format!("{}{}", active, title), Style::default().fg(Color::DarkGray))));
                    }
                    v
                }
                "about" => vec![
                    Line::from(Span::styled("TermWeb Browser v1.0.0", Style::default().fg(Color::Rgb(0x88, 0xcc, 0x88)))),
                    Line::from(Span::styled("Rust client · Node.js server · headless Chrome", Style::default().fg(Color::DarkGray))),
                    Line::from(Span::styled("", Style::default())),
                    Line::from(Span::styled("https://github.com/adittaya/termweb-browser", Style::default().fg(Color::Cyan))),
                ],
                _ => vec![Line::from(Span::styled("Press Enter to open", Style::default().fg(Color::DarkGray)))],
            }
        }
        _ => vec![],
    };

    // Layout: menu left, info right (if space)
    let menu_width = 50.min(inner.width.saturating_sub(2));
    let menu_area = Rect { width: menu_width, ..inner };
    let info_area = Rect {
        x: inner.x + menu_width + 1,
        width: inner.width.saturating_sub(menu_width + 2),
        ..inner
    };

    let p = Paragraph::new(Text::from(lines));
    frame.render_widget(p, menu_area);

    if info_area.width > 20 {
        let info_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::Rgb(0x33, 0x33, 0x33)));
        let info_inner = info_block.inner(info_area);
        frame.render_widget(info_block, info_area);
        let ip = Paragraph::new(Text::from(info_lines));
        frame.render_widget(ip, info_inner);
    }
}

/// Render all UI elements
pub fn render_all(frame: &mut Frame, state: &mut DisplayState) -> Result<()> {
    let area = frame.area();
    let (url_area, img_area, status_area) = build_layout(area);

    render_url_bar(frame, url_area, state);
    match state.mode {
        Mode::Browser { .. } => {
            render_browser_mode(frame, img_area, state);
        }
        _ => {
            if state.dotted {
                render_dotted(frame, img_area, state);
            } else {
                render_image(frame, img_area, state)?;
            }
        }
    }
    render_status_bar(frame, status_area, state);

    Ok(())
}
