#!/usr/bin/env node

/*
  Terminal Client (The Viewer)
  ============================
  Connects to the background browser daemon, receives screenshot frames,
  renders them in the terminal, and forwards mouse/keyboard events back.

  Usage:
    node client/index.js --connect ws://127.0.0.1:9222/browser

  Controls (while connected):
    Ctrl+C  — Disconnect and exit
    Ctrl+R  — Reload / request fresh screenshot
    Ctrl+T  — Toggle input mode (browse vs type)
    Ctrl+W  — Show current URL
*/

const WebSocket = require('ws');
const { program } = require('commander');
const config = require('../config/default');
const { MessageTypes, encode, decode } = require('../shared/protocol');
const { renderFrame, clearScreen, resetTerminal } = require('./display');
const { computeMapping, cellToPixel } = require('./coord-translator');
const {
  enableMouseAndKeyboard,
  disableMouseAndKeyboard,
  parseEvent,
  KEY_MAP,
} = require('./input');


// ─── CLI ────────────────────────────────────────────────────────────────────

program
  .name('termweb-client')
  .description('Terminal Browser Viewer — connect to a running TermWeb server')
  .requiredOption('-c, --connect <url>', 'WebSocket server URL (e.g., ws://127.0.0.1:9222/browser)')
  .option('-W, --width <number>', 'Viewport width override')
  .option('-H, --height <number>', 'Viewport height override')
  .parse(process.argv);

const opts = program.opts();


// ─── State ──────────────────────────────────────────────────────────────────

let ws = null;
let reconnectTimer = null;
let isConnected = false;
let termSize = { cols: 80, rows: 24 };
let viewport = { width: 1280, height: 720 };
let currentUrl = '';
let isTypingMode = false;
let dragState = null;  // { button, startCol, startRow }


// ─── Terminal Resize Handler ────────────────────────────────────────────────

function getTerminalSize() {
  try {
    const { stdout } = require('child_process').execSync('stty size', { encoding: 'utf-8' });
    const [rows, cols] = stdout.trim().split(' ').map(Number);
    return { cols, rows };
  } catch {
    return { cols: 80, rows: 24 };
  }
}


function handleResize() {
  termSize = getTerminalSize();
  // Resize the browser viewport to match
  if (ws && isConnected) {
    const newWidth = Math.round(termSize.cols * 10);  // ~10px per cell
    const newHeight = Math.round(termSize.rows * 20);  // ~20px per cell
    ws.send(encode(MessageTypes.RESIZE, {
      width: Math.min(newWidth, 1920),
      height: Math.min(newHeight, 1080),
    }));
  }
}


// ─── Event Handlers ─────────────────────────────────────────────────────────

function handleMouseEvent(event) {
  if (!ws || !isConnected) return;

  // Compute the mapping from terminal coords to browser pixels
  const mapping = computeMapping(termSize, viewport);

  switch (event.action) {
    case 'click': {
      const { x, y } = cellToPixel(event.col, event.row, mapping, viewport);
      ws.send(encode(MessageTypes.CLICK, {
        x, y, button: event.button,
      }));
      break;
    }

    case 'drag': {
      // Start of drag or drag motion
      if (!dragState) {
        dragState = {
          button: event.button,
          startCol: event.col,
          startRow: event.row,
        };
        const { x, y } = cellToPixel(event.col, event.row, mapping, viewport);
        ws.send(encode(MessageTypes.MOUSE_DOWN, {
          x, y, button: event.button,
        }));
      } else {
        const { x, y } = cellToPixel(event.col, event.row, mapping, viewport);
        ws.send(encode(MessageTypes.MOUSE_MOVE, { x, y }));
      }
      break;
    }

    case 'release': {
      if (dragState) {
        const { x, y } = cellToPixel(event.col, event.row, mapping, viewport);
        ws.send(encode(MessageTypes.MOUSE_UP, {
          x, y, button: event.button || dragState.button,
        }));
        dragState = null;
      }
      break;
    }

    case 'scroll': {
      // Send scroll event — we use a fixed scroll amount
      ws.send(encode(MessageTypes.SCROLL, {
        deltaX: 0,
        deltaY: event.deltaY * 50,  // Scale up scroll amount
      }));
      break;
    }
  }
}


