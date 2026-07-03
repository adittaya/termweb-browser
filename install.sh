#!/usr/bin/env bash
# TermWeb Browser — One-line Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/refs/heads/main/install.sh | bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" 2>/dev/null && pwd || echo "")"

if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/scripts/install.sh" ]; then
  exec "$SCRIPT_DIR/scripts/install.sh" "$@"
fi

# Running via pipe (curl | bash) — fetch scripts/install.sh from GitHub
REPO="adittaya/termweb-browser"
BRANCH="main"
URL="https://raw.githubusercontent.com/${REPO}/refs/heads/${BRANCH}/scripts/install.sh"
echo "▸ Fetching installer from $URL"
curl -fsSL "$URL" | bash -s -- "$@"
