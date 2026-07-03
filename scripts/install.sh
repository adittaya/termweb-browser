#!/usr/bin/env bash
# ─── termweb-browser: Production Installer ─────────────────────────────────
# Detects OS/arch, downloads prebuilt binary from GitHub Releases,
# falls back to source build. Zero dependencies beyond curl/wget + bash.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/scripts/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/scripts/install.sh | bash -s v1.0.0
#
# Env vars:
#   INSTALL_DIR    Binary installation directory  (default: ~/.local/bin)
#   DATA_DIR       Data/cache directory           (default: ~/.termweb)
#   CHROME_PATH    Pre-existing Chrome binary      (skip auto-download)
#   TERMWEB_PORT   Server port                    (default: 9222)
#   NO_BUILD       Skip source build fallback      (any non-empty)
#   NO_CHROME      Skip Chrome download            (any non-empty)
#   GITHUB_TOKEN   GitHub API token for rate limit
# ────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO="adittaya/termweb-browser"
REPO_URL="https://github.com/${REPO}"
API_URL="https://api.github.com/repos/${REPO}"
VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
DATA_DIR="${DATA_DIR:-$HOME/.termweb}"
TEMP_DIR=""

# ─── Cleanup ────────────────────────────────────────────────────────────────
cleanup() {
    local ec=$?
    [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ] && rm -rf "$TEMP_DIR" 2>/dev/null || true
    exit $ec
}
trap cleanup EXIT INT TERM

# ─── Output helpers ─────────────────────────────────────────────────────────
info()  { printf "\033[1;34m•\033[0m %s\n" "$*"; }
ok()    { printf "\033[1;32m✓\033[0m %s\n" "$*"; }
warn()  { printf "\033[1;33m!\033[0m %s\n" "$*" >&2; }
err()   { printf "\033[1;31m✗\033[0m %s\n" "$*" >&2; exit 1; }
header(){ printf "\n\033[1;36m═══ %s ═══\033[0m\n" "$*"; }

# ─── Platform detection ─────────────────────────────────────────────────────
detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux)  echo "linux" ;;
        Darwin) echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)      err "Unsupported OS: ${os}. We support Linux, macOS, and Windows." ;;
    esac
}
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64)  echo "aarch64" ;;
        armv7l|armhf)   echo "armv7" ;;
        *) err "Unsupported architecture: ${arch}. We support x86_64, aarch64, armv7." ;;
    esac
}
release_platform() {
    local arch os
    arch=$(detect_arch)
    os=$(detect_os)
    case "${os}-${arch}" in
        linux-x86_64)  echo "linux-x64" ;;
        linux-aarch64) echo "linux-arm64" ;;
        macos-x86_64)  echo "macos-x64" ;;
        macos-aarch64) echo "macos-arm64" ;;
        windows-x86_64) echo "windows-x64" ;;
        windows-aarch64) echo "windows-arm64" ;;
        *) err "No prebuilt release for ${arch}-${os}. Try: NO_BUILD=1 to skip." ;;
    esac
}

# ─── Download helper ────────────────────────────────────────────────────────
download() {
    local url="$1" out="$2"
    mkdir -p "$(dirname "$out")"
    if command -v curl >/dev/null 2>&1; then
        if [ -n "${GITHUB_TOKEN:-}" ]; then
            curl -fsSL -H "Authorization: token ${GITHUB_TOKEN}" -o "$out" "$url"
        else
            curl -fsSL -o "$out" "$url"
        fi
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$out" "$url"
    else
        err "Need curl or wget to download files."
    fi
    [ -f "$out" ] && [ -s "$out" ]
}

download_with_retry() {
    local url="$1" out="$2" max_attempts="${3:-3}"
    local attempt=0
    while [ $attempt -lt "$max_attempts" ]; do
        attempt=$((attempt + 1))
        if download "$url" "$out" 2>/dev/null; then
            return 0
        fi
        [ $attempt -lt "$max_attempts" ] && sleep 2
    done
    return 1
}

# ─── Sudo helper ────────────────────────────────────────────────────────────
maybe_sudo() {
    if [ "$(id -u)" = "0" ] || ! command -v sudo >/dev/null 2>&1; then
        "$@"
    else
        sudo "$@"
    fi
}

