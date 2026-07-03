/*
  AI Agent REST API
  =================
  Provides a text/JSON-based interface for AI agents to interact with
  the browser without needing terminal graphics.

  AI agents cannot see images. This API extracts:
    - Page text content (clean, readable text)
    - All links with href and visible text
    - Interactive elements (buttons, inputs, forms)
    - Page title and URL
    - HTML structure (simplified)

  All responses are JSON. No terminal graphics required.

  Endpoints:
    GET    /ai/status           — Session and page status
    GET    /ai/page             — Full page content extraction
    GET    /ai/text             — Page text content only
    GET    /ai/links            — All links on the page
    GET    /ai/buttons          — All clickable elements
    GET    /ai/forms            — All form elements
    GET    /ai/html             — Simplified HTML structure
    POST   /ai/navigate         — Navigate to URL
    POST   /ai/click            — Click element by selector or coords
    POST   /ai/type             — Type text into an element
    POST   /ai/scroll           — Scroll the page
    POST   /ai/evaluate         — Execute JavaScript
    POST   /ai/screenshot       — Get screenshot (base64, for reference)
    POST   /ai/wait             — Wait for condition
*/

const { URL } = require('url');


/**
 * Main AI API request handler.
 * Called by the server for any request with path starting with /ai/
 *
 * @param {http.IncomingMessage} req
 * @param {http.ServerResponse} res
 * @param {Function} getSession  — returns the active BrowserSession
 */
