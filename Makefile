.PHONY: dev build check test lint audit clean version help go-build go-test go-lint go-dev

CARGO := cargo
MANIFEST := src-tauri/Cargo.toml
GO := go
GOMOD := server/go.mod
GOAPI := server/cmd/api
API_PORT ?= 8080

go-build: ## Build Go backend binary
	cd server && $(GO) build -o bin/api ./cmd/api

go-test: ## Run Go backend tests
	cd server && $(GO) test ./...

go-lint: ## Lint Go backend (gofumpt + golangci-lint if available)
	cd server && test -z "$$(gofumpt -l .)" && echo "  ✓ gofumpt clean" || (gofumpt -d . && exit 1)
	cd server && command -v golangci-lint >/dev/null && golangci-lint run ./... || echo "  (golangci-lint 未安装，跳过)"

go-dev: ## Start Go backend (placeholder) + Vite dev server
	@echo "==> starting Go API on :$(API_PORT)"
	@cd server && KAIROS_API_PORT=$(API_PORT) $(GO) run ./cmd/api &
	@npm run dev

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
	@grep -rn 'https\?://' src/ --exclude-dir=assets | grep -v 'placeholder=' && exit 1 || echo "  ✓ clean"
	# Rust 侧不检查：AI/LZU/WebDAV 的后端网络访问是功能需求（默认配置、业务地址、测试数据均为合法 URL）。

clean: ## Remove build artifacts
	cargo clean --manifest-path $(MANIFEST)
	rm -rf dist node_modules/.vite

verify: check lint test audit ## Full CI pipeline
	@echo "  ✓ all checks passed"

bump: ## Bump version (usage: make bump V=0.1.1)
	@test -n "$(V)" || (echo "Usage: make bump V=0.1.1" && exit 1)
	sed -i 's/"version": "[^"]*"/"version": "$(V)"/' src-tauri/tauri.conf.json
	sed -i 's/^version = "[^"]*"/version = "$(V)"/' src-tauri/Cargo.toml
	sed -i 's/"version": "[^"]*"/"version": "$(V)"/' package.json
	@echo "  ✓ bumped to $(V)"

android-dev: ## Start Tauri Android dev (emulator / USB)
	npm run android:dev

android-build-debug: ## Android debug APK
	npm run android:install-debug -- --rebuild

android-build: ## Android release build (AAB + APK)
	npm run android:build

check-all: check ## Full check including Android target
	$(CARGO) check --target aarch64-linux-android --manifest-path $(MANIFEST)

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
