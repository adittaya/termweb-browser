#!/usr/bin/env node
/**
 * Downloads Chrome/Chromium for the current platform.
 * Used by the install script and first-run launcher.
 */
const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const DATA_DIR = process.env.DATA_DIR || path.join(require('os').homedir(), '.termweb');
const CHROME_DIR = path.join(DATA_DIR, 'chrome');

async function downloadWithPuppeteer() {
    // Try using @puppeteer/browsers if available
    try {
        const { install } = require('@puppeteer/browsers');
        const platform = { linux: 'linux', darwin: 'mac', win32: 'win32' }[process.platform] || 'linux';
        const installed = await install({
            browser: 'chrome',
            buildId: 'latest',
            path: CHROME_DIR,
            platform,
            extract: true,
        });
        const binary = installed.executablePath;
        fs.writeFileSync(path.join(DATA_DIR, 'chrome-path.txt'), binary);
        console.log(`Chrome downloaded: ${binary}`);
        return binary;
    } catch (e) {
        console.warn(`@puppeteer/browsers not available: ${e.message}`);
        return null;
    }
}

async function downloadFallback() {
    // Fallback: download a known Chromium revision manually
    const LAST_CHROME_REVISION = '1351523'; // Recent known-good revision
    const arch = process.arch === 'arm64' || process.arch === 'aarch64' ? 'arm64' : 'x64';
    const urls = {
        'linux': {
            'x64': `https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.0/linux64/chrome-linux64.zip`,
            'arm64': `https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.0/linux-arm64/chrome-linux-arm64.zip`,
        },
        'darwin': {
            'x64': `https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.0/mac-x64/chrome-mac-x64.zip`,
            'arm64': `https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.0/mac-arm64/chrome-mac-arm64.zip`,
        },
        'win32': {
            'x64': `https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.0/win64/chrome-win64.zip`,
            'arm64': `https://storage.googleapis.com/chrome-for-testing-public/135.0.7049.0/win-arm64/chrome-win-arm64.zip`,
        },
    };

    const platform = process.platform;
    const url = urls[platform] && urls[platform][arch];
    if (!url) {
        console.error(`No Chrome URL for platform: ${platform}`);
        return null;
    }

    const zipPath = path.join(DATA_DIR, 'chrome.zip');
    console.log(`Downloading Chrome from ${url}...`);

    await new Promise((resolve, reject) => {
        const file = fs.createWriteStream(zipPath);
        https.get(url, (res) => {
            if (res.statusCode !== 200) {
                reject(new Error(`HTTP ${res.statusCode}`));
                return;
            }
            res.pipe(file);
            file.on('finish', () => file.close(resolve));
        }).on('error', reject);
    });

    // Extract
    execSync(`unzip -o "${zipPath}" -d "${CHROME_DIR}"`, { stdio: 'inherit' });
    fs.unlinkSync(zipPath);

    // Find the chrome binary
    const chromeName = process.platform === 'win32' ? 'chrome.exe' : 'chrome';
    const find = execSync(`find "${CHROME_DIR}" -name "${chromeName}" -type f 2>/dev/null | head -1`).toString().trim();
    if (find) {
        fs.writeFileSync(path.join(DATA_DIR, 'chrome-path.txt'), find);
        console.log(`Chrome ready: ${find}`);
        return find;
    }
    return null;
}

async function main() {
    fs.mkdirSync(CHROME_DIR, { recursive: true });

    // Check if already present
    const existing = fs.readFileSync(path.join(DATA_DIR, 'chrome-path.txt'), 'utf-8').trim();
    if (existing && fs.existsSync(existing)) {
        console.log(`Chrome already at: ${existing}`);
        return;
    }

    let binary = await downloadWithPuppeteer();
    if (!binary) {
        binary = await downloadFallback();
    }
    if (!binary) {
        console.error('Failed to download Chrome. Install manually.');
        process.exit(1);
    }
}

main().catch(err => {
    console.error('Chrome download failed:', err.message);
    process.exit(1);
});
