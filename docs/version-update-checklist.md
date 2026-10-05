# バージョン更新チェックリスト

## 今回の更新

- 変更前: `0.2.1`
- 変更後: `0.3.0`
- 更新日: 2026-10-05

## 更新対象

- [x] `Cargo.toml` の `[package].version`
- [x] `Cargo.lock` の `pdr` パッケージのバージョン
- [x] `CHANGELOG.md` のリリース見出しと比較リンク
- [x] `CHANGELOG.md` に0.3.0のページ単位回転機能を記録

## 変更不要であることを確認する対象

- [x] `src/main.rs` の `--version` 表示は `CARGO_PKG_VERSION` を参照している
- [x] `scripts/package.ps1` は `Cargo.toml` からバージョンを取得している
- [x] `scripts/package-deb.sh` は `Cargo.toml` からバージョンを取得している
- [x] `scripts/package-windows-inno.ps1` は `Cargo.toml` からバージョンを取得している
- [x] README の配布ファイル名は固定バージョンではなく `x.y.z` 表記である
- [x] `docs/pdf-editing-decision.md` と引き継ぎ資料の `0.2.0` はフェーズ0の履歴表記である

## 更新後の確認

- [x] `Cargo.lock` の `pdr` パッケージが `0.3.0` である
- [x] `cargo check --offline` が成功する
- [ ] PowerShell から `cargo test` を実行する（未実施）
- [x] `cargo run --offline -- --version` が `PDR 0.3.0` を表示する
- [x] パッケージ作成を行っていない
- [x] タグ作成、リリース発行を行っていない
