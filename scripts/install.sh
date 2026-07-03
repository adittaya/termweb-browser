#!/usr/bin/env bash
set -euo pipefail

REPO="adittaya/termweb-browser"
REPO_URL="https://github.com/${REPO}"
VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
DATA_DIR="${DATA_DIR:-$HOME/.termweb}"

# ─── Colors ───────────────────────────────────────────────────────────────────
info()  { printf "\033[1;34m•\033[0m %s\n" "$*"; }
ok()    { printf "\033[1;32m✓\033[0m %s\n" "$*"; }
warn()  { printf "\033[1;33m!\033[0m %s\n" "$*" >&2; }
err()   { printf "\033[1;31m✗\033[0m %s\n" "$*" >&2; exit 1; }

# ─── Platform detection ───────────────────────────────────────────────────────
detect_os() {
    case "$(uname -s)" in
        Linux)  echo "linux" ;;
        Darwin) echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)      err "Unsupported OS: $(uname -s)" ;;
    esac
}
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        armv7l)        echo "armv7" ;;
        *) err "Unsupported architecture: $(uname -m)" ;;
    esac
}
detect_platform() { echo "$(detect_arch)-$(detect_os)"; }

# ─── Sudo helper (skip if root or sudo missing) ──────────────────────────────────
maybe_sudo() {
    if [ "$(id -u)" = "0" ] || ! command -v sudo >/dev/null 2>&1; then
        "$@"
    else
        sudo "$@"
    fi
}

# ─── System dep installer ─────────────────────────────────────────────────────
install_system_deps() {
    local os
    os=$(detect_os)

    case "$os" in
        linux)
            if command -v apt-get >/dev/null 2>&1; then
                info "Installing system dependencies (apt)..."
                maybe_sudo apt-get update -qq
                maybe_sudo apt-get install -y -qq curl wget git build-essential pkg-config libssl-dev unzip python3 2>/dev/null || true
            elif command -v dnf >/dev/null 2>&1; then
                info "Installing system dependencies (dnf)..."
                maybe_sudo dnf install -y curl wget git gcc gcc-c++ make pkg-config openssl-devel unzip python3 2>/dev/null || true
            elif command -v apk >/dev/null 2>&1; then
                info "Installing system dependencies (apk)..."
                maybe_sudo apk add curl wget git build-base openssl-dev pkgconfig unzip python3 2>/dev/null || true
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
                brew install curl wget git 2>/dev/null || true
            fi
            ;;
        windows) ;; # Assume Git Bash / MSYS2 has deps
    esac
}

