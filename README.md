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
- `pdfium.dll`（Windows x64）— ライセンスの都合でリポジトリには含めていません。各自取得してください。

### pdfium.dll の取得

[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) のリリースから Windows x64 版を取得し、`pdfium.dll` をリポジトリ直下（または実行ファイルと同じフォルダ）に置きます。

```sh
curl -sL -o pdfium.tgz https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz
tar -xzf pdfium.tgz bin/pdfium.dll
mv bin/pdfium.dll ./pdfium.dll
```

`pdfium.dll` は次の順で探索されます: 実行ファイルと同じフォルダ → カレントディレクトリ → `./lib/bin` → システム。

## ビルドと実行

```sh
cargo run                 # 起動
cargo run -- path/to.pdf  # 指定 PDF を開いて起動
cargo test                # テスト
```

配布する場合は `pdfium.dll` を実行ファイルと同じフォルダに置いてください。

## 補助バイナリ

- `cargo run --bin smoke -- <pdf> [page] [none|contrast|binarize]` — ヘッドレスで 1 ページを PNG 出力
- `cargo run --bin smoke -- <pdf> toc` — 目次（しおり）をダンプ
- `cargo run --bin smoke -- <pdf> bench [width] [count]` — 1 ページ描画の所要時間を計測

## ライセンス

本体のコードは **Apache License 2.0**（[LICENSE](LICENSE)）。

本ソフトウェアは描画に [PDFium](https://pdfium.googlesource.com/pdfium/)（`pdfium.dll`, **BSD-3-Clause**）を利用します。`pdfium.dll` は本リポジトリには含めていません（[取得方法](#pdfiumdll-の取得)を参照）。exe と一緒にバイナリ配布する場合は、PDFium の著作権表示とライセンス全文を同梱してください。
