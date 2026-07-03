#!/usr/bin/env bash
set -euo pipefail
# TermWeb Browser — master launcher
# Installed by scripts/install.sh or included in release archives.
DATA_DIR="${HOME}/.termweb"
CHROME_PATH=""
if [ -n "${CHROME_BIN:-}" ]; then
    CHROME_PATH="$CHROME_BIN"
elif [ -n "${CHROME_PATH_ENV:-}" ]; then
    CHROME_PATH="$CHROME_PATH_ENV"
elif [ -f "${DATA_DIR}/chrome-path.txt" ]; then
    CHROME_PATH="$(cat "${DATA_DIR}/chrome-path.txt")"
elif command -v google-chrome >/dev/null 2>&1; then
    CHROME_PATH="$(command -v google-chrome)"
elif command -v google-chrome-stable >/dev/null 2>&1; then
    CHROME_PATH="$(command -v google-chrome-stable)"
elif command -v chromium-browser >/dev/null 2>&1; then
    CHROME_PATH="$(command -v chromium-browser)"
elif command -v chromium >/dev/null 2>&1; then
    CHROME_PATH="$(command -v chromium)"
fi
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
        node "$(dirname "$0")/download-chrome.js" 2>/dev/null || \
        node "$DATA_DIR/download-chrome.js" 2>/dev/null || true
        CHROME_PATH="$(cat "${DATA_DIR}/chrome-path.txt" 2>/dev/null || echo "")"
    fi

    echo "Starting termweb-server..."
    SERVER_BIN="${DATA_DIR}/termweb-server"
    if [ ! -f "$SERVER_BIN" ]; then
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
    for arg in "$@"; do
        case "$arg" in
            --url=*) ARGS="$ARGS $arg" ;;
            --url) ;;
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

exec "$DATA_DIR/bcli" "$@"
