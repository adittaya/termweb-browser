/*
  Terminal Input Capture Module
  =============================
  Captures mouse and keyboard events from the terminal and translates
  them into commands for the browser server.

  Mouse:
    We enable SGR extended mouse mode which gives precise pixel coordinates
    (in terminal cell units, 1-indexed). The escape sequences are:
      \x1b[?1000h  — Enable mouse button events
      \x1b[?1002h  — Enable mouse drag events (button-event tracking)
      \x1b[?1006h  — Enable SGR extended mode (precise coords)
      \x1b[?1000l  — Disable mouse events
      \x1b[?1002l  — Disable drag events
      \x1b[?1006l  — Disable SGR mode

    Incoming mouse events look like:
      \x1b[<{button};{col};{row}{M|m}
        M = press (uppercase)
        m = release (lowercase)

    Button encoding:
      0 = left
      1 = middle
      2 = right
      3 = release (no button)
      32 = +32 for motion with button pressed
      64 = +64 for scroll wheel (65 = wheel up, 66 = wheel down)

  Keyboard:
    We read raw stdin and capture all keypress events, including
    special keys (arrows, function keys, modifiers).

  Raw Mode:
    We switch the terminal to raw mode using:
      process.stdin.setRawMode(true)
    This gives us every keystroke without waiting for Enter.
*/

const { cellToPixel, computeMapping } = require('./coord-translator');


// ─── Terminal Mode Management ───────────────────────────────────────────────


/**
 * Enable raw mode and mouse capture on the terminal.
 * Call before starting the event loop.
 */
function enableMouseAndKeyboard() {
  // Enable raw mode (capture all keystrokes immediately)
  if (process.stdin.isTTY) {
    process.stdin.setRawMode(true);
  }

  // Enable SGR extended mouse mode (gives precise 1-based coords)
  process.stdout.write('\x1b[?1000h');   // Enable mouse button events
  process.stdout.write('\x1b[?1002h');   // Enable mouse drag events
  process.stdout.write('\x1b[?1006h');   // Enable SGR extended mode
  process.stdout.write('\x1b[?1004h');   // Enable focus events
  process.stdout.write('\x1b[?25l');     // Hide cursor

  // Resume stdin (it may be paused)
  process.stdin.resume();
}


/**
 * Disable raw mode and mouse capture.
 * Call on exit to restore terminal to normal.
 */
function disableMouseAndKeyboard() {
  process.stdout.write('\x1b[?1000l');   // Disable mouse events
  process.stdout.write('\x1b[?1002l');   // Disable drag events
  process.stdout.write('\x1b[?1006l');   // Disable SGR mode
  process.stdout.write('\x1b[?1004l');   // Disable focus events
  process.stdout.write('\x1b[?25h');     // Show cursor
  process.stdout.write('\x1b[2J\x1b[H'); // Clear screen

  if (process.stdin.isTTY) {
    process.stdin.setRawMode(false);
  }
  process.stdin.pause();
}


// ─── Event Parser ───────────────────────────────────────────────────────────


/**
 * Parse a raw data buffer from stdin into a structured event object.
 * Returns null if the data doesn't match a known event pattern.
 *
 * Event types returned:
 *   { type: 'mouse', action: 'click'|'drag'|'release'|'scroll', button, col, row }
 *   { type: 'keyboard', key: string, raw: Buffer }
 *   { type: 'focus', gained: boolean }
 *   { type: 'resize', cols, rows }
 */
