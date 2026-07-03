#!/usr/bin/env node
/*
  TermWeb Browser Server
  ======================
  Main entry point — starts the HTTP + WebSocket server on 127.0.0.1:9222.

  The server:
    1. Launches a headless Chrome via PuppeteerExtra + StealthPlugin
    2. Serves WebSocket connections from the Rust TUI client on /browser
    3. Serves the AI agent REST API on /ai/* endpoints
    4. Serves health check on /health and session info on /sessions

  Usage:
    node server/index.js --url https://example.com
    node server/index.js --port 9222 --host 127.0.0.1
*/

const http = require('http');
const { URL } = require('url');
const fs = require('fs');
const path = require('path');
const { WebSocketServer } = require('ws');
const config = require('../config/default');
const { BrowserSession, createSession } = require('./browser');
const { handleAIRequest } = require('./ai-api');
const { humanClick, humanMouseMove, humanType, humanScroll } = require('./human-emulation');

// ─── CLI Arguments ─────────────────────────────────────────────────────────

const opts = {
  host: process.argv.includes('--host')
    ? process.argv[process.argv.indexOf('--host') + 1]
    : config.server.host,
  port: parseInt(
    process.argv.includes('--port')
      ? process.argv[process.argv.indexOf('--port') + 1]
      : config.server.port,
    10
  ),
  url: process.argv.includes('--url')
    ? process.argv[process.argv.indexOf('--url') + 1]
    : 'about:blank',
  width: parseInt(
    process.argv.includes('--width')
      ? process.argv[process.argv.indexOf('--width') + 1]
      : config.browser.viewport.width,
    10
  ),
  height: parseInt(
    process.argv.includes('--height')
      ? process.argv[process.argv.indexOf('--height') + 1]
      : config.browser.viewport.height,
    10
  ),
  dataDir: process.argv.includes('--data-dir')
    ? process.argv[process.argv.indexOf('--data-dir') + 1]
    : null,
  proxy: process.argv.includes('--proxy')
    ? process.argv[process.argv.indexOf('--proxy') + 1]
    : null,
  chrome: process.argv.includes('--chrome')
    ? process.argv[process.argv.indexOf('--chrome') + 1]
    : null,
  noAutoSession: process.argv.includes('--no-auto-session'),
};

// ─── WebSocket Server ─────────────────────────────────────────────────────

const server = http.createServer((req, res) => {
  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  const url = new URL(req.url, `http://${req.headers.host}`);

  // Health check
  if (url.pathname === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      status: 'ok',
      sessions: sessions.size,
      uptime: (Date.now() - startTime) / 1000,
    }));
    return;
  }

  // Session info
  if (url.pathname === '/sessions') {
    const sessionList = [];
    for (const [id, session] of sessions) {
      sessionList.push({
        sessionId: id,
        url: session.url,
        tabs: session.getTabsInfo(),
      });
    }
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(sessionList));
    return;
  }

  // AI API
  if (url.pathname.startsWith('/ai/')) {
    return handleAIRequest(req, res, () => defaultSession, (s) => { defaultSession = s; if (s) sessions.set(s.sessionId, s); else sessions.clear(); });
  }

  // 404
  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ error: 'Not found' }));
});

const wss = new WebSocketServer({
  server,
  path: config.server.wsPath,
});

// ─── State ─────────────────────────────────────────────────────────────────

const sessions = new Map();
let defaultSession = null;
const startTime = Date.now();
let wsClientCount = 0;
let sessionDestroyTimer = null;

// ─── Screenshot Streaming ─────────────────────────────────────────────────

function startScreenshotStream(ws, session, interval = config.server.screenshotInterval) {
  let sending = false;

  async function sendFrame() {
    if (ws.readyState !== ws.OPEN) { ws._screenshotTimer = null; return; }
    // Backpressure: skip if previous frame still queued
    if (ws.bufferedAmount > 65536) { scheduleNext(); return; }

    try {
      const result = await session.captureScreenshot();
      if (!result || !result.buffer) { scheduleNext(); return; }

      ws.send(JSON.stringify({
        type: 'frame',
        payload: {
          data: result.buffer.toString('base64'),
          encoding: 'jpeg',
          width: session.viewport.width,
          height: session.viewport.height,
          tabId: result.tabId,
        },
      }));
    } catch (err) { /* silently continue */ }
    scheduleNext();
  }

  function scheduleNext() {
    ws._screenshotTimer = setTimeout(sendFrame, interval);
  }

  scheduleNext();
}

function stopScreenshotStream(ws) {
  if (ws._screenshotTimer) {
    clearInterval(ws._screenshotTimer);
    ws._screenshotTimer = null;
  }
}

// ─── Message Encoding ─────────────────────────────────────────────────────

function encode(type, payload) {
  return JSON.stringify({ type, payload });
}