# ─── System dependencies ────────────────────────────────────────────────────
install_system_deps() {
    local os
    os=$(detect_os)
    case "$os" in
        linux)
            if command -v apt-get >/dev/null 2>&1; then
                info "Installing system dependencies (apt)..."
                maybe_sudo apt-get update -qq
                maybe_sudo apt-get install -y -qq curl ca-certificates git build-essential pkg-config libssl-dev unzip python3 2>/dev/null || true
            elif command -v dnf >/dev/null 2>&1; then
                info "Installing system dependencies (dnf)..."
                maybe_sudo dnf install -y curl ca-certificates git gcc gcc-c++ make pkg-config openssl-devel unzip python3 2>/dev/null || true
            elif command -v apk >/dev/null 2>&1; then
                info "Installing system dependencies (apk)..."
                maybe_sudo apk add curl ca-certificates git build-base openssl-dev pkgconfig unzip python3 2>/dev/null || true
            else
                warn "Unknown package manager. Ensure curl, git, build tools are installed."
            fi
            ;;
        macos)
            if ! command -v xcode-select >/dev/null 2>&1; then
                info "Installing Xcode CLI tools..."
                xcode-select --install 2>/dev/null || true
            fi
            if command -v brew >/dev/null 2>&1; then
                info "Installing dependencies (brew)..."
                brew install curl git 2>/dev/null || true
            fi
            ;;
    esac
}

