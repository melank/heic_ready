# GitHub Pages 運用メモ

このリポジトリではランディングページを `site/` 配下で管理する。

## 配信方式

- ワークフロー: `.github/workflows/pages.yml`
- トリガー: `master` への push / 手動実行 (`workflow_dispatch`)
- 配信ソース: `site/`

## 初回セットアップ

1. GitHub の Repository Settings を開く
2. `Pages` を開く
3. Source を `GitHub Actions` に設定する

## 公開URL

- `https://melank.github.io/heic_ready/`

## 更新フロー

1. `site/` を更新
2. `master` に反映
3. `Pages` ワークフロー完了後に公開ページを確認
