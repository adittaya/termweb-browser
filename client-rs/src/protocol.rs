/*
  Protocol — Shared Message Types (Rust)
  =======================================
  Mirrors the shared/protocol.js definitions. All WebSocket messages
  are JSON-encoded with { type, payload, _t: timestamp } envelope.

  This module provides type-safe structs for every message type
  exchanged between the Rust client and the Node.js server.
*/

use serde::{Deserialize, Serialize};

// ─── Envelope ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: serde_json::Value,
    #[serde(rename = "_t")]
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub msg_type: String,
    pub payload: serde_json::Value,
}

impl Message {
    pub fn new(msg_type: &str, payload: serde_json::Value) -> Self {
        Self {
            msg_type: msg_type.to_string(),
            payload,
            timestamp: Some(chrono_now()),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ─── Message Type Constants ─────────────────────────────────────────────────

pub mod types {
    // Client → Server
    pub const NAVIGATE: &str = "navigate";
    pub const CLICK: &str = "click";
    pub const MOUSE_DOWN: &str = "mouseDown";
    pub const MOUSE_MOVE: &str = "mouseMove";
    pub const MOUSE_UP: &str = "mouseUp";
    pub const SCROLL: &str = "scroll";
    pub const TYPE: &str = "type";
    pub const KEY_PRESS: &str = "keyPress";
    pub const EVALUATE: &str = "evaluate";
    pub const RESIZE: &str = "resize";
    pub const REQUEST_SCREENSHOT: &str = "requestScreenshot";
    pub const SET_PROXY: &str = "setProxy";
    pub const GO_BACK: &str = "goBack";
    pub const GO_FORWARD: &str = "goForward";
    pub const CREATE_TAB: &str = "createTab";
    pub const SWITCH_TAB: &str = "switchTab";
    pub const CLOSE_TAB: &str = "closeTab";
    pub const FIND_IN_PAGE: &str = "findInPage";

    // Server → Client
    pub const FRAME: &str = "frame";
    pub const URL_CHANGED: &str = "urlChanged";
    pub const CONSOLE: &str = "console";
    pub const ERROR: &str = "error";
    pub const SESSION_INFO: &str = "sessionInfo";
    pub const PONG: &str = "pong";
    pub const EVALUATE_RESULT: &str = "evaluateResult";
    pub const LOADING_STATE: &str = "loadingState";
    pub const TAB_LIST: &str = "tabList";
    pub const FIND_RESULTS: &str = "findResults";
}

// ─── Payload Structs ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigatePayload {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickPayload {
    pub x: u32,
    pub y: u32,
    #[serde(default = "default_button")]
    pub button: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

fn default_button() -> String {
    "left".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseDownPayload {
    pub x: u32,
    pub y: u32,
    #[serde(default = "default_button")]
    pub button: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMovePayload {
    pub x: u32,
    pub y: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseUpPayload {
    pub x: u32,
    pub y: u32,
    #[serde(default = "default_button")]
    pub button: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollPayload {
    #[serde(default)]
    pub delta_x: i32,
    #[serde(default)]
    pub delta_y: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePayload {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPressPayload {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<KeyModifiers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyModifiers {
    #[serde(default, skip_serializing_if = "is_false")]
    pub alt: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ctrl: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub shift: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub meta: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizePayload {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetProxyPayload {
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

// ─── Server → Client Payloads ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePayload {
    pub data: String,   // base64-encoded JPEG bytes
    pub encoding: String,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlChangedPayload {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfoPayload {
    pub session_id: String,
    pub viewport: ViewportInfo,
    pub tabs: Vec<TabInfo>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportInfo {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchTabPayload {
    pub tab_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindInPagePayload {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingStatePayload {
    pub loading: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResultsPayload {
    pub text: String,
    pub found: bool,
    pub count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}


/// Build a JSON payload for any serializable struct.
pub fn to_payload<T: Serialize>(val: &T) -> serde_json::Value {
    serde_json::to_value(val).unwrap_or(serde_json::Value::Null)
}
