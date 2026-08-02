#!/bin/bash
# Android 构建后重命名 APK/AAB 为 Kairos_<version>_<abi>.<ext>，与桌面安装包命名一致。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# git-bash 下转为 Windows 风格路径，供 node 读取（require 需要 D:/ 而非 /d/）
if command -v cygpath >/dev/null 2>&1; then
  CONF="$(cygpath -w "${PROJECT_DIR}/src-tauri/tauri.conf.json")"
else
  CONF="${PROJECT_DIR}/src-tauri/tauri.conf.json"
fi
VERSION=$(node -e "console.log(require(process.argv[1]).version)" "$CONF")

APK_DIR="${PROJECT_DIR}/src-tauri/gen/android/app/build/outputs/apk"
AAB_DIR="${PROJECT_DIR}/src-tauri/gen/android/app/build/outputs/bundle"

echo "=== 重命名 Android 产物 → Kairos_${VERSION}_*.apk / *.aab ==="

# 重命名 release APK：app-<abi>-release.apk → Kairos_<version>_<abi>.apk
if [ -d "$APK_DIR" ]; then
  find "$APK_DIR" -path "*/release/app-*.apk" | while read -r apk; do
    dir=$(dirname "$apk")
    name=$(basename "$apk")
    # 提取 ABI: app-arm64-release.apk → arm64（仅处理 tauri 原始命名，避免二次重命名）
    abi=$(echo "$name" | sed -E 's/^app-(.*)-release\.apk$/\1/')
    [ -z "$abi" ] && continue
    new_name="Kairos_${VERSION}_${abi}.apk"
    mv "$apk" "${dir}/${new_name}"
    echo "  ${name} → ${new_name}"
  done
fi

# 重命名 AAB：app-arm64-release.aab → Kairos_<version>_arm64.aab（仅 --aab 构建时存在）
if [ -d "$AAB_DIR" ]; then
  find "$AAB_DIR" -path "*Release/app-*.aab" | while read -r aab; do
    dir=$(dirname "$aab")
    name=$(basename "$aab")
    abi=$(echo "$name" | sed -E 's/^app-(.*)-release\.aab$/\1/')
    [ -z "$abi" ] && continue
    new_name="Kairos_${VERSION}_${abi}.aab"
    mv "$aab" "${dir}/${new_name}"
    echo "  ${name} → ${new_name}"
  done
fi

echo "=== 重命名完成 ==="