# ─── Check prerequisites ──────────────────────────────────────────────────────
check_prereqs() {
    for cmd in curl wget; do
        if command -v "$cmd" >/dev/null 2>&1; then
            DOWNLOAD_CMD="$cmd"
            break
        fi
    done
    [ -n "${DOWNLOAD_CMD:-}" ] || err "Need curl or wget."

    # Check git (needed for source build fallback)
    command -v git >/dev/null 2>&1 || warn "git not found. Source build will fail."

    # Check/install Node.js via nvm
    if ! command -v node >/dev/null 2>&1; then
        warn "Node.js not found. Installing via nvm..."
        export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
        # Install nvm
        if [ ! -s "$NVM_DIR/nvm.sh" ]; then
            if [ "$DOWNLOAD_CMD" = "curl" ]; then
                curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
            else
                wget -qO- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
            fi
        fi
        # Source and install Node
        [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
        nvm install 22 2>/dev/null || nvm install --lts 2>/dev/null
        nvm use 22 2>/dev/null || nvm use default 2>/dev/null
        # Re-check
        command -v node >/dev/null 2>&1 || err "Node.js install failed."
        ok "Node.js $(node -v) installed"
    fi
    # Ensure npm and npx are available
    command -v npm >/dev/null 2>&1 || err "npm not found."
    command -v npx >/dev/null 2>&1 || npm install -g npx 2>/dev/null || true

    # Check/install Rust via rustup
    if ! command -v cargo >/dev/null 2>&1; then
        warn "Rust not found. Installing via rustup..."
        if [ "$DOWNLOAD_CMD" = "curl" ]; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        else
            wget -qO- https://sh.rustup.rs | sh -s -- -y
        fi
        [ -s "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
        command -v cargo >/dev/null 2>&1 || err "Rust install failed."
        ok "Rust $(rustc --version) installed"
    fi
}

# ─── Download pre-built release ────────────────────────────────────────────────
download_release() {
    local platform="$1"
    local url

    if [ "$VERSION" = "latest" ]; then
        url="${REPO_URL}/releases/latest/download/termweb-${platform}.tar.gz"
    else
        url="${REPO_URL}/releases/download/${VERSION}/termweb-${platform}.tar.gz"
    fi

    info "Downloading termweb for ${platform}..."
    mkdir -p "$DATA_DIR"

    local tmpdir
    tmpdir=$(mktemp -d)
    local archive="${tmpdir}/termweb.tar.gz"

    if [ "$DOWNLOAD_CMD" = "curl" ]; then
        curl -fsSL "$url" -o "$archive"
    else
        wget -qO "$archive" "$url"
    fi

    # Verify it's a valid tar
    if ! tar tzf "$archive" >/dev/null 2>&1; then
        rm -rf "$tmpdir"
        return 1
    fi

    tar xzf "$archive" -C "$DATA_DIR" --strip-components=0 2>/dev/null || \
    tar xzf "$archive" -C "$DATA_DIR" 2>/dev/null || {
        rm -rf "$tmpdir"
        return 1
    }

    rm -rf "$tmpdir"

    # Verify essential files exist
    if [ -f "$DATA_DIR/bcli" ] || [ -f "$DATA_DIR/termweb-server" ]; then
        chmod +x "$DATA_DIR/bcli" "$DATA_DIR/termweb-server" "$DATA_DIR/bai" 2>/dev/null || true
        ok "Extracted to $DATA_DIR"
        return 0
    fi
    return 1
}

# ─── Build from source ────────────────────────────────────────────────────────
build_from_source() {
    local src_dir="$DATA_DIR/source"
    mkdir -p "$src_dir"

    info "Building from source..."

    # Clone or pull
    if [ -d "$src_dir/.git" ]; then
        (cd "$src_dir" && git pull --ff-only)
    else
        git clone "${REPO_URL}.git" "$src_dir"
    fi

    cd "$src_dir"

    info "Installing npm dependencies..."
    npm install --omit=optional 2>&1 | tail -3 || npm install 2>&1 | tail -3

    info "Building Rust client (this may take a while)..."
    (cd client-rs && cargo build --release)

    info "Copying binaries to ${DATA_DIR}..."
    cp client-rs/target/release/bcli "$DATA_DIR/bcli"
    chmod +x "$DATA_DIR/bcli"

    # Create server launcher
    cat > "$DATA_DIR/termweb-server" << 'SEA_LAUNCHER'
#!/usr/bin/env node
require('./server/index.js');
SEA_LAUNCHER
    chmod +x "$DATA_DIR/termweb-server"
    cp -r server "$DATA_DIR/server"
    cp -r shared "$DATA_DIR/shared"
    cp -r config "$DATA_DIR/config"

    # Copy bai
    cp bin/bai "$DATA_DIR/bai"
    chmod +x "$DATA_DIR/bai"
    cp scripts/download-chrome.js "$DATA_DIR/download-chrome.js"

    # Copy node_modules for server
    cp -r node_modules "$DATA_DIR/node_modules" 2>/dev/null || true

    cd "$DATA_DIR"
    ok "Build complete"
}

# ─── Download Chrome ──────────────────────────────────────────────────────────
setup_chrome() {
    local chrome_path_file="$DATA_DIR/chrome-path.txt"

    if [ -f "$chrome_path_file" ]; then
        local existing
        existing=$(cat "$chrome_path_file")
        if [ -n "$existing" ] && [ -f "$existing" ]; then
            ok "Chrome already at: $existing"
            return
        fi
    fi

    info "Downloading Chrome (headless browser for page rendering)..."

    # Check if system Chrome exists
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
        ok "Chrome configured: ${system_chrome}"
        return
    fi

    # Try using @puppeteer/browsers to download Chrome for Testing
    if command -v npx >/dev/null 2>&1; then
        local platform_flag
        case "$(detect_os)" in
            linux)   platform_flag="linux" ;;
            macos)   platform_flag="mac" ;;
            windows) platform_flag="win32" ;;
        esac

        info "Downloading Chrome via @puppeteer/browsers..."
        mkdir -p "$DATA_DIR/chrome"
        npx -y @puppeteer/browsers install chrome@stable \
            --path "$DATA_DIR/chrome" \
            --platform "$platform_flag" 2>&1 | tail -3 || \
        npx -y playwright install chromium 2>&1 | tail -3 || {
            warn "Auto Chrome download failed."
            warn "Install Chrome manually or set CHROME_PATH env var."
            return
        }

        local chrome_binary
        chrome_binary=$(find "$DATA_DIR/chrome" -name "chrome" -o -name "chrome.exe" 2>/dev/null | head -1 || true)
        if [ -n "$chrome_binary" ]; then
            echo "$chrome_binary" > "$chrome_path_file"
            ok "Chrome ready: ${chrome_binary}"
        else
            # Try to find in standard puppeteer path
            chrome_binary=$(find "$HOME/.cache" -name "chrome" -type f 2>/dev/null | head -1 || true)
            if [ -n "$chrome_binary" ]; then
                echo "$chrome_binary" > "$chrome_path_file"
                ok "Chrome found: ${chrome_binary}"
            fi
        fi
    fi
}

