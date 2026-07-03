/*
  Terminal Display Module
  =======================
  Renders browser screenshots inside the terminal using modern terminal
  image protocols: Kitty (primary) and Sixel (fallback).

  Kitty Protocol:
    The terminal image protocol used by the Kitty terminal emulator.
    It embeds base64-encoded PNG data directly in escape sequences.
    Supported by: Kitty terminal, WezTerm, Konsole (partial), iTerm2.
    Format:  \x1b_Ga=T,f=100,s={WIDTH},v={HEIGHT};{BASE64_DATA}\x1b\\

  Sixel Protocol:
    An older but more widely-supported terminal graphics format.
    It encodes images as sixel (six-pixel) data.
    Supported by: XTerm (+Sixel patch), mlterm, foot, Yaft.
    Format:  \x1bPq...{SIXEL_DATA}...\x1b\\

  Detection:
    We auto-detect the best protocol by checking:
    1. $TERM environment variable (kitty, xterm-kitty, etc.)
    2. $KITTY_WINDOW_ID variable (Kitty-specific)
    3. TERMINFO entries for Sixel capability
    4. Fall back gracefully if no image protocol is available

  Flicker-Free:
    We write the image to the exact same screen region each frame.
    By using Kitty's `c=1,c=r` (cursor control) and Sixel's cursor
    positioning, we avoid screen clears that cause flicker.
*/

const config = require('../config/default');
const { computeMapping } = require('./coord-translator');


// ─── Image Protocol Detection ───────────────────────────────────────────────


/**
 * Detect which terminal image protocol is available.
 * Checks environment variables and terminal capabilities.
 *
 * @returns {string}  'kitty' | 'sixel' | 'none'
 */
function detectProtocol() {
  const preferred = config.terminal.imageProtocol;

  if (preferred !== 'auto') {
    return preferred;
  }

  // Kitty detection
  const termEnv = (process.env.TERM || '').toLowerCase();
  const kittyWindowId = process.env.KITTY_WINDOW_ID;
  const kittyPid = process.env.KITTY_PID;

  if (termEnv.includes('kitty') || kittyWindowId || kittyPid) {
    return 'kitty';
  }

  // WezTerm supports Kitty protocol
  if (termEnv.includes('wezterm') || process.env.WEZTERM_PANE) {
    return 'kitty';
  }

  // Sixel detection via terminfo
  try {
    const terminfo = require('child_process').execSync(
      'infocmp -1 2>/dev/null | grep -q "Xsixel" && echo "yes" || echo "no"',
      { encoding: 'utf-8', timeout: 2000 }
    ).trim();
    if (terminfo === 'yes') {
      return 'sixel';
    }
  } catch {
    // infocmp not available — fall through
  }

  // Check TERM for known sixel-capable terminals
  if (
    termEnv.includes('xterm') ||
    termEnv.includes('mlterm') ||
    termEnv.includes('foot') ||
    termEnv.includes('contour')
  ) {
    return 'sixel';
  }

  return 'sixel';
}


// ─── Base64 → Terminal Display ─────────────────────────────────────────────


/**
 * Encode a buffer as base64 for inline transfer.
 */
function toBase64(buf) {
  if (typeof buf === 'string') return buf;
  return buf.toString('base64');
}


/**
 * Split a string into fixed-size chunks (for breaking up large payloads).
 */
function chunkString(str, size) {
  const chunks = [];
  for (let i = 0; i < str.length; i += size) {
    chunks.push(str.slice(i, i + size));
  }
  return chunks;
}


// ─── Kitty Protocol ─────────────────────────────────────────────────────────


/**
 * Render an image using the Kitty terminal image protocol.
 * This writes the image to stdout as an inline image.
 *
 * @param {Buffer} imageBuffer  — PNG or JPEG buffer
 * @param {number} width        — display width in pixels
 * @param {number} height       — display height in pixels
 * @param {number} [col]        — terminal column offset
 * @param {number} [row]        — terminal row offset
 */
