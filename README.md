# TermWeb Browser

**Terminal-based web browser & automation tool** — browse, scrape, and automate the web entirely from your terminal. No display, no GUI, no X server needed.

Run `bcli` for an interactive terminal browser with mouse, keyboard, and 30+ FPS rendering. Or use `bai` for AI-driven automation via REST API.

---

## Quick Start

```bash
# One-line install (auto: Node.js, Rust, Chrome, all commands)
curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/install.sh | bash

# Start the server and open a URL
termweb-server --url https://example.com

# Interactive browser (Rust + ratatui)
bcli -c ws://127.0.0.1:9222/browser

# AI agent automation (no graphics needed)
bai page
bai click "a[href*='signin']"
bai type "#email" "user@example.com"
```

---

## Features

- **Interactive terminal browser** — `bcli` with mouse support, scrolling, keyboard input, Kitty/Sixel image rendering
- **AI agent REST API** — `bai` CLI and HTTP endpoints for text-based automation (no display needed)
- **Stealth / anti-detection** — Bezier mouse curves, human typing delays, canvas/WebGL/audio fingerprint spoofing, `puppeteer-extra-plugin-stealth`
- **Automation playbooks** — JSON-based multi-step workflows with `bai run playbook.json`
- **Session management** — create, destroy, and switch browser sessions via API
- **Auto AI skill install** — detects opencode, Claude Code, Gemini CLI, aider, cursor, codex and installs the browser skill automatically

---

## Commands

| Command | Description |
|---------|-------------|
| `bcli` | Interactive terminal browser (Rust, ratatui) |
| `termweb-server` | HTTP/WS server daemon (Node.js + Puppeteer) |
| `bai` | AI agent CLI (Python, text-only REST API wrapper) |
| `termweb` | Launcher (starts server + connects client) |

### `bai` automation commands

```
bai status         bai page          bai text          bai links
bai buttons        bai forms         bai html          bai session
bai navigate       bai click         bai click-xy      bai type
bai scroll         bai eval          bai screenshot    bai wait
bai wait-for       bai run
```

---

## Documentation

| File | What it covers |
|------|----------------|
| [`AUTOMATION.md`](AUTOMATION.md) | Full automation guide — playbooks, REST API, waiting strategies, AI agent patterns |
| [`AGENTS.md`](AGENTS.md) | Developer reference — architecture, protocol, hard rules, common mistakes |
| `.skills/bcli-web-agent/SKILL.md` | Auto-installed AI agent skill for opencode, Claude Code, etc. |

---

## Architecture

```
┌─────────────┐     REST/WS     ┌──────────────────┐     Puppeteer     ┌─────────┐
│  bcli       │ ◄─────────────► │  termweb-server  │ ◄──────────────► │ Chrome  │
│  (Rust)     │    WebSocket    │  (Node.js)       │                  │ (hidden)│
├─────────────┤                 │  AI REST API     │                  └─────────┘
│  bai        │ ◄─────────────► │  /ai/* endpoints │
│  (Python)   │    HTTP/JSON    └──────────────────┘
├─────────────┤
│  AI Agent   │ ◄── auto skill install ──
│  (opencode  │
│   claude…)  │
└─────────────┘
```

- **Server:** Node.js + Puppeteer Extra + Stealth Plugin (binds `127.0.0.1` only)
- **Interactive client:** Rust + ratatui + crossterm + ratatui-image (Kitty/Sixel)
- **AI client:** Python script (`bai`) wrapping the REST API
- **Browser:** Headless Chrome with anti-fingerprinting

---

## Install Options

### One-liner (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/install.sh | bash
```

Auto-installs Node.js, Rust, Chrome, and all commands. Detects your AI agents and installs the browser skill.

### From source

```bash
git clone https://github.com/adittaya/termweb-browser.git
cd termweb-browser
npm install
node bin/install.js
```

### Verify

```bash
bai status
# → Status: Connected, URL: https://example.com
```

---

## Requirements

- **Linux, macOS, or Windows** (via WSL/Git Bash)
- **Node.js 18+** and **npm** (auto-installed by one-liner)
- **Rust** (auto-installed by one-liner)

---

## License

MIT
