# CLAUDE.md

## リリース時のランディングページ更新ルール

新しいバージョンをリリースする際、以下の3ファイルを更新すること:

### 1. `site/index.html`

`.release-list` 内の **先頭** に新しいエントリを追加する（新しいものが上）:

```html
<article class="release-entry">
  <a class="release-version" href="https://github.com/melank/heic_ready/releases/tag/vX.Y.Z">vX.Y.Z</a>
  <p class="release-summary" data-i18n="releaseSummary_vX_Y_Z">
    English summary of this release.
  </p>
</article>
```

### 2. `site/i18n.js`

`en` と `ja` の両方に `releaseSummary_vX_Y_Z` キーを追加する:

```js
// en
releaseSummary_vX_Y_Z: "English summary.",

// ja
releaseSummary_vX_Y_Z: "日本語の要約。",
```

- キー名は `releaseSummary_vX_Y_Z`（ドットをアンダースコアに置換）
- 要約は1〜2文で、そのリリースの主な変更点を簡潔に記述する
