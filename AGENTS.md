# TermWeb Browser — Agent Instructions

You are an Expert Full-Stack Developer working on **TermWeb Browser**, a terminal-based background web browser & automation tool. Read this file before **every** response to avoid hallucinations.

## Tech Stack (DO NOT DEVIATE)

| Layer | Technology | Why |
|-------|-----------|-----|
| **Server** | Node.js + `puppeteer-extra` + `puppeteer-extra-plugin-stealth` | Captcha bypass, anti-detection |
| **Server WS** | `ws` bound to `127.0.0.1` only | PRoot/Termux networking compat |
| **Client** | **Rust** (`ratatui` + `ratatui-image` + `crossterm`) | NOT Node.js. 30+ FPS, zero flicker |
| **Images** | Kitty Graphics Protocol (primary) or Sixel (fallback) | NOT ASCII art or block chars |
| **Async** | `tokio` (Rust client) | Required for WS + input + render |

## Directory Structure

```
termweb-browser/
├── AGENTS.md                ← You are here. Read this first.
├── package.json
├── bin/                      Global CLI commands
│   ├── bcli                  Shell wrapper for the Rust client
│   ├── bai                   AI agent CLI (Python, no graphics)
│   └── install.js            Symlinks bcli + termweb-server + bai into PATH
├── .skills/
│   └── bcli-web-agent/       Skill for teaching AI agents to use BCLI
│       ├── skill.json        Metadata (triggers, models)
│       └── skill.md          Full agent instructions + workflows
├── config/default.js
├── server/                   Node.js daemon (background)
│   ├── index.js              HTTP/WS server, command router
│   ├── browser.js            PuppeteerExtra session manager
│   ├── human-emulation.js    Bezier curves, human typing
│   ├── anti-fingerprint.js   Custom stealth patches
│   └── proxy.js              SOCKS5 support
├── shared/protocol.js        WS message types (JS)
├── client/                   Node.js CLIENT (legacy reference only)
│   ├── coord-translator.js   Terminal cell → pixel math
│   ├── display.js            Kitty/Sixel rendering
│   ├── input.js              Raw mode input capture
│   └── index.js              Client entry
└── client-rs/                RUST CLIENT (the real one, binary name: bcli)
    ├── Cargo.toml
    └── src/
        ├── main.rs           Async tokio + ratatui
        ├── display.rs        ratatui-image rendering
        ├── input.rs          crossterm events
        └── protocol.rs       Message types (mirrors protocol.js)
```

## Hard Rules (Never Violate)

1. **Client is Rust, not Node.js.** The `client/` directory is legacy reference only. All new client work goes in `client-rs/`. The global command is `bcli` (binary name in Cargo.toml). The user types `bcli` to launch the interactive browser.

2. **Server address is always `127.0.0.1`.** Never `0.0.0.0` or `localhost`. PRoot/Termux breaks on non-127 addresses.

3. **Browser flags must include** `--no-sandbox`, `--disable-setuid-sandbox`, `--disable-dev-shm-usage`. These are non-negotiable for Termux/PRoot.

4. **Stealth is mandatory.** Always use `puppeteer-extra-plugin-stealth` + custom anti-fingerprinting. Never suggest removing or disabling it.

5. **Mouse moves use Bezier curves.** Teleporting the mouse is NOT allowed. Every `page.mouse.move()` must go through `humanMouseMove()` or `humanClick()` from `server/human-emulation.js`.

6. **Typing has randomized delays.** Use `humanType()` not `page.type()` directly.

7. **Server keeps running when client disconnects.** Closing the client must NOT close the browser. The daemon stays alive 24/7.

8. **Flicker-free rendering.** Use ratatui's immediate-mode + ratatui-image's inline escape codes. Never clear screen between frames.

9. **PRoot compatibility.** No `/proc/net` reads, no strict network checks. Only localhost connections.

10. **Extensions can be loaded** via `--disable-extensions-except` and `--load-extension` flags in `browser.js`. Never strip this feature.

## Common Mistakes (Check These)

