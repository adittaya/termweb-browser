# TermWeb Browser — Automation Guide

A practical guide to building reliable browser automations with TermWeb's REST API and `bai` CLI. No display, no GUI — just JSON in, JSON out.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Core Concepts](#core-concepts)
3. [The `bai` CLI](#the-bai-cli)
4. [REST API Reference](#rest-api-reference)
5. [Automation Playbooks](#automation-playbooks)
6. [Waiting Strategies](#waiting-strategies)
7. [Selectors Guide](#selectors-guide)
8. [Error Handling & Retries](#error-handling--retries)
9. [Human Emulation](#human-emulation)
10. [AI Agent Integration](#ai-agent-integration)
11. [Full Examples](#full-examples)
12. [Troubleshooting](#troubleshooting)

---

## Quick Start

### One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/install.sh | bash
```

Installs everything: Node.js, Rust, Chrome, the `termweb`/`bcli`/`bai`/`termweb-server` commands, **and automatically detects your AI agents** (opencode, claude, gemini-cli, etc.) to install the browser automation skill into each one.

### Try it out

```bash
# 1. Start the server (auto-creates a browser session)
termweb-server --url https://example.com

# 2. Check it's working
bai status

# 3. See what's on the page
bai page

# 4. Do something
bai click "a[href*='signin']"
bai type "#email" "user@example.com"
bai click "#submit"
```

That's it. Every command returns JSON. No screen, no mouse, no display needed.

---

## Core Concepts

### Session

A **session** wraps a Puppeteer browser instance (one tab). The server auto-creates one on startup. You can manage it manually:

```bash
bai session              # check status
bai session create       # fresh browser (solves cookie/stale problems)
bai session destroy      # close current session
```

The session persists until destroyed, even if your script crashes. This is intentional — you can inspect state between runs.

### Page

Each session has one active page. Navigation replaces the page content. The page object gives you access to the DOM, JavaScript execution, and screenshots.

### Stateless vs Stateful

TermWeb is **stateful** — the session retains cookies, localStorage, and scroll position between commands. This is the foundation of multi-step automation. If you need a clean slate, create a new session.

---

## The `bai` CLI

The `bai` command wraps every REST endpoint. It's the fastest way to interact with the browser.

### Read Commands (no side effects)

```bash
bai status          # session info, URL, title, tabs
bai text            # visible text content
bai page            # text + interactives + links (full dump)
bai links           # all links on the page
bai buttons         # clickable elements with CSS selectors + positions
bai forms           # all form fields with names and types
bai html            # simplified HTML tree
bai session         # session active status
```

### Write Commands (modify state)

```bash
bai navigate <url>              # go to URL (waits for load)
bai click <selector>            # click element by CSS selector
bai click-xy <x> <y>            # click at pixel coordinates
bai type <selector> <text>      # type into an element
bai scroll <delta_y>            # scroll down (negative = up)
bai eval "<code>"               # run arbitrary JavaScript
bai screenshot [path]           # save screenshot to file
bai wait <ms>                   # wait N milliseconds
bai wait-for <selector> <ms>    # wait for element to appear
bai session create              # create new browser session
bai session destroy             # close current session
bai run playbook.json           # execute automation playbook
```

### Mixing Commands in Scripts

```bash
# Shell pipeline example
bai text | grep "Sign in" && bai click "a[href*='signin']"

# Capture navigation result
bai navigate "https://example.com" > /tmp/result.json
```

---

## REST API Reference

All endpoints live at `http://127.0.0.1:9222/ai/*`.

### GET endpoints (read state)

| Endpoint | Returns |
|----------|---------|
| `/ai/status` | Connection status, URL, title, viewport, sessionId, tabs |
| `/ai/page` | Full page text, interactive elements (tag, text, selector, rect), links |
| `/ai/text` | Visible text content (stripped of scripts/styles) |
| `/ai/links` | All `{text, href}` pairs |
| `/ai/buttons` | All interactive elements with selectors, positions, attributes |
| `/ai/forms` | All forms with fields, types, placeholders, options |
| `/ai/html` | Simplified nested HTML structure |
| `/ai/session` | Session active status, URL, title, viewport, tabs |

### POST endpoints (modify state)

| Endpoint | Body | Effect |
|----------|------|--------|
| `/ai/navigate` | `{"url": "..."}` | Navigate to URL |
| `/ai/click` | `{"selector": "..."}` or `{"x": n, "y": n}` | Click element or position |
| `/ai/type` | `{"selector": "...", "text": "..."}` | Type text into field |
| `/ai/scroll` | `{"delta_y": 300}` | Scroll vertically |
| `/ai/evaluate` | `{"code": "document.title"}` | Execute JS, return result |
| `/ai/screenshot` | `{}` | Return base64 JPEG screenshot |
| `/ai/wait` | `{"ms": 1000}` or `{"selector": "..."}` or `{"text": "..."}` | Wait for time, element, or text |
| `/ai/session` | `{"action": "create"\|"destroy"\|"status"}` | Manage session lifecycle |

### Calling from any language

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

# First: create a session
ai("POST", "/ai/session", {"action": "create"})

# Then automate
ai("POST", "/ai/navigate", {"url": "https://github.com/login"})
ai("POST", "/ai/type", {"selector": "#login_field", "text": "username"})
ai("POST", "/ai/type", {"selector": "#password", "text": "token"})
ai("POST", "/ai/click", {"selector": "input[type='submit']"})
result = ai("GET", "/ai/text")
print(result["text"][:500])
```

---

## Automation Playbooks

Playbooks are JSON files that describe a sequence of automation steps. They're the quickest way to codify a workflow.

### Format

```json
{
  "name": "My Automation",
  "steps": [
    { "action": "navigate", "url": "https://..." },
    { "action": "wait", "selector": "#target", "ms": 5000 },
    { "action": "type", "selector": "#input", "text": "hello" },
    { "action": "click", "selector": "#submit" }
  ]
}
```

### Running a playbook

```bash
bai run login.json
```

Each step is executed in order. The playbook runner prints progress:

```
Running playbook: Login to Example (5 steps)

  [1/5] navigate {"url": "https://example.com/login"}
  [2/5] wait {"selector": "#email", "ms": 5000}
  [3/5] type {"selector": "#email", "text": "user@example.com"}
  ...
```

### Supported step actions

Every REST API action is available as a playbook step: `navigate`, `click`, `type`, `scroll`, `evaluate`, `screenshot`, `wait`, `session`.

### Optional steps

Add `"optional": true` to continue on error:

```json
{ "action": "click", "selector": ".maybe-present", "optional": true }
```

### Real-world example: login + scrape

```json
{
  "name": "GitHub Stars Scraper",
  "steps": [
    { "action": "session", "action": "create" },
    { "action": "navigate", "url": "https://github.com/login" },
    { "action": "wait", "selector": "#login_field", "ms": 5000 },
    { "action": "type", "selector": "#login_field", "text": "myuser" },
    { "action": "type", "selector": "#password", "text": "mytoken" },
    { "action": "click", "selector": "input[type='submit']" },
    { "action": "wait", "text": "Dashboard", "ms": 10000 },
    { "action": "navigate", "url": "https://github.com/anomalyco/opencode" },
    { "action": "wait", "selector": ".star-count", "ms": 5000 },
    { "action": "evaluate", "code": "document.querySelector('.star-count').textContent" },
    { "action": "screenshot" }
  ]
}
```

---

## Waiting Strategies

The #1 cause of brittle automations is **race conditions** — acting before the page is ready. TermWeb gives you three ways to wait:

### 1. Wait for element (`selector`)

```bash
bai wait-for "#my-button" 5000
```

Polls the DOM until the element exists. Best for SPAs where content appears dynamically. Fails after timeout.

### 2. Wait for text (`text`)

```bash
bai wait 3000
```

Wait, then check if `"Welcome"` appears in `document.body.textContent`.

### 3. Wait for time (`ms`)

```bash
bai wait 2000
```

Simple fixed delay. Use as a fallback when you can't find a reliable selector.

### Choosing the right strategy

| Situation | Strategy |
|-----------|----------|
| Page navigation | `navigate` already waits for `load` event |
| Element appears after click | `wait-for <selector>` |
| SPA route change | `wait-for <text>` or `wait-for <selector>` |
| Animation completes | `wait 500` (short timeout, then check) |
| API response renders data | `wait-for <text> "data-loaded"` |

### In playbooks

```json
{ "action": "wait", "selector": ".loading-spinner", "ms": 3000, "optional": true },
{ "action": "wait", "text": "Welcome", "ms": 5000 }
```

---

## Selectors Guide

### Best practices

| Selector | Reliability | Notes |
|----------|-------------|-------|
| `#id` | High | Unique, stable |
| `[data-testid="..."]` | High | Designed for testing, stable |
| `a[href*="partial"]` | Medium | Good for links |
| `button:contains("text")` | Medium | Text can change |
| `.class > div:nth-child(3)` | Low | Fragile to layout changes |
| `xpath` | Medium | Verbose but powerful |

### Finding selectors

```bash
# See all interactive elements with their selectors
bai buttons

# Focused grep
bai buttons | grep -i "sign in"

# Get all links
bai links | grep "profile"
```

### Coordinate-based clicking

When selectors are impractical (canvas, maps, custom widgets):

```bash
# Get element positions from page dump
bai page | grep "my-element"
# Note x, y coordinates
bai click-xy 450 320
```

---

## Error Handling & Retries

### Automatic retries

The `bai` CLI retries on connection errors (3 attempts, 1s apart). No configuration needed.

### Manual retry pattern

```bash
# Retry loop in shell
for i in 1 2 3; do
  bai click "#submit" && break
  bai wait 1000
done
```

### Defensive playbook design

```json
{
  "name": "Resilient Scraper",
  "steps": [
    { "action": "navigate", "url": "https://site.com" },
    { "action": "wait", "selector": ".content", "ms": 5000 },
    { "action": "click", "selector": ".cookie-accept", "optional": true },
    { "action": "click", "selector": ".popup-close", "optional": true },
    { "action": "evaluate", "code": "document.querySelector('main').textContent" }
  ]
}
```

Common failure points and mitigations:

| Failure | Mitigation |
|---------|------------|
| Cookie/GDPR banner | Add optional click at start |
| Element not found | Use `wait-for` before interaction |
| Navigation timeout | Check network, increase timeout |
| Session crash | Create fresh session before critical flows |
| Stale element | Wait and retry |

---

## Human Emulation

TermWeb uses stealth techniques to avoid bot detection. These apply automatically — you don't need to configure anything.

### What's built in

- **Bezier mouse paths** — Cursor doesn't teleport. Every click follows a human-like curved path with realistic speed variance.
- **Typing delays** — Characters are typed with jittered inter-key delays (30-120ms), not instantly.
- **Browser fingerprinting protection** — Canvas/WebGL/audio fingerprinting is spoofed. Navigator properties are patched.
- **Stealth plugin** — `puppeteer-extra-plugin-stealth` is always active.

### When you need extra realism

```bash
# Add realistic delays between actions
bai type "#search" "query"
bai wait 400           # human pause before clicking
bai click "#search-btn"
```

### Anti-detection checklist

- [x] Headless mode uses `--headless=new` (not old headless)
- [x] WebGL vendor/renderer spoofed
- [x] Canvas fingerprint randomized
- [x] Navigator.plugins populated
- [x] Chrome runtime flags hidden
- [x] WebDriver flag removed

---

## AI Agent Integration

TermWeb's REST API is designed for LLM consumption. AI agents can read the page, decide what to do, and execute commands — all through text.

### Auto-detection of AI agents

The installer (`scripts/install-skill.sh`) automatically detects which AI coding agents you have installed and installs the `bcli-web-agent` skill into each one:

| AI Agent | Config location | Skill format |
|----------|----------------|--------------|
| **opencode** | `~/.config/opencode/skills/bcli-web-agent/SKILL.md` | Native SKILL.md with YAML frontmatter |
| **Claude Code** | `~/.claude/skills/bcli-web-agent/SKILL.md` | Compatible SKILL.md |
| **Gemini CLI** | `~/.gemini/skills/bcli-web-agent/SKILL.md` | SKILL.md format |
| **Aider** | `~/.aider/bcli-web-agent.md` | Conventions file |
| **Cursor** | `~/.cursor/rules/bcli-web-agent.mdc` | Cursor rules format |
| **Codex CLI** | `~/.codex/bcli-web-agent.md` | SKILL.md format |

You can also run the skill installer manually at any time:

```bash
bash scripts/install-skill.sh            # auto-detect all agents
bash scripts/install-skill.sh opencode   # install for specific agent only
```

Once installed, your AI agent can automatically discover and load the browser skill without any configuration.

### Pattern: Observe → Think → Act

```python
import urllib.request, json

BASE = "http://127.0.0.1:9222"

def api(method, path, body=None):
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(
        f"{BASE}{path}", data=data, method=method,
        headers={"Content-Type": "application/json"} if data else {}
    )
    return json.loads(urllib.request.urlopen(req).read())

# Step 1: Observe
page = api("GET", "/ai/page")
print(f"Page: {page['title']}")
print(f"Text: {page['text'][:2000]}")
print(f"Links: {[l['text'] for l in page['links'][:10]]}")
print(f"Interactives: {[(e['text'], e['selector']) for e in page['interactives'][:10]]}")

# Step 2: Decide (this is where your LLM logic goes)
target_url = "https://example.com"

# Step 3: Act
api("POST", "/ai/navigate", {"url": target_url})

# Step 4: Verify
new_page = api("GET", "/ai/text")
print(new_page["text"][:500])
```

### Minimal agent loop

```python
import urllib.request, json, time

BASE = "http://127.0.0.1:9222"
api = lambda m, p, b=None: json.loads(
    urllib.request.urlopen(
        urllib.request.Request(f"{BASE}{p}",
            data=json.dumps(b).encode() if b else None,
            method=m,
            headers={"Content-Type": "application/json"} if b else {})
    ).read()
)

def agent_loop(goal, max_steps=10):
    """Simple agent that observes and acts."""
    api("POST", "/ai/session", {"action": "create"})

    for step in range(max_steps):
        # Observe
        page = api("GET", "/ai/page")

        # Check if goal is met
        if goal in page["text"]:
            print(f"Goal reached at step {step}")
            return api("POST", "/ai/screenshot")

        # Decide on action
        # (In practice, you'd send page.text to an LLM here)
        interactives = page.get("interactives", [])
        if interactives:
            # Click first interactive element as example
            el = interactives[0]
            api("POST", "/ai/click", {"selector": el["selector"]})
            time.sleep(1)

    print("Max steps reached")
    return None
```

### Skill for AI coding agents

For AI coding agents (like Claude Code, Copilot, etc.) that need to control a browser, use the built-in skill:

```
.skills/bcli-web-agent/skill.md
```

This skill teaches AI agents how to use the `bai` CLI and REST API correctly. Load it when the AI needs to perform web tasks.

---

## Full Examples

### Example 1: Search and scrape

```json
{
  "name": "Search and Scrape",
  "steps": [
    { "action": "navigate", "url": "https://duckduckgo.com" },
    { "action": "wait", "selector": "#search_form_input_homepage", "ms": 5000 },
    { "action": "type", "selector": "#search_form_input_homepage", "text": "termweb browser" },
    { "action": "click", "selector": "#search_button_homepage" },
    { "action": "wait", "selector": ".result", "ms": 5000 },
    { "action": "evaluate", "code": "Array.from(document.querySelectorAll('.result__title a')).map(a => ({title: a.textContent, href: a.href}))" },
    { "action": "screenshot" }
  ]
}
```

### Example 2: Multi-page form fill

```bash
#!/bin/bash
# Form fill automation using shell

bai session create

bai navigate "https://example.com/register"
bai wait-for "#name" 5000

bai type "#name" "Jane Doe"
bai type "#email" "jane@example.com"
bai type "#password" "s3cret!"
bai click "#terms"
bai click "#submit"

bai wait-for ".confirmation" 5000
bai screenshot "confirmation.png"

echo "Registration complete"
bai session destroy
```

### Example 3: Resilient scraper (Python)

```python
#!/usr/bin/env python3
import urllib.request, json, time, sys

BASE = "http://127.0.0.1:9222"

def api(method, path, body=None):
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(
        f"{BASE}{path}", data=data, method=method,
        headers={"Content-Type": "application/json"} if data else {}
    )
    return json.loads(urllib.request.urlopen(req).read())

def safe_click(selector, retries=3):
    for i in range(retries):
        result = api("POST", "/ai/click", {"selector": selector})
        if "error" not in result:
            return result
        time.sleep(1)
    raise Exception(f"Failed to click {selector} after {retries} retries")

def scrape_with_retry(url, max_retries=3):
    for attempt in range(max_retries):
        try:
            api("POST", "/ai/session", {"action": "create"})
            api("POST", "/ai/navigate", {"url": url})
            api("POST", "/ai/wait", {"selector": "body", "ms": 5000})

            # Dismiss intercepts
            for sel in [".cookie-accept", ".modal-close", ".popup-close"]:
                api("POST", "/ai/click", {"selector": sel})

            page = api("GET", "/ai/page")
            api("POST", "/ai/session", {"action": "destroy"})
            return page

        except Exception as e:
            print(f"Attempt {attempt+1} failed: {e}", file=sys.stderr)
            time.sleep(2)

    return None

if __name__ == "__main__":
    result = scrape_with_retry(sys.argv[1])
    if result:
        print(json.dumps(result, indent=2)[:3000])
```

### Example 4: Playbook with error recovery

```json
{
  "name": "E-commerce Monitor",
  "steps": [
    { "action": "session", "action": "create" },
    { "action": "navigate", "url": "https://store.example.com/product/123" },
    { "action": "wait", "selector": ".price", "ms": 5000 },
    { "action": "evaluate", "code": "document.querySelector('.price').textContent" },
    { "action": "click", "selector": ".add-to-cart" },
    { "action": "wait", "text": "Cart updated", "ms": 5000, "optional": true },
    { "action": "screenshot", "optional": true },
    { "action": "session", "action": "destroy" }
  ]
}
```

---

## Troubleshooting

### Server won't start

```bash
# Check if already running
curl http://127.0.0.1:9222/health

# Check for port conflict
ss -tlnp | grep 9222

# Check Chrome/puppeteer installation
node -e "require('puppeteer-extra')"
```

### Session errors (503)

```bash
# Check session status
bai session

# Create a fresh session
bai session create

# Or restart server
termweb-server --url https://example.com
```

### Elements not found

```bash
# Debug: see what's actually on the page
bai page | head -50

# Check if element exists
bai eval "document.querySelector('#my-id') !== null"

# Check if it's in an iframe
bai eval "document.querySelectorAll('iframe').length"
```

### Navigation timeouts

```bash
# Increase server-level timeout (server/env)
# Or add explicit wait after navigation
bai navigate "https://slow-site.com"
bai wait-for "body" 15000
```

### "No active browser session"

The REST API needs a session to work. Sessions are created:
- **Automatically** when the server starts (with `--url` flag)
- **Manually** via `bai session create`
- **Via WebSocket** when a `bcli` client connects

If you start the server without `--url`, no session exists until a WS client connects or you create one manually.

### Known limitations

- **One session at a time** — The REST API uses a single "default session." Advanced multi-session scenarios require the WebSocket protocol.
- **No JavaScript events** — `type` triggers `input` + `change` events, but not custom JS handlers. Use `evaluate` for complex interactions.
- **Screenshot is JPEG** — Base64 JPEG only. For PNG or full-page screenshots, use `evaluate` to capture via `html2canvas`.
- **No file uploads** — Use the WebSocket client for file inputs.
- **No multi-tab** — The REST API operates on one page. Use `evaluate` to open and switch tabs if needed.

---

## Configuration

### Environment variables

### One-liner install

```bash
curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/install.sh | bash
```

| Variable | Default | Description |
|----------|---------|-------------|
| `BAI_URL` | `http://127.0.0.1:9222` | Server address for `bai` CLI |
| `BAI_TIMEOUT` | `30` | HTTP request timeout (seconds) |

### Server flags

| Flag | Description |
|------|-------------|
| `--url <URL>` | Navigate to URL on startup |
| `--port <n>` | Server port (default: 9222) |
| `--host <addr>` | Bind address (default: 127.0.0.1) |
| `--width <n>` | Viewport width (default: 1280) |
| `--height <n>` | Viewport height (default: 720) |
| `--chrome <path>` | Custom Chrome/Chromium executable |
| `--proxy <url>` | SOCKS5 proxy URL |
| `--data-dir <path>` | Browser user data directory |
| `--no-auto-session` | Don't auto-create session on startup |
| `--no-sandbox` | Disable Chrome sandbox (needed for containers) |

---

## Reference

- `AGENTS.md` — Full developer reference for the project
- `bai --help` — CLI usage
- `server/ai-api.js` — REST API source (read this for edge cases)
- `.skills/bcli-web-agent/skill.md` — Instructions for AI coding agents

---

> **Tip:** Start simple. Run `bai navigate https://example.com && bai page` to verify the server works before building complex playbooks. The text output tells you everything the browser sees — which is exactly what your automation will work with.