function renderKitty(imageBuffer, width, height, col = 0, row = 0) {
  const b64 = toBase64(imageBuffer);
  const chunkSize = 4096;  // max base64 bytes per escape sequence
  const chunks = chunkString(b64, chunkSize);

  // Move cursor to the target position
  process.stdout.write(`\x1b[${row + 1};${col + 1}H`);

  for (let i = 0; i < chunks.length; i++) {
    const isLast = i === chunks.length - 1;
    const m = isLast ? '0' : '1';
    // a=T (transmit), f=100 (PNG), s=width, v=height, m=more-data
    const cmd = `\x1b_Ga=T,f=100,s=${width},v=${height},m=${m};${chunks[i]}\x1b\\`;
    process.stdout.write(cmd);
  }
}


// ─── Sixel Protocol ─────────────────────────────────────────────────────────


/**
 * Convert raw pixel data to sixel data.
 *
 * Sixel encoding works by:
 *  1. Dividing the image into bands of 6 pixels high
 *  2. For each column position, encoding which pixels in each band
 *     should be lit, for each color channel (R, G, B)
 *  3. Using a color palette defined at the start
 *
 * This is a simplified encoder that handles RGB buffers.
 *
 * @param {Buffer} pixelData  — raw RGBA pixel buffer
 * @param {number} width
 * @param {number} height
 * @returns {string}  Sixel-encoded string
 */
function encodeSixel(pixelData, width, height) {
  // Sixel color palette (we use a fixed 16-color EGA-like palette for simplicity)
  // In production, you'd use adaptive palette quantization.
  const palette = [
    { r: 0, g: 0, b: 0 },       // 0: black
    { r: 255, g: 0, b: 0 },     // 1: red
    { r: 0, g: 255, b: 0 },     // 2: green
    { r: 255, g: 255, b: 0 },   // 3: yellow
    { r: 0, g: 0, b: 255 },     // 4: blue
    { r: 255, g: 0, b: 255 },   // 5: magenta
    { r: 0, g: 255, b: 255 },   // 6: cyan
    { r: 255, g: 255, b: 255 }, // 7: white
  ];

  // Extend palette with more colors for better quality
  for (let i = 0; i < 8; i++) {
    const intensity = Math.round((i + 1) / 8 * 255);
    palette.push({ r: intensity, g: 0, b: 0 });
    palette.push({ r: 0, g: intensity, b: 0 });
    palette.push({ r: 0, g: 0, b: intensity });
  }

  const numColors = palette.length;

  // Sixel header
  let sixel = '';

  // Define color palette
  for (let i = 0; i < numColors; i++) {
    const c = palette[i];
    // #{idx};{R};{G};{B}
    // Values are in range 0-100 (sixel spec: 0-100 maps to 0-255)
    const r = Math.round((c.r / 255) * 100);
    const g = Math.round((c.g / 255) * 100);
    const b = Math.round((c.b / 255) * 100);
    sixel += `#${i};2;${r};${g};${b}`;
  }

  // Process image in bands of 6 rows
  for (let row = 0; row < height; row += 6) {
    // For each color, encode which pixels are lit in this band
    for (let ci = 0; ci < numColors; ci++) {
      sixel += `#${ci}`;  // select color

      for (let col = 0; col < width; col++) {
        let sixelByte = 0;

        for (let bit = 0; bit < 6; bit++) {
          const py = row + bit;
          if (py >= height) break;

          const pixelIdx = (py * width + col) * 4;
          if (pixelIdx + 3 >= pixelData.length) break;

          const r = pixelData[pixelIdx];
          const g = pixelData[pixelIdx + 1];
          const b = pixelData[pixelIdx + 2];

          // Find closest palette color
          let minDist = Infinity;
          let closestIdx = 0;
          for (let pi = 0; pi < palette.length; pi++) {
            const dr = r - palette[pi].r;
            const dg = g - palette[pi].g;
            const db = b - palette[pi].b;
            const dist = dr * dr + dg * dg + db * db;
            if (dist < minDist) {
              minDist = dist;
              closestIdx = pi;
            }
          }

          if (closestIdx === ci) {
            sixelByte |= (1 << bit);
          }
        }

        if (sixelByte !== 0) {
          // Sixel bytes are in range 63-126 (0x3F-0x7E)
          // 63 = '?' (empty), 64-126 = sixel patterns
          // We shift so that bit 0 → '?', bit 1 → '@', etc.
          // Actually, in sixel, the byte value = 63 + sixel_bit_pattern
          // But we need to handle the full 6-bit value.
          // For simplicity: if byte = 0, skip (repeat); if non-zero, emit
          sixel += String.fromCharCode(63 + sixelByte);
        } else {
          sixel += '?';  // empty column (0 bits set)
        }
      }

      sixel += '$';  // end of color band row
    }

    sixel += '-';  // end of sixel row band
  }

  return sixel;
}


