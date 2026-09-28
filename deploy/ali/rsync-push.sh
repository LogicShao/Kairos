#!/usr/bin/env bash
set -Eeuo pipefail

MODE="${1:-push}"
SRC="${SRC:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
DEST="${DEST:-ali:~/proj/Kairos/}"
SSH_OPTS="${SSH_OPTS:-}"
DELETE="${DELETE:-1}"

REMOTE_HOST="${DEST%%:*}"
REMOTE_DIR="${DEST#*:}"
REMOTE_DIR="${REMOTE_DIR%/}"

log() { printf '[INFO] %s\n' "$*"; }

case "$MODE" in
push | all) ;;
*)
	printf '[ERROR] 用法: %s [push|all]\n' "$0" >&2
	exit 2
	;;
esac

command -v rsync >/dev/null 2>&1 || {
	printf '[ERROR] 缺少 rsync\n' >&2
	exit 1
}

RSYNC_ARGS=(-az -h --info=progress2
	-e "ssh ${SSH_OPTS}"
	--exclude '.git/'
	--exclude 'node_modules/'
	--exclude 'dist/'
	--exclude '.env'
	--exclude '.trellis/'
	--exclude '.omo/'
	--exclude '.agents/'
	--exclude '.codex/'
	--exclude '.claude/'
	--exclude 'deploy/certs/'
	--exclude 'server/bin/'
	--exclude '*.log'
	--exclude '.DS_Store')

if [ "$DELETE" = "1" ]; then
	RSYNC_ARGS+=(--delete)
fi

log "创建远端目录 ${REMOTE_HOST}:${REMOTE_DIR}"
ssh ${SSH_OPTS} "$REMOTE_HOST" "mkdir -p $REMOTE_DIR"

log "rsync $SRC/ -> $DEST"
rsync "${RSYNC_ARGS[@]}" "$SRC/" "$DEST"

if [ "$MODE" = "all" ]; then
	log "远端部署 ${REMOTE_HOST}:${REMOTE_DIR}"
	ssh ${SSH_OPTS} "$REMOTE_HOST" "cd $REMOTE_DIR && bash deploy/ali/deploy.sh"
	log "全部完成（.env 需已在远端创建）"
else
	log "推送完成。远端 .env 需自行创建（未同步）；随后远端执行 bash deploy/ali/deploy.sh，或用 all 模式一键完成"
fi