function handleKeyboardEvent(event) {
  if (!ws || !isConnected) return;

  // ─── Inline commands ──────────────────────────────────────────────────
  if (event.modifiers.ctrl && event.key) {
    switch (event.key) {
      case 'c':  // Ctrl+C — exit
        cleanup();
        process.exit(0);
        return;

      case 'r':  // Ctrl+R — request fresh screenshot
        ws.send(encode(MessageTypes.REQUEST_SCREENSHOT));
        return;

      case 't':  // Ctrl+T — toggle typing mode
        isTypingMode = !isTypingMode;
        showStatusLine(`Typing mode: ${isTypingMode ? 'ON' : 'OFF'}`);
        return;

      case 'w':  // Ctrl+W — show URL
        showStatusLine(`URL: ${currentUrl}`);
        return;

      case 'l':  // Ctrl+L — clear console
        clearScreen();
        return;
    }
  }

  // ─── Forward keyboard to browser ──────────────────────────────────────

  if (event.name && KEY_MAP[event.name]) {
    // Special key
    ws.send(encode(MessageTypes.KEY_PRESS, {
      key: KEY_MAP[event.name],
      modifiers: event.modifiers,
    }));
  } else if (event.modifiers.ctrl || event.modifiers.alt || event.modifiers.meta) {
    // Modified keys
    ws.send(encode(MessageTypes.KEY_PRESS, {
      key: event.key,
      modifiers: event.modifiers,
    }));
  } else if (event.key.length === 1 && !event.isEscape) {
    // Regular printable character — use TYPE for realistic typing
    ws.send(encode(MessageTypes.TYPE, { text: event.key }));
  } else if (event.key === 'Enter') {
    ws.send(encode(MessageTypes.KEY_PRESS, { key: 'Enter' }));
  } else if (event.key === 'Backspace') {
    ws.send(encode(MessageTypes.KEY_PRESS, { key: 'Backspace' }));
  } else if (event.key === 'Tab') {
    ws.send(encode(MessageTypes.KEY_PRESS, { key: 'Tab' }));
  } else if (event.key === 'Escape') {
    ws.send(encode(MessageTypes.KEY_PRESS, { key: 'Escape' }));
  } else if (event.name) {
    // Fallback for other special keys
    const mappedKey = KEY_MAP[event.name];
    if (mappedKey) {
      ws.send(encode(MessageTypes.KEY_PRESS, { key: mappedKey }));
    }
  }
}


// ─── Status Line ────────────────────────────────────────────────────────────

let statusTimeout = null;

function showStatusLine(msg) {
  // Write a status message at the bottom of the terminal
  const { rows } = termSize;
  process.stdout.write(`\x1b[${rows};1H\x1b[K\x1b[7m ${msg} \x1b[0m`);

  if (statusTimeout) clearTimeout(statusTimeout);
  statusTimeout = setTimeout(() => {
    process.stdout.write(`\x1b[${rows};1H\x1b[K`);
    statusTimeout = null;
  }, 3000);

  // Move cursor back to typing area
  process.stdout.write(`\x1b[${rows - 1};1H`);
}


// ─── WebSocket Connection ──────────────────────────────────────────────────

function connect(url) {
  if (ws) {
    ws.close();
    ws = null;
  }

  console.log(`[+] Connecting to ${url}...`);

  ws = new WebSocket(url);

  ws.on('open', () => {
    isConnected = true;
    console.log('[+] Connected! Starting session...\n');

    // Enable terminal capture
    enableMouseAndKeyboard();

    // Set up stdin data handler
    process.stdin.on('data', handleStdinData);

    // Handle terminal resize
    process.on('SIGWINCH', handleResize);
  });

  ws.on('message', (raw) => {
    const msg = decode(raw.toString());

    switch (msg.type) {
      case MessageTypes.FRAME: {
        const { data, encoding, width, height } = msg.payload;
        if (data) {
          viewport = { width, height };
          const buffer = Buffer.from(data, 'base64');
          renderFrame(buffer, width, height, termSize);
        }
        break;
      }

      case MessageTypes.SESSION_INFO: {
        if (msg.payload.viewport) {
          viewport = msg.payload.viewport;
        }
        if (msg.payload.tabs && msg.payload.tabs.length > 0) {
          currentUrl = msg.payload.tabs[0].url || '';
        }
        if (msg.payload.url) {
          currentUrl = msg.payload.url;
        }
        break;
      }

      case MessageTypes.URL_CHANGED: {
        currentUrl = msg.payload.url || currentUrl;
        break;
      }

      case MessageTypes.ERROR: {
        const errMsg = msg.payload.message || 'Unknown error';
        showStatusLine(`ERROR: ${errMsg}`);
        break;
      }

      case 'evaluateResult': {
        // Display eval results somehow
        break;
      }
    }
  });

  ws.on('close', () => {
    console.log('\n[-] Disconnected from server');
    isConnected = false;
    disableMouseAndKeyboard();
    cleanup();

    // Auto-reconnect
    console.log('[+] Reconnecting in 2 seconds...');
    reconnectTimer = setTimeout(() => connect(url), 2000);
  });

  ws.on('error', (err) => {
    console.error(`[!] WebSocket error: ${err.message}`);
    isConnected = false;
  });
}


// ─── Stdin Data Handler ────────────────────────────────────────────────────

function handleStdinData(data) {
  const event = parseEvent(data);

  if (!event) return;

  switch (event.type) {
    case 'mouse':
      handleMouseEvent(event);
      break;

    case 'keyboard':
      handleKeyboardEvent(event);
      break;

    case 'focus':
      // Could be used to pause/resume screenshot streaming
      break;
  }
}


// ─── Cleanup ────────────────────────────────────────────────────────────────

function cleanup() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  disableMouseAndKeyboard();
  process.stdin.removeAllListeners('data');
  process.removeAllListeners('SIGWINCH');
  resetTerminal();
}


// ─── Boot ───────────────────────────────────────────────────────────────────

process.on('exit', cleanup);
process.on('SIGINT', () => { cleanup(); process.exit(0); });
process.on('SIGTERM', () => { cleanup(); process.exit(0); });
process.on('uncaughtException', (err) => {
  console.error('Uncaught:', err.message);
  cleanup();
  process.exit(1);
});

// Handle unhandled rejections but don't crash
process.on('unhandledRejection', (err) => {
  console.error('Unhandled rejection:', err.message);
});

// Get initial terminal size
termSize = getTerminalSize();

// Connect
connect(opts.connect);