# ─── Create launcher scripts ──────────────────────────────────────────────────
create_launchers() {
    mkdir -p "$INSTALL_DIR"

    # termweb — master launcher
    cat > "$INSTALL_DIR/termweb" << 'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail
DATA_DIR="${HOME}/.termweb"
CHROME_PATH="$(cat "${DATA_DIR}/chrome-path.txt" 2>/dev/null || echo "")"
PID_FILE="/tmp/termweb-server.pid"

# Handle --help
if [ "$*" = "--help" ] || [ "$*" = "-h" ]; then
    echo "TermWeb Browser — Terminal-based web browser"
    echo ""
    echo "Usage:"
    echo "  termweb                          Connect to running server"
    echo "  termweb --server <url>           Start server + connect"
    echo "  termweb --server --url <url>     (same)"
    echo ""
    echo "Server options:"
    echo "  --chrome <path>    Chrome binary path"
    echo "  --port <port>      Server port (default: 9222)"
    echo "  --url <url>        Initial URL to navigate to"
    echo ""
    echo "Examples:"
    echo "  termweb --server --url https://example.com"
    exit 0
fi

# Start server if requested (or no server running)
START_SERVER=false
if [ "$*" = "--server" ] || [ "$*" = "-s" ] || [[ "$*" == *"--server"* ]]; then
    START_SERVER=true
fi
if [ ! -f "$PID_FILE" ] || ! kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    START_SERVER=true
fi

if [ "$START_SERVER" = true ]; then
    # Ensure Chrome exists
    if [ ! -f "$CHROME_PATH" ]; then
        echo "Chrome not found. Downloading..."
        node "$(dirname "$0")/../download-chrome.js" 2>/dev/null || \
        node "$DATA_DIR/download-chrome.js" 2>/dev/null || true
        CHROME_PATH="$(cat "${DATA_DIR}/chrome-path.txt" 2>/dev/null || echo "")"
    fi

    echo "Starting termweb-server..."
    SERVER_BIN="${DATA_DIR}/termweb-server"
    if [ ! -f "$SERVER_BIN" ]; then
        # Try node-based server
        if [ -f "${DATA_DIR}/server/index.js" ]; then
            SERVER_BIN="node ${DATA_DIR}/server/index.js"
        else
            echo "Error: termweb-server not found in ${DATA_DIR}"
            exit 1
        fi
    fi

    ARGS=""
    if [ -n "$CHROME_PATH" ]; then
        ARGS="$ARGS --chrome \"$CHROME_PATH\""
    fi
    if [ -n "${TERMWEB_PORT:-}" ]; then
        ARGS="$ARGS --port ${TERMWEB_PORT}"
    fi

    # Extract URL from args if present
    for arg in "$@"; do
        case "$arg" in
            --url=*) ARGS="$ARGS $arg" ;;
            --url) ;; # skip, next arg will be the url
            http://*|https://*) ARGS="$ARGS --url $arg" ;;
        esac
    done

    eval "nohup $SERVER_BIN $ARGS > \"${DATA_DIR}/server.log\" 2>&1 &"
    echo $! > "$PID_FILE"
    sleep 2

    if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo "Server started (PID $(cat "$PID_FILE"))"
    else
        echo "Server failed to start. Check ${DATA_DIR}/server.log"
        exit 1
    fi
fi

