# CI運用ガイド

このドキュメントは、`GitHub Actions` のCI/Release運用手順を定義する。

対象ワークフロー:

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

関連方針:

- リリースは `tag push` で発火する（コミット push では発火しない）
- リリース運用は `docs/release-checklist.md` に従う

## 実行条件

- `master` への `push`
- `pull_request`
- `vX.Y.Z` 形式の `tag push`（例: `v0.1.0`）

## 日常フロー

1. ローカルで変更を作成する
2. 必要なテストをローカルで実行する（最低 `cargo test --lib`）
3. PRを作成する
4. CIが `green` であることを確認してからマージする
5. リリース時は `git tag vX.Y.Z && git push origin vX.Y.Z` を実行する
6. `release.yml` が成功し、GitHub Release に `.dmg` が添付されていることを確認する
7. `Verify codesign and notarization` が成功していることを確認する

## 失敗時の対応

1. CIログを確認し、失敗箇所を特定する
2. ローカルで同じコマンドを再実行して再現させる
3. 修正後に再度PRへpushし、CI再実行結果を確認する

補足:

- CIが通っていないPRはマージしない
- 一時的な外部要因を除き、`retry` 前に原因を説明できる状態にする

## 変更ルール

- CI内容を変更する場合は、同時にこのドキュメントを更新する
- CI変更PRでは、変更理由と影響範囲をPR本文に明記する
