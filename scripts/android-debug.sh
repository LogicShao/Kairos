#!/bin/bash
set -e

MUMU_ADB="D:/software/game/MuMu/MuMuPlayer/nx_main/adb.exe"
MUMU_PORT="127.0.0.1:16416"
FRONTEND_PORT=5173

echo "=== 1. 连接 MuMu 模拟器 ==="
"$MUMU_ADB" connect "$MUMU_PORT" 2>/dev/null || true

echo "=== 2. adb reverse ($FRONTEND_PORT) ==="
"$MUMU_ADB" -s "$MUMU_PORT" reverse tcp:$FRONTEND_PORT tcp:$FRONTEND_PORT

echo "=== 3. 验证 ==="
"$MUMU_ADB" -s "$MUMU_PORT" shell "echo OK"

echo ""
echo "就绪。现在运行："
echo "  cd D:/proj/Kairos && npm run tauri android dev"
