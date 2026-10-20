# バージョン更新チェックリスト

## 今回の更新

- 変更前: `0.4.1`
- 変更後: `0.4.2`
- 更新日: 2026-10-20

## 更新対象

- [x] `Cargo.toml` の `[package].version`
- [x] `Cargo.lock` の `pdr` パッケージのバージョン
- [x] `CHANGELOG.md` に 0.4.2 向けの変更内容を `Unreleased` として記録
- [x] `README.md` を英語、`README.ja.md` を日本語にし、翻訳ウィンドウの説明を現行 UI に合わせる
- [x] 比較リンクとリリース見出しは実際のリリース時に更新する

## 変更不要であることを確認する対象

- [x] `src/main.rs` の `--version` 表示は `CARGO_PKG_VERSION` を参照している
- [x] Windows、Linux、macOS のパッケージスクリプトは `Cargo.toml` からバージョンを取得している
- [x] README の配布ファイル名は固定バージョンではなく `x.y.z` 表記である

## 更新後の確認

- [x] `Cargo.lock` の `pdr` パッケージが `0.4.2` である
- [x] `cargo check --offline --locked` が成功する
- [x] PowerShell から `cargo test` を実行する（22件成功）
- [x] `cargo run --offline -- --version` が `PDR 0.4.2` を表示する
- [x] パッケージ作成、タグ作成、リリース発行を行っていない
