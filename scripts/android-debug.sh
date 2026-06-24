#!/bin/bash
set -e

# MuMu Player 12 配置
MUMU_PORT="127.0.0.1:16416"
FRONTEND_PORT=5173
PACKAGE_NAME="com.kairos.app.debug"
APK_PATH="src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"

# --rebuild: 强制重新构建 APK
REBUILD=false
if [ "${1:-}" = "--rebuild" ]; then
  REBUILD=true
fi

# 优先系统 adb，fallback MuMu adb
SDK_ADB="D:/software/code/Android/SDK/platform-tools/adb.exe"
MUMU_ADB="D:/software/game/MuMu/MuMuPlayer/nx_main/adb.exe"
if [ -f "$SDK_ADB" ]; then
  ADB="$SDK_ADB"
elif [ -f "$MUMU_ADB" ]; then
  ADB="$MUMU_ADB"
else
  echo "✗ 找不到 adb"
  exit 1
fi

echo "  ADB: $ADB"

# 1. 连接设备
echo "=== 1. 连接 MuMu ($MUMU_PORT) ==="
"$ADB" connect "$MUMU_PORT" 2>/dev/null || true
sleep 1

# 2. 确认在线
echo "=== 2. 确认设备 ==="
for i in $(seq 1 10); do
  if "$ADB" -s "$MUMU_PORT" shell "echo OK" 2>/dev/null | grep -q OK; then
    ABI=$("$ADB" -s "$MUMU_PORT" shell getprop ro.product.cpu.abi 2>/dev/null | tr -d '\r')
    echo "  ✓ 已连接 (ABI: $ABI)"
    break
  fi
  sleep 1
done

# 3. 构建 APK
echo "=== 3. 构建 debug APK ==="
if [ "$REBUILD" = true ] || [ ! -f "$APK_PATH" ]; then
  echo "  构建中... (使用 --rebuild 强制重建)"
  npx tauri android build --debug
  echo "  ✓ $APK_PATH"
else
  echo "  使用现有 APK (加 --rebuild 强制重建)"
  echo "  $APK_PATH"
fi

# 4. 端口转发
echo "=== 4. adb reverse ($FRONTEND_PORT) ==="
"$ADB" -s "$MUMU_PORT" reverse tcp:$FRONTEND_PORT tcp:$FRONTEND_PORT
echo "  ✓ 已设置"

# 5. 安装
echo "=== 5. 安装 APK ==="
"$ADB" -s "$MUMU_PORT" install -r "$APK_PATH"

# 6. 启动
echo "=== 6. 启动应用 ==="
"$ADB" -s "$MUMU_PORT" shell monkey -p "$PACKAGE_NAME" -c android.intent.category.LAUNCHER 1 2>/dev/null || \
  "$ADB" -s "$MUMU_PORT" shell am start -S -n "$PACKAGE_NAME/.MainActivity" 2>/dev/null || \
  echo "  ⚠ 请手动启动应用"

echo ""
echo "✓ 完成!"
echo "  限制: Rust 变更需重新运行本脚本 (--rebuild) 以重新构建安装"
echo "  提示: 前端 UI 开发建议在桌面端 (cargo tauri dev) 进行"