# Launch client
exec "$DATA_DIR/bcli" "$@"
LAUNCHER
    chmod +x "$INSTALL_DIR/termweb"

    # Convenience symlinks
    for bin in bcli termweb-server bai; do
        if [ -f "$DATA_DIR/$bin" ]; then
            ln -sf "$DATA_DIR/$bin" "$INSTALL_DIR/$bin" 2>/dev/null || true
        fi
    done

    ok "Launchers created in $INSTALL_DIR"
}

# ─── PATH setup ────────────────────────────────────────────────────────────────
add_to_path() {
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) return ;;
    esac

    local rc_files=()
    if [ -f "$HOME/.zshrc" ]; then rc_files+=("$HOME/.zshrc"); fi
    if [ -f "$HOME/.bashrc" ]; then rc_files+=("$HOME/.bashrc"); fi
    if [ -f "$HOME/.bash_profile" ] && [ ! -f "$HOME/.bashrc" ]; then rc_files+=("$HOME/.bash_profile"); fi
    if [ -f "$HOME/.config/fish/config.fish" ]; then rc_files+=("$HOME/.config/fish/config.fish"); fi
    # If no rc file found, create .bashrc
    if [ ${#rc_files[@]} -eq 0 ]; then
        rc_files+=("$HOME/.bashrc")
    fi

    local added=false
    for rc in "${rc_files[@]}"; do
        if [ -f "$rc" ]; then
            echo "" >> "$rc"
            echo "# Added by termweb-browser installer" >> "$rc"
            echo "export PATH=\"\$PATH:${INSTALL_DIR}\"" >> "$rc"
            ok "Added ${INSTALL_DIR} to ${rc}"
            added=true
            break
        fi
    done

    if [ "$added" = false ]; then
        echo "export PATH=\"\$PATH:${INSTALL_DIR}\"" >> "$HOME/.bashrc"
        ok "Added ${INSTALL_DIR} to ~/.bashrc"
    fi
}

# ─── Help ──────────────────────────────────────────────────────────────────────
show_help() {
    cat << 'HELP'
TermWeb Browser — One-line Installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/scripts/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/adittaya/termweb-browser/main/scripts/install.sh | bash -s v1.0.0

Environment variables:
  INSTALL_DIR    Binary installation directory (default: ~/.local/bin)
  DATA_DIR       Data directory for binaries and config (default: ~/.termweb)
  CHROME_PATH    Path to existing Chrome/Chromium binary (skip auto-download)
  TERMWEB_PORT   Server port (default: 9222)

What it does:
  1. Detects your OS and architecture
  2. Installs Node.js (via nvm) and Rust (via rustup) if missing
  3. Downloads pre-built release from GitHub, or builds from source
  4. Downloads Chrome/Chromium for headless page rendering
  5. Creates 'termweb' command in ~/.local/bin
  6. Adds ~/.local/bin to your PATH

After install:
  termweb --server --url https://example.com
HELP
    exit 0
}

# ─── Main ──────────────────────────────────────────────────────────────────────
main() {
    for arg in "$@"; do
        case "$arg" in
            --help|-h) show_help ;;
        esac
    done

    echo ""
    printf "\033[1;36m╔══════════════════════════════════════════╗\033[0m\n"
    printf "\033[1;36m║   TermWeb Browser — Zero-Setup Installer  ║\033[0m\n"
    printf "\033[1;36m╚══════════════════════════════════════════╝\033[0m\n"
    echo ""

    local os arch platform
    os=$(detect_os)
    arch=$(detect_arch)
    platform="${arch}-${os}"
    info "Detected: ${platform}"

    install_system_deps
    check_prereqs

    # Try pre-built first
    if download_release "$platform"; then
        ok "Pre-built binary installed"
    else
        warn "No pre-built release for ${platform}. Building from source..."
        info "This will take 5-10 minutes on the first run."
        build_from_source
    fi

    setup_chrome
    create_launchers
    add_to_path

    echo ""
    ok "Installation complete!"
    echo ""
    echo "  Run:  termweb --server --url https://example.com"
    echo ""
    echo "  Or:   bcli -c ws://127.0.0.1:9222/browser"
    echo "        (if server is already running)"
    echo ""

    # Check if we need to source profile
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo "  Restart your shell or run:"
            echo "    export PATH=\"\$PATH:${INSTALL_DIR}\""
            echo "    termweb --server --url https://example.com"
            echo ""
            ;;
    esac
}

main "$@"
