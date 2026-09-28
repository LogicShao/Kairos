#!/usr/bin/env bash
set -Eeuo pipefail

APP_NAME="${APP_NAME:-kairos}"
REPO_DIR="${REPO_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
COMPOSE_FILE="${COMPOSE_FILE:-compose.ali.yml}"
ENV_FILE="${ENV_FILE:-.env}"
REMOTE="${REMOTE:-origin}"
BRANCH="${BRANCH:-}"
GOPROXY="${GOPROXY:-https://goproxy.cn,direct}"
WEB_PORT="${KAIROS_WEB_PORT:-18081}"
HEALTH_URL="${HEALTH_URL:-http://127.0.0.1:${WEB_PORT}/}"
HEALTH_RETRIES="${HEALTH_RETRIES:-30}"
HEALTH_INTERVAL="${HEALTH_INTERVAL:-2}"

log() { printf '[INFO] %s\n' "$*"; }
fail() { printf '[ERROR] %s\n' "$*" >&2; exit 1; }

cd "$REPO_DIR"

command -v curl >/dev/null 2>&1 || fail "缺少 curl"

if docker info >/dev/null 2>&1; then
	DOCKER=(docker)
elif command -v sudo >/dev/null 2>&1 && sudo -n docker info >/dev/null 2>&1; then
	DOCKER=(sudo docker)
else
	fail "无 docker 权限：把用户加入 docker 组，或用 sudo -E bash $0 运行"
fi

compose() { "${DOCKER[@]}" compose -f "$COMPOSE_FILE" "$@"; }

[[ -f "$COMPOSE_FILE" ]] || fail "缺少 $COMPOSE_FILE"
[[ -f "$ENV_FILE" ]] || fail "缺少 $ENV_FILE：先 cp .env.example .env 并填写"

if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
	if [ "${ALLOW_DIRTY:-0}" != "1" ]; then
		git diff --quiet || fail "存在未提交改动（可设 ALLOW_DIRTY=1 跳过）"
		git diff --cached --quiet || fail "存在已暂存改动（可设 ALLOW_DIRTY=1 跳过）"
	fi
	if [ -n "$BRANCH" ]; then
		git fetch --prune "$REMOTE" "$BRANCH"
		git switch "$BRANCH"
		git merge --ff-only "$REMOTE/$BRANCH"
	fi
else
	log "非 git 工作区（rsync 直传模式），跳过版本检查与拉取"
fi

export GOPROXY
log "校验 compose 配置"
compose config >/dev/null

log "构建并启动容器"
compose up -d --build --remove-orphans

log "等待健康检查 $HEALTH_URL"
ok=0
for i in $(seq 1 "$HEALTH_RETRIES"); do
	web_code="$(curl -s -o /dev/null -w '%{http_code}' "$HEALTH_URL" || true)"
	# /api/auth/me 无 token 期望 401：同时验证 web 与 api 链路
	api_code="$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:${WEB_PORT}/api/auth/me" || true)"
	if [ "$web_code" = "200" ] && [ "$api_code" = "401" ]; then
		ok=1
		break
	fi
	log "未就绪（web=$web_code api=$api_code, ${i}/${HEALTH_RETRIES}）"
	sleep "$HEALTH_INTERVAL"
done

compose ps
if [ "$ok" != "1" ]; then
	compose logs --tail=120 >&2 || true
	fail "健康检查失败"
fi

log "$APP_NAME 部署完成（本地入口 http://127.0.0.1:${WEB_PORT}）"
log "下一步：在 1Panel 建站并反代到 http://127.0.0.1:${WEB_PORT}"