async function handleAIRequest(req, res, getSession) {

  // ─── Helper: Extract clean text from the page ─────────────────────────
  async function extractPageText(page) {
    return page.evaluate(() => {
      // Remove script, style, noscript tags
      const clone = document.body.cloneNode(true);
      const removals = clone.querySelectorAll('script, style, noscript, svg, canvas');
      removals.forEach(el => el.remove());

      // Get text, collapse whitespace, limit length
      let text = clone.textContent || '';
      text = text.replace(/[\t\n\r]+/g, '\n')
                 .replace(/ {2,}/g, ' ')
                 .replace(/\n{3,}/g, '\n\n')
                 .trim();
      return text.substring(0, 100000); // 100k char limit
    });
  }


  // ─── Helper: Get all interactive elements ─────────────────────────────
  async function extractInteractiveElements(page) {
    return page.evaluate(() => {
      const results = [];

      // Collect all clickable elements
      const selectors = [
        'a[href]', 'button', 'input[type="button"]',
        'input[type="submit"]', 'input[type="text"]',
        'input[type="search"]', 'input[type="email"]',
        'input[type="password"]', 'textarea', 'select',
        '[role="button"]', '[onclick]', '[tabindex]',
      ];

      const seen = new Set();

      selectors.forEach(sel => {
        document.querySelectorAll(sel).forEach(el => {
          if (seen.has(el)) return;
          seen.add(el);

          const rect = el.getBoundingClientRect();
          if (rect.width === 0 || rect.height === 0) return;

          const tag = el.tagName.toLowerCase();
          const type = el.type || '';
          const text = (el.textContent || '').trim().substring(0, 200);
          const placeholder = el.placeholder || '';
          const name = el.name || el.id || '';
          const href = el.href || '';
          const aria = el.getAttribute('aria-label') || '';

          results.push({
            tag, type,
            text: text || aria || placeholder || name || tag,
            selector: buildSelector(el),
            rect: {
              x: Math.round(rect.x), y: Math.round(rect.y),
              width: Math.round(rect.width), height: Math.round(rect.height),
            },
            attributes: {
              name: el.name || null,
              id: el.id || null,
              class: (el.className && typeof el.className === 'string') ? el.className : null,
              href: href || null,
              placeholder: placeholder || null,
              value: el.value || null,
              'aria-label': aria || null,
            },
          });
        });
      });

      return results;

      function buildSelector(el) {
        if (el.id) return `#${el.id}`;
        const tag = el.tagName.toLowerCase();
        if (el.name) return `${tag}[name="${el.name}"]`;
        if (el.className && typeof el.className === 'string') {
          const cls = el.className.trim().split(/\s+/).filter(Boolean).join('.');
          if (cls) return `${tag}.${cls}`;
        }
        // Build path-based selector
        const path = [];
        let current = el;
        while (current && current !== document.body) {
          let s = current.tagName.toLowerCase();
          if (current.id) { s = `#${current.id}`; path.unshift(s); break; }
          if (current.className && typeof current.className === 'string') {
            const cls = current.className.trim().split(/\s+/).filter(Boolean).join('.');
            if (cls) s += `.${cls}`;
          }
          // Add nth-child if siblings match
          const parent = current.parentElement;
          if (parent) {
            const siblings = Array.from(parent.children).filter(
              c => c.tagName === current.tagName
            );
            if (siblings.length > 1) {
              const idx = siblings.indexOf(current) + 1;
              s += `:nth-child(${idx})`;
            }
          }
          path.unshift(s);
          current = current.parentElement;
        }
        return path.join(' > ');
      }
    });
  }


  // ─── Helper: Get links ────────────────────────────────────────────────
  async function extractLinks(page) {
    return page.evaluate(() => {
      return Array.from(document.querySelectorAll('a[href]')).map(a => ({
        text: (a.textContent || '').trim().substring(0, 200) || '(image)',
        href: a.href,
        title: a.title || null,
        rel: a.rel || null,
        target: a.target || null,
      })).filter(a => a.href && !a.href.startsWith('javascript:'));
    });
  }


  // ─── Helper: Simplified HTML structure ────────────────────────────────
  async function extractSimplifiedHTML(page) {
    return page.evaluate(() => {
      function simplify(node, depth = 0) {
        if (depth > 6 || !node || node.nodeType !== 1) return null;
        const tag = node.tagName.toLowerCase();
        if (['script', 'style', 'noscript', 'svg', 'canvas'].includes(tag)) return null;

        const children = [];
        for (const child of node.children) {
          const s = simplify(child, depth + 1);
          if (s) children.push(s);
        }

        const text = (node.textContent || '').trim().substring(0, 100);

        return {
          tag,
          id: node.id || undefined,
          class: (node.className && typeof node.className === 'string')
            ? node.className.split(/\s+/).filter(Boolean).slice(0, 3).join(' ')
            : undefined,
          text: children.length === 0 ? text : undefined,
          children: children.length > 0 ? children : undefined,
        };
      }
      return simplify(document.body);
    });
  }


  // ─── Helper: Get forms ────────────────────────────────────────────────
  async function extractForms(page) {
    return page.evaluate(() => {
      return Array.from(document.forms).map(form => ({
        action: form.action,
        method: form.method,
        id: form.id || null,
        name: form.name || null,
        fields: Array.from(form.elements).map(el => ({
          tag: el.tagName.toLowerCase(),
          type: el.type || '',
          name: el.name || '',
          id: el.id || null,
          placeholder: el.placeholder || null,
          value: el.value || null,
          required: el.required || false,
          disabled: el.disabled || false,
          options: el.options ? Array.from(el.options).map(o => ({
            text: o.text, value: o.value, selected: o.selected,
          })) : undefined,
        })),
      }));
    });
  }


  // ─── Helper: Click element by selector ────────────────────────────────
  async function clickElement(page, selector) {
    return page.evaluate((sel) => {
      const el = document.querySelector(sel);
      if (!el) throw new Error(`Element not found: ${sel}`);
      const rect = el.getBoundingClientRect();
      el.dispatchEvent(new MouseEvent('mousedown', {
        bubbles: true, cancelable: true,
        clientX: rect.x + rect.width / 2,
        clientY: rect.y + rect.height / 2,
      }));
      el.dispatchEvent(new MouseEvent('mouseup', {
        bubbles: true, cancelable: true,
        clientX: rect.x + rect.width / 2,
        clientY: rect.y + rect.height / 2,
      }));
      el.click();
      return { clicked: true, selector: sel, x: rect.x, y: rect.y };
    }, selector);
  }


  // ─── Helper: Type into element ────────────────────────────────────────
  async function typeIntoElement(page, selector, text) {
    return page.evaluate((sel, txt) => {
      const el = document.querySelector(sel);
      if (!el) throw new Error(`Element not found: ${sel}`);
      el.focus();
      el.value = '';
      el.value = txt;
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return { typed: true, selector: sel, length: txt.length };
    }, selector, text);
  }


  // ─── Route table ──────────────────────────────────────────────────────
  // Map "METHOD /path" → async handler(req, res, session, page)
  const routes = {
    'GET /ai/status':    route_noarg(handleStatus),
    'GET /ai/page':      route_noarg(handlePage),
    'GET /ai/text':      route_noarg(handleText),
    'GET /ai/links':     route_noarg(handleLinks),
    'GET /ai/buttons':   route_noarg(handleButtons),
    'GET /ai/forms':     route_noarg(handleForms),
    'GET /ai/html':      route_noarg(handleHTML),
    'POST /ai/navigate':  route_body(handleNavigate),
    'POST /ai/click':     route_body(handleClick),
    'POST /ai/type':      route_body(handleType),
    'POST /ai/scroll':    route_body(handleScroll),
    'POST /ai/evaluate':  route_body(handleEvaluate),
    'POST /ai/screenshot': route_body(handleScreenshot),
    'POST /ai/wait':      route_body(handleWait),
  };

  // Route the request
  const parsedReqUrl = new URL(req.url, `http://${req.headers.host}`);
  const pathname = parsedReqUrl.pathname;
  const key = `${req.method} ${pathname}`;
  const handler = routes[key];

  if (handler) {
    return handler(req, res, getSession);
  }

  // 404 for unknown /ai/ paths
  sendJSON(res, 404, { error: 'Unknown AI endpoint. See server/ai-api.js for available routes.' });


  // ─── Route wrappers ──────────────────────────────────────────────────

  function route_noarg(fn) {
    return async (req, res, getSession) => {
      const session = getSession();
      if (!session) return sendJSON(res, 503, { error: 'No active browser session' });
      const page = session._resolvePage();
      if (!page) return sendJSON(res, 503, { error: 'No active page' });
      try {
        await fn(req, res, session, page);
      } catch (err) {
        sendJSON(res, 500, { error: err.message });
      }
    };
  }

  function route_body(fn) {
    return async (req, res, getSession) => {
      const session = getSession();
      if (!session) return sendJSON(res, 503, { error: 'No active browser session' });
      const page = session._resolvePage();
      if (!page) return sendJSON(res, 503, { error: 'No active page' });
      try {
        const body = await parseBody(req);
        await fn(req, res, session, page, body);
      } catch (err) {
        sendJSON(res, 500, { error: err.message });
      }
    };
  }


  // ─── Route handlers ──────────────────────────────────────────────────

  async function handleStatus(req, res, session, page) {
    const info = {
      connected: true,
      url: await page.url(),
      title: await page.title(),
      viewport: session.viewport,
      tabs: session.getTabsInfo(),
      sessionId: session.sessionId,
      timestamp: Date.now(),
    };
    sendJSON(res, 200, info);
  }

  async function handlePage(req, res, session, page) {
    const [text, links, interactives, title, pageUrl] = await Promise.all([
      extractPageText(page),
      extractLinks(page),
      extractInteractiveElements(page),
      page.title(),
      page.url(),
    ]);
    sendJSON(res, 200, {
      url: pageUrl, title,
      text, links, interactives,
      textLength: text.length,
      linkCount: links.length,
      elementCount: interactives.length,
    });
  }

  async function handleText(req, res, session, page) {
    const text = await extractPageText(page);
    sendJSON(res, 200, { text, length: text.length, url: await page.url() });
  }

  async function handleLinks(req, res, session, page) {
    const links = await extractLinks(page);
    sendJSON(res, 200, { links, count: links.length, url: await page.url() });
  }

  async function handleButtons(req, res, session, page) {
    const elements = await extractInteractiveElements(page);
    sendJSON(res, 200, { elements, count: elements.length, url: await page.url() });
  }

  async function handleForms(req, res, session, page) {
    const forms = await extractForms(page);
    sendJSON(res, 200, { forms, count: forms.length, url: await page.url() });
  }

  async function handleHTML(req, res, session, page) {
    const html = await extractSimplifiedHTML(page);
    sendJSON(res, 200, { html, url: await page.url() });
  }

  async function handleNavigate(req, res, session, page, body) {
    const targetUrl = body.url || body.q;
    if (!targetUrl) return sendJSON(res, 400, { error: 'Missing url' });
    const result = await session.navigate(targetUrl);
    sendJSON(res, 200, { url: result.url, title: result.title, success: true });
  }

  async function handleClick(req, res, session, page, body) {
    const { selector, x, y } = body;
    if (selector) {
      const { humanClick } = require('./human-emulation');
      const pos = await page.evaluate((sel) => {
        const el = document.querySelector(sel);
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
      }, selector);
      if (!pos) return sendJSON(res, 404, { error: `Element not found: ${selector}` });
      await humanClick(page, Math.round(pos.x), Math.round(pos.y));
      sendJSON(res, 200, { clicked: true, selector, x: Math.round(pos.x), y: Math.round(pos.y) });
    } else if (x !== undefined && y !== undefined) {
      const { humanClick } = require('./human-emulation');
      await humanClick(page, Math.round(x), Math.round(y));
      sendJSON(res, 200, { clicked: true, x: Math.round(x), y: Math.round(y) });
    } else {
      sendJSON(res, 400, { error: 'Provide selector, or x and y' });
    }
  }

  async function handleType(req, res, session, page, body) {
    const { selector, text, value } = body;
    const textToType = text || value || '';
    if (!textToType) return sendJSON(res, 400, { error: 'Missing text' });
    if (selector) {
      await typeIntoElement(page, selector, textToType);
      sendJSON(res, 200, { typed: true, selector, length: textToType.length });
    } else {
      const { humanType } = require('./human-emulation');
      await humanType(page, textToType);
      sendJSON(res, 200, { typed: true, length: textToType.length });
    }
  }

  async function handleScroll(req, res, session, page, body) {
    const deltaX = body.delta_x || body.x || 0;
    const deltaY = body.delta_y || body.y || 0;
    await page.evaluate((dx, dy) => {
      window.scrollBy({ left: dx, top: dy, behavior: 'smooth' });
    }, deltaX, deltaY);
    const scrollPos = await page.evaluate(() => ({
      x: window.scrollX, y: window.scrollY,
      maxX: document.documentElement.scrollWidth - window.innerWidth,
      maxY: document.documentElement.scrollHeight - window.innerHeight,
    }));
    sendJSON(res, 200, { scrolled: true, ...scrollPos });
  }

  async function handleEvaluate(req, res, session, page, body) {
    const code = body.code || body.script || body.js;
    if (!code) return sendJSON(res, 400, { error: 'Missing code' });
    const result = await page.evaluate(code);
    sendJSON(res, 200, { result });
  }

  async function handleScreenshot(req, res, session, page, body) {
    const result = await session.captureScreenshot();
    if (!result) return sendJSON(res, 500, { error: 'Screenshot failed' });
    sendJSON(res, 200, {
      data: result.buffer.toString('base64'),
      encoding: 'jpeg',
      width: session.viewport.width,
      height: session.viewport.height,
    });
  }

  async function handleWait(req, res, session, page, body) {
    const ms = body.ms || body.timeout || body.delay || 1000;
    const selector = body.selector || null;
    const text = body.text || null;
    if (selector) {
      await page.waitForSelector(selector, { timeout: ms });
      sendJSON(res, 200, { waited: true, selector, found: true });
    } else if (text) {
      await page.waitForFunction((t) => document.body.textContent.includes(t), { timeout: ms }, text);
      sendJSON(res, 200, { waited: true, text, found: true });
    } else {
      await new Promise(r => setTimeout(r, ms));
      sendJSON(res, 200, { waited: true, duration: ms });
    }
  }
}


// ─── HTTP Helpers ───────────────────────────────────────────────────────────

function sendJSON(res, status, data) {
  res.writeHead(status, {
    'Content-Type': 'application/json',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  });
  res.end(JSON.stringify(data));
}


function parseBody(req) {
  return new Promise((resolve) => {
    if (req.method === 'GET') {
      const parsed = new URL(req.url, `http://${req.headers.host}`);
      const query = {};
      for (const [key, value] of parsed.searchParams) {
        query[key] = value;
      }
      return resolve(query);
    }

    let data = '';
    req.on('data', chunk => { data += chunk; });
    req.on('end', () => {
      try { resolve(JSON.parse(data)); }
      catch { resolve({}); }
    });
    // Safety timeout
    setTimeout(() => resolve({}), 5000);
  });
}


module.exports = { handleAIRequest };
