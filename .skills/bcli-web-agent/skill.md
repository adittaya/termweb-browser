# BCLI Web Agent Skill — Browser Control for AI Agents

You are an AI agent with access to **BCLI**, a terminal-based web browser. Use BCLI to browse the web, read page content, click elements, fill forms, and extract data — all without a GUI. This skill teaches you exactly how.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Your AI Agent (Claude Code / Codex / OpenCode / etc)   │
│                                                          │
│  bai navigate https://...   bai text   bai click "#btn"  │
│         │                       │            │            │
│         ▼                       ▼            ▼            │
│  ┌──────────────────────────────────────────────────┐    │
│  │  BCLI Server (http://127.0.0.1:9222)             │    │
│  │  ┌──────────────┐  ┌────────────────────────┐    │    │
│  │  │  AI REST API  │  │  WebSocket (bcli client)│    │    │
│  │  │  /ai/status   │  │  real-time frames       │    │    │
│  │  │  /ai/page     │  │  mouse/keyboard events   │    │    │
│  │  │  /ai/click    │  └────────────────────────┘    │    │
│  │  │  /ai/type     │                                 │    │
│  │  │  /ai/...      │  ┌────────────────────────┐    │    │
│  │  └──────┬───────┘  │  Puppeteer (headless)    │    │    │
│  │         │          │  stealth + anti-fingerprint│    │    │
│  │         └──────────►  real browser engine       │    │    │
│  │                    └────────────────────────┘    │    │
│  └──────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

**Key insight:** You (the AI) use the **REST API** (`bai` CLI or direct HTTP). Humans use the **WebSocket client** (`bcli`). Both connect to the same running server.

## Quick Start

```bash
# 1. Start the server (background daemon)
termweb-server --url https://example.com &
# or:  node server/index.js --url https://example.com &

# 2. Verify it's running
bai status

# 3. Browse and interact
bai page              # See what's on the page
bai click "#login"    # Click a button
bai type "#search" q  # Type into a field
```

## All `bai` Commands

### Page Discovery

| Command | What it returns | Use case |
|---------|----------------|----------|
| `bai status` | URL, title, viewport, connection status | Check server is alive |
| `bai text` | Clean readable text (no HTML tags) | Read article / page content |
| `bai page` | Full dump: text + all interactive elements + links | Deep analysis |
| `bai links` | All `href`s with visible link text | Find navigation targets |
| `bai buttons` | Clickable elements with CSS selectors + positions | Find what to click |
| `bai forms` | All form fields (name, type, placeholder, options) | Find form inputs |
| `bai html` | Simplified HTML tree (tag > child > text) | Understand page structure |

### Actions

| Command | Effect |
|---------|--------|
| `bai navigate https://x.com` | Go to URL (waits for page load) |
| `bai click "#submit-btn"` | Click by CSS selector (uses human Bezier movement) |
| `bai click-xy 500 300` | Click at exact pixel coordinates |
| `bai type "#email" "user@x.com"` | Type text into an element (human-like delays) |
| `bai scroll 300` | Scroll down 300px |
| `bai eval "document.title"` | Execute arbitrary JavaScript |
| `bai screenshot` | Save screenshot to `bai_screenshot_*.jpg` |
| `bai wait 2000` | Wait N milliseconds |
| `bai wait-for "#loaded" 5000` | Wait for element to appear (up to 5s) |

## AI Agent Workflows

### Workflow 1: Web Research (Read an Article)

```bash
# Step 1: Navigate
bai navigate "https://en.wikipedia.org/wiki/Artificial_intelligence"

# Step 2: Wait for page
bai wait-for "#content" 10000

# Step 3: Read content (first 100k chars of clean text)
bai text

# Step 4: Find related links
bai links

# Step 5: Follow a link
bai click 'a[href*="History"]'
bai wait 2000
bai text
```

### Workflow 2: Login to a Service

```bash
# Step 1: Go to login page
bai navigate "https://github.com/login"

# Step 2: See what fields exist
bai forms

# Step 3: Fill credentials
bai type "#login_field" "your_username"
bai type "#password" "your_password"

# Step 4: Click sign in
bai click 'input[type="submit"]'

# Step 5: Wait for redirect
bai wait-for ".dashboard" 10000
bai status
```

### Workflow 3: Data Extraction (Scrape a List)

```bash
# Navigate to target
bai navigate "https://news.ycombinator.com"

# Extract structured data via JavaScript
bai eval "JSON.stringify(Array.from(document.querySelectorAll('.athing')).map(a => ({ title: a.querySelector('.titleline a')?.textContent, url: a.querySelector('.titleline a')?.href })))"
```

### Workflow 4: Search + Click Through

```bash
bai navigate "https://google.com"
bai wait-for "input[name=q]" 5000
bai type "input[name=q]" "terminal browser AI agent"
bai click "input[type=submit]"    # or press Enter
bai wait 3000
bai text                          # See search results
bai click 'a[href*="github.com"]' # Click a result
bai wait 3000
bai text
```

### Workflow 5: Slider Captcha Solving

```bash
# Identify the slider
bai buttons   # Find the slider element's position

# Get the slider dimensions and compute target position
# Then perform a human-like drag
bai click-xy 100 450              # Click and hold slider
bai click-xy 350 450              # Drag to target
bai click-xy 350 450              # Release
```

## Programmatic Usage (Python)

When you are an AI agent embedded in a Python environment, use direct HTTP:

```python
import urllib.request, json, base64

BAI = "http://127.0.0.1:9222"

def bai(method, path, body=None):
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(
        f"{BAI}{path}", data=data, method=method,
        headers={"Content-Type": "application/json"} if data else {}
    )
    return json.loads(urllib.request.urlopen(req).read())

# Agent loop example:
def research_topic(url):
    bai("POST", "/ai/navigate", {"url": url})
    page = bai("GET", "/ai/page")
    print(f"Title: {page['title']}")
    print(f"Content ({page['textLength']} chars):")
    print(page['text'][:3000])
    print(f"\nFound {page['linkCount']} links, {page['elementCount']} interactives")
    return page

# Click by text
def click_link(text):
    page = bai("GET", "/ai/page")
    for el in page.get("interactives", []):
        if text.lower() in el["text"].lower():
            bai("POST", "/ai/click", {"selector": el["selector"]})
            return el["selector"]
    return None

# Extract table data
def extract_table(selector):
    return bai("POST", "/ai/evaluate", {"code": f"""
        JSON.stringify(Array.from(document.querySelectorAll('{selector} tr')).map(row =>
            Array.from(row.querySelectorAll('td, th')).map(cell => cell.textContent.trim())
        ))
    """})
```

## Server Management

### Starting the Server

```bash
# Simple background daemon
termweb-server --url about:blank &
# or
node /path/to/termweb-browser/server/index.js --url about:blank &
```

The server:
- Starts on `127.0.0.1:9222` (PRoot-safe)
- Launches headless Chromium with `--no-sandbox` (Termux-safe)
- Applies `puppeteer-extra-plugin-stealth` (anti-detection)
- Keeps running 24/7 even with no client connected
- Auto-restarts on crash (up to 3 retries)

### Production Daemon (tmux / screen)

```bash
tmux new-session -d -s termweb 'termweb-server --url about:blank'
# Server is now persistent. Check it:
bai status
# At any time from any terminal:
bai navigate https://example.com
bai text
```

### Health Checks

```bash
# Quick health
curl -s http://127.0.0.1:9222/health

# AI status
bai status

# Raw page text (pipe-friendly)
bai text | head -100

# Check if server is alive (exit code)
bai status > /dev/null 2>&1 && echo "ALIVE" || echo "DEAD"
```

## Safety & Best Practices

### DO:
- ✅ Start server once, reuse for many requests
- ✅ Use `bai page` for full discovery, then act
- ✅ Wait for elements with `bai wait-for` before clicking
- ✅ Check `bai status` after navigation to confirm page loaded
- ✅ Use `bai forms` before filling login/registration forms
- ✅ Handle errors gracefully (server may be starting)

### DON'T:
- ❌ Don't start multiple servers on the same port
- ❌ Don't send keystrokes faster than human speed (`bai type` handles this)
- ❌ Don't try to access `localhost` — use `127.0.0.1` (PRoot requirement)
- ❌ Don't use `page.mouse.move()` directly — always go through the AI API
- ❌ Don't close the server when your task finishes (it stays alive for other agents)

### Error Recovery

```bash
# Server not responding?
bai status
# If error: restart server
kill $(lsof -ti:9222) 2>/dev/null
termweb-server --url about:blank &
sleep 2
bai status

# Element not found?
bai page  # Re-discover the page (may have changed)

# Navigation timeout?
bai status            # Check current URL
bai wait 2000         # Give it more time
bai text              # See partial content
```

## Cookie Persistence

The server saves cookies to `.browser-data/<session>/cookies.json`. This means:
- Logins persist across AI agent sessions
- Session state is maintained 24/7
- You can navigate, close, reconnect — cookies survive

If you need a fresh session:
```bash
# Restart the server with new data directory
termweb-server --url about:blank --data-dir /tmp/fresh-session &
```

## Restriction: Anti-Bot Detection

The server applies multiple stealth layers:
1. `puppeteer-extra-plugin-stealth` — webdriver, chrome.runtime, WebGL, permissions, plugins
2. Custom canvas fingerprint noise (1px R-channel shift)
3. AudioContext fingerprint spoofing
4. Font metric variance
5. SwiftShader detection bypass

**Zero extra configuration needed.** Just use `bai navigate` and the stealth is automatic.

## Performance Tips

- **Batch reads**: Use `bai page` once instead of `bai text` + `bai links` + `bai buttons`
- **Minimize navigations**: Each `bai navigate` waits for full page load
- **Use `bai eval` for complex extraction**: It's faster than clicking through multiple pages
- **Screenshots are expensive**: Only use `bai screenshot` when you need visual confirmation
- **Keep server warm**: The server keeps pages alive; subsequent requests are faster

## Troubleshooting

| Problem | Likely Cause | Fix |
|---------|-------------|-----|
| `bai status` returns error | Server not running | Start `termweb-server &` |
| `"No active browser session"` | Server started but browser crashed | Check server logs, restart |
| Navigation hangs | Page is slow / requires auth | Increase timeout, check credentials |
| Element not found | Page changed or not loaded | Run `bai page` to rediscover |
| `bai type` doesn't work | Wrong selector or iframe | Use `bai forms` to find correct selector |
| Server port in use | Previous instance still alive | `kill $(lsof -ti:9222)` then restart |

## Quick Reference Card

```bash
# ─── DISCOVERY ──────────────────────────────
bai status                    # Is server alive? What page?
bai page                      # Everything: text + links + elements
bai text                      # Just the words
bai links                     # All clickable links
bai buttons                   # All interactive elements
bai forms                     # All form inputs
bai html                      # Simplified DOM tree

# ─── NAVIGATION ─────────────────────────────
bai navigate <url>            # Go to page
bai wait <ms>                 # Wait N ms
bai wait-for <selector> <ms>  # Wait for element

# ─── INTERACTION ─────────────────────────────
bai click <selector>          # Click by CSS selector (human-like)
bai click-xy <x> <y>          # Click at pixel coords
bai type <selector> <text>    # Type into element (human-like)
bai scroll <dy>               # Scroll vertically

# ─── ADVANCED ────────────────────────────────
bai eval <javascript>         # Run JS in page context
bai screenshot                # Save JPEG screenshot
```

## Complete Agent Example

```python
#!/usr/bin/env python3
"""Research agent: find the top 3 HN stories and save them."""

import urllib.request, json, time

BAI = "http://127.0.0.1:9222"

def api(method, path, body=None):
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(
        f"{BAI}{path}", data=data, method=method,
        headers={"Content-Type": "application/json"} if data else {}
    )
    try:
        return json.loads(urllib.request.urlopen(req, timeout=15).read())
    except Exception as e:
        return {"error": str(e)}

# 1. Navigate
api("POST", "/ai/navigate", {"url": "https://news.ycombinator.com"})
time.sleep(2)

# 2. Extract top stories
result = api("POST", "/ai/evaluate", {"code": """
    JSON.stringify(
        Array.from(document.querySelectorAll('.athing')).slice(0, 3).map(a => ({
            title: a.querySelector('.titleline a')?.textContent || '',
            url: a.querySelector('.titleline a')?.href || '',
            rank: a.querySelector('.rank')?.textContent || ''
        }))
    )
"""})

stories = json.loads(result.get("result", "[]"))
for s in stories:
    print(f"{s['rank']} {s['title']}")
    print(f"   {s['url']}")
    print()
```
