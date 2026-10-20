#!/usr/bin/env bash
# macOS 用の署名済み・公証済み PKG を作成する。
#
# release ビルドの実行ファイルと PDFium を PDR.app にまとめて署名し
# (Developer ID Application、hardened runtime)、署名付き PKG
# (Developer ID Installer) に包んで公証に出し、チケットを stapling する。
#
# 使い方:
#   scripts/package-macos.sh [オプション]
#
# オプション:
#   --no-build               cargo build --release を実行しない
#   --identity ID            "Developer ID Application: ..." の署名 ID
#                            (既定: $PDR_SIGN_IDENTITY、なければキーチェーンから自動検出)
#   --installer-identity ID  "Developer ID Installer: ..." の署名 ID
#                            (既定: $PDR_INSTALLER_IDENTITY、なければ自動検出)
#   --profile NAME           notarytool のキーチェーンプロファイル名
#                            (既定: $PDR_NOTARY_PROFILE、なければよく使う名前を試して自動検出)
#   --apple-id ID            Apple ID             (既定: $PDR_APPLE_ID)
#   --team-id ID             Team ID              (既定: $PDR_TEAM_ID)
#   --password PW            App 用パスワード     (既定: $PDR_APP_PASSWORD)
#   --skip-notarize          署名のみ行い、公証と stapling を省略する
#                            (環境変数 SKIP_NOTARIZE=1 でも同じ)
#
# 公証の認証情報: 一度キーチェーンにプロファイルを保存しておく
#   xcrun notarytool store-credentials notarytool --apple-id <id> --team-id <team> --password <app-pw>
# ('notarytool'、'pdr'、'notary' という名前のプロファイルは自動検出する。
# それ以外の名前は --profile で指定する)。または --apple-id/--team-id/--password を直接渡す。
#
# 出力:
#   dist/pdr-<version>-macos-<arch>.pkg  (署名・公証・stapling 済み)

set -euo pipefail
umask 022

usage() {
    sed -n '2,31p' "$0" | sed 's/^# \{0,1\}//'
}

build=1
identity="${PDR_SIGN_IDENTITY:-}"
installer_identity="${PDR_INSTALLER_IDENTITY:-}"
profile="${PDR_NOTARY_PROFILE:-}"
apple_id="${PDR_APPLE_ID:-}"
team_id="${PDR_TEAM_ID:-}"
app_password="${PDR_APP_PASSWORD:-}"
skip_notarize="${SKIP_NOTARIZE:-0}"

need_value() {
    if [ "$#" -lt 2 ]; then
        echo "オプションに値がありません: $1" >&2
        exit 2
    fi
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-build)           build=0; shift ;;
        --identity)           need_value "$@"; identity="$2"; shift 2 ;;
        --installer-identity) need_value "$@"; installer_identity="$2"; shift 2 ;;
        --profile)            need_value "$@"; profile="$2"; shift 2 ;;
        --apple-id)           need_value "$@"; apple_id="$2"; shift 2 ;;
        --team-id)            need_value "$@"; team_id="$2"; shift 2 ;;
        --password)           need_value "$@"; app_password="$2"; shift 2 ;;
        --skip-notarize)      skip_notarize=1; shift ;;
        -h|--help)            usage; exit 0 ;;
        *)
            echo "不明なオプション: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/.." && pwd)"

if [ "$(uname -s)" != "Darwin" ]; then
    echo "このスクリプトは macOS 上で実行してください。" >&2
    exit 1
fi

for tool in codesign pkgbuild security spctl xattr xcrun lipo; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "必要なコマンドが見つかりません: $tool" >&2
        exit 1
    fi
done

echo "=== PDR macOS パッケージ作成 ==="

version="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)"
if [ -z "$version" ]; then
    echo "Cargo.toml からバージョンを取得できませんでした。" >&2
    exit 1
fi

# ── 署名 ID の決定 ───────────────────────────────────────────────────────
if [ -z "$identity" ]; then
    identity="$(security find-identity -v -p codesigning 2>/dev/null \
        | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -n 1)"
    if [ -z "$identity" ]; then
        echo "キーチェーンに 'Developer ID Application' の署名 ID が見つかりません。" >&2
        echo "--identity か \$PDR_SIGN_IDENTITY で指定してください。" >&2
        exit 1
    fi
    echo "署名 ID を自動検出しました: $identity"
fi