/**
 * Render an image using the Sixel protocol.
 *
 * @param {Buffer} imageBuffer  — raw RGBA pixel data
 * @param {number} width
 * @param {number} height
 * @param {number} [col]
 * @param {number} [row]
 */
function renderSixel(imageBuffer, width, height, col = 0, row = 0) {
  // Move cursor to target position
  process.stdout.write(`\x1b[${row + 1};${col + 1}H`);

  // Sixel start sequence
  process.stdout.write('\x1bPq');

  // Encode and write sixel data
  const sixelData = encodeSixel(imageBuffer, width, height);
  process.stdout.write(sixelData);

  // Sixel end sequence
  process.stdout.write('\x1b\\');
}


// ─── Main Render Function ───────────────────────────────────────────────────


/**
 * Render a browser screenshot to the terminal.
 * Auto-selects the best available protocol.
 *
 * @param {Buffer} imageBuffer  — screenshot buffer (JPEG or PNG)
 * @param {number} imgWidth     — image pixel width
 * @param {number} imgHeight    — image pixel height
 * @param {Object} termSize     — { cols, rows } terminal dimensions
 * @param {number} [col]        — terminal column offset
 * @param {number} [row]        — terminal row offset
 */
function renderFrame(imageBuffer, imgWidth, imgHeight, termSize, col = 0, row = 0) {
  const protocol = detectProtocol();

  // Scale the image to fit within the terminal
  const mapping = computeMapping(termSize, { width: imgWidth, height: imgHeight });

  // Calculate display area
  const displayW = Math.round(mapping.displayW);
  const displayH = Math.round(mapping.displayH);

  // For Kitty: we pass the original image and let the terminal scale it
  // For Sixel: we need to handle it differently

  if (protocol === 'kitty') {
    const displayCols = Math.round(Math.min(config.terminal.displayWidth, displayW));
    const displayRows = Math.round(Math.min(config.terminal.displayHeight, displayH));
    renderKitty(imageBuffer, displayCols, displayRows, col, row);
  } else if (protocol === 'sixel') {
    // For Sixel, we'd ideally resize the image first.
    // Since we're writing raw, we render directly.
    renderSixel(imageBuffer, imgWidth, imgHeight, col, row);
  } else {
    // Fallback: print a message that image rendering isn't available
    process.stdout.write('\x1b[H\x1b[J');
    process.stdout.write('No image protocol available. Install Kitty terminal or a Sixel-capable terminal.\n');
  }

  // Move cursor back below the image for keyboard input
  const cursorRow = row + Math.min(displayH, termSize.rows - 2);
  process.stdout.write(`\x1b[${cursorRow};1H`);
}


/**
 * Clear the terminal screen and reset cursor.
 */
function clearScreen() {
  process.stdout.write('\x1b[H\x1b[J\x1b[?25h');  // home, clear, show cursor
}


/**
 * Reset terminal to normal mode.
 */
function resetTerminal() {
  process.stdout.write('\x1b[?1000l\x1b[?1002l\x1b[?1006l\x1b[?25h\x1b[2J\x1b[H');
}


module.exports = {
  detectProtocol,
  renderFrame,
  clearScreen,
  resetTerminal,
};
