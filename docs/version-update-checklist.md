# バージョン更新チェックリスト

## 今回の更新

- 変更前: `0.3.0`
- 変更後: `0.4.0`
- 更新日: 2026-10-16

## 更新対象

- [x] `Cargo.toml` の `[package].version`
- [x] `Cargo.lock` の `pdr` パッケージのバージョン
- [x] `CHANGELOG.md` に 0.4.0 向けの変更内容を `Unreleased` として記録
- [x] 比較リンクとリリース見出しは実際のリリース時に更新する

## 変更不要であることを確認する対象

- [x] `src/main.rs` の `--version` 表示は `CARGO_PKG_VERSION` を参照している
- [x] Windows、Linux、macOS のパッケージスクリプトは `Cargo.toml` からバージョンを取得している
- [x] README の配布ファイル名は固定バージョンではなく `x.y.z` 表記である

## 更新後の確認

- [x] `Cargo.lock` の `pdr` パッケージが `0.4.0` である
- [x] `cargo check --offline --locked` が成功する
- [ ] PowerShell から `cargo test` を実行する（未実施）
- [x] `cargo run --offline -- --version` が `PDR 0.4.0` を表示する
- [x] パッケージ作成、タグ作成、リリース発行を行っていない
