#!/bin/bash
set -euo pipefail

FRONTEND_PORT="${FRONTEND_PORT:-5173}"
DEV_HOST="${KAIROS_ANDROID_DEV_HOST:-127.0.0.1}"
USE_REVERSE=true
TAURI_ARGS=()

SDK_ADB="D:/software/code/Android/SDK/platform-tools/adb.exe"
MUMU_ADB="D:/software/game/MuMu/MuMuPlayer/nx_main/adb.exe"
if command -v adb >/dev/null 2>&1; then
  ADB="adb"
elif [ -f "$SDK_ADB" ]; then
  ADB="$SDK_ADB"
elif [ -f "$MUMU_ADB" ]; then
  ADB="$MUMU_ADB"
else
  ADB=""
fi

while [ "$#" -gt 0 ]; do
  case "$1" in
    --lan)
      USE_REVERSE=false
      shift
      ;;
    --host)
      if [ "$#" -lt 2 ]; then
        echo "用法: npm run android:dev -- --host <电脑局域网IP>"
        exit 1
      fi
      DEV_HOST="$2"
      USE_REVERSE=false
      shift 2
      ;;
    --no-reverse)
      USE_REVERSE=false
      shift
      ;;
    *)
      TAURI_ARGS+=("$1")
      shift
      ;;
  esac
done

if [ "$USE_REVERSE" = true ]; then
  echo "=== Android dev: USB/模拟器 reverse 模式 ==="
  echo "  应用访问: http://127.0.0.1:$FRONTEND_PORT"
  if [ -n "$ADB" ]; then
    "$ADB" reverse "tcp:$FRONTEND_PORT" "tcp:$FRONTEND_PORT" >/dev/null || true
  else
    echo "  未找到 adb，Tauri CLI 仍会尝试启动；如连接失败请先配置 Android SDK platform-tools。"
  fi
  npx tauri android dev --host "127.0.0.1" "${TAURI_ARGS[@]}"
else
  echo "=== Android dev: 局域网模式 ==="
  echo "  应用访问: http://$DEV_HOST:$FRONTEND_PORT"
  echo "  请确认手机和电脑在同一网络，并放行 Windows 防火墙 TCP $FRONTEND_PORT。"
  TAURI_DEV_HOST="$DEV_HOST" VITE_DEV_HOST="$DEV_HOST" npx tauri android dev --host "$DEV_HOST" "${TAURI_ARGS[@]}"
fi
