/*
  Default Configuration
  ---------------------
  All settings are centralized here. Override any value via CLI flags or env vars.
  Designed to work out-of-the-box on Termux/PRoot, standard Linux, macOS, and Windows.
*/

const config = {
  // ─── Server ────────────────────────────────────────────────────────────────
  server: {
    host: '127.0.0.1',
    port: 9222,
    // WebSocket path for client connections
    wsPath: '/browser',

    // How often (ms) the server captures and sends screenshots to clients
    screenshotInterval: 250,

    // Maximum number of concurrent browser sessions
    maxSessions: 5,

    // How often (ms) to send keepalive pings to connected clients
    pingInterval: 30000,
  },

  // ─── Browser (Puppeteer) ──────────────────────────────────────────────────
  browser: {
    headless: true,

    // Launch args required for restricted environments (Termux/PRoot/Docker)
    launchArgs: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
      '--disable-gpu',
      '--disable-extensions',
      '--disable-background-networking',
      '--disable-sync',
      '--no-first-run',
      '--disable-blink-features=AutomationControlled',
      '--disable-automation',
      '--window-size=1280,720',
    ],

    // Default viewport for pages
    viewport: {
      width: 1280,
      height: 720,
      deviceScaleFactor: 1,     // Keep at 1 for performance; increase for HiDPI
    },

    // Default user agent (spoofed to look like a real Chrome browser)
    userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',

    // Path to Chromium executable; null = search CHROME_PATH env, then PATH for google-chrome/chromium-browser
    executablePath: process.env.CHROME_PATH || process.env.CHROME_BIN || null,

    // Timeout for page navigation (ms)
    navigationTimeout: 30000,
  },

  // ─── Human Emulation ──────────────────────────────────────────────────────
  human: {
    // Mouse movement: number of interpolation steps along the Bezier path
    mouseSteps: { min: 25, max: 50 },

    // Mouse movement: total duration range (ms)
    mouseDuration: { min: 200, max: 600 },

    // How far (as fraction of distance) control points can deviate from the line
    bezierControlPointJitter: 0.3,

    // Typing: delay between keystrokes (ms)
    typingDelay: { min: 40, max: 120 },

    // Click: delay between mousedown and mouseup (ms)
    clickHoldDelay: { min: 80, max: 150 },
  },

  // ─── Terminal Display ─────────────────────────────────────────────────────
  terminal: {
    // Preferred image protocol: 'kitty', 'sixel', or 'auto'
    imageProtocol: 'auto',

    // Max width/height for displayed images (in terminal cells)
    displayWidth: 80,
    displayHeight: 40,

    // JPEG quality for compressed image transfer (1-100)
    jpegQuality: 55,

    // Background color for padding areas (hex)
    bgColor: '#1a1a2e',
  },

  // ─── Anti-Fingerprinting ──────────────────────────────────────────────────
  stealth: {
    // Whether to apply stealth patches automatically
    enabled: true,

    // Spoof WebGL renderer string
    webGLVendor: 'Intel Inc.',
    webGLRenderer: 'Intel Iris OpenGL Engine',

    // Override navigator properties
    navigatorOverrides: {
      platform: 'Win32',
      hardwareConcurrency: 8,
      deviceMemory: 8,
      languages: ['en-US', 'en'],
    },
  },

  // ─── Proxy ─────────────────────────────────────────────────────────────────
  proxy: {
    // Default proxy (null = no proxy). Format: 'socks5://127.0.0.1:9050'
    server: null,
    // Optional username/password for authenticated proxies
    username: null,
    password: null,
  },
};

module.exports = config;
