# PDR — ポータブル・ドキュメント・リーダー

[English](README.md) | 日本語

egui/eframe と PDFium で作られた Rust 製デスクトップ PDF ビューアです。単ページ・見開き表示、左右の綴じ方向、目次、バックグラウンドでの適応解像度描画に対応しています。

## 機能

- **PDF 表示**: 単ページ / 見開き、左綴じ / 右綴じ、表紙単独表示
- **目次（しおり）**: 対応する PDF では自動表示。項目をクリックして移動でき、パネル幅も変更できます。
- **ページ移動**: 前後ボタン、矢印キー、Page Up/Down、Space、下部スライダー
- **文字の範囲選択**: プライマリボタンのドラッグでページ内の文字をハイライトします。単ページ・見開き、拡大・回転表示に対応します。文字情報のない画像だけの PDF は対象外です。
- **コピー・翻訳**: ハイライトした文字上で右クリックし、コピーまたは Ollama のモデルによる翻訳を選べます。日本語・英語・中国語・ドイツ語・スペイン語・フランス語に対応しています。
- **表示位置の移動**: セカンダリボタンのドラッグ、ホイール、スクロールバーで移動します。セカンダリクリックではページ回転メニューを表示します。
- **拡大縮小**: マウスホイールで拡大・縮小します。横フィット・縦フィットへスナップし、ウィンドウのリサイズに追従します。
- **画像補正**: スキャン PDF 向けに、ページ単位の自動レベル補正、強度調整可能なコントラスト / アンシャープマスク、大津法二値化に対応しています。
- **高速描画**: UI をブロックせずに別スレッドで描画します。表示サイズに応じた適応解像度と前後ページの先読みを行います。
- **設定の復元**: 表示モード、綴じ方向、表紙単独表示、画像補正、目次幅、翻訳設定、最近開いた PDF の一覧を次回起動時に復元します。起動時はブランクページを表示し、PDF は自動で開きません。ページ位置、一時回転、文字選択、フィット方法、ズーム倍率も復元しません。PDF は「履歴」から選んで開きます。

## Ollama による翻訳

### Ollama のインストールとテスト済みモデルの取得

