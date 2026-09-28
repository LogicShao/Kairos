#!/usr/bin/env bash
set -Eeuo pipefail

SRC="${SRC:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
DEST="${DEST:-ali:~/proj/Kairos/}"
SSH_OPTS="${SSH_OPTS:-}"
DELETE="${DELETE:-1}"

log() { printf '[INFO] %s\n' "$*"; }

command -v rsync >/dev/null 2>&1 || { printf '[ERROR] 缺少 rsync\n' >&2; exit 1; }

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

log "创建远端目录 $DEST"
ssh ${SSH_OPTS} "${DEST%%:*}" 'mkdir -p ~/proj/Kairos'

log "rsync $SRC/ -> $DEST"
rsync "${RSYNC_ARGS[@]}" "$SRC/" "$DEST"

log "完成。远端 .env 需自行创建（未同步），随后执行 bash deploy/ali/deploy.sh"
