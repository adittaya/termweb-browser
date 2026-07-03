#!/usr/bin/env node
/*
  Global Installer
  ================
  Symlinks the bcli script and server into a system PATH directory.

  Usage:
    node bin/install.js              — Install to ~/.local/bin (default)
    node bin/install.js /usr/local   — Install to custom prefix

  This creates:
    /usr/local/bin/bcli          → bin/bcli (shell wrapper)
    /usr/local/bin/termweb-server → server/index.js (node daemon)
*/

const fs = require('fs');
const path = require('path');

const REPO_DIR = path.resolve(__dirname, '..');

function install(prefix) {
  const binDir = path.join(prefix, 'bin');
  fs.mkdirSync(binDir, { recursive: true });

  // ── bcli (shell wrapper) ──────────────────────────────────────────────
  const bcliTarget = path.join(binDir, 'bcli');
  const bcliSource = path.join(REPO_DIR, 'bin', 'bcli');
  symlinkForce(bcliTarget, bcliSource);
  fs.chmodSync(bcliTarget, 0o755);
  console.log(`  ✓ ${bcliTarget} → ${bcliSource}`);

  // ── termweb-server (Node.js daemon) ───────────────────────────────────
  const serverTarget = path.join(binDir, 'termweb-server');
  const serverSource = path.join(REPO_DIR, 'server', 'index.js');
  symlinkForce(serverTarget, serverSource);
  fs.chmodSync(serverTarget, 0o755);
  console.log(`  ✓ ${serverTarget} → ${serverSource}`);

  // ── bai (AI agent CLI) ──────────────────────────────────────────────
  const baiTarget = path.join(binDir, 'bai');
  const baiSource = path.join(REPO_DIR, 'bin', 'bai');
  symlinkForce(baiTarget, baiSource);
  fs.chmodSync(baiTarget, 0o755);
  console.log(`  ✓ ${baiTarget} → ${baiSource}`);

  console.log('\n✅ Installed. You may need to restart your shell or run:');
  console.log(`   export PATH="$PATH:${binDir}"`);
  console.log('\nThen use:');
  console.log('   bcli --help              Interactive browser');
  console.log('   bcli --server            Start daemon + connect');
  console.log('   bai status               AI agent: check status');
  console.log('   bai navigate https://... AI agent: browse');
  console.log('   bai text                 AI agent: read page');
  console.log('   bai click "#btn"         AI agent: click');
  console.log('   termweb-server           Background daemon');
}

function symlinkForce(target, source) {
  try {
    fs.unlinkSync(target);
  } catch {}
  fs.symlinkSync(source, target);
}

// ── Main ────────────────────────────────────────────────────────────────────
const prefix = process.argv[2] || path.join(process.env.HOME || '/root', '.local');
console.log(`Installing to ${prefix}/bin ...\n`);
install(prefix);
