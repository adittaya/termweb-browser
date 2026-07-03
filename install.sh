#!/usr/bin/env bash
# TermWeb Browser — One-line Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/install.sh | bash
set -euo pipefail
exec "$(dirname "$0")/scripts/install.sh" "$@"