// ─── WebSocket Connection Handler ─────────────────────────────────────────

wss.on('connection', async (ws, req) => {
  console.log(`[+] Client connected from ${req.socket.remoteAddress}`);
  wsClientCount++;
  // Cancel pending session destroy
  if (sessionDestroyTimer) {
    clearTimeout(sessionDestroyTimer);
    sessionDestroyTimer = null;
  }

  if (!defaultSession) {
    try {
      console.log('[+] Launching browser session...');
      defaultSession = await createSession(`session_${Date.now()}`, {
        viewport: { width: opts.width, height: opts.height },
        userDataDir: opts.dataDir,
        proxyServer: opts.proxy,
      });

      await defaultSession.navigate(opts.url);

      defaultSession.onEvent = (type, payload) => {
        ws.send(encode(type, payload));
      };

      sessions.set(defaultSession.sessionId, defaultSession);
      console.log(`[+] Browser session created: ${defaultSession.sessionId}`);
    } catch (err) {
      console.error('[!] Failed to launch browser:', err.message);
      ws.send(encode('error', { message: `Browser launch failed: ${err.message}` }));
      ws.close();
      return;
    }
  }

  ws.send(encode('sessionInfo', {
    sessionId: defaultSession.sessionId,
    viewport: defaultSession.viewport,
    tabs: defaultSession.getTabsInfo(),
    url: (defaultSession.getTabsInfo()[0] || {}).url || '',
  }));

  startScreenshotStream(ws, defaultSession);

  ws.on('message', async (raw) => {
    let msg;
    try {
      msg = JSON.parse(raw.toString());
    } catch {
      ws.send(encode('error', { message: 'Invalid JSON' }));
      return;
    }

    const { type, payload } = msg;

    try {
      switch (type) {
        case 'navigate': {
          const url = payload.url || payload;
          await defaultSession.navigate(url);
          ws.send(encode('urlChanged', { url }));
          break;
        }

        case 'click': {
          const page = resolveActivePage();
          if (page) {
            if (payload.selector) {
              // Click by CSS selector (used by dotted/element mode)
              await page.evaluate((sel) => {
                const el = document.querySelector(sel);
                if (!el) throw new Error(`Element not found: ${sel}`);
                const rect = el.getBoundingClientRect();
                el.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, clientX: rect.x + rect.width / 2, clientY: rect.y + rect.height / 2 }));
                el.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true, clientX: rect.x + rect.width / 2, clientY: rect.y + rect.height / 2 }));
                el.click();
              }, payload.selector);
            } else {
              // Click by coordinates with human emulation
              const { x, y } = defaultSession._mapCoord(payload.x, payload.y);
              await humanClick(page, x, y, payload.button || 'left');
            }
          }
          break;
        }

        case 'mouseDown': {
          const page = resolveActivePage();
          if (page) {
            const { x, y } = defaultSession._mapCoord(payload.x, payload.y);
            await humanMouseMove(page, x, y);
            await page.mouse.down({ button: payload.button || 'left' });
          }
          break;
        }

        case 'mouseMove': {
          const page = resolveActivePage();
          if (page) {
            const { x, y } = defaultSession._mapCoord(payload.x, payload.y);
            await humanMouseMove(page, x, y);
          }
          break;
        }

        case 'mouseUp': {
          const page = resolveActivePage();
          if (page) {
            await page.mouse.up({ button: payload.button || 'left' });
          }
          break;
        }

        case 'scroll': {
          const page = resolveActivePage();
          if (page) {
            await humanScroll(page, payload.delta_y || 0);
          }
          break;
        }

        case 'type': {
          const page = resolveActivePage(payload.tabId);
          if (page) {
            await humanType(page, payload.text);
          }
          break;
        }

        case 'keyPress': {
          const page = resolveActivePage(payload.tabId);
          if (page) {
            await page.keyboard.press(payload.key);
          }
          break;
        }

        case 'evaluate': {
          const page = resolveActivePage(payload.tabId);
          if (page) {
            const result = await page.evaluate(payload.code);
            ws.send(encode('evaluateResult', { result }));
          }
          break;
        }

        case 'resize': {
          await defaultSession.resize(payload.width, payload.height);
          break;
        }

        case 'requestScreenshot': {
          const result = await defaultSession.captureScreenshot(payload.tabId);
          if (result && result.buffer) {
            ws.send(encode('frame', {
              data: result.buffer.toString('base64'),
              encoding: 'jpeg',
              width: result.width,
              height: result.height,
              tabId: result.tabId,
            }));
          }
          break;
        }

        case 'setProxy': {
          await defaultSession.setProxy(payload.server, payload.username, payload.password);
          ws.send(encode('proxyChanged', { server: payload.server }));
          break;
        }

        case 'goBack': {
          await defaultSession.goBack(payload.tabId);
          break;
        }

        case 'goForward': {
          await defaultSession.goForward(payload.tabId);
          break;
        }

        case 'createTab': {
          const result = await defaultSession.createTab(payload.url);
          ws.send(encode('tabList', { tabs: result.tabs }));
          break;
        }

        case 'switchTab': {
          const result = await defaultSession.switchTab(payload.tabId);
          ws.send(encode('tabList', { tabs: result.tabs }));
          break;
        }

        case 'closeTab': {
          await defaultSession.close(payload.tabId);
          ws.send(encode('tabList', { tabs: defaultSession.getTabsInfo() }));
          break;
        }

        case 'findInPage': {
          const result = await defaultSession.findInPage(payload.text, payload.tabId);
          ws.send(encode('findResults', { text: payload.text, found: result.found, count: result.count || 0 }));
          break;
        }

        case 'ping': {
          ws.send(encode('pong', { timestamp: Date.now() }));
          break;
        }

        default:
          ws.send(encode('error', { message: `Unknown command: ${type}` }));
      }
    } catch (err) {
      console.error(`[!] Command ${type} failed:`, err.message);
      ws.send(encode('error', { message: `Command ${type} failed: ${err.message}` }));
    }
  });

  ws.on('close', () => {
    console.log('[-] Client disconnected');
    stopScreenshotStream(ws);
    wsClientCount--;
    // Destroy session after 30s idle (no clients)
    if (wsClientCount <= 0 && defaultSession) {
      sessionDestroyTimer = setTimeout(async () => {
        if (wsClientCount <= 0 && defaultSession) {
          console.log('[+] No clients connected. Destroying browser session.');
          try { await defaultSession.close(); } catch {}
          defaultSession = null;
          sessions.clear();
        }
      }, 30000);
    }
  });

  ws.on('error', (err) => {
    console.error('[!] WebSocket error:', err.message);
  });
});

