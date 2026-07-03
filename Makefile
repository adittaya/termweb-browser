.PHONY: all install server bcli release clean

all: bcli

# ─── Development ──────────────────────────────────────────────────────────────

## Build Rust client (debug)
bcli:
	cd client-rs && cargo build

## Build Rust client (release)
bcli-release:
	cd client-rs && cargo build --release

## Install npm dependencies
deps:
	npm install

## Start server
server:
	node server/index.js

## Start server with a URL
server-url:
	node server/index.js --url $(url)

## Install global symlinks
install:
	node bin/install.js

# ─── Release ──────────────────────────────────────────────────────────────────

## Build release artifacts for all platforms
release:
	bash scripts/build-release.sh

## Build Rust client only (release)
release-bcli:
	cd client-rs && cargo build --release

## Bundle server with pkg
release-server:
	npm run build:server

# ─── Utility ──────────────────────────────────────────────────────────────────

## Clean build artifacts
clean:
	cd client-rs && cargo clean
	rm -rf dist/
	rm -rf node_modules/

## Show available targets
help:
	@echo "Targets:"
	@echo "  make bcli         — Build Rust client (debug)"
	@echo "  make bcli-release — Build Rust client (release)"
	@echo "  make deps         — Install npm dependencies"
	@echo "  make server       — Start development server"
	@echo "  make install      — Install global symlinks"
	@echo "  make release      — Build all release artifacts"
	@echo "  make clean        — Clean build artifacts"
