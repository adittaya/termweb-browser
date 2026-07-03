/*
  Browser Session Manager
  =======================
  Wraps PuppeteerExtra with the stealth plugin to launch and manage
  the headless browser. Handles:
    - Launch with Termux/PRoot-safe flags
    - Tab creation, navigation, and lifecycle
    - Screenshot capture as WebP/JPEG for streaming
    - Cookie persistence between sessions
    - Per-tab proxy configuration
    - Auto-restart on crash (24/7 operation)
*/

const puppeteerExtra = require('puppeteer-extra');
const StealthPlugin = require('puppeteer-extra-plugin-stealth');
const fs = require('fs');
const path = require('path');
const config = require('../config/default');
const { applyCustomStealth } = require('./anti-fingerprint');

// Register the stealth plugin (must be done once at module level)
puppeteerExtra.use(StealthPlugin());


class BrowserSession {
  constructor(sessionId, opts = {}) {
    this.sessionId = sessionId;
    this.browser = null;
    this.pages = new Map();
    this.activeTabId = null;
    this.viewport = opts.viewport || { ...config.browser.viewport };
    this.userDataDir = opts.userDataDir || path.join(
      process.cwd(), '.browser-data', sessionId
    );
    this.proxyServer = opts.proxyServer || config.proxy.server;
    this.proxyCreds = opts.proxyCreds || {};
    this.extensions = opts.extensions || [];
    this.terminalCols = 80;
    this.terminalRows = 24;
    this._loadingTabs = new Set();

    this._keepaliveTimer = null;
    this._crashRetries = 0;
    this.maxRetries = 3;
  }


  /**
   * Store terminal dimensions for coordinate mapping.
   */
  setTerminalSize(cols, rows) {
    this.terminalCols = cols;
    this.terminalRows = rows;
  }


  /**
   * Map terminal cell coordinates to viewport pixel coordinates.
   */
  _mapCoord(col, row) {
    const px = Math.round((col / this.terminalCols) * this.viewport.width);
    const py = Math.round((row / this.terminalRows) * this.viewport.height);
    return { x: Math.min(px, this.viewport.width - 1), y: Math.min(py, this.viewport.height - 1) };
  }


  /**
   * Whether the active tab is currently loading.
   */
  isLoading() {
    return this._loadingTabs.has(this.activeTabId);
  }


  /**
   * Launch the headless browser with Termux/PRoot-safe flags
   * using puppeteer-extra (which includes the stealth plugin).
   */
  async launch() {
    const launchOpts = {
      headless: true,
      args: [...config.browser.launchArgs],
      defaultViewport: null,
      userDataDir: this.userDataDir,
      ignoreHTTPSErrors: true,
    };

    // Custom executable path (resolve relative paths)
    if (config.browser.executablePath) {
      launchOpts.executablePath = path.resolve(config.browser.executablePath);
    }

    // Proxy at browser level
    if (this.proxyServer) {
      launchOpts.args.push(`--proxy-server=${this.proxyServer}`);
    }

    // Load browser extensions
    for (const extPath of this.extensions) {
      if (fs.existsSync(extPath)) {
        launchOpts.args.push(`--disable-extensions-except=${extPath}`);
        launchOpts.args.push(`--load-extension=${extPath}`);
      }
    }

    // Launch via puppeteer-extra (stealth plugin already applied)
    let browser;
    try {
      browser = await puppeteerExtra.launch(launchOpts);
    } catch (err) {
      // Fallback to regular puppeteer with bundled Chromium
      const puppeteerFull = require('puppeteer');
      browser = await puppeteerFull.launch({
        ...launchOpts,
        executablePath: undefined,
      });
    }

    this.browser = browser;
    this._crashRetries = 0;

    // Auto-restart on crash
    this.browser.on('disconnected', () => this._handleCrash());

    // Create initial tab
    const page = await this._createPage();
    this.activeTabId = this._tabId(page);
    this.pages.set(this.activeTabId, page);

    // Start keepalive pings
    this._startKeepalive();

    // Apply additional custom stealth patches (beyond the plugin)
    await this._applyStealthToAllPages();

    return this;
  }