function resolveActivePage(tabId) {
  if (!defaultSession) return null;
  const page = defaultSession._resolvePage(tabId);
  return page || null;
}

// ─── Auto-Create Session ─────────────────────────────────────────────────

async function createAutoSession() {
  if (opts.noAutoSession) {
    console.log('[+] Auto-session creation disabled. REST API will wait for first WS client.');
    return;
  }
  try {
    console.log('[+] Creating browser session for automation...');
    if (opts.chrome) config.browser.executablePath = opts.chrome;
    defaultSession = await createSession(`session_${Date.now()}`, {
      viewport: { width: opts.width, height: opts.height },
      userDataDir: opts.dataDir,
      proxyServer: opts.proxy,
    });
    if (opts.url && opts.url !== 'about:blank') {
      await defaultSession.navigate(opts.url).catch(() => {});
    }
    sessions.set(defaultSession.sessionId, defaultSession);
    console.log(`[+] Browser session created: ${defaultSession.sessionId}`);
  } catch (err) {
    console.error('[!] Failed to create auto-session:', err.message);
    console.log('[+] Server will create session on first WS client connection.');
  }
}

// ─── Startup ──────────────────────────────────────────────────────────────

server.listen(opts.port, opts.host, async () => {
  console.log('');
  console.log('╔══════════════════════════════════════════════╗');
  console.log('║   TermWeb Browser Server                     ║');
  console.log(`║   REST API: http://${opts.host}:${opts.port}/ai/*              ║`);
  console.log(`║   WebSocket: ws://${opts.host}:${opts.port}${config.server.wsPath}  ║`);
  console.log(`║   Sessions:  http://${opts.host}:${opts.port}/sessions  ║`);
  console.log(`║   Health:    http://${opts.host}:${opts.port}/health    ║`);
  console.log('╚══════════════════════════════════════════════╝');
  console.log('');
  console.log('[+] Server is running. Automation ready at:');
  console.log(`    bai status    — check session`);
  console.log(`    bai navigate https://example.com`);
  console.log(`    bai page      — view page content`);
  console.log('');

  await createAutoSession();
});

// ─── Config Hot-Reload ────────────────────────────────────────────────────

process.on('SIGHUP', () => {
  console.log('[+] Reloading config...');
  delete require.cache[require.resolve('../config/default')];
  const newConfig = require('../config/default');
  Object.assign(config, newConfig);
  console.log(`[+] Config reloaded. screenshotInterval=${config.server.screenshotInterval}ms, jpegQuality=${config.terminal.jpegQuality}`);
});

// ─── Graceful Shutdown ────────────────────────────────────────────────────

function shutdown() {
  console.log('\n[+] Shutting down...');
  wss.close();
  for (const [, session] of sessions) {
    session.close().catch(() => {});
  }
  server.close(() => {
    process.exit(0);
  });
}

process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);
