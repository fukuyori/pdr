#!/usr/bin/env bash
# macOS 用の署名済み・公証済み PKG を作成する。
#
# 事前準備:
#   CODESIGN_CERT       Developer ID Application の署名 ID
#   INSTALLER_SIGN_CERT Developer ID Installer の署名 ID
#   NOTARY_PROFILE      xcrun notarytool store-credentials で保存したプロファイル名
#
# 使い方:
#   scripts/package-macos.sh
#   scripts/package-macos.sh --no-build
#
# 出力:
#   dist/pdr-<version>-macos-<arch>.pkg

set -euo pipefail
umask 022

usage() {
    sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
}

build=1
while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-build)
            build=0
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "不明なオプション: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/.." && pwd)"

if [ "$(uname -s)" != "Darwin" ]; then
    echo "このスクリプトは macOS 上で実行してください。" >&2
    exit 1
fi

for tool in codesign pkgbuild spctl xcrun; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "必要なコマンドが見つかりません: $tool" >&2
        exit 1
    fi
done

version="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)"
if [ -z "$version" ]; then
    echo "Cargo.toml からバージョンを取得できませんでした。" >&2
    exit 1
fi

case "$(uname -m)" in
    arm64|aarch64) arch=arm64 ;;
    x86_64|amd64) arch=x64 ;;
    *)
        echo "未対応の macOS アーキテクチャです: $(uname -m)" >&2
        exit 1
        ;;
esac

app_sign_cert="${CODESIGN_CERT:-}"
installer_sign_cert="${INSTALLER_SIGN_CERT:-}"
notary_profile="${NOTARY_PROFILE:-}"
if [ -z "$app_sign_cert" ]; then
    echo "Developer ID Application の署名 ID を CODESIGN_CERT に設定してください。" >&2
    exit 1
fi
if [ -z "$installer_sign_cert" ]; then
    echo "Developer ID Installer の署名 ID を INSTALLER_SIGN_CERT に設定してください。" >&2
    exit 1
fi
if [ -z "$notary_profile" ]; then
    echo "notarytool のキーチェーンプロファイル名を NOTARY_PROFILE に設定してください。" >&2
    exit 1
fi

binary="$root/target/release/pdr"
pdfium="$root/third_party/pdfium/libpdfium.dylib"
if [ "$build" -eq 1 ]; then
    cargo build --manifest-path "$root/Cargo.toml" --release
fi
if [ ! -x "$binary" ]; then
    echo "実行ファイルがありません: $binary" >&2
    exit 1
fi
if [ ! -f "$pdfium" ]; then
    echo "macOS 用 PDFium ライブラリがありません: $pdfium" >&2
    exit 1
fi

out_dir="$root/dist"
out_pkg="$out_dir/pdr-$version-macos-$arch.pkg"
if [ -e "$out_pkg" ]; then
    echo "出力先が既に存在します。移動または削除してから再実行してください: $out_pkg" >&2
    exit 1
fi

mkdir -p "$out_dir" "$root/target"
work_dir="$(mktemp -d "$root/target/package-macos.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

app="$work_dir/root/Applications/PDR.app"
contents="$app/Contents"
mkdir -p "$contents/MacOS" "$contents/Resources/pdfium/licenses"
install -m 0755 "$binary" "$contents/MacOS/pdr"
install -m 0644 "$pdfium" "$contents/MacOS/libpdfium.dylib"
install -m 0644 "$root/assets/AppIcon.icns" "$contents/Resources/AppIcon.icns"
install -m 0644 "$root/LICENSE" "$contents/Resources/LICENSE"
install -m 0644 "$root/NOTICE" "$contents/Resources/NOTICE"
install -m 0644 "$root/third_party/pdfium/LICENSE" "$contents/Resources/pdfium/LICENSE"
install -m 0644 "$root/third_party/pdfium/VERSION" "$contents/Resources/pdfium/VERSION"
cp -R "$root/third_party/pdfium/licenses/." "$contents/Resources/pdfium/licenses/"

cat > "$contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>pdr</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>jp.fukuyori.pdr</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>PDR</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>PDF Document</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>com.adobe.pdf</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF

# 先に同梱 dylib と実行ファイルを署名し、その後にアプリバンドルを署名する。
codesign --force --timestamp --options runtime --sign "$app_sign_cert" \
    "$contents/MacOS/libpdfium.dylib"
codesign --force --timestamp --options runtime --sign "$app_sign_cert" \
    --identifier jp.fukuyori.pdr "$contents/MacOS/pdr"
codesign --force --timestamp --options runtime --sign "$app_sign_cert" "$app"
codesign --verify --deep --strict --verbose=2 "$app"

unsigned_pkg="$work_dir/pdr.pkg"
pkgbuild \
    --root "$work_dir/root" \
    --identifier jp.fukuyori.pdr \
    --version "$version" \
    --install-location / \
    --sign "$installer_sign_cert" \
    "$unsigned_pkg"

# 公証後にチケットを PKG へ stapling してから dist へ配置する。
xcrun notarytool submit "$unsigned_pkg" --keychain-profile "$notary_profile" --wait
xcrun stapler staple "$unsigned_pkg"
xcrun stapler validate "$unsigned_pkg"
spctl --assess --type install --verbose "$unsigned_pkg"
mv "$unsigned_pkg" "$out_pkg"

echo "作成しました: $out_pkg"
