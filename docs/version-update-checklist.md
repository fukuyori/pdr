# バージョン更新チェックリスト

## 今回の更新

- 変更前: `0.1.8`
- 変更後: `0.2.0`
- 更新日: 2026-09-06

## 更新対象

- [x] `Cargo.toml` の `[package].version`
- [x] `Cargo.lock` の `pdr` パッケージのバージョン
- [x] `CHANGELOG.md` のリリース見出しと比較リンク
- [x] `docs/pdf-editing-decision.md` のフェーズ0とバージョン変更状況

## 変更不要であることを確認する対象

- [x] `src/main.rs` の `--version` 表示は `CARGO_PKG_VERSION` を参照している
- [x] `scripts/package.ps1` は `Cargo.toml` からバージョンを取得している
- [x] `scripts/package-deb.sh` は `Cargo.toml` からバージョンを取得している
- [x] `scripts/package-windows-inno.ps1` は `Cargo.toml` からバージョンを取得している
- [x] README の配布ファイル名は固定バージョンではなく `x.y.z` 表記である

## 更新後の確認

- [x] リポジトリ内に旧プロジェクトバージョン `0.1.8` が意図した履歴表記以外に残っていない
- [x] `cargo check` が成功する
- [x] PowerShell から `cargo test` が成功する
- [x] `cargo run -- --version` が `PDR 0.2.0` を表示する
- [x] パッケージ作成を行っていない
- [x] タグ作成、リリース発行を行っていない