  /**
   * Create a new tab with proper viewport, UA, and event hooks.
   */
  async _createPage() {
    const page = await this.browser.newPage();
    await page.setViewport({ ...this.viewport });
    await page.setUserAgent(config.browser.userAgent);
    page.setDefaultNavigationTimeout(config.browser.navigationTimeout);

    // Forward console messages
    page.on('console', (msg) => {
      this._emit('console', {
        type: msg.type(),
        args: msg.args().map(a => {
          try { return a._remoteObject?.value ?? a.toString(); }
          catch { return String(a); }
        }),
        tabId: this._tabId(page),
      });
    });

    // Track URL changes
    page.on('framenavigated', (frame) => {
      if (frame === page.mainFrame()) {
        page.title().then(title => {
          this._emit('urlChanged', {
            url: frame.url(),
            title: title || '',
            tabId: this._tabId(page),
          });
        }).catch(() => {});
      }
    });

    // Track page load state
    page.on('load', () => {
      this._loadingTabs.delete(this._tabId(page));
      this._emit('loadingState', { loading: false, tabId: this._tabId(page) });
    });
    page.on('domcontentloaded', () => {
      this._loadingTabs.add(this._tabId(page));
      this._emit('loadingState', { loading: true, tabId: this._tabId(page) });
    });

    // Apply custom stealth patches on top of the stealth plugin
    await applyCustomStealth(page);

    return page;
  }


  /**
   * Navigate the active tab to a URL.
   */
  async navigate(url, tabId = null) {
    const page = this._resolvePage(tabId);
    if (!page) throw new Error('No active tab');
    await page.goto(url, {
      waitUntil: 'networkidle2',
      timeout: config.browser.navigationTimeout,
    }).catch(err => {
      this._emit('error', {
        message: `Navigation warning: ${err.message}`,
        tabId: this._tabId(page),
      });
    });
    return { url: page.url(), title: await page.title(), tabId: this._tabId(page) };
  }


  /**
   * Capture screenshot as compressed JPEG buffer (small, fast).
   */
  async captureScreenshot(tabId = null) {
    const page = this._resolvePage(tabId);
    if (!page) return null;
    try {
      const buffer = await page.screenshot({
        type: 'jpeg',
        quality: config.terminal.jpegQuality,
        fullPage: false,
        captureBeyondViewport: false,
      });
      return { buffer, tabId: this._tabId(page) };
    } catch {
      return null;
    }
  }


  /**
   * Capture screenshot as WebP (smaller, but slower to encode).
   * Used when bandwidth is limited.
   */
  async captureScreenshotWebP(tabId = null) {
    const page = this._resolvePage(tabId);
    if (!page) return null;
    try {
      // Use page.evaluate with canvas to produce WebP
      const b64 = await page.evaluate((quality) => {
        const canvas = document.createElement('canvas');
        canvas.width = document.documentElement.clientWidth;
        canvas.height = document.documentElement.clientHeight;
        const ctx = canvas.getContext('2d');
        ctx.drawWindow
          ? ctx.drawWindow(window, 0, 0, canvas.width, canvas.height, 'rgb(255,255,255)')
          : ctx.fillRect(0, 0, canvas.width, canvas.height);
        return canvas.toDataURL('image/webp', quality / 100).split(',')[1];
      }, config.terminal.jpegQuality);
      if (!b64) return null;
      return { buffer: Buffer.from(b64, 'base64'), tabId: this._tabId(page) };
    } catch {
      return null;
    }
  }


  /**
   * Execute JavaScript in the page context.
   */
  async evaluate(code, tabId = null) {
    const page = this._resolvePage(tabId);
    if (!page) throw new Error('No active tab');
    return await page.evaluate(code);
  }


  /**
   * Resize viewport to pixel dimensions, or update terminal resolution.
   * If both cols and rows are provided (≤500 each), treat as terminal cell resize.
   */
  async resize(width, height) {
    if (width <= 500 && height <= 500) {
      this.terminalCols = width;
      this.terminalRows = height;
      return;
    }
    this.viewport = { width, height, deviceScaleFactor: this.viewport.deviceScaleFactor || 1 };
    for (const page of this.pages.values()) {
      try { await page.setViewport({ ...this.viewport }); } catch {}
    }
  }


