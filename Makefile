.PHONY: dev build check test lint audit clean verify bump gen-password help go-build go-test go-lint go-dev up down logs ps db-dev db-stop

GO := go
API_PORT ?= 8080

# 本地 dev PostgreSQL 容器参数（与 server/internal/config 的默认 DATABASE_URL 一致）
DEV_PG_NAME ?= kairos-pg
DEV_PG_USER ?= kairos
DEV_PG_PASS ?= kairos_dev
DEV_PG_DB   ?= kairos_dev
DEV_DB_URL  ?= postgres://$(DEV_PG_USER):$(DEV_PG_PASS)@localhost:5432/$(DEV_PG_DB)

## ---------------------------------------------------------------- 后端（Go）

go-build: ## Build Go backend binary
	cd server && $(GO) build -o bin/api ./cmd/api

go-test: ## Run Go backend tests
	cd server && $(GO) test ./...

go-lint: ## Lint Go backend (gofumpt + golangci-lint if available)
	cd server && test -z "$$(gofumpt -l .)" && echo "  ✓ gofumpt clean" || (gofumpt -d . && exit 1)
	cd server && command -v golangci-lint >/dev/null && golangci-lint run ./... || echo "  (golangci-lint 未安装，跳过)"

go-dev: ## Start the Go API against the local dev database
	@echo "==> starting Go API on :$(API_PORT)"
	@cd server && DATABASE_URL="$(DEV_DB_URL)" KAIROS_API_PORT=$(API_PORT) $(GO) run ./cmd/api

gen-password: ## Print a bcrypt hash for a password (usage: make gen-password PASSWORD=secret)
	@test -n "$(PASSWORD)" || (echo "Usage: make gen-password PASSWORD=<password>" && exit 1)
	@cd server && $(GO) run ./cmd/genpassword "$(PASSWORD)"

## ------------------------------------------------------- 开发 / 构建 / 检查

dev: db-dev ## Start a throwaway dev PostgreSQL + Go API + Vite dev server
	@echo "==> starting Go API on :$(API_PORT)"
	@cd server && DATABASE_URL="$(DEV_DB_URL)" KAIROS_API_PORT=$(API_PORT) $(GO) run ./cmd/api & \
		npm run dev

build: ## Build the Go API binary and the frontend bundle
	cd server && $(GO) build -o bin/api ./cmd/api
	npm run build

check: ## Type-check the Go backend and the TypeScript frontend
	cd server && $(GO) vet ./...
	npx tsc --noEmit

test: ## Run backend tests
	cd server && $(GO) test ./...

lint: ## Lint the Go backend and the frontend
	cd server && test -z "$$(gofumpt -l .)" && echo "  ✓ gofumpt clean" || (gofumpt -d . && exit 1)
	cd server && command -v golangci-lint >/dev/null && golangci-lint run ./... || echo "  (golangci-lint 未安装，跳过)"
	npm run lint

audit: ## Offline compliance check — must return zero matches
	@grep -rn 'https\?://' src/ --exclude-dir=assets | grep -v 'placeholder=' && exit 1 || echo "  ✓ clean"

clean: ## Remove build artifacts
	rm -rf dist node_modules/.vite server/bin

verify: check lint test audit ## Full CI pipeline
	@echo "  ✓ all checks passed"

bump: ## Bump version (usage: make bump V=0.1.1)
	@test -n "$(V)" || (echo "Usage: make bump V=0.1.1" && exit 1)
	sed -i 's/"version": "[^"]*"/"version": "$(V)"/' package.json
	@echo "  ✓ bumped to $(V)"

## ---------------------------------------------------- 本地数据库 / 生产编排

db-dev: ## Start a throwaway PostgreSQL container for local development
	@docker rm -f $(DEV_PG_NAME) >/dev/null 2>&1 || true
	@docker run --rm -d --name $(DEV_PG_NAME) \
		-e POSTGRES_USER=$(DEV_PG_USER) \
		-e POSTGRES_PASSWORD=$(DEV_PG_PASS) \
		-e POSTGRES_DB=$(DEV_PG_DB) \
		-p 5432:5432 postgres:16-alpine >/dev/null
	@printf "==> waiting for PostgreSQL "; \
	until docker exec $(DEV_PG_NAME) pg_isready -U $(DEV_PG_USER) >/dev/null 2>&1; do printf "."; sleep 1; done; echo " ready"

db-stop: ## Stop and remove the dev PostgreSQL container
	@docker rm -f $(DEV_PG_NAME) >/dev/null 2>&1 && echo "  ✓ stopped $(DEV_PG_NAME)" || echo "  (no dev PostgreSQL running)"

up: ## Build and start the production stack (docker compose)
	docker compose up -d --build

down: ## Stop and remove the compose stack
	docker compose down

logs: ## Tail compose logs
	docker compose logs -f

ps: ## Show compose service status
	docker compose ps

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