# ─── Download prebuilt release ──────────────────────────────────────────────
download_release() {
    local platform="$1"
    local os
    os=$(detect_os)

    # Determine archive extension and download URL
    local ext="tar.gz"
    [ "$os" = "windows" ] && ext="zip"

    local url
    if [ "$VERSION" = "latest" ]; then
        url="${REPO_URL}/releases/latest/download/termweb-${platform}.${ext}"
    else
        url="${REPO_URL}/releases/download/${VERSION}/termweb-${platform}.${ext}"
    fi

    info "Downloading release for ${platform}..."
    info "  ${url}"

    TEMP_DIR="$(mktemp -d)"
    local archive="${TEMP_DIR}/termweb-${platform}.${ext}"

    if ! download_with_retry "$url" "$archive" 3; then
        warn "Release download failed. ${platform} may not be built yet."
        return 1
    fi

    # Validate & extract
    mkdir -p "$DATA_DIR"

    if [ "$os" = "windows" ]; then
        if ! unzip -t "$archive" >/dev/null 2>&1; then
            warn "Downloaded archive is invalid/corrupt."
            return 1
        fi
        unzip -o "$archive" -d "$TEMP_DIR" 2>/dev/null || true
    else
        if ! tar tzf "$archive" >/dev/null 2>&1; then
            warn "Downloaded archive is invalid/corrupt."
            return 1
        fi
        tar xzf "$archive" -C "$TEMP_DIR" 2>/dev/null || true
    fi

    # Handle both tar.gz structures: with/without subdirectory
    if [ -d "$TEMP_DIR/$platform" ]; then
        cp -r "$TEMP_DIR/$platform"/* "$DATA_DIR/" 2>/dev/null || true
    else
        # Try extracting directly
        if [ "$os" = "windows" ]; then
            cp -r "$TEMP_DIR"/* "$DATA_DIR/" 2>/dev/null || true
        else
            tar xzf "$archive" -C "$DATA_DIR" 2>/dev/null || true
        fi
    fi

    # Verify essential files exist
    if [ -f "$DATA_DIR/bcli" ] || [ -f "$DATA_DIR/bcli.exe" ]; then
        chmod +x "$DATA_DIR/bcli" "$DATA_DIR/termweb-server" "$DATA_DIR/bai" "$DATA_DIR/termweb" 2>/dev/null || true
        ok "Prebuilt binaries extracted to ${DATA_DIR}"
        return 0
    fi

    warn "Release archive didn't contain expected binaries."
    return 1
}

# ─── Build from source ──────────────────────────────────────────────────────
build_from_source() {
    local src_dir="$DATA_DIR/source"
    mkdir -p "$src_dir"

    info "Building from source (may take 5-10 minutes)..."

    # Clone or pull
    if [ -d "$src_dir/.git" ]; then
        (cd "$src_dir" && git pull --ff-only)
    else
        git clone --depth 1 "${REPO_URL}.git" "$src_dir"
    fi

    cd "$src_dir"

    # Install npm deps
    info "Installing npm dependencies..."
    npm install --omit=optional 2>&1 | tail -2 || true

    # Build Rust client
    info "Building Rust client (cargo build --release)..."
    (cd client-rs && cargo build --release 2>&1 | tail -5)
    cp client-rs/target/release/bcli "$DATA_DIR/bcli"
    chmod +x "$DATA_DIR/bcli"

    # Create termweb-server launcher
    cp -r server shared config "$DATA_DIR/" 2>/dev/null || true
    cat > "$DATA_DIR/termweb-server" << 'SEA'
#!/usr/bin/env node
require('./server/index.js');
SEA
    chmod +x "$DATA_DIR/termweb-server"

    # Copy other assets
    cp bin/bai "$DATA_DIR/bai"
    chmod +x "$DATA_DIR/bai"
    cp scripts/download-chrome.js "$DATA_DIR/download-chrome.js"
    cp -r node_modules "$DATA_DIR/node_modules" 2>/dev/null || true

    ok "Source build complete"
    cd "$OLDPWD"
}

# ─── Chrome setup ───────────────────────────────────────────────────────────
setup_chrome() {
    [ -n "${NO_CHROME:-}" ] && { info "Skipping Chrome download (NO_CHROME set)"; return; }

    local chrome_path_file="$DATA_DIR/chrome-path.txt"

    # Check if already configured
    if [ -f "$chrome_path_file" ]; then
        local existing
        existing=$(cat "$chrome_path_file")
        if [ -n "$existing" ] && [ -f "$existing" ]; then
            ok "Chrome already at: ${existing}"
            return
        fi
    fi

    # Check env var
    if [ -n "${CHROME_PATH:-}" ] && [ -f "$CHROME_PATH" ]; then
        echo "$CHROME_PATH" > "$chrome_path_file"
        ok "Chrome configured: ${CHROME_PATH}"
        return
    fi

    # Check system Chrome
    local system_chrome=""
    for c in google-chrome google-chrome-stable chromium chromium-browser chrome; do
        if command -v "$c" >/dev/null 2>&1; then
            system_chrome=$(command -v "$c")
            break
        fi
    done

    if [ -n "$system_chrome" ]; then
        info "Using system Chrome: ${system_chrome}"
        echo "$system_chrome" > "$chrome_path_file"
        return
    fi

    # Download Chrome for Testing via npx
    info "Downloading Chrome (headless browser for page rendering)..."
    if command -v npx >/dev/null 2>&1; then
        local platform_flag
        case "$(detect_os)" in
            linux)   platform_flag="linux" ;;
            macos)   platform_flag="mac" ;;
            windows) platform_flag="win32" ;;
        esac

        mkdir -p "$DATA_DIR/chrome"
        npx -y @puppeteer/browsers install chrome@stable \
            --path "$DATA_DIR/chrome" \
            --platform "$platform_flag" 2>&1 | tail -2 || \
        npx -y playwright install chromium 2>&1 | tail -2 || {
            warn "Chrome auto-download failed. Install Chrome manually or set CHROME_PATH."
            return
        }

        local chrome_binary
        chrome_binary=$(find "$DATA_DIR/chrome" -name "chrome" -o -name "chrome.exe" 2>/dev/null | head -1 || true)
        if [ -n "$chrome_binary" ]; then
            echo "$chrome_binary" > "$chrome_path_file"
            ok "Chrome downloaded: ${chrome_binary}"
        fi
    fi
}

# ─── Create launchers ───────────────────────────────────────────────────────
create_launchers() {
    mkdir -p "$INSTALL_DIR"

    # termweb master launcher
    cat > "$INSTALL_DIR/termweb" << 'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail
DATA_DIR="${HOME}/.termweb"
PID_FILE="/tmp/termweb-server.pid"

show_help() {
    cat << 'HELP'
TermWeb Browser — Terminal-based web browser & automation tool

Usage:
  termweb                          Connect to running server (default)
  termweb --server [--url URL]     Start server + connect
  termweb -s [--url URL]           Short alias for --server

Server options:
  --chrome <path>    Chrome binary path (default: auto-detect)
  --port <port>      Server port (default: 9222)
  --url <url>        Initial URL to navigate to

Controls (text mode):
  Arrow keys         Navigate elements / scroll
  Enter              Click focused element
  Tab/Shift+Tab      Cycle focus
  Ctrl+L             Enter URL
  Ctrl+F             Find in page
  Ctrl+R             Refresh
  Ctrl+0             Browser mode (settings panel)
  Ctrl+B / Alt+←     Go back
  Ctrl+N / Alt+→     Go forward
  Ctrl+T             New tab
  Ctrl+W             Close tab
  Ctrl+1-9           Switch tab
  Ctrl+C             Disconnect (server keeps running)

CLI commands (`termweb nav https://x.com`):
  nav <url>          Navigate to URL
  text               Extract page text
  links              Extract all links
  elements           Extract interactive elements
  click <selector>   Click element
  type <sel> <text>  Type into element
  eval <code>        Run JavaScript
  screenshot [path]  Take screenshot
  status             Session status
  back/forward       History navigation
  scroll <px>        Scroll page
  wait <ms>          Wait
  find <text>        Find in page
  session <action>   Create/destroy/status

Examples:
  termweb --server --url https://example.com   # Quick start
  termweb https://example.com                   # Navigate
  termweb text                                  # Read page
  termweb click '#btn'                          # Click button
HELP
}

if [ "$*" = "--help" ] || [ "$*" = "-h" ]; then
    show_help
    exit 0
fi

BIN_DIR="$(cd "$(dirname "$0")" && pwd)"
BROWSER_BIN="${BIN_DIR}/bcli"
SERVER_BIN="${BIN_DIR}/termweb-server"

# Start server if --server flag or no server running
START_SERVER=false
if [[ " $* " == *" --server "* ]] || [[ " $* " == *" -s "* ]]; then
    START_SERVER=true
fi
if [ ! -f "$PID_FILE" ] || ! kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    START_SERVER=true
fi

if [ "$START_SERVER" = true ]; then
    if [ -f "${DATA_DIR}/chrome-path.txt" ]; then
        CHROME_PATH="$(cat "${DATA_DIR}/chrome-path.txt")"
    fi

    ARGS=""
    [ -n "${CHROME_PATH:-}" ] && ARGS="$ARGS --chrome \"$CHROME_PATH\""
    [ -n "${TERMWEB_PORT:-}" ] && ARGS="$ARGS --port ${TERMWEB_PORT}"

    # Extract URL from args
    for arg in "$@"; do
        case "$arg" in
            --url=*) ARGS="$ARGS $arg" ;;
            http://*|https://*) ARGS="$ARGS --url $arg" ;;
        esac
    done

    export NODE_ENV=production
    nohup "$SERVER_BIN" $ARGS > "${DATA_DIR}/server.log" 2>&1 &
    echo $! > "$PID_FILE"
    sleep 2

    if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo "Server started (PID $(cat "$PID_FILE"))"
    else
        echo "Server failed. Check ${DATA_DIR}/server.log"
        exit 1
    fi

    # Remove --server and -s from args before forwarding to bcli
    NEW_ARGS=()
    for arg in "$@"; do
        case "$arg" in --server|-s) ;; *) NEW_ARGS+=("$arg") ;; esac
    done
    set -- "${NEW_ARGS[@]}"
fi

exec "$BROWSER_BIN" "$@"
LAUNCHER
    chmod +x "$INSTALL_DIR/termweb"

    # Symlink individual commands
    for bin in bcli termweb-server bai; do
        if [ -f "$DATA_DIR/$bin" ]; then
            ln -sf "$DATA_DIR/$bin" "$INSTALL_DIR/$bin" 2>/dev/null || true
        fi
    done

    ok "Launchers created in ${INSTALL_DIR}"
}

# ─── PATH setup ─────────────────────────────────────────────────────────────
add_to_path() {
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) return ;;
    esac

    local rc_files=()
    [ -f "$HOME/.zshrc" ] && rc_files+=("$HOME/.zshrc")
    [ -f "$HOME/.bashrc" ] && rc_files+=("$HOME/.bashrc")
    [ -f "$HOME/.bash_profile" ] && [ ! -f "$HOME/.bashrc" ] && rc_files+=("$HOME/.bash_profile")
    [ -f "$HOME/.config/fish/config.fish" ] && rc_files+=("$HOME/.config/fish/config.fish")
    [ ${#rc_files[@]} -eq 0 ] && rc_files+=("$HOME/.bashrc")

    for rc in "${rc_files[@]}"; do
        if [ -f "$rc" ]; then
            {
                echo ""
                echo "# Added by termweb-browser installer"
                echo "export PATH=\"\$PATH:${INSTALL_DIR}\""
            } >> "$rc"
            ok "Added ${INSTALL_DIR} to ${rc}"
            return
        fi
    done
    # Fallback
    echo "export PATH=\"\$PATH:${INSTALL_DIR}\"" >> "$HOME/.bashrc"
    ok "Added PATH to ~/.bashrc"
}

# ─── Install AI agent skill ────────────────────────────────────────────────
install_skill() {
    local skill_script
    skill_script="$(dirname "$0")/install-skill.sh"
    if [ -f "$skill_script" ]; then
        header "AI Agent Skill Installation"
        info "Detecting AI agents and installing browser skill..."
        bash "$skill_script" 2>&1 | sed 's/^/  /' || true
    fi
}

# ─── Help ──────────────────────────────────────────────────────────────────
show_help() {
    cat << 'HELP'
╔══════════════════════════════════════════════════════════╗
║    TermWeb Browser — Zero-Setup Installer                 ║
╚══════════════════════════════════════════════════════════╝

Usage:
  curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/scripts/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/scripts/install.sh | bash -s v1.0.0

Environment:
  INSTALL_DIR    Binary installation directory  (default: ~/.local/bin)
  DATA_DIR       Data directory                 (default: ~/.termweb)
  CHROME_PATH    Path to existing Chrome        (skip auto-download)
  TERMWEB_PORT   Server port                    (default: 9222)
  NO_BUILD       Skip source build fallback     (any non-empty)
  NO_CHROME      Skip Chrome download           (any non-empty)
  GITHUB_TOKEN   For authenticated API requests

What it does:
  1. Detects OS + architecture
  2. Installs system dependencies (curl, git, build tools)
  3. Installs Node.js + Rust if needed (for source build fallback)
  4. Downloads prebuilt binary from GitHub Releases
  5. Falls back to source build if no prebuilt release
  6. Downloads Chrome/Chromium for headless rendering
  7. Creates 'termweb' launcher in INSTALL_DIR
  8. Adds INSTALL_DIR to PATH
  9. Installs AI agent skill for opencode, claude, etc.

After install:
  termweb --server --url https://example.com
  bcli text
  bai status
HELP
    exit 0
}

# ─── Check Node.js and Rust (for source build) ─────────────────────────────
ensure_node() {
    if command -v node >/dev/null 2>&1; then
        return
    fi
    warn "Node.js not found. Installing via nvm..."
    export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
    if [ ! -s "$NVM_DIR/nvm.sh" ]; then
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
        else
            wget -qO- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
        fi
    fi
    [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
    nvm install 22 2>/dev/null || nvm install --lts 2>/dev/null
    nvm use 22 2>/dev/null || nvm use default 2>/dev/null
    command -v node >/dev/null 2>&1 || err "Node.js install failed."
    ok "Node.js $(node -v) installed"
}

ensure_rust() {
    if command -v cargo >/dev/null 2>&1; then
        return
    fi
    warn "Rust not found. Installing via rustup..."
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    else
        wget -qO- https://sh.rustup.rs | sh -s -- -y
    fi
    [ -s "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
    command -v cargo >/dev/null 2>&1 || err "Rust install failed."
    ok "Rust $(rustc --version) installed"
}

# ─── Main ──────────────────────────────────────────────────────────────────
main() {
    for arg in "$@"; do
        case "$arg" in --help|-h) show_help ;; esac
    done

    echo ""
    printf "\033[1;36m╔══════════════════════════════════════════════╗\033[0m\n"
    printf "\033[1;36m║   TermWeb Browser — Production Installer     ║\033[0m\n"
    printf "\033[1;36m╚══════════════════════════════════════════════╝\033[0m\n"
    echo ""

    local os arch platform
    os=$(detect_os)
    arch=$(detect_arch)
    platform=$(release_platform)
    info "Detected: ${os} ${arch}  →  package: ${platform}"

    # Step 1: System deps
    header "System Dependencies"
    install_system_deps

    # Step 2: Try prebuilt binary
    header "Installing termweb-browser"
    if download_release "$platform"; then
        ok "Prebuilt binary installed"
    elif [ -n "${NO_BUILD:-}" ]; then
        err "No prebuilt binary for ${platform} and NO_BUILD is set. Create a release first."
    else
        warn "No prebuilt release for ${platform}. Building from source..."
        ensure_node
        ensure_rust
        build_from_source
    fi

    # Step 3: Chrome
    header "Chrome/Chromium"
    setup_chrome

    # Step 4: Launchers
    header "Launcher Scripts"
    create_launchers

    # Step 5: PATH
    header "PATH Setup"
    add_to_path

    # Step 6: AI Agent skill
    install_skill

    # Done
    echo ""
    ok "Installation complete!"
    echo ""
    echo "  Quick start:"
    echo "    termweb --server --url https://example.com"
    echo ""
    echo "  Commands:"
    echo "    termweb <command>     Interactive or CLI mode"
    echo "    bcli <command>        Same as above"
    echo "    bai status            AI agent: check status"
    echo "    termweb-server        Start daemon only"
    echo ""

    # Remind about PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo "  Restart your shell or run:"
            echo "    export PATH=\"\$PATH:${INSTALL_DIR}\""
            echo ""
            ;;
    esac
}

main "$@"
