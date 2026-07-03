/*
  Shared Protocol Module
  ----------------------
  Defines all WebSocket message types shared between server and client.
  Messages are JSON-encoded with a fixed { type, payload } envelope.
*/

const MessageTypes = {
  // ─── Client → Server ──────────────────────────────────────────────────────

  // Navigate to a URL
  NAVIGATE: 'navigate',
  // { url: string, tabId?: string }

  // Mouse click (x, y are viewport pixel coordinates)
  CLICK: 'click',
  // { x: number, y: number, button: 'left'|'right'|'middle', tabId?: string }

  // Mouse down (start of drag)
  MOUSE_DOWN: 'mouseDown',
  // { x: number, y: number, button: 'left'|'right'|'middle', tabId?: string }

  // Mouse move (during drag or hover)
  MOUSE_MOVE: 'mouseMove',
  // { x: number, y: number, tabId?: string }

  // Mouse up (end of drag)
  MOUSE_UP: 'mouseUp',
  // { x: number, y: number, button: 'left'|'right'|'middle', tabId?: string }

  // Scroll by delta pixels
  SCROLL: 'scroll',
  // { deltaX: number, deltaY: number, tabId?: string }

  // Type text into the currently focused element
  TYPE: 'type',
  // { text: string, tabId?: string }

  // Press a single key
  KEY_PRESS: 'keyPress',
  // { key: string, modifiers?: { alt?: boolean, ctrl?: boolean, shift?: boolean, meta?: boolean }, tabId?: string }

  // Execute arbitrary JavaScript in the page
  EVALUATE: 'evaluate',
  // { code: string, tabId?: string }

  // Resize the browser viewport
  RESIZE: 'resize',
  // { width: number, height: number }

  // Take a fresh screenshot immediately
  REQUEST_SCREENSHOT: 'requestScreenshot',

  // Set proxy for current session
  SET_PROXY: 'setProxy',
  // { server: string, username?: string, password?: string }

  // Go back in history
  GO_BACK: 'goBack',

  // Go forward in history
  GO_FORWARD: 'goForward',

  // Create a new tab
  CREATE_TAB: 'createTab',
  // { url?: string }

  // Switch to a specific tab
  SWITCH_TAB: 'switchTab',
  // { tabId: string }

  // Close a tab
  CLOSE_TAB: 'closeTab',
  // { tabId: string }

  // Find text in page
  FIND_IN_PAGE: 'findInPage',
  // { text: string, tabId?: string }

  // ─── Server → Client ──────────────────────────────────────────────────────

  // Screen frame data (base64-encoded JPEG or PNG)
  FRAME: 'frame',
  // { data: string (base64), encoding: 'jpeg'|'png', width: number, height: number, tabId: string }

  // Current page URL changed
  URL_CHANGED: 'urlChanged',
  // { url: string, title: string, tabId: string }

  // Console messages from the page
  CONSOLE: 'console',
  // { type: string, args: string[], tabId: string }

  // Navigation error
  ERROR: 'error',
  // { message: string, tabId?: string }

  // Session info (sent on connect)
  SESSION_INFO: 'sessionInfo',
  // { sessionId: string, viewport: { width, height }, tabs: TabInfo[] }

  // Keepalive pong
  PONG: 'pong',

  // Page loading state changed
  LOADING_STATE: 'loadingState',
  // { loading: boolean, tabId?: string }

  // Updated tab list
  TAB_LIST: 'tabList',
  // { tabs: TabInfo[] }

  // Find in page results
  FIND_RESULTS: 'findResults',
  // { text: string, found: boolean, count: number, tabId?: string }
};


function encode(type, payload = {}) {
  return JSON.stringify({ type, payload, _t: Date.now() });
}


function decode(raw) {
  try {
    const msg = JSON.parse(raw);
    if (!msg.type || !MessageTypes[msg.type.toUpperCase()]) {
      return { type: 'unknown', payload: msg };
    }
    return msg;
  } catch {
    return { type: 'error', payload: { message: 'Invalid message format' } };
  }
}


module.exports = { MessageTypes, encode, decode };
