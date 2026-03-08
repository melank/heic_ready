# リリースチェックリスト（GitHub配布）

このチェックリストは、GitHubでリリースを公開する前提の最小確認項目を定義する。

## リリーストリガー方針

- 正式リリースのトリガーは `tag push` のみとする
- タグ形式は `vX.Y.Z`（例: `v0.3.1`）を使用する
- コミット push ではリリースを作成しない

## 1. 事前確認

- `master` が最新である
- CIが `green` である
- 未コミット変更がない
- GitHub Actions Secrets が設定済みである
  - 必須: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`
  - Notarization（どちらか一式）:
    - `APPLE_ID`, `APPLE_PASSWORD`（または `APPLE_APP_SPECIFIC_PASSWORD`）, `APPLE_TEAM_ID`
    - `APPLE_API_KEY_ID`, `APPLE_API_ISSUER`, `APPLE_API_KEY_CONTENT`

## 2. 品質確認

- `cargo test --lib` がローカルで成功する
- 手動E2Eを実施し、結果を記録する（`docs/e2e-manual-test.md`）
- 最近の変更に対応するドキュメントが更新済みである

## 3. リリース準備

- バージョン番号を確定する
- `docs/release-notes-template.md` を元にリリースノート草案を作成する
  - `Added / Changed / Fixed / Known Issues / Upgrade Notes` を埋める
  - 公開時にGitHub Release本文へ転記する
- タグ作成コマンドを準備する
  - `git tag vX.Y.Z`
  - `git push origin vX.Y.Z`

## 4. 公開後確認

- `release.yml` が成功している
- リリースページに `.dmg` 成果物とノートが正しく表示される
- ワークフローログで `Verify codesign and notarization` が成功している
- 主要な起動/変換フローを再確認する
- 問題がある場合は修正方針をIssue化する

## 運用ルール

- チェックリストを満たさないリリースは公開しない
- 手順を変更した場合は、このドキュメントを必ず更新する
- 既存タグの再利用（上書き）は行わない
