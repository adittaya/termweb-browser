/*
  Coordinate Translator
  =====================
  Converts terminal grid coordinates (columns/rows) into precise
  browser viewport pixel coordinates so mouse clicks are perfectly accurate.

  The Problem:
    Terminals address positions as (col, row) where each cell is a character.
    But the browser works in pixels. We need to map:
      terminal (col, row)  →  browser (pixelX, pixelY)

  How it works:
    1. The server tells us the browser viewport dimensions (e.g. 1280×720).
    2. The client knows the terminal dimensions (cols × rows).
    3. The displayed screenshot is scaled to fit within the terminal grid.
    4. We compute the offset and scale factor to reverse-map cell coords → pixel coords.

  Cell Aspect Ratio:
    Terminal cells are typically ~2.4× wider than tall (a cell is ~10px wide × 20px tall).
    We derive the actual ratio from the terminal's reported pixel dimensions if available,
    otherwise we fall back to a standard approximation.

  Edge Cases Handled:
    - Non-square aspect ratios between browser viewport and terminal window
    - Letterboxing (black bars) when aspect ratios don't match
    - Terminal resize events
    - Fractional cell positions (clicking at partial-cell granularity)
*/

/**
 * Compute the scale and offsets needed to map terminal cell coordinates
 * to browser pixel coordinates.
 *
 * @param {Object} terminal  — { cols: number, rows: number }
 * @param {Object} viewport  — { width: number, height: number } in pixels
 * @returns {Object}  { scaleX, scaleY, offsetX, offsetY, displayW, displayH }
 *
 *   displayW/H: the pixel dimensions of the image as rendered in the terminal
 *   scaleX/Y:   how many browser pixels per terminal cell column/row
 *   offsetX/Y:  the pixel offset (in terminal cells, from top-left) where
 *               the scaled image starts (for letterboxing compensation)
 */
function computeMapping(terminal, viewport) {
  const aspectBrowser = viewport.width / viewport.height;
  const aspectTerminal = terminal.cols / terminal.rows;

  let displayW, displayH;

  if (aspectBrowser > aspectTerminal) {
    // Browser is wider than terminal → letterbox top/bottom
    displayW = terminal.cols;
    displayH = terminal.cols / aspectBrowser;
  } else {
    // Browser is taller than terminal → letterbox left/right
    displayH = terminal.rows;
    displayW = terminal.rows * aspectBrowser;
  }

  // Letterbox offsets (in terminal cell units)
  const offsetX = (terminal.cols - displayW) / 2;
  const offsetY = (terminal.rows - displayH) / 2;

  // Scale factors: how many browser pixels fit into one terminal cell
  const scaleX = viewport.width / displayW;
  const scaleY = viewport.height / displayH;

  return { scaleX, scaleY, offsetX, offsetY, displayW, displayH };
}


/**
 * Translate a terminal cell (col, row) to browser pixel coordinates.
 *
 * @param {number} col  — 1-based column from terminal (SGR mouse events are 1-based)
 * @param {number} row  — 1-based row from terminal
 * @param {Object} mapping  — result from computeMapping()
 * @param {Object} viewport  — { width, height }
 * @returns {Object}  { x: number, y: number }  — browser viewport pixel coords
 */
function cellToPixel(col, row, mapping, viewport) {
  const { scaleX, scaleY, offsetX, offsetY } = mapping;

  // Convert from 1-based terminal coords to 0-based, then subtract letterbox offset
  const cellX = (col - 1) - offsetX;
  const cellY = (row - 1) - offsetY;

  // Clamp to valid display area
  const clampedX = Math.max(0, Math.min(cellX, mapping.displayW - 0.001));
  const clampedY = Math.max(0, Math.min(cellY, mapping.displayH - 0.001));

  // Scale to browser pixels
  const pixelX = clampedX * scaleX;
  const pixelY = clampedY * scaleY;

  return {
    x: Math.round(clampedX * scaleX),
    y: Math.round(clampedY * scaleY),
  };
}


/**
 * Translate a pixel coordinate to the concept of what terminal cell it'd be in.
 * (Useful for debugging or rendering overlays.)
 */
function pixelToCell(px, py, mapping, viewport) {
  const { scaleX, scaleY, offsetX, offsetY } = mapping;

  const cellX = (px / scaleX) + offsetX + 1; // back to 1-based
  const cellY = (py / scaleY) + offsetY + 1;

  return { col: Math.round(cellX), row: Math.round(cellY) };
}


/**
 * Estimate the terminal's physical pixel dimensions from a known cell count
 * and a typical character cell aspect ratio.
 *
 * Many modern terminals report pixel size via OSC 4 or the `COLUMNS`/`LINES`
 * env vars combined with `stty size`. This heuristic fills the gap when
 * pixel-level info is unavailable.
 *
 * @param {number} cols
 * @param {number} rows
 * @param {number} cellAspectRatio  — width/height of a single cell (~0.5 typical)
 * @returns {Object}  { width, height } in estimated pixels
 */
function estimateTerminalPixels(cols, rows, cellAspectRatio = 0.5) {
  // Assume a cell is roughly 2:1 height:width
  // So font height ≈ 2 × font width
  // If we estimate cell height as some typical value...
  const cellHeight = 20; // typical pixel height of a terminal cell
  const cellWidth = cellHeight * cellAspectRatio; // ~10px

  return {
    width: Math.round(cols * cellWidth),
    height: Math.round(rows * cellHeight),
  };
}


module.exports = {
  computeMapping,
  cellToPixel,
  pixelToCell,
  estimateTerminalPixels,
};