  /**
   * Save cookies to disk for 24/7 session persistence.
   */
  async saveCookies(filePath = null) {
    filePath = filePath || path.join(this.userDataDir, 'cookies.json');
    const allCookies = [];
    for (const page of this.pages.values()) {
      try { allCookies.push(...(await page.cookies())); } catch {}
    }
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, JSON.stringify(allCookies, null, 2));
  }


  /**
   * Restore cookies from disk.
   */
  async loadCookies(filePath = null) {
    filePath = filePath || path.join(this.userDataDir, 'cookies.json');
    if (!fs.existsSync(filePath)) return;
    const cookies = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
    for (const page of this.pages.values()) {
      try { await page.setCookie(...cookies); } catch {}
    }
  }


  /**
   * Get info about all open tabs.
   */
  getTabsInfo() {
    const tabs = [];
    for (const [tabId, page] of this.pages) {
      tabs.push({
        tabId,
        url: (page.url && page.url()) || '',
        title: '',
        active: tabId === this.activeTabId,
      });
    }
    return tabs;
  }


  /**
   * Close a specific tab or the whole session.
   */
  async close(tabId = null) {
    if (tabId) {
      const page = this.pages.get(tabId);
      if (page) {
        await page.close().catch(() => {});
        this.pages.delete(tabId);
      }
      if (this.activeTabId === tabId) {
        this.activeTabId = this.pages.keys().next().value || null;
      }
    } else {
      this._stopKeepalive();
      if (this.browser) {
        try { await this.saveCookies(); } catch {}
        await this.browser.close().catch(() => {});
        this.browser = null;
      }
      this.pages.clear();
    }
  }


  /**
   * Go back in browser history.
   */
  async goBack(tabId = null) {
    const page = this._resolvePage(tabId);
    if (!page) throw new Error('No active tab');
    await page.goBack({ waitUntil: 'networkidle2', timeout: config.browser.navigationTimeout }).catch(() => {});
  }


  /**
   * Go forward in browser history.
   */
  async goForward(tabId = null) {
    const page = this._resolvePage(tabId);
    if (!page) throw new Error('No active tab');
    await page.goForward({ waitUntil: 'networkidle2', timeout: config.browser.navigationTimeout }).catch(() => {});
  }


  /**
   * Switch the active tab.
   */
  async switchTab(tabId) {
    if (!this.pages.has(tabId)) throw new Error(`Tab ${tabId} not found`);
    this.activeTabId = tabId;
    return { tabId, tabs: this.getTabsInfo() };
  }


  /**
   * Create a new tab and optionally navigate to a URL.
   */
  async createTab(url = 'about:blank') {
    const page = await this._createPage();
    const tabId = this._tabId(page);
    this.pages.set(tabId, page);
    this.activeTabId = tabId;
    if (url && url !== 'about:blank') {
      await page.goto(url, { waitUntil: 'networkidle2', timeout: config.browser.navigationTimeout }).catch(() => {});
    }
    return { tabId, tabs: this.getTabsInfo() };
  }


  /**
   * Find text in the current page using window.find().
   */
  async findInPage(text, tabId = null) {
    const page = this._resolvePage(tabId);
    if (!page) throw new Error('No active tab');
    const result = await page.evaluate((searchText) => {
      const found = window.find(searchText, false, false, true, false, false);
      return { found, text: searchText };
    }, text);
    return result;
  }


  // ─── Internal ─────────────────────────────────────────────────────────


  _tabId(page) {
    return page.target()._targetId
      || `tab_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  }

  _resolvePage(tabId) {
    if (tabId && this.pages.has(tabId)) return this.pages.get(tabId);
    if (this.activeTabId && this.pages.has(this.activeTabId))
      return this.pages.get(this.activeTabId);
    return this.pages.values().next().value || null;
  }

  async _applyStealthToAllPages() {
    for (const page of this.pages.values()) {
      try { await applyCustomStealth(page); } catch {}
    }
  }

  _startKeepalive() {
    this._stopKeepalive();
    this._keepaliveTimer = setInterval(async () => {
      for (const [tid, page] of this.pages) {
        try {
          await page.evaluate(() => Date.now());
        } catch {
          this.pages.delete(tid);
          this._emit('error', { message: 'Tab crashed', tabId: tid });
        }
      }
    }, config.server.pingInterval);
  }

  _stopKeepalive() {
    if (this._keepaliveTimer) {
      clearInterval(this._keepaliveTimer);
      this._keepaliveTimer = null;
    }
  }

  async _handleCrash() {
    this._crashRetries++;
    this._emit('error', {
      message: `Browser disconnected (retry ${this._crashRetries}/${this.maxRetries})`,
    });
    if (this._crashRetries <= this.maxRetries) {
      await new Promise(r => setTimeout(r, 2000 * this._crashRetries));
      try { await this.launch(); }
      catch (err) {
        this._emit('error', { message: `Restart failed: ${err.message}` });
      }
    }
  }

  _emit(type, payload) {
    if (this.onEvent) this.onEvent(type, { ...payload, sessionId: this.sessionId });
  }
}


async function createSession(sessionId, opts = {}) {
  const session = new BrowserSession(sessionId, opts);
  await session.launch();
  return session;
}


module.exports = { BrowserSession, createSession };