- [ ] Did I write Rust when the user asked for client changes? (GO TO `client-rs/`)
- [ ] Did I add a new npm dependency? (ASK the user first)
- [ ] Did I change the WS bind address from 127.0.0.1? (DON'T)
- [ ] Did I use `page.mouse.move()` directly instead of `humanMouseMove()`? (FIX IT)
- [ ] Did I forget to `.await` a tokio future? (Rust client is async)
- [ ] Did I suggest an alternative tech stack? (DON'T — the stack is fixed)
- [ ] Did I try to read `/proc/net` or similar? (PRoot blocks this)
- [ ] Is the binary name `bcli`? (`Cargo.toml` [[bin]] name must be `bcli`)
- [ ] Does `protocol.rs` mirror `shared/protocol.js`? (New types must be added to both)

## Protocol Message Types

Messages are `{ type, payload, _t }` JSON over WebSocket.

**Client → Server:** `navigate`, `click`, `mouseDown`, `mouseMove`, `mouseUp`, `scroll`, `type`, `keyPress`, `evaluate`, `resize`, `requestScreenshot`, `setProxy`

**Server → Client:** `frame` (base64 JPEG), `urlChanged`, `console`, `error`, `sessionInfo`

The Rust `protocol.rs` **must always mirror** `shared/protocol.js`. If you add a message type to one, add it to the other.

## Installation

### One-liner (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/install.sh | bash
```

Auto-installs Node.js (via nvm), Rust (via rustup), Chrome, and all commands into `~/.local/bin`.

**After install, the installer automatically detects your AI agents** (opencode, claude, gemini-cli, etc.) and installs the `bcli-web-agent` skill into each one's config directory — no manual setup needed.

### Manual

```bash
# Clone and install
git clone https://github.com/adittaya/termweb-browser.git
cd termweb-browser
npm install

# Symlink into PATH
node bin/install.js                    # ~/.local/bin/bcli
node bin/install.js /usr/local         # /usr/local/bin/bcli

# Or via npm:
npm link

# Install skill for AI agents (opencode, claude, etc.)
bash scripts/install-skill.sh
```

The shell wrapper at `bin/bcli` auto-builds the Rust client on first run.

## Usage

```bash
# Quick start: start server + connect
bcli --server --url https://example.com

# Connect to an already-running server
bcli -c ws://127.0.0.1:9222/browser

# Start just the server daemon
termweb-server --url https://example.com
# Or via npm: node server/index.js --url https://example.com

# Build Rust client manually (for development)
cd client-rs && cargo build --release
```

## BCLI Controls

| Key | Action |
|-----|--------|
| Click / drag | Interact with page |
| Scroll wheel | Scroll up/down |
| Keyboard | Type into web forms |
| Ctrl+C | Disconnect (server keeps running) |
| Ctrl+R | Request fresh screenshot |
| Ctrl+W | Show current URL |
| Ctrl+T | Toggle typing mode indicator |
| `bcli --help` | Full help text |

## If You Are Uncertain

- Ask the user before adding new dependencies (npm or Cargo)
- Ask the user before changing the protocol format
- Ask the user before restructuring files

## AI Agent Mode (No Terminal Graphics)

AI agents can interact with the browser through a **text/JSON REST API** on the same server port. No terminal, no images needed.

### REST API (port 9222)

```
GET  /ai/status      — Connection + page status
GET  /ai/page        — Full page (text + links + interactive elements)
GET  /ai/text        — Page text content only
GET  /ai/links       — All links on the page
GET  /ai/buttons     — All clickable elements with CSS selectors
GET  /ai/forms       — All form fields
GET  /ai/html        — Simplified HTML tree structure
GET  /ai/session     — Session info (active, URL, title, tabs)
POST /ai/session     — { action: "create"|"destroy"|"status" } optional { url, viewport }
POST /ai/navigate    — { url: "https://..." }
POST /ai/click       — { selector: "#button" } or { x: 100, y: 200 }
POST /ai/type        — { selector: "#input", text: "hello" }
POST /ai/scroll      — { delta_y: 300 }
POST /ai/evaluate    — { code: "document.title" }
POST /ai/screenshot  — Returns base64 JPEG
POST /ai/wait        — { ms: 1000 } or { selector: "#loaded" }
```

### `bai` CLI (for AI agents)

The `bin/bai` Python script wraps the REST API for easy command-line use:

```bash
bai status                    # Session info
bai text                      # Page text
bai page                      # Full page dump
bai links                     # All links
bai buttons                   # Interactive elements
bai forms                     # All forms
bai navigate https://x.com    # Go to URL
bai click "#login-btn"        # Click by CSS selector
bai click-xy 500 300          # Click at pixel coords
bai type "#search" "query"    # Type into element
bai scroll 500                # Scroll down
bai eval "document.title"     # Run JavaScript
bai screenshot [path]         # Save screenshot to file
bai wait 2000                 # Wait 2 seconds
bai wait-for "#loaded" 5000   # Wait for element
bai session [create|destroy]  # Manage browser session
bai run playbook.json         # Execute automation playbook
```

### Programmatic AI Integration (Python)

```python
import urllib.request, json

BASE = "http://127.0.0.1:9222"

def ai(method, path, body=None):
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(
        f"{BASE}{path}", data=data, method=method,
        headers={"Content-Type": "application/json"} if data else {}
    )
    return json.loads(urllib.request.urlopen(req).read())

# Agent loop example:
page = ai("GET", "/ai/page")
print(page["text"][:2000])           # See the page
print(page["interactives"][:5])      # See clickable elements

ai("POST", "/ai/click", {"selector": 'a[href*="signin"]'})
ai("POST", "/ai/type", {"selector": "#email", "text": "user@x.com"})
ai("POST", "/ai/click", {"selector": "#submit"})
result = ai("GET", "/ai/text")
print(result["text"])
```

### Automation Playbooks

Run a sequence of steps from a JSON file with `bai run playbook.json`:

```json
{
  "name": "Login to Example",
  "steps": [
    { "action": "navigate", "url": "https://example.com/login" },
    { "action": "wait", "selector": "#email", "ms": 5000 },
    { "action": "type", "selector": "#email", "text": "user@example.com" },
    { "action": "type", "selector": "#password", "text": "s3cret" },
    { "action": "click", "selector": "#login-btn" },
    { "action": "wait", "text": "Dashboard", "ms": 10000 },
    { "action": "screenshot" }
  ]
}
```

Steps are executed in order. Add `"optional": true` to non-critical steps to continue on error.

## Quick Reference — Key Files

| File | Purpose | ~Lines |
|------|---------|--------|
| `server/human-emulation.js` | Bezier path generation, human click/type | 328 |
| `client/coord-translator.js` | Terminal → browser pixel math | 148 |
| `server/anti-fingerprint.js` | Canvas/WebGL/audio spoofing | 132 |
| `server/ai-api.js` | AI agent REST API (no-graphics mode) | 570+ |
| `bin/bai` | AI agent CLI (Python, wraps REST API) | 300+ |
| `client-rs/src/main.rs` | Async event loop, WS, ratatui | 423 |
| `client-rs/src/display.rs` | ratatui-image frame rendering | 166 |
| `client-rs/src/input.rs` | crossterm mouse/key parsing | 187 |
| `client-rs/src/protocol.rs` | Type-safe WS message structs | 244 |
