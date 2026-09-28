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

## ページ構成（`site/index.html`）

1. ヒーロー（1文の訴求 + CTA）
2. 動作イメージ（`.flow`）: iPhone → 監視フォルダ → SNS の流れを CSS アニメーションで表現。JS は使わず、`prefers-reduced-motion` では投稿完了の場面で静止する
3. 機能カード（`.grid`）
4. 使いかた・利用の流れ（`.steps`）
5. よくある質問（`.faq`）
6. リリース一覧（`.release-sidebar`）

文言は `site/i18n.js` の `data-i18n` キーで EN/JA を切り替える。

## 更新フロー

1. `site/` を更新
2. `master` に反映
3. `Pages` ワークフロー完了後に公開ページを確認

## 関連ドキュメント

- `docs/landing-page-quality.md`
