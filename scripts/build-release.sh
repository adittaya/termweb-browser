#!/usr/bin/env bash
# Builds termweb-browser release artifacts for all platforms.
# Usage: ./scripts/build-release.sh [version]
set -euo pipefail

VERSION="${1:-$(git describe --tags --always 2>/dev/null || echo "dev")}"
DIST_DIR="$(pwd)/dist"
mkdir -p "$DIST_DIR"

echo "━━━ Building termweb-browser release ${VERSION} ━━━"

# ─── Platform matrix ─────────────────────────────────────────────────────────
# Format: target-triple  |  rust-target  |  dir-suffix
BUILDS=(
    "x86_64-unknown-linux-gnu:x86_64-unknown-linux-gnu:linux-x64"
    "aarch64-unknown-linux-gnu:aarch64-unknown-linux-gnu:linux-arm64"
    "x86_64-pc-windows-msvc:x86_64-pc-windows-msvc:windows-x64"
    "x86_64-apple-darwin:x86_64-apple-darwin:macos-x64"
    "aarch64-apple-darwin:aarch64-apple-darwin:macos-arm64"
)

build_rust_client() {
    local rust_target="$1"
    local dir_name="$2"

    echo "→ Building Rust client for ${rust_target}..."
    (cd client-rs && cross build --release --target "${rust_target}")

    local src
    if [[ "$rust_target" == *"windows"* ]]; then
        src="client-rs/target/${rust_target}/release/bcli.exe"
    else
        src="client-rs/target/${rust_target}/release/bcli"
    fi

    mkdir -p "${DIST_DIR}/${dir_name}"
    cp "$src" "${DIST_DIR}/${dir_name}/bcli" 2>/dev/null || cp "$src" "${DIST_DIR}/${dir_name}/bcli.exe"
    echo "  ✓ bcli built"
}

bundle_server() {
    local node_target="$1"
    local dir_name="$2"

    echo "→ Bundling server for ${node_target}..."

    # Use pkg to create standalone server binary
    npx -y @yao-pkg/pkg . \
        --targets "${node_target}" \
        --output "${DIST_DIR}/${dir_name}/termweb-server" \
        --compress GZip \
        2>/dev/null || {
        echo "  ! pkg failed, falling back to source distribution"
        # Fallback: copy server files
        mkdir -p "${DIST_DIR}/${dir_name}/server" "${DIST_DIR}/${dir_name}/shared" "${DIST_DIR}/${dir_name}/config"
        cp -r server/*.js "${DIST_DIR}/${dir_name}/server/"
        cp -r shared/*.js "${DIST_DIR}/${dir_name}/shared/"
        cp config/default.js "${DIST_DIR}/${dir_name}/config/"
        cat > "${DIST_DIR}/${dir_name}/termweb-server" << 'SEA'
#!/usr/bin/env node
require('./server/index.js');
SEA
        chmod +x "${DIST_DIR}/${dir_name}/termweb-server"
    }
    echo "  ✓ termweb-server bundled"
}

copy_assets() {
    local dir_name="$1"
    cp scripts/download-chrome.js "${DIST_DIR}/${dir_name}/"
    cp bin/bai "${DIST_DIR}/${dir_name}/bai"
    chmod +x "${DIST_DIR}/${dir_name}/bai"

    # Create version file
    echo "${VERSION}" > "${DIST_DIR}/${dir_name}/VERSION"

    # Create README
    cat > "${DIST_DIR}/${dir_name}/README.txt" << README
TermWeb Browser ${VERSION}
==========================
See https://github.com/anomalyco/termweb-browser

Quick start:
  ./termweb-server --url https://example.com   # Terminal 1: start server
  ./bcli                                       # Terminal 2: connect client

Or combined:
  ./bcli --server --url https://example.com

AI agent mode (no graphics):
  ./bai status
  ./bai navigate https://example.com
  ./bai text
README
}

package_archive() {
    local dir_name="$1"

    echo "→ Packaging ${dir_name}..."
    cd "$DIST_DIR"

    if [[ "$dir_name" == *"windows"* ]]; then
        zip -r "termweb-${dir_name}.zip" "${dir_name}/"
        echo "  ✓ termweb-${dir_name}.zip"
    else
        tar czf "termweb-${dir_name}.tar.gz" "${dir_name}/"
        echo "  ✓ termweb-${dir_name}.tar.gz"
    fi
    rm -rf "${dir_name}"
    cd - >/dev/null
}

# Node.js pkg targets
declare -A NODE_TARGETS
NODE_TARGETS["linux-x64"]="node22-linux-x64"
NODE_TARGETS["linux-arm64"]="node22-linux-arm64"
NODE_TARGETS["windows-x64"]="node22-win-x64"
NODE_TARGETS["macos-x64"]="node22-macos-x64"
NODE_TARGETS["macos-arm64"]="node22-macos-arm64"

# ─── Build all platforms ─────────────────────────────────────────────────────
for build in "${BUILDS[@]}"; do
    IFS=":" read -r node_target rust_target dir_name <<< "$build"

    echo ""
    echo "═══ Platform: ${dir_name} ═══"

    build_rust_client "$rust_target" "$dir_name"
    bundle_server "${NODE_TARGETS[$dir_name]}" "$dir_name"
    copy_assets "$dir_name"
    package_archive "$dir_name"
done

echo ""
echo "━━━ Release ${VERSION} complete ━━━"
echo "Artifacts in: ${DIST_DIR}"
ls -lh "${DIST_DIR}/"