if [ -z "$installer_identity" ]; then
    installer_identity="$(security find-identity -v 2>/dev/null \
        | sed -n 's/.*"\(Developer ID Installer: [^"]*\)".*/\1/p' | head -n 1)"
    if [ -z "$installer_identity" ]; then
        echo "キーチェーンに 'Developer ID Installer' の署名 ID が見つかりません。" >&2
        echo "--installer-identity か \$PDR_INSTALLER_IDENTITY で指定してください。" >&2
        exit 1
    fi
    echo "インストーラ署名 ID を自動検出しました: $installer_identity"
fi

# ── 公証の認証情報の確認 (ビルド前に済ませる) ───────────────────────────
notary_args=()
if [ "$skip_notarize" != "1" ]; then
    if [ -z "$profile" ] && [ -z "$apple_id" ] && [ -z "$team_id" ] && [ -z "$app_password" ]; then
        # notarytool のプロファイルはデータ保護キーチェーンにあり、security
        # コマンドでは列挙できないので、よく使う名前を順に試す。存在しない
        # プロファイルは即座に失敗するため、通信は実在するものの 1 回で済む。
        for candidate in notarytool pdr notary; do
            if ! xcrun notarytool history --keychain-profile "$candidate" 2>&1 \
                | grep -q "No Keychain password item found"; then
                profile="$candidate"
                echo "公証プロファイルを自動検出しました: $profile"
                break
            fi
        done
    fi
    if [ -n "$profile" ]; then
        notary_args=(--keychain-profile "$profile")
    elif [ -n "$apple_id" ] && [ -n "$team_id" ] && [ -n "$app_password" ]; then
        notary_args=(--apple-id "$apple_id" --team-id "$team_id" --password "$app_password")
    else
        echo "公証の認証情報がありません。" >&2
        echo "--profile NAME ('xcrun notarytool store-credentials' を参照) か" >&2
        echo "--apple-id/--team-id/--password を指定してください。" >&2
        exit 1
    fi
fi

# ── ビルドと入力ファイルの確認 ───────────────────────────────────────────
binary="$root/target/release/pdr"
pdfium="$root/third_party/pdfium/libpdfium.dylib"
if [ "$build" -eq 1 ]; then
    cargo build --manifest-path "$root/Cargo.toml" --release
fi
if [ ! -x "$binary" ]; then
    echo "実行ファイルがありません: $binary" >&2
    echo "'cargo build --release' を実行するか、--no-build を外してください。" >&2
    exit 1
fi
if [ ! -f "$pdfium" ]; then
    echo "macOS 用 PDFium ライブラリがありません: $pdfium" >&2
    exit 1
fi

archs="$(lipo -archs "$binary")"
case "$archs" in
    *arm64*x86_64*|*x86_64*arm64*) arch=universal ;;
    arm64)  arch=arm64 ;;
    x86_64) arch=x64 ;;
    *)
        echo "未対応のアーキテクチャです: $archs" >&2
        exit 1
        ;;
esac
echo "実行ファイル: $binary ($archs)"
echo "バージョン:   $version"

out_dir="$root/dist"
out_pkg="$out_dir/pdr-$version-macos-$arch.pkg"
if [ -e "$out_pkg" ]; then
    echo "出力先が既に存在します。移動または削除してから再実行してください: $out_pkg" >&2
    exit 1
fi

mkdir -p "$out_dir" "$root/target"
work_dir="$(mktemp -d "$root/target/package-macos.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

# ── 1. アプリバンドルの組み立て ─────────────────────────────────────────
echo ""
echo "PDR.app を組み立てています ..."
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
cp -RX "$root/third_party/pdfium/licenses/." "$contents/Resources/pdfium/licenses/"

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

# 拡張属性があると codesign が失敗し、pkgbuild が AppleDouble (._*) を出すので消しておく。
xattr -cr "$work_dir/root"

# ── 2. コード署名 (hardened runtime) ────────────────────────────────────
# 先に同梱 dylib と実行ファイルを署名し、その後にアプリバンドルを署名する。
echo ""
echo "署名しています ..."
codesign --force --timestamp --options runtime --sign "$identity" \
    "$contents/MacOS/libpdfium.dylib"
codesign --force --timestamp --options runtime --sign "$identity" \
    --identifier jp.fukuyori.pdr "$contents/MacOS/pdr"
codesign --force --timestamp --options runtime --sign "$identity" "$app"
codesign --verify --deep --strict --verbose=2 "$app"
echo "署名を確認しました"

# ── 3. 署名付き PKG の作成 (/Applications/PDR.app へインストール) ───────
echo ""
echo "署名付き PKG を作成しています ..."
work_pkg="$work_dir/pdr.pkg"
pkgbuild \
    --root "$work_dir/root" \
    --identifier jp.fukuyori.pdr \
    --version "$version" \
    --install-location / \
    --sign "$installer_identity" \
    "$work_pkg"

# ── 4. 公証と stapling ──────────────────────────────────────────────────
if [ "$skip_notarize" = "1" ]; then
    echo ""
    echo "公証を省略しました (--skip-notarize)"
else
    echo ""
    echo "公証に提出しています (数分かかることがあります) ..."
    result="$(xcrun notarytool submit "$work_pkg" "${notary_args[@]}" --wait --output-format json)"
    status="$(printf '%s' "$result" | /usr/bin/python3 -c 'import json,sys; print(json.load(sys.stdin).get("status",""))')"
    submission_id="$(printf '%s' "$result" | /usr/bin/python3 -c 'import json,sys; print(json.load(sys.stdin).get("id",""))')"
    echo "提出 ${submission_id}: $status"
    if [ "$status" != "Accepted" ]; then
        echo "公証に失敗しました。ログを取得します:" >&2
        if [ -n "$submission_id" ]; then
            xcrun notarytool log "$submission_id" "${notary_args[@]}" >&2 || true
        fi
        exit 1
    fi

    echo "チケットを stapling しています ..."
    xcrun stapler staple "$work_pkg"
    xcrun stapler validate "$work_pkg"

    echo "Gatekeeper で検証しています ..."
    spctl --assess --type install -vv "$work_pkg"
fi

mv "$work_pkg" "$out_pkg"

size="$(du -h "$out_pkg" | cut -f1)"
echo ""
echo "=== 完了 ==="
echo "作成しました: $out_pkg (${size})"
