use crate::protocol;
use crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers as CrosstermModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};

#[derive(Debug, Clone)]
pub enum InputEvent {
    Mouse(MouseInput),
    Key(KeyInput),
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Clone)]
pub struct MouseInput {
    pub action: MouseAction,
    pub button: MouseButtonType,
    pub col: u16,
    pub row: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseAction {
    Click,
    Drag,
    Release,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseButtonType {
    Left,
    Middle,
    Right,
    None,
}

#[derive(Debug, Clone)]
pub struct KeyInput {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub is_special: bool,
}

pub fn parse_mouse(event: &MouseEvent) -> Option<InputEvent> {
    let (col, row) = (event.column, event.row);
    let kind = &event.kind;

    let (action, button) = match kind {
        MouseEventKind::Down(btn) => {
            let b = match_crossterm_button(*btn);
            (MouseAction::Click, b)
        }
        MouseEventKind::Up(btn) => {
            let b = match_crossterm_button(*btn);
            (MouseAction::Release, b)
        }
        MouseEventKind::Drag(btn) => {
            let b = match_crossterm_button(*btn);
            (MouseAction::Drag, b)
        }
        MouseEventKind::Moved => {
            (MouseAction::Drag, MouseButtonType::None)
        }
        MouseEventKind::ScrollDown => {
            (MouseAction::ScrollDown, MouseButtonType::None)
        }
        MouseEventKind::ScrollUp => {
            (MouseAction::ScrollUp, MouseButtonType::None)
        }
        MouseEventKind::ScrollLeft => {
            (MouseAction::ScrollLeft, MouseButtonType::None)
        }
        MouseEventKind::ScrollRight => {
            (MouseAction::ScrollRight, MouseButtonType::None)
        }
    };

    Some(InputEvent::Mouse(MouseInput {
        action,
        button,
        col,
        row,
    }))
}

fn match_crossterm_button(btn: MouseButton) -> MouseButtonType {
    match btn {
        MouseButton::Left => MouseButtonType::Left,
        MouseButton::Middle => MouseButtonType::Middle,
        MouseButton::Right => MouseButtonType::Right,
    }
}

pub fn parse_key(event: &KeyEvent) -> InputEvent {
    let ctrl = event.modifiers.contains(CrosstermModifiers::CONTROL);
    let alt = event.modifiers.contains(CrosstermModifiers::ALT);
    let shift = event.modifiers.contains(CrosstermModifiers::SHIFT);

    let (key, is_special) = match event.code {
        KeyCode::Char(c) => {
            if ctrl {
                (c.to_ascii_lowercase().to_string(), false)
            } else if alt {
                (c.to_string(), false)
            } else {
                (c.to_string(), false)
            }
        }
        KeyCode::Enter => ("Enter".to_string(), true),
        KeyCode::Tab => ("Tab".to_string(), true),
        KeyCode::Backspace => ("Backspace".to_string(), true),
        KeyCode::Esc => ("Escape".to_string(), true),
        KeyCode::Delete => ("Delete".to_string(), true),
        KeyCode::Home => ("Home".to_string(), true),
        KeyCode::End => ("End".to_string(), true),
        KeyCode::PageUp => ("PageUp".to_string(), true),
        KeyCode::PageDown => ("PageDown".to_string(), true),
        KeyCode::Up => ("ArrowUp".to_string(), true),
        KeyCode::Down => ("ArrowDown".to_string(), true),
        KeyCode::Left => ("ArrowLeft".to_string(), true),
        KeyCode::Right => ("ArrowRight".to_string(), true),
        KeyCode::F(n) => (format!("F{n}"), true),
        KeyCode::Insert => ("Insert".to_string(), true),
        KeyCode::Null => ("Unidentified".to_string(), true),
        _ => ("Unidentified".to_string(), true),
    };

    InputEvent::Key(KeyInput {
        key,
        ctrl,
        alt,
        shift,
        is_special,
    })
}

pub fn key_to_modifiers(key: &KeyInput) -> protocol::KeyModifiers {
    protocol::KeyModifiers {
        alt: key.alt,
        ctrl: key.ctrl,
        shift: key.shift,
        meta: false,
    }
}

impl std::fmt::Display for MouseButtonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MouseButtonType::Left => write!(f, "left"),
            MouseButtonType::Middle => write!(f, "middle"),
            MouseButtonType::Right => write!(f, "right"),
            MouseButtonType::None => write!(f, "left"),
        }
    }
}