function parseEvent(data) {
  const str = data.toString('utf-8');

  // ─── Mouse events (SGR format) ──────────────────────────────────────────
  // \x1b[<{button};{col};{row}M  (press)     — uppercase M
  // \x1b[<{button};{col};{row}m  (release)   — lowercase m
  const mouseMatch = str.match(/^\x1b\[<(\d+);(\d+);(\d+)([Mm])$/);
  if (mouseMatch) {
    const buttonCode = parseInt(mouseMatch[1], 10);
    const col = parseInt(mouseMatch[2], 10);
    const row = parseInt(mouseMatch[3], 10);
    const isPress = mouseMatch[4] === 'M';

    // Decode button
    let button;
    let isScroll = false;
    let scrollDir = 0;

    if (buttonCode >= 64) {
      // Scroll events (64 + direction)
      isScroll = true;
      scrollDir = buttonCode === 65 ? -1 : 1;  // 65 = up, 66 = down
      button = null;
    } else if (buttonCode >= 32) {
      // Motion with button pressed (drag)
      const baseButton = buttonCode - 32;
      button = baseButton === 0 ? 'left' : baseButton === 1 ? 'middle' : baseButton === 2 ? 'right' : 'left';
    } else {
      button = buttonCode === 0 ? 'left' : buttonCode === 1 ? 'middle' : buttonCode === 2 ? 'right' : 'left';
    }

    if (isScroll) {
      return { type: 'mouse', action: 'scroll', deltaY: scrollDir * 3, col, row };
    }

    if (!isPress) {
      return { type: 'mouse', action: 'release', button, col, row };
    }

    if (buttonCode >= 32) {
      return { type: 'mouse', action: 'drag', button, col, row };
    }

    return { type: 'mouse', action: 'click', button, col, row };
  }

  // ─── Focus events ──────────────────────────────────────────────────────
  // \x1b[[I  (focused)   or  \x1b[[O  (unfocused)
  if (str === '\x1b[[I') {
    return { type: 'focus', gained: true };
  }
  if (str === '\x1b[[O') {
    return { type: 'focus', gained: false };
  }

  // ─── Terminal resize ───────────────────────────────────────────────────
  // Note: SIGWINCH is handled separately via a signal handler.
  // We don't get resize events via stdin.

  // ─── Keyboard events ───────────────────────────────────────────────────
  // Return the raw data as a keyboard event. We handle most cases.
  return parseKeyboard(data);
}


/**
 * Parse raw keyboard data into a structured key event.
 *
 * @param {Buffer} data
 * @returns {Object}  { type: 'keyboard', key: string, name?: string, modifiers?: {...}, raw: Buffer }
 */
function parseKeyboard(data) {
  const str = data.toString('utf-8');
  const keyEvent = { type: 'keyboard', raw: data, key: str, modifiers: {} };

  // ─── Escape sequences for special keys ─────────────────────────────────
  if (str.startsWith('\x1b')) {
    keyEvent.isEscape = true;

    // Arrow keys
    if (str === '\x1b[A') { keyEvent.key = 'ArrowUp'; keyEvent.name = 'up'; }
    else if (str === '\x1b[B') { keyEvent.key = 'ArrowDown'; keyEvent.name = 'down'; }
    else if (str === '\x1b[C') { keyEvent.key = 'ArrowRight'; keyEvent.name = 'right'; }
    else if (str === '\x1b[D') { keyEvent.key = 'ArrowLeft'; keyEvent.name = 'left'; }

    // Shift + arrows
    else if (str === '\x1b[1;2A') { keyEvent.key = 'ArrowUp'; keyEvent.modifiers.shift = true; }
    else if (str === '\x1b[1;2B') { keyEvent.key = 'ArrowDown'; keyEvent.modifiers.shift = true; }
    else if (str === '\x1b[1;2C') { keyEvent.key = 'ArrowRight'; keyEvent.modifiers.shift = true; }
    else if (str === '\x1b[1;2D') { keyEvent.key = 'ArrowLeft'; keyEvent.modifiers.shift = true; }

    // Ctrl + arrows
    else if (str === '\x1b[1;5A') { keyEvent.key = 'ArrowUp'; keyEvent.modifiers.ctrl = true; }
    else if (str === '\x1b[1;5B') { keyEvent.key = 'ArrowDown'; keyEvent.modifiers.ctrl = true; }
    else if (str === '\x1b[1;5C') { keyEvent.key = 'ArrowRight'; keyEvent.modifiers.ctrl = true; }
    else if (str === '\x1b[1;5D') { keyEvent.key = 'ArrowLeft'; keyEvent.modifiers.ctrl = true; }

    // Home / End / PageUp / PageDown
    else if (str === '\x1b[H') { keyEvent.key = 'Home'; keyEvent.name = 'home'; }
    else if (str === '\x1b[F') { keyEvent.key = 'End'; keyEvent.name = 'end'; }
    else if (str === '\x1b[5~') { keyEvent.key = 'PageUp'; keyEvent.name = 'pageup'; }
    else if (str === '\x1b[6~') { keyEvent.key = 'PageDown'; keyEvent.name = 'pagedown'; }

    // Delete / Insert
    else if (str === '\x1b[3~') { keyEvent.key = 'Delete'; keyEvent.name = 'delete'; }
    else if (str === '\x1b[2~') { keyEvent.key = 'Insert'; keyEvent.name = 'insert'; }

    // Function keys F1-F12
    else if (str === '\x1b[11~' || str === '\x1bOP') { keyEvent.key = 'F1'; keyEvent.name = 'f1'; }
    else if (str === '\x1b[12~' || str === '\x1bOQ') { keyEvent.key = 'F2'; keyEvent.name = 'f2'; }
    else if (str === '\x1b[13~' || str === '\x1bOR') { keyEvent.key = 'F3'; keyEvent.name = 'f3'; }
    else if (str === '\x1b[14~' || str === '\x1bOS') { keyEvent.key = 'F4'; keyEvent.name = 'f4'; }
    else if (str === '\x1b[15~') { keyEvent.key = 'F5'; keyEvent.name = 'f5'; }
    else if (str === '\x1b[17~') { keyEvent.key = 'F6'; keyEvent.name = 'f6'; }
    else if (str === '\x1b[18~') { keyEvent.key = 'F7'; keyEvent.name = 'f7'; }
    else if (str === '\x1b[19~') { keyEvent.key = 'F8'; keyEvent.name = 'f8'; }
    else if (str === '\x1b[20~') { keyEvent.key = 'F9'; keyEvent.name = 'f9'; }
    else if (str === '\x1b[21~') { keyEvent.key = 'F10'; keyEvent.name = 'f10'; }
    else if (str === '\x1b[23~') { keyEvent.key = 'F11'; keyEvent.name = 'f11'; }
    else if (str === '\x1b[24~') { keyEvent.key = 'F12'; keyEvent.name = 'f12'; }
  }

  // ─── Ctrl-key combinations ─────────────────────────────────────────────
  // Ctrl+A = 0x01, Ctrl+B = 0x02, ..., Ctrl+Z = 0x1A
  if (data.length === 1) {
    const byte = data[0];
    if (byte >= 1 && byte <= 26) {
      keyEvent.key = String.fromCharCode(96 + byte); // a-z
      keyEvent.modifiers.ctrl = true;
    } else if (byte === 127) {
      keyEvent.key = 'Backspace';
      keyEvent.name = 'backspace';
    } else if (byte === 13) {
      keyEvent.key = 'Enter';
      keyEvent.name = 'enter';
    } else if (byte === 9) {
      keyEvent.key = 'Tab';
      keyEvent.name = 'tab';
    } else if (byte === 27) {
      keyEvent.key = 'Escape';
      keyEvent.name = 'escape';
    }
  }

  // Alt+key detection: \x1b followed by a character
  if (str.length === 2 && str[0] === '\x1b' && str[1] >= ' ') {
    keyEvent.key = str[1];
    keyEvent.modifiers.alt = true;
  }

  return keyEvent;
}


// ─── Standard Key Names for Puppeteer ───────────────────────────────────────

const KEY_MAP = {
  'enter': 'Enter',
  'tab': 'Tab',
  'backspace': 'Backspace',
  'escape': 'Escape',
  'delete': 'Delete',
  'home': 'Home',
  'end': 'End',
  'pageup': 'PageUp',
  'pagedown': 'PageDown',
  'up': 'ArrowUp',
  'down': 'ArrowDown',
  'left': 'ArrowLeft',
  'right': 'ArrowRight',
  'f1': 'F1', 'f2': 'F2', 'f3': 'F3', 'f4': 'F4',
  'f5': 'F5', 'f6': 'F6', 'f7': 'F7', 'f8': 'F8',
  'f9': 'F9', 'f10': 'F10', 'f11': 'F11', 'f12': 'F12',
};


module.exports = {
  enableMouseAndKeyboard,
  disableMouseAndKeyboard,
  parseEvent,
  parseKeyboard,
  KEY_MAP,
};
