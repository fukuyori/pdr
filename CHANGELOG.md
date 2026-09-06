# 変更履歴

このプロジェクトの主要な変更点を記録します。
書式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に準拠し、
バージョニングは [セマンティック バージョニング](https://semver.org/lang/ja/) に従います。

## [Unreleased]

## [0.2.0] - 2026-09-06

### 修正
- macOS/Windows ビルドで `file_dialog_rx` フィールドが未使用となり dead_code
  警告が出ていたのを解消（Linux 限定に条件付けした）。

### 変更
- PDFium の初期化、描画ワーカー、目次抽出を `pdf_worker` モジュールへ分離。
- 見開き、先読み、綴じ方向のページ計算を `navigation` モジュールへ分離し、
  単体テストを追加。

## [0.1.8] - 2026-07-23

### 変更
- Linux のファイル選択を `zenity`（あれば）優先にし、無い場合は従来どおり
  `rfd` にフォールバックするようにした。

## [0.1.7] - 2026-07-23

### 追加
- Linux 用 deb パッケージ作成スクリプト `scripts/package-deb.sh` を追加
  （`--download-pdfium` / `--no-build` オプション対応）。
- `build.rs` が `libpdfium.so`（Linux）も実行ファイルと同じ場所へコピーする
  ようにした。
- README に Linux deb パッケージの作成手順を追記。

### 変更
- Linux ではファイルダイアログの起動が遅い環境に備え、選択を UI スレッド外で
  待つようにした。またウィンドウタイトルを英語表記にした。

## [0.1.6] - 2026-07-18

### 修正
- Windows 版の `pdr.exe` にアプリアイコンを埋め込み、「アプリで開く」などの
  Windows シェル UI でも PDR のアイコンが表示されるようにした。小さい表示でも
  絵柄が見やすいよう、Windows 用アイコンの外周余白も調整した。

## [0.1.5] - 2026-07-01

### 追加
- macOS で Finder から PDF を開けるようにした（ダブルクリック・「PDR で開く」、
  コールドローンチ含む）。open-documents Apple Event を受け取り、起動中／起動時
  いずれのオープンも処理する。
- コマンドラインの `--version` / `-V` でバージョンを表示できるようにした。

## [0.1.4] - 2026-07-01

### 追加
- アプリアイコンを追加（macOS `.app` に `AppIcon.icns` を埋め込み、実行時にも
  `with_icon` でウィンドウ／Dock アイコンを設定）。
- macOS 用の署名・公証済み DMG を作成するスクリプト（`.app` 生成 → 署名 →
  公証 → ステープル）。※スクリプトは署名 ID を含むため未追跡。

## [0.1.3] - 2026-07-01

### 追加
- 表示エリアの左端／右端クリックでページを移動できるようにした（綴じ方向に連動。
  左綴じ=右で次・左で前、右綴じ=左で次・右で前）。
- トラックパッドのピンチで拡大・縮小できるようにした。

### 変更
- 二本指スクロール（およびマウスホイール）を画像のパン（移動）に割り当てた。
  拡大・縮小はピンチ、または修飾キー＋ホイール（macOS: Cmd、Windows: Ctrl）で行う。

## [0.1.2] - 2026-07-01

### 追加
- macOS 向けに `libpdfium.dylib`（arm64, build 151.0.7920.0）を同梱。
- `build.rs` が `pdfium.dll` / `libpdfium.dylib` のうち存在するものを実行ファイルと
  同じ場所へコピーするようにした（macOS でも起動ディレクトリに依存せず読み込める）。

### 修正
- **メニューの文字化け（macOS/Linux）**: 日本語フォントの探索先が Windows の
  パスのみだったため、日本語が豆腐（□）になっていた。macOS（ヒラギノ角ゴシック等）
  と Linux（Noto Sans CJK）のフォントパスを候補に追加した。
- **ウィンドウが真っ白／真っ黒（macOS）**: 既定の wgpu(Metal) レンダラーで描画され
  なかったため、macOS では glow(OpenGL) レンダラーを使うようにした。Windows は
  従来どおり wgpu のまま。

## [0.1.1] - 2026-06-30

### 追加
- `pdfium.dll` とサードパーティ ライセンスを同梱。
- Apache-2.0 ライセンスとパッケージ メタデータを追加。

## [0.1.0] - 2026-06-30

### 追加
- 初回リリース。PDF ポータブル ドキュメント リーダー（egui/eframe + pdfium）。
  見開き表示、縦／横綴じ、目次（しおり）、適応解像度の別スレッド描画に対応。

[Unreleased]: https://github.com/fukuyori/pdr/compare/0.2.0...HEAD
[0.2.0]: https://github.com/fukuyori/pdr/compare/0.1.8...0.2.0
[0.1.8]: https://github.com/fukuyori/pdr/compare/0.1.7...0.1.8
[0.1.7]: https://github.com/fukuyori/pdr/compare/0.1.6...0.1.7
[0.1.6]: https://github.com/fukuyori/pdr/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/fukuyori/pdr/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/fukuyori/pdr/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/fukuyori/pdr/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/fukuyori/pdr/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/fukuyori/pdr/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/fukuyori/pdr/releases/tag/0.1.0
