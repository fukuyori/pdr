# PDR — ポータブル・ドキュメント・リーダー

PDF を快適に読むための Rust 製デスクトップ・ビューア（Windows）。
egui/eframe + pdfium による描画で、見開き・縦横綴じ・目次・適応解像度の別スレッド描画に対応。

## 機能

- **PDF 表示**: 見開き / 単ページ、右綴じ(和) / 左綴じ(洋)、表紙単独
- **目次（しおり）**: ある PDF では左に自動表示。クリックでジャンプ、幅も変更可
- **ページ移動**: 前後ボタン、矢印/PageUp/Down/Space、下部スライダー
- **文字の範囲選択**: プライマリボタンのドラッグで、選択したページの文字をハイライト。単ページ・見開き、拡大・回転表示に対応。文字情報のない画像だけの PDF は対象外
- **コピー・翻訳**: ハイライトした文字上のセカンダリクリックで「コピー」「翻訳」。翻訳は指定した Ollama のモデルを使用し、日本語・英語・中国語・ドイツ語・スペイン語・フランス語に対応
- **表示位置の移動**: セカンダリボタンのドラッグ、ホイール、スクロールバーでパン。セカンダリクリックではページ回転メニューを表示
- **拡大縮小**: マウスホイール。横/縦フィットへスナップ、ウィンドウのリサイズに追従
- **画像補正**: スキャンPDF向けに、ページ単位の自動レベル補正、強度調整可能なコントラスト／アンシャープマスク、大津法二値化
- **高速描画**: 別スレッドでレンダリングし UI をブロックしない。表示サイズに応じた適応解像度＋前後ページの先読み
- **復元**: 表示モード、綴じ方向、表紙単独表示、画像補正、目次幅、翻訳設定を次回起動時に復元。起動時はブランクページを表示し、PDF・ページ位置・一時回転・文字選択・フィット方法・ズーム倍率は復元しない

## 必要なもの

### Ollama による翻訳

ツールバーの「翻訳設定」で、Ollama の接続先（例: `http://localhost:11434`）を入力し、
「モデル一覧を更新」を押してください。使用モデルと翻訳先言語をドロップダウンで選びます。
接続先・モデル・言語は保存されます。接続先を変更した場合は、モデル一覧を再取得してください。
翻訳設定の折りたたみ状態も保存されます。翻訳ウィンドウはサイズ変更でき、原文・訳文欄は個別スクロールと高さ調整に対応します。

表示モード、綴じ方向、表紙単独表示、画像補正、目次幅は次回起動時に復元されます。
起動時はブランクページを表示し、前回の PDF は自動では開きません。ファイルを指定して起動した場合も、保存済みのこれらの設定を適用します。
表示設定は Windows の `%APPDATA%\pdr\settings.json`、macOS の
`~/Library/Application Support/pdr/settings.json`、Linux の
`$XDG_CONFIG_HOME/pdr/settings.json`（未設定時は `~/.config/pdr/settings.json`）に保存します。

文字を選択し、そのハイライト上で右クリックして「翻訳」を選ぶと翻訳画面が開きます。
モデル一覧の取得・設定が済んでいれば翻訳を開始します。初回や再起動後はモデル一覧を取得し、
「翻訳する」を押してください。結果は同じ画面に表示され、「結果をコピー」でコピーできます。
通信はバックグラウンドで実行され、失敗時は画面にエラーを表示します。
Ollama の起動と使用するモデルの準備は事前に行ってください。

翻訳設定は Windows では `%APPDATA%\pdr\translation.json`、macOS では
`~/Library/Application Support/pdr/translation.json`、Linux では
`$XDG_CONFIG_HOME/pdr/translation.json`（未設定時は `~/.config/pdr/translation.json`）に保存します。

### ビューアの動作環境

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

## Linux deb パッケージ作成

Linux 用の deb は次で作成できます。

```sh
scripts/package-deb.sh --download-pdfium
```

出力は `dist/pdr_<version>_<arch>.deb` です。既に `third_party/pdfium/libpdfium.so` がある場合は `--download-pdfium` なしで実行できます。ビルド済みバイナリを使う場合は `scripts/package-deb.sh --no-build` を指定してください。

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

- `cargo run --bin smoke -- <pdf> [page] [none|contrast|auto|sharpen|binarize] [strength]` — ヘッドレスで1ページをPNG出力
- `cargo run --bin smoke -- <pdf> toc` — 目次（しおり）をダンプ
- `cargo run --bin smoke -- <pdf> bench [width] [count]` — 1 ページ描画の所要時間を計測

## ライセンス

本体のコードは **Apache License 2.0**（[LICENSE](LICENSE)）。

本ソフトウェアは描画に [PDFium](https://pdfium.googlesource.com/pdfium/)（**BSD-3-Clause** 他）を利用し、ビルド済みの `pdfium.dll` を `third_party/pdfium/` に**同梱**しています。PDFium および同梱コンポーネントのライセンスは以下にあります。配布物（バイナリ）にはこれらを同梱してください（[NOTICE](NOTICE) 参照）。

- `third_party/pdfium/LICENSE` — pdfium-binaries 梱包（MIT, Benoit Blanchon）
- `third_party/pdfium/licenses/pdfium.txt` — PDFium 本体（BSD-3-Clause, Google）
- `third_party/pdfium/licenses/` — FreeType・libpng・zlib・ICU 等の各ライセンス
