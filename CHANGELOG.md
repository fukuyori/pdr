# 変更履歴

このプロジェクトの主要な変更点を記録します。
書式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に準拠し、
バージョニングは [セマンティック バージョニング](https://semver.org/lang/ja/) に従います。

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

[0.1.4]: https://github.com/fukuyori/pdr/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/fukuyori/pdr/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/fukuyori/pdr/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/fukuyori/pdr/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/fukuyori/pdr/releases/tag/0.1.0
