# 手元ビルド配布導線デザイン

日付: 2026-08-11  
ステータス: 承認済み（実装前）

## 背景

heic-ready は Apple Developer Program 未加入のため、配布用 DMG を公証できない。
現状の GitHub Releases DMG はアドホック署名のみで、一般ユーザーがダウンロードすると Gatekeeper で詰まりやすい。

ローカルでは開発者向けの `tauri dev` しか案内しておらず、一般ユーザーが手元でビルドして使う導線がない。

## ゴール

一般ユーザーが「クローン → スクリプト1本 → /Applications に入る → 初回起動」まで辿り着けるようにする。
README と GitHub Pages では手元ビルドを第一推奨とし、DMG は未公証・自己責任／開発者向けに格下げする。

## 非ゴール

- Apple Developer 証明書・公証の導入
- 前提ツール（Rust / Xcode CLT / cargo-tauri）の自動インストール
- Homebrew 対応
- Intel（x86_64）向けビルド保証の追加（Apple Silicon 中心のまま。docs に明記）
- `release.yml` の削除（DMG 生成は残すが推奨導線からは外す）
- ランディングページの全面リデザイン（文言・CTA 変更に留める）

## ユーザー導線（優先順）

1. リポジトリをクローンする
2. `./scripts/build-and-install.sh` を実行する
3. `/Applications/HEIC Ready.app` を起動する（Gatekeeper 案内に従う）
4. トレイから監視フォルダを設定して使う

## 成果物

| ファイル | 役割 |
|---|---|
| `scripts/build-and-install.sh` | 前提チェック → ビルド → アドホック署名 → `/Applications` へコピー → 初回起動案内 |
| `docs/build-from-source.md` | 前提ツール、手動手順、Gatekeeper、トラブルシュート |
| `README.md` | Get Started を第一推奨。Releases を格下げ |
| `site/index.html` / `site/i18n.js` | CTA・FAQ・JSON-LD を手元ビルド寄りに更新 |

## スクリプト仕様: `scripts/build-and-install.sh`

### 前提

- macOS のみ（それ以外は即終了）
- リポジトリルートから実行可能（スクリプト位置からリポジトリルートを解決）

### 前提チェック

不足時はインストールせず、導入手順を表示して `exit 1` する。

1. Command Line Tools: `xcode-select -p`
2. Rust: `rustc` / `cargo`
3. Tauri CLI: `cargo tauri --version`  
   不足時は `cargo install tauri-cli --version "^2"` を案内する

### ビルド〜インストール

1. `cargo tauri build --bundles app`（ホストの native target。現状ドキュメント上は Apple Silicon 中心）
2. 生成された `.app` を `codesign --force --deep -s -` でアドホック署名し、`codesign --verify` する
3. 既存の `/Applications/HEIC Ready.app` がある場合のみ上書き確認（`y/N`）
4. `/Applications` へコピー
5. 終了時に Gatekeeper 案内を表示する  
   - 初回は右クリック → 開く  
   - またはシステム設定 → プライバシーとセキュリティ

### スクリプトがやらないこと

- DMG 作成
- 公証
- 前提ツールの自動インストール
- アプリの自動 `open`（ユーザー操作を残す）

## README 変更方針

- 冒頭近くに **Get Started（Build from source）** を追加する
  - 推奨手段は `./scripts/build-and-install.sh`
  - 詳細は `docs/build-from-source.md` へリンク
- **Releases** を格下げする
  - DMG は未公証・アドホック署名のみである旨を明示
  - 一般ユーザーには手元ビルドを推奨
  - GitHub Releases のリンクは残す（開発者・自己責任向け）
- 既存の **Development**（`tauri dev`）は開発者向けとして残す

## docs/build-from-source.md

含める内容:

- システム要件（macOS / Apple Silicon 中心）
- 前提ツール一覧と手動確認方法
- スクリプト利用手順
- 手動ビルド手順（スクリプトを使わない場合）
- Gatekeeper / 「開発元を確認できない」対処
- よくある失敗（CLT 未導入、cargo-tauri なし、権限）

## GitHub Pages（site/）変更方針

- 主 CTA を「DMG をダウンロード」から「ソースからビルド」系へ変更する
  - リンク先は固定: `https://github.com/melank/heic_ready/blob/master/docs/build-from-source.md`
- 副 CTA は GitHub リポジトリ（現状の「ソースを見る」を維持、必要なら文言調整）
- FAQ に「なぜ DMG を第一推奨しないか / 手元ビルドが必要か」を1項目追加する
- JSON-LD の `downloadUrl` も同じく `docs/build-from-source.md` の blob URL に更新する（バイナリ配布ではなくインストール手順への誘導）
- リリースサイドバーは残す（履歴表示）。未公証である短注記を足す
- ランディングの全面リデザインはしない

## 成功条件

1. 一般ユーザーが README / LP を見て「まず手元ビルド」と分かる
2. 前提が揃っていればスクリプト1本で `/Applications` に入り、初回起動手順が分かる
3. DMG が未公証であることが明示され、誤解してダウンロードして詰まる導線が弱まる

## 実装メモ

- スクリプトは `set -euo pipefail` を使う
- 既存 `scripts/setup-githooks.sh` と同様、実行ビット付きで置く
- UI 変更ではないため `docs/e2e-manual-test.md` の更新は必須ではない（必要なら手元ビルド確認の1項目を任意追加）
- コミットは日本語メッセージ、レビュー可能な粒度で分割する
