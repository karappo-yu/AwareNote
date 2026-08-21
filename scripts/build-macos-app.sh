#!/bin/zsh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/native-macos/build"
DIST_DIR="$ROOT_DIR/native-macos/dist"
APP_DIR="$DIST_DIR/AwareNote.app"
SETTINGS_APP_DIR="$DIST_DIR/AwareNote Settings.app"
FRONTEND_DIR="$ROOT_DIR/art-gallery"
STATIC_DIR="$ROOT_DIR/src/frontend"

mkdir -p "$BUILD_DIR" "$DIST_DIR"

# ── 构建前端 ──
echo "==> Building frontend..."
cd "$FRONTEND_DIR"
npm run build

echo "==> Copying frontend dist to backend static dir..."
# STATIC_DIR 被 .gitignore 忽略，新环境可能不存在，先确保目录存在
mkdir -p "$STATIC_DIR"
# 清理旧产物（保留 favicon.ico）
rm -rf "$STATIC_DIR/assets" "$STATIC_DIR/icons.svg" "$STATIC_DIR/index.html"
cp -R "$FRONTEND_DIR/dist/"* "$STATIC_DIR/"

# ── 构建后端 ──
echo "==> Building backend..."
cargo build --release --bin awarenotes --manifest-path "$ROOT_DIR/Cargo.toml"

# 注：macOS 27 beta 的 Command Line Tools 缺少 SwiftUIMacros 宏插件，
# 默认 SDK 编译 @State 等宏会报 "plugin for module 'SwiftUIMacros' not found"。
# 自动回退到可用的旧版 SDK（如 MacOSX26.5.sdk）编译，产物向后兼容。
echo "==> Compiling settings app..."
if ! swiftc \
  -parse-as-library \
  "$ROOT_DIR/native-macos/AwarenotesSettings.swift" \
  -o "$BUILD_DIR/awarenotes-settings" 2>/dev/null; then
  echo "    默认 SDK 编译失败（缺少 SwiftUI 宏插件），尝试旧版 SDK..."
  FALLBACK_SDK="$(ls -d /Library/Developer/CommandLineTools/SDKs/MacOSX2[0-6]*.sdk 2>/dev/null | sort -V | tail -1 || true)"
  if [ -n "${FALLBACK_SDK}" ]; then
    echo "    使用 SDK: $FALLBACK_SDK"
    swiftc \
      -parse-as-library \
      -sdk "$FALLBACK_SDK" \
      "$ROOT_DIR/native-macos/AwarenotesSettings.swift" \
      -o "$BUILD_DIR/awarenotes-settings"
  else
    swiftc \
      -parse-as-library \
      "$ROOT_DIR/native-macos/AwarenotesSettings.swift" \
      -o "$BUILD_DIR/awarenotes-settings"
  fi
fi

rm -rf "$APP_DIR"
rm -rf "$SETTINGS_APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
mkdir -p "$SETTINGS_APP_DIR/Contents/MacOS" "$SETTINGS_APP_DIR/Contents/Resources"

cp "$ROOT_DIR/native-macos/Info.plist" "$APP_DIR/Contents/Info.plist"
cp "$ROOT_DIR/target/release/awarenotes" "$APP_DIR/Contents/MacOS/awarenotes"
cp "$ROOT_DIR/icon/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"

cp "$ROOT_DIR/native-macos/SettingsInfo.plist" "$SETTINGS_APP_DIR/Contents/Info.plist"
cp "$BUILD_DIR/awarenotes-settings" "$SETTINGS_APP_DIR/Contents/MacOS/awarenotes-settings"
cp "$ROOT_DIR/icon/AppIcon.icns" "$SETTINGS_APP_DIR/Contents/Resources/AppIcon.icns"

cp -R "$SETTINGS_APP_DIR" "$APP_DIR/Contents/Resources/AwareNote Settings.app"

cp -R "$ROOT_DIR/src/frontend" "$APP_DIR/Contents/Resources/frontend"

chmod +x "$APP_DIR/Contents/MacOS/awarenotes"
chmod +x "$SETTINGS_APP_DIR/Contents/MacOS/awarenotes-settings"

echo "Built app at $APP_DIR"
