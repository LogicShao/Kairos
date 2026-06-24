#!/bin/bash
set -euo pipefail

FRONTEND_PORT="${FRONTEND_PORT:-5173}"
MUMU_PORT="127.0.0.1:16416"
PACKAGE_NAME="com.kairos.app.debug"
DEV_URL="http://127.0.0.1:${FRONTEND_PORT}"
APK_PATH="src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
PID_FILE=".vite-hmr.pid"

REBUILD=false
STOP=false
for arg in "$@"; do
  case "$arg" in
    --rebuild) REBUILD=true ;;
    --stop) STOP=true ;;
    -h|--help)
      echo "用法: bash scripts/android-hmr.sh [--rebuild] [--stop]"
      echo ""
      echo "  (无参数)    启动 Vite + 安装已有 APK"
      echo "  --rebuild   重新构建 APK 后安装"
      echo "  --stop      停止后台 Vite"
      exit 0
      ;;
  esac
done

cd "$(dirname "$0")/.."

SDK_ADB="D:/software/code/Android/SDK/platform-tools/adb.exe"
MUMU_ADB="D:/software/game/MuMu/MuMuPlayer/nx_main/adb.exe"
if [ -f "$SDK_ADB" ]; then ADB="$SDK_ADB"
elif [ -f "$MUMU_ADB" ]; then ADB="$MUMU_ADB"
else echo "✗ 找不到 adb"; exit 1; fi

# ── stop 模式 ──
if [ "$STOP" = true ]; then
  if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    kill "$PID" 2>/dev/null && echo "✓ Vite (PID $PID) 已停止" || echo "⚠ PID $PID 已不存在"
    rm -f "$PID_FILE"
  else
    echo "⚠ 未找到 PID 文件，尝试按端口清理..."
  fi
  # 强制释放端口
  PID_ON_PORT=$(netstat -ano 2>/dev/null | grep ":${FRONTEND_PORT}" | grep LISTENING | awk '{print $NF}' || true)
  if [ -n "$PID_ON_PORT" ]; then
    taskkill //PID "$PID_ON_PORT" //F 2>/dev/null && echo "✓ 端口 ${FRONTEND_PORT} 已释放"
  fi
  exit 0
fi

# ── 启动 Vite (如未运行) ──
VITE_RUNNING=false
if curl -s -o /dev/null -w "%{http_code}" "http://localhost:${FRONTEND_PORT}" 2>/dev/null | grep -q 200; then
  VITE_RUNNING=true
  echo "  Vite 已在运行: http://localhost:${FRONTEND_PORT}"
else
  echo "=== 启动 Vite HMR ==="
  npm run dev &
  VITE_PID=$!
  echo $VITE_PID > "$PID_FILE"

  # 等待 Vite 就绪
  for i in $(seq 1 15); do
    if curl -s -o /dev/null -w "%{http_code}" "http://localhost:${FRONTEND_PORT}" 2>/dev/null | grep -q 200; then
      echo "  ✓ Vite 就绪 (PID $VITE_PID)"
      break
    fi
    sleep 1
  done
fi

# ── 连接 MuMu ──
echo "=== 连接 MuMu ==="
"$ADB" connect "$MUMU_PORT" 2>/dev/null || true
sleep 1
"$ADB" -s "$MUMU_PORT" shell "echo OK" 2>/dev/null | grep -q OK || {
  echo "✗ 无法连接 MuMu，请确认模拟器正在运行"
  exit 1
}
echo "  ✓ 已连接 ($("$ADB" -s "$MUMU_PORT" shell getprop ro.product.cpu.abi 2>/dev/null | tr -d '\r'))"

# ── 构建 APK ──
echo "=== 构建 APK ==="
if [ "$REBUILD" = true ] || [ ! -f "$APK_PATH" ]; then
  echo "  构建中..."
  npx tauri android build --debug --config "{\"build\":{\"devUrl\":\"${DEV_URL}\"}}"
else
  echo "  使用现有 APK (加 --rebuild 重建)"
fi
echo "  ✓ $APK_PATH"

# ── adb reverse ──
echo "=== adb reverse ==="
"$ADB" -s "$MUMU_PORT" reverse "tcp:${FRONTEND_PORT}" "tcp:${FRONTEND_PORT}"
echo "  ✓ Android -> 开发机 :${FRONTEND_PORT}"

# ── 安装启动 ──
echo "=== 安装 & 启动 ==="
"$ADB" -s "$MUMU_PORT" install -r "$APK_PATH"
"$ADB" -s "$MUMU_PORT" shell monkey -p "$PACKAGE_NAME" \
  -c android.intent.category.LAUNCHER 1 2>/dev/null

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Vite HMR: http://localhost:${FRONTEND_PORT}"
echo "  停止 Vite: bash scripts/android-hmr.sh --stop"
echo "  重建 APK: bash scripts/android-hmr.sh --rebuild"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
