.PHONY: dev build check test lint audit clean version help

CARGO := cargo
MANIFEST := src-tauri/Cargo.toml

dev: ## Start Tauri dev server (hot reload)
	cargo tauri dev

build: ## Production build
	NO_STRIP=1 cargo tauri build

check: ## Type-check both Rust and TypeScript
	$(CARGO) check --manifest-path $(MANIFEST)
	npx tsc --noEmit

test: ## Run all tests
	$(CARGO) test --manifest-path $(MANIFEST) --lib
	@echo "  ✓ cargo test done"

lint: ## Lint both Rust and TypeScript
	$(CARGO) clippy --manifest-path $(MANIFEST) -- -D warnings
	npm run lint

audit: ## Offline compliance check — must return zero matches
	@echo "=== Frontend ==="
	@grep -rn 'https\?://' src/ && exit 1 || echo "  ✓ clean"
	@echo "=== Rust source ==="
	@grep -rn 'https\?://' src-tauri/src/ && exit 1 || echo "  ✓ clean"

clean: ## Remove build artifacts
	cargo clean --manifest-path $(MANIFEST)
	rm -rf dist node_modules/.vite

verify: check lint test audit ## Full CI pipeline
	@echo "  ✓ all checks passed"

bump: ## Bump version (usage: make bump V=0.1.1)
	@test -n "$(V)" || (echo "Usage: make bump V=0.1.1" && exit 1)
	sed -i 's/"version": "[^"]*"/"version": "$(V)"/' src-tauri/tauri.conf.json
	sed -i 's/version = "[^"]*"/version = "$(V)"/' src-tauri/Cargo.toml
	sed -i 's/"version": "[^"]*"/"version": "$(V)"/' package.json
	@echo "  ✓ bumped to $(V)"

android-dev: ## Start Tauri Android dev (emulator / USB)
	cargo tauri android dev

android-build-debug: ## Android debug APK
	cargo tauri android build --debug

android-build: ## Android release build (AAB + APK)
	cargo tauri android build

check-all: check ## Full check including Android target
	$(CARGO) check --target aarch64-linux-android --manifest-path $(MANIFEST)

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