[Ollama の公式ダウンロードページ](https://ollama.com/download)からインストールできます。公式のコマンドラインインストール方法は次のとおりです。

```sh
# macOS / Linux
curl -fsSL https://ollama.com/install.sh | sh
```

```powershell
# Windows PowerShell
irm https://ollama.com/install.ps1 | iex
```

PDR では **`translategemma:4b`** で動作確認しています。Ollama のインストール後、ターミナルでモデルをダウンロードしてください。

```sh
ollama pull translategemma:4b
```

モデルの詳細は[TranslateGemma のモデルページ](https://ollama.com/library/translategemma)を参照してください。

### 接続設定と翻訳

翻訳ウィンドウの「翻訳設定」を開き、Ollama の接続先（例: `http://localhost:11434`）を入力して「モデル一覧を更新」を押します。モデルのドロップダウンで **`translategemma:4b`** を選び、翻訳先言語を指定します。接続先・モデル・翻訳先言語と翻訳設定の折りたたみ状態は保存されます。

翻訳ウィンドウはサイズ変更できます。原文と翻訳の欄は左右同じ幅で、スクロールバーはそれぞれ独立しています。ウィンドウ内で Ctrl キーを押しながらマウスホイールを回すと、フォントサイズを変更できます。

PDF 内の文字を選択し、ハイライト上で右クリックして翻訳ウィンドウを開きます。選択中のモデルが利用可能な場合は自動で翻訳を開始します。まだモデル一覧を取得していない場合は一覧を更新し、「翻訳する」を押してください。結果は同じウィンドウに表示され、「コピー」でコピーできます。翻訳はバックグラウンドで実行し、エラーはウィンドウ内に表示します。翻訳の前に Ollama を起動し、使用するモデルを利用可能な状態にしてください。

翻訳設定の保存先:

- Windows: `%APPDATA%\pdr\translation.json`
- macOS: `~/Library/Application Support/pdr/translation.json`
- Linux: `$XDG_CONFIG_HOME/pdr/translation.json`（未設定時は `~/.config/pdr/translation.json`）

表示設定の保存先:

- Windows: `%APPDATA%\pdr\settings.json`
- macOS: `~/Library/Application Support/pdr/settings.json`
- Linux: `$XDG_CONFIG_HOME/pdr/settings.json`（未設定時は `~/.config/pdr/settings.json`）

最近開いた PDF の一覧（最大12件）は、同じ設定フォルダの `recent_files.json` に保存されます。起動時に一覧を読み込みますが、PDF は自動では開きません。

## 必要なもの

- Rust（stable。Windows では MSVC）
- Windows x64 用 `pdfium.dll` は `third_party/pdfium/pdfium.dll` に同梱しています。Windows ではクローンしてそのままビルド・実行できます。

`pdfium.dll` は次の順で探索されます: 実行ファイルと同じフォルダ → カレントディレクトリ → `./third_party/pdfium` → `./lib/bin` → システム。リポジトリ直下で `cargo run` を実行する場合、同梱ファイルが自動で使われます。配布時は `pdfium.dll` を実行ファイルと同じフォルダに置いてください。

同梱 PDFium は [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) の Windows x64 版です（`third_party/pdfium/VERSION` を参照）。更新する場合:

```sh
curl -sL -o pdfium.tgz https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz
tar -xzf pdfium.tgz bin/pdfium.dll LICENSE VERSION licenses
mv bin/pdfium.dll third_party/pdfium/pdfium.dll
mv LICENSE VERSION licenses third_party/pdfium/
```

## ビルドと実行

```sh
cargo run                 # 起動
cargo run -- path/to.pdf  # PDF を開いて起動
cargo test                # テスト
```

配布時は `pdfium.dll` を実行ファイルと同じフォルダに置いてください。

## Linux deb パッケージの作成

次のコマンドで Linux 用 deb パッケージを作成できます。

```sh
scripts/package-deb.sh --download-pdfium
```

出力先は `dist/pdr_<version>_<arch>.deb` です。`third_party/pdfium/libpdfium.so` がすでにある場合は `--download-pdfium` を省略できます。ビルド済みバイナリを使う場合は `scripts/package-deb.sh --no-build` を指定してください。

## Windows でのインストールと PATH の設定

Inno Setup のインストーラーには、初期状態でオフになっている「PDR を PATH に追加する」項目があります。選択するとターミナルから `pdr` を実行できます。アンインストール時には、インストーラーが追加した項目を PATH から削除します。変更を反映するには、新しいターミナルを開いてください。

`pdr.exe` と `pdfium.dll` は**同じフォルダ**に置いてください。そのフォルダを PATH に追加すると、ターミナルやファイル関連付けから `pdr` を実行できます。

1. 配布アーカイブ（`pdr-x.y.z-win-x64.zip`）を展開します。または `cargo build --release` の後、`target\release\pdr.exe` と `third_party\pdfium\pdfium.dll` を用意します。
2. 保存先フォルダ（例: `%USERPROFILE%\bin`）を作成し、**`pdr.exe` と `pdfium.dll` の両方**をコピーします。
3. フォルダをユーザー PATH に追加します。次の PowerShell の例は、新しく開いたターミナルから有効になります。

   ```powershell
   setx PATH "$env:USERPROFILE\bin;$env:PATH"
   ```

4. 以降はどのディレクトリからでも実行できます。

   ```powershell
   pdr "C:\path\to\file.pdf"
   ```

> **重要:** `pdfium.dll` は必ず `pdr.exe` と同じフォルダに置いてください。PATH 上の別フォルダでは不十分です。PDR は実行ファイルと同じフォルダを最初に探索するため、作業ディレクトリによらず DLL を読み込めます。

## 補助バイナリ

- `cargo run --bin smoke -- <pdf> [page] [none|contrast|auto|sharpen|binarize] [strength]` — GUI を使わずに1ページを PNG 出力
- `cargo run --bin smoke -- <pdf> toc` — 目次（しおり）を出力
- `cargo run --bin smoke -- <pdf> bench [width] [count]` — 1ページの描画時間を計測

## ライセンス

アプリケーション本体のコードは **Apache License 2.0**（[LICENSE](LICENSE)）です。

描画には [PDFium](https://pdfium.googlesource.com/pdfium/)（**BSD-3-Clause** ほか）を利用し、ビルド済み `pdfium.dll` を `third_party/pdfium/` に同梱しています。配布物には該当するライセンスを含めてください（[NOTICE](NOTICE) を参照）。

- `third_party/pdfium/LICENSE` — pdfium-binaries の梱包（MIT、Benoit Blanchon）
- `third_party/pdfium/licenses/pdfium.txt` — PDFium 本体（BSD-3-Clause、Google）
- `third_party/pdfium/licenses/` — FreeType、libpng、zlib、ICU などの各ライセンス
