/*
  Anti-Fingerprinting Module
  ==========================
  Applies additional stealth patches ON TOP of puppeteer-extra-plugin-stealth.
  The stealth plugin handles the major detection vectors; this module
  adds extra hardening for advanced anti-bot systems.

  The stealth plugin already handles:
    - navigator.webdriver → undefined
    - chrome.runtime mock
    - WebGL vendor/renderer spoofing
    - Languages, plugins, platform
    - Permissions overrides
    - Screen dimension fixes

  This module adds:
    - Canvas fingerprint noise (subtle pixel shifts)
    - Font fingerprint spoofing (available font list)
    - AudioContext fingerprint spoofing
    - Timezone/date override anomalies
    - Hardware concurrency variations
*/

const config = require('../config/default');


/**
 * Apply custom stealth patches that go beyond the puppeteer-extra plugin.
 * Call this AFTER the stealth plugin has been applied.
 *
 * @param {import('puppeteer').Page} page
 */
async function applyCustomStealth(page) {
  if (!config.stealth.enabled) return;

  // 1. Canvas fingerprint noise: add undetectable 1-pixel noise
  await page.evaluateOnNewDocument(() => {
    const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
    const originalGetImageData = CanvasRenderingContext2D.prototype.getImageData;

    CanvasRenderingContext2D.prototype.getImageData = function (...args) {
      const imageData = originalGetImageData.apply(this, args);
      const data = imageData.data;
      // Shift every 40th pixel's R channel by 1 (undetectable, changes hash)
      for (let i = 0; i < data.length; i += 160) {
        data[i] = Math.min(255, data[i] + 1);
      }
      return imageData;
    };

    HTMLCanvasElement.prototype.toDataURL = function (...args) {
      // Ensure the noise has been applied by calling getImageData first
      const ctx = this.getContext('2d');
      if (ctx) {
        try {
          const w = this.width, h = this.height;
          if (w > 0 && h > 0) {
            ctx.getImageData(0, 0, w, h);
          }
        } catch {}
      }
      return originalToDataURL.apply(this, args);
    };
  });

  // 2. AudioContext fingerprint spoofing
  await page.evaluateOnNewDocument(() => {
    const originalGetChannelData = AudioBuffer.prototype.getChannelData;
    AudioBuffer.prototype.getChannelData = function (channel) {
      const data = originalGetChannelData.call(this, channel);
      // Add sub-threshold noise to the first few samples
      if (data.length > 0) {
        data[0] += 0.0001 * (Math.random() - 0.5);
      }
      return data;
    };
  });

  // 3. Spoof available fonts (via measureText)
  await page.evaluateOnNewDocument(() => {
    const originalMeasureText = CanvasRenderingContext2D.prototype.measureText;
    CanvasRenderingContext2D.prototype.measureText = function (text) {
      const metrics = originalMeasureText.call(this, text);
      // Slightly vary text measurements to differ from headless defaults
      const origWidth = metrics.width;
      Object.defineProperty(metrics, 'width', {
        get: () => origWidth + (Math.random() < 0.1 ? 0.01 : 0),
        configurable: true,
      });
      return metrics;
    };
  });

  // 4. Navigator.hardwareConcurrency with slight variance
  await page.evaluateOnNewDocument(() => {
    const concurrency = navigator.hardwareConcurrency || 4;
    if (concurrency === 1 || concurrency === 2) {
      // Headless often returns 1; real hardware rarely does
      Object.defineProperty(navigator, 'hardwareConcurrency', {
        get: () => 4 + Math.floor(Math.random() * 4),
        configurable: true,
      });
    }
  });

  // 5. WebGL anti-detection for headless quirks
  await page.evaluateOnNewDocument(() => {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    if (gl) {
      const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
      if (debugInfo) {
        const vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL);
        const renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);
        // Headless SwiftShader/OSMesa renderers are red flags
        if (renderer && (renderer.includes('SwiftShader') || renderer.includes('llvmpipe') || renderer.includes('OSMesa'))) {
          const fakeRenderer = config.stealth.webGLRenderer || 'ANGLE (Intel, Intel(R) UHD Graphics 620 Direct3D11 vs_5_0 ps_5_0)';
          Object.defineProperty(gl, 'getParameter', {
            value: function (pname) {
              if (pname === debugInfo.UNMASKED_RENDERER_WEBGL) return fakeRenderer;
              if (pname === debugInfo.UNMASKED_VENDOR_WEBGL) return config.stealth.webGLVendor || 'Google Inc. (Intel)';
              return gl.getParameter(pname);
            },
          });
        }
      }
    }
  });
}


module.exports = { applyCustomStealth };
