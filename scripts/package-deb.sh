#!/usr/bin/env bash
# Linux 用の deb パッケージを作成する。
#
# 使い方:
#   scripts/package-deb.sh
#   scripts/package-deb.sh --no-build
#   scripts/package-deb.sh --download-pdfium
#
# 出力:
#   dist/pdr-<version>-linux-<arch>.deb
#
# 既定では cargo build --release を実行する。PDFium は
# third_party/pdfium/libpdfium.so を同梱する。無い場合は
# --download-pdfium で bblanchon/pdfium-binaries から target/ に取得する。

set -euo pipefail
umask 022

usage() {
    sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
}

build=1
download_pdfium=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-build)
            build=0
            ;;
        --download-pdfium)
            download_pdfium=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/.." && pwd)"

version="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)"
if [ -z "$version" ]; then
    echo "Cargo.toml から version を取得できませんでした" >&2
    exit 1
fi

if command -v dpkg >/dev/null 2>&1; then
    deb_arch="$(dpkg --print-architecture)"
else
    case "$(uname -m)" in
        x86_64|amd64) deb_arch=amd64 ;;
        aarch64|arm64) deb_arch=arm64 ;;
        armv7l) deb_arch=armhf ;;
        *)
            echo "dpkg が無いため deb アーキテクチャを判定できません: $(uname -m)" >&2
            exit 1
            ;;
    esac
fi

case "$deb_arch" in
    amd64) pdfium_asset="pdfium-linux-x64.tgz" ;;
    arm64) pdfium_asset="pdfium-linux-arm64.tgz" ;;
    *)
        pdfium_asset=""
        ;;
esac

pdfium_src_dir="$root/third_party/pdfium"
pdfium_so="$pdfium_src_dir/libpdfium.so"
pdfium_doc_dir="$pdfium_src_dir"

if [ ! -f "$pdfium_so" ] && [ "$download_pdfium" -eq 1 ]; then
    if [ -z "$pdfium_asset" ]; then
        echo "--download-pdfium は現在 amd64/arm64 のみ対応です: $deb_arch" >&2
        exit 1
    fi
    pdfium_cache="$root/target/package-deb/pdfium-$deb_arch"
    if [ ! -f "$pdfium_cache/lib/libpdfium.so" ] && [ ! -f "$pdfium_cache/libpdfium.so" ]; then
        rm -rf "$pdfium_cache"
        mkdir -p "$pdfium_cache"
        url="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$pdfium_asset"
        echo "PDFium を取得しています: $url"
        curl -fsSL -o "$pdfium_cache/pdfium.tgz" "$url"
        tar -xzf "$pdfium_cache/pdfium.tgz" -C "$pdfium_cache"
    fi
    if [ -f "$pdfium_cache/lib/libpdfium.so" ]; then
        pdfium_so="$pdfium_cache/lib/libpdfium.so"
    elif [ -f "$pdfium_cache/libpdfium.so" ]; then
        pdfium_so="$pdfium_cache/libpdfium.so"
    else
        echo "取得したアーカイブに libpdfium.so がありません" >&2
        exit 1
    fi
    pdfium_doc_dir="$pdfium_cache"
fi

if [ ! -f "$pdfium_so" ]; then
    cat >&2 <<EOF
third_party/pdfium/libpdfium.so がありません。

次のどちらかで用意してください:
  scripts/package-deb.sh --download-pdfium
  または Linux 用 libpdfium.so を third_party/pdfium/ に配置
EOF
    exit 1
fi

if ! command -v dpkg-deb >/dev/null 2>&1; then
    echo "dpkg-deb が見つかりません。Debian/Ubuntu では dpkg-dev または dpkg をインストールしてください。" >&2
    exit 1
fi

if [ "$build" -eq 1 ]; then
    cargo build --release
fi

bin="$root/target/release/pdr"
if [ ! -x "$bin" ]; then
    echo "実行ファイルがありません。先に cargo build --release を実行してください: $bin" >&2
    exit 1
fi

pkg_name="pdr"
stage="$root/target/package-deb/${pkg_name}_${version}_${deb_arch}"
rm -rf "$stage"

install -d -m 0755 \
    "$stage/DEBIAN" \
    "$stage/usr/bin" \
    "$stage/usr/lib/pdr" \
    "$stage/usr/share/applications" \
    "$stage/usr/share/doc/pdr/pdfium" \
    "$stage/usr/share/icons/hicolor/256x256/apps"

install -m 0755 "$bin" "$stage/usr/lib/pdr/pdr-bin"
install -m 0644 "$pdfium_so" "$stage/usr/lib/pdr/libpdfium.so"

cat > "$stage/usr/bin/pdr" <<'EOF'
#!/bin/sh
set -eu
export LD_LIBRARY_PATH="/usr/lib/pdr${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec /usr/lib/pdr/pdr-bin "$@"
EOF
chmod 0755 "$stage/usr/bin/pdr"

if [ -f "$root/assets/AppIcon.png" ]; then
    install -m 0644 "$root/assets/AppIcon.png" "$stage/usr/share/icons/hicolor/256x256/apps/pdr.png"
fi

cat > "$stage/usr/share/applications/pdr.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=PDR
Comment=Portable Document Reader
Exec=pdr %f
Icon=pdr
Terminal=false
Categories=Office;Viewer;
MimeType=application/pdf;
StartupNotify=true
EOF

for f in README.md LICENSE NOTICE; do
    if [ -f "$root/$f" ]; then
        install -m 0644 "$root/$f" "$stage/usr/share/doc/pdr/$f"
    fi
done

if [ -f "$pdfium_doc_dir/LICENSE" ]; then
    install -m 0644 "$pdfium_doc_dir/LICENSE" "$stage/usr/share/doc/pdr/pdfium/LICENSE"
fi
if [ -f "$pdfium_doc_dir/VERSION" ]; then
    install -m 0644 "$pdfium_doc_dir/VERSION" "$stage/usr/share/doc/pdr/pdfium/VERSION"
fi
if [ -d "$pdfium_doc_dir/licenses" ]; then
    cp -R "$pdfium_doc_dir/licenses" "$stage/usr/share/doc/pdr/pdfium/licenses"
    find "$stage/usr/share/doc/pdr/pdfium/licenses" -type f -exec chmod 0644 {} +
fi

installed_size="$(du -sk "$stage/usr" | awk '{print $1}')"
depends="libc6, libgcc-s1, zenity, fonts-noto-cjk | fonts-ipafont-gothic | fonts-vlgothic"

cat > "$stage/DEBIAN/control" <<EOF
Package: pdr
Version: $version
Section: utils
Priority: optional
Architecture: $deb_arch
Installed-Size: $installed_size
Maintainer: fukuyori <fukuyori@users.noreply.github.com>
Depends: $depends
Description: Portable Document Reader
 Rust/egui/eframe and PDFium based PDF viewer.
EOF

dist="$root/dist"
mkdir -p "$dist"
deb="$dist/${pkg_name}-${version}-linux-${deb_arch}.deb"
rm -f "$deb"
dpkg-deb --build --root-owner-group "$stage" "$deb"

size="$(du -h "$deb" | awk '{print $1}')"
echo "作成しました: $deb ($size)"
