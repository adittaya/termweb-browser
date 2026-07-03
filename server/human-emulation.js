/*
  Human Emulation Module
  ======================
  Makes automated mouse movements and typing look like a real human.
  Captchas (especially sliding puzzles) detect instant teleportation —
  this module defeats that by generating natural Bezier-curved motion paths.

  Key Techniques:
    1. Bezier curves with 2 random control points to create curved paths
    2. Speed variation: fast in the middle, slow at start/end (easing)
    3. Micro-fluctuations: slight position noise at each step
    4. Overshoot correction: sometimes the mouse "misses" and adjusts
    5. Typing jitter: randomized delays between keystrokes
*/

const config = require('../config/default');


/**
 * Generate a random floating-point number in [min, max].
 */
function rand(min, max) {
  return Math.random() * (max - min) + min;
}


/**
 * Linear interpolation between two values.
 */
function lerp(a, b, t) {
  return a + (b - a) * t;
}


/**
 * Evaluate a cubic Bezier curve at parameter t ∈ [0, 1].
 * P0 = start, P1/P2 = control, P3 = end.
 * Each Pi = { x, y }.
 */
function cubicBezier(t, P0, P1, P2, P3) {
  const mt = 1 - t;
  const mt2 = mt * mt;
  const mt3 = mt2 * mt;
  const t2 = t * t;
  const t3 = t2 * t;

  return {
    x: mt3 * P0.x + 3 * mt2 * t * P1.x + 3 * mt * t2 * P2.x + t3 * P3.x,
    y: mt3 * P0.y + 3 * mt2 * t * P1.y + 3 * mt * t2 * P2.y + t3 * P3.y,
  };
}


/**
 * Generate a set of Bezier control points that create a natural,
 * slightly curved path from start to end.
 *
 * @param {Object} start  — { x, y }
 * @param {Object} end    — { x, y }
 * @param {number} jitter — How far control points can stray (fraction of distance)
 * @returns {Array} [P1, P2]  — two control points
 */
function generateControlPoints(start, end, jitter = null) {
  if (jitter === null) jitter = config.human.bezierControlPointJitter;

  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const dist = Math.sqrt(dx * dx + dy * dy);

  // Determine a perpendicular direction for the curve to bend
  const angle = Math.atan2(dy, dx);
  const perpAngle = angle + (Math.random() > 0.5 ? 1 : -1) * (Math.PI / 2 + rand(-0.5, 0.5));

  // How much to deviate from the straight line (proportional to distance)
  const deviation = dist * jitter * rand(0.5, 1.2);

  // Control point 1: ~1/3 along the path, offset perpendicularly
  const t1 = rand(0.2, 0.4);
  const P1 = {
    x: start.x + dx * t1 + Math.cos(perpAngle) * deviation * t1,
    y: start.y + dy * t1 + Math.sin(perpAngle) * deviation * t1,
  };

  // Control point 2: ~2/3 along the path, offset in opposite direction
  const t2 = rand(0.6, 0.8);
  const P2 = {
    x: start.x + dx * t2 + Math.cos(perpAngle + rand(-0.3, 0.3)) * deviation * (1 - t2),
    y: start.y + dy * t2 + Math.sin(perpAngle + rand(-0.3, 0.3)) * deviation * (1 - t2),
  };

  return [P1, P2];
}


/**
 * Generate a complete sequence of mouse positions from 'start' to 'end'
 * that follows a natural human-like curved path with variable speed.
 *
 * @param {Object} start     — { x, y } start pixel coordinates
 * @param {Object} end       — { x, y } target pixel coordinates
 * @param {Object} [opts]    — { steps, duration, jitter } overrides
 * @returns {Array<{x, y, delay}>}  — ordered path points with delays (ms)
 */
function generateMousePath(start, end, opts = {}) {
  const numSteps = opts.steps || Math.round(
    rand(config.human.mouseSteps.min, config.human.mouseSteps.max)
  );
  const totalDuration = opts.duration || rand(
    config.human.mouseDuration.min,
    config.human.mouseDuration.max
  );
  const jitter = opts.jitter !== undefined ? opts.jitter : config.human.bezierControlPointJitter;

  // Handle zero-distance case
  if (start.x === end.x && start.y === end.y) {
    return [{ x: start.x, y: start.y, delay: 0 }];
  }

  // Generate control points for a natural curved path
  const [P1, P2] = generateControlPoints(start, end, jitter);

  const path = [];

  for (let i = 0; i <= numSteps; i++) {
    const t = i / numSteps;

    // --- Easing function (ease-in-out-quad) for speed variation ---
    // Slow at start, fast in middle, slow at end
    const eased = t < 0.5
      ? 2 * t * t
      : 1 - Math.pow(-2 * t + 2, 2) / 2;

    // Evaluate Bezier at eased parameter
    const point = cubicBezier(eased, start, P1, P2, end);

    // --- Micro-fluctuations (subtle noise) ---
    // Humans can't hold a perfectly steady mouse; add tiny random offsets
    const noise = (i > 0 && i < numSteps) ? {
      x: rand(-1.5, 1.5),
      y: rand(-1.5, 1.5),
    } : { x: 0, y: 0 };

    // --- Per-step delay ---
    // Distribute total duration across steps, with extra pauses near start/end
    const baseDelay = totalDuration / numSteps;
    const pauseFactor = 1 + 2 * Math.sin(Math.PI * t) * rand(0, 0.3);
    const delay = Math.round(baseDelay * pauseFactor);

    path.push({
      x: Math.round(point.x + noise.x),
      y: Math.round(point.y + noise.y),
      delay: Math.max(1, delay),
    });
  }

  // --- Optional overshoot ---
  // 30% chance of slight overshoot + correction (very human-like)
  if (Math.random() < 0.3) {
    const overshootDist = rand(3, 12);
    const overshootAngle = Math.atan2(end.y - start.y, end.x - start.x) + rand(-0.3, 0.3);
    const overshoot = {
      x: Math.round(end.x + Math.cos(overshootAngle) * overshootDist),
      y: Math.round(end.y + Math.sin(overshootAngle) * overshootDist),
    };
    const correction = {
      x: end.x,
      y: end.y,
    };

    // Insert overshoot + correction after the main path
    path.push({ ...overshoot, delay: rand(30, 80) });
    path.push({ ...correction, delay: rand(20, 50) });
  }

  return path;
}


