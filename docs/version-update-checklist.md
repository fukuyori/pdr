# バージョン更新チェックリスト

## 今回の更新

- 変更前: `0.2.0`
- 変更後: `0.2.1`
- 更新日: 2026-09-06

## 更新対象

- [x] `Cargo.toml` の `[package].version`
- [x] `Cargo.lock` の `pdr` パッケージのバージョン
- [x] `CHANGELOG.md` のリリース見出しと比較リンク
- [x] `CHANGELOG.md` に0.2.1の画像補正変更を記録

## 変更不要であることを確認する対象

- [x] `src/main.rs` の `--version` 表示は `CARGO_PKG_VERSION` を参照している
- [x] `scripts/package.ps1` は `Cargo.toml` からバージョンを取得している
- [x] `scripts/package-deb.sh` は `Cargo.toml` からバージョンを取得している
- [x] `scripts/package-windows-inno.ps1` は `Cargo.toml` からバージョンを取得している
- [x] README の配布ファイル名は固定バージョンではなく `x.y.z` 表記である
- [x] `docs/pdf-editing-decision.md` と引き継ぎ資料の `0.2.0` はフェーズ0の履歴表記である

## 更新後の確認

- [x] `Cargo.lock` の `pdr` パッケージが `0.2.1` である
- [x] `cargo check` が成功する
- [x] PowerShell から `cargo test` が成功する
- [x] `cargo run -- --version` が `PDR 0.2.1` を表示する
- [x] パッケージ作成を行っていない
- [x] タグ作成、リリース発行を行っていない
