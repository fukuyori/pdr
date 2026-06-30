# PDR — ポータブル・ドキュメント・リーダー

PDF を快適に読むための Rust 製デスクトップ・ビューア（Windows）。
egui/eframe + pdfium による描画で、見開き・縦横綴じ・目次・適応解像度の別スレッド描画に対応。

## 機能

- **PDF 表示**: 見開き / 単ページ、右綴じ(和) / 左綴じ(洋)、表紙単独
- **目次（しおり）**: ある PDF では左に自動表示。クリックでジャンプ、幅も変更可
- **ページ移動**: 前後ボタン、矢印/PageUp/Down/Space、下部スライダー
- **拡大縮小**: マウスホイール。横/縦フィットへスナップ、ウィンドウのリサイズに追従
- **画像補正**: スキャン PDF 向けにコントラスト伸長 / 大津法二値化
- **高速描画**: 別スレッドでレンダリングし UI をブロックしない。表示サイズに応じた適応解像度＋前後ページの先読み
- **履歴**: 最近開いたファイルを記憶（`%APPDATA%\pdr\recent.txt`）

## 必要なもの

- Rust（stable, MSVC）
- `pdfium.dll`（Windows x64）— **同梱済み**（`third_party/pdfium/pdfium.dll`）。クローンしてそのままビルド・実行できます。

`pdfium.dll` は次の順で探索されます: 実行ファイルと同じフォルダ → カレントディレクトリ → `./third_party/pdfium` → `./lib/bin` → システム。`cargo run`（カレント = リポジトリ直下）なら同梱物がそのまま使われます。配布する場合は `pdfium.dll` を実行ファイルと同じフォルダに置いてください。

同梱している PDFium のビルドは [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) の Windows x64 版です（バージョンは `third_party/pdfium/VERSION`）。更新する場合:

```sh
curl -sL -o pdfium.tgz https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz
tar -xzf pdfium.tgz bin/pdfium.dll LICENSE VERSION licenses
mv bin/pdfium.dll third_party/pdfium/pdfium.dll
mv LICENSE VERSION licenses third_party/pdfium/
```

## ビルドと実行

```sh
cargo run                 # 起動
cargo run -- path/to.pdf  # 指定 PDF を開いて起動
cargo test                # テスト
```

配布する場合は `pdfium.dll` を実行ファイルと同じフォルダに置いてください。

## インストール（PATH に置いてどこからでも使う）

`pdr.exe` と `pdfium.dll` を**同じフォルダにまとめて置く**だけで、どこからでも起動できます。さらにそのフォルダを PATH に通すと、ターミナルや関連付けから `pdr` を直接呼べます。

1. 配布 zip（`pdr-x.y.z-win-x64.zip`）を展開する。
   （または `cargo build --release` 後に `target\release\pdr.exe` と `third_party\pdfium\pdfium.dll` を用意）
2. 置き場所を作り（例: `%USERPROFILE%\bin`）、**`pdr.exe` と `pdfium.dll` の両方**をそこへコピーする。
3. そのフォルダを PATH に追加する（PowerShell・ユーザー環境変数の例。設定後はターミナルを開き直す）:

   ```powershell
   setx PATH "$env:USERPROFILE\bin;$env:PATH"
   ```

4. 以降はどこからでも実行できる:

   ```powershell
   pdr "C:\path\to\file.pdf"
   ```

> **重要**: `pdfium.dll` は必ず `pdr.exe` と**同じフォルダ**に置いてください（PATH 上の別フォルダではなく exe と同居が必要）。本アプリは実行ファイルと同じフォルダを最優先で探すため、これで起動ディレクトリに関係なく動作します。

## 補助バイナリ

- `cargo run --bin smoke -- <pdf> [page] [none|contrast|binarize]` — ヘッドレスで 1 ページを PNG 出力
- `cargo run --bin smoke -- <pdf> toc` — 目次（しおり）をダンプ
- `cargo run --bin smoke -- <pdf> bench [width] [count]` — 1 ページ描画の所要時間を計測

## ライセンス

本体のコードは **Apache License 2.0**（[LICENSE](LICENSE)）。

本ソフトウェアは描画に [PDFium](https://pdfium.googlesource.com/pdfium/)（**BSD-3-Clause** 他）を利用し、ビルド済みの `pdfium.dll` を `third_party/pdfium/` に**同梱**しています。PDFium および同梱コンポーネントのライセンスは以下にあります。配布物（バイナリ）にはこれらを同梱してください（[NOTICE](NOTICE) 参照）。

- `third_party/pdfium/LICENSE` — pdfium-binaries 梱包（MIT, Benoit Blanchon）
- `third_party/pdfium/licenses/pdfium.txt` — PDFium 本体（BSD-3-Clause, Google）
- `third_party/pdfium/licenses/` — FreeType・libpng・zlib・ICU 等の各ライセンス