/**
 * Move the mouse naturally using Puppeteer's mouse API.
 *
 * @param {import('puppeteer').Page} page
 * @param {number} targetX  — target viewport x
 * @param {number} targetY  — target viewport y
 * @param {Object} [opts]   — optional overrides
 */
async function humanMouseMove(page, targetX, targetY, opts = {}) {
  const startPos = await page.mouse._position || { x: 0, y: 0 };

  // Fallback: if we can't get the current position, read it from JS
  let currentPos = startPos;
  if (currentPos.x === 0 && currentPos.y === 0) {
    try {
      currentPos = await page.evaluate(() => ({
        x: window.__lastMouseX || 0,
        y: window.__lastMouseY || 0,
      }));
    } catch {
      currentPos = { x: 0, y: 0 };
    }
  }

  const path = generateMousePath(currentPos, { x: targetX, y: targetY }, opts);

  for (const step of path) {
    await page.mouse.move(step.x, step.y);
    await new Promise(r => setTimeout(r, step.delay));
  }

  // Store last position for next call
  try {
    await page.evaluate((x, y) => {
      window.__lastMouseX = x;
      window.__lastMouseY = y;
    }, targetX, targetY);
  } catch { /* page may be navigating */ }
}


/**
 * Click a point with human-like behavior:
 *   move naturally → slight pause → mousedown → hold → mouseup → slight pause
 *
 * @param {import('puppeteer').Page} page
 * @param {number} x
 * @param {number} y
 * @param {string} button  — 'left' | 'right' | 'middle'
 */
async function humanClick(page, x, y, button = 'left') {
  await humanMouseMove(page, x, y);

  // Brief hover pause before clicking
  await new Promise(r => setTimeout(r, rand(50, 150)));

  await page.mouse.down({ button });
  await new Promise(r => setTimeout(r, rand(
    config.human.clickHoldDelay.min,
    config.human.clickHoldDelay.max
  )));
  await page.mouse.up({ button });

  // Post-click pause
  await new Promise(r => setTimeout(r, rand(30, 80)));
}


/**
 * Perform a drag operation (e.g., for slider captchas):
 *   mousedown → move along path → mouseup
 *
 * @param {import('puppeteer').Page} page
 * @param {Object} from  — { x, y } start
 * @param {Object} to    — { x, y } end
 * @param {string} button
 */
async function humanDrag(page, from, to, button = 'left') {
  await page.mouse.move(from.x, from.y);
  await new Promise(r => setTimeout(r, rand(100, 200)));
  await page.mouse.down({ button });
  await new Promise(r => setTimeout(r, rand(50, 100)));

  // For drags, use a path with fewer steps (smoother, faster)
  const path = generateMousePath(from, to, {
    steps: Math.round(rand(15, 30)),
    duration: rand(300, 800),
  });

  for (const step of path) {
    await page.mouse.move(step.x, step.y);
    await new Promise(r => setTimeout(r, step.delay));
  }

  await new Promise(r => setTimeout(r, rand(50, 100)));
  await page.mouse.up({ button });
}


/**
 * Type text with human-like variable delays between characters.
 *
 * @param {import('puppeteer').Page} page
 * @param {string} text
 */
async function humanType(page, text) {
  for (const char of text) {
    await page.keyboard.type(char, { delay: rand(
      config.human.typingDelay.min,
      config.human.typingDelay.max
    ) });

    // Occasionally pause longer (simulating thinking)
    if (Math.random() < 0.05) {
      await new Promise(r => setTimeout(r, rand(200, 600)));
    }
  }
}


/**
 * Scroll the page naturally (smooth incremental scrolls).
 *
 * @param {import('puppeteer').Page} page
 * @param {number} deltaY  — positive = scroll down, negative = scroll up
 * @param {number} steps   — how many incremental scrolls
 */
async function humanScroll(page, deltaY, steps = null) {
  if (!steps) steps = Math.round(rand(3, 8));

  const perStep = deltaY / steps;

  for (let i = 0; i < steps; i++) {
    await page.evaluate((dy) => {
      window.scrollBy({ top: dy, behavior: 'instant' });
    }, perStep);
    await new Promise(r => setTimeout(r, rand(20, 60)));
  }
}


module.exports = {
  generateMousePath,
  generateControlPoints,
  humanMouseMove,
  humanClick,
  humanDrag,
  humanType,
  humanScroll,
};
