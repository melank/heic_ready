# HEIC Ready

**Language:** [English](./README.md) | [日本語](./README.ja.md)

[![Platform: macOS](https://img.shields.io/badge/Platform-macOS-111111?style=flat-square&logo=apple&logoColor=white)](https://www.apple.com/macos/)
[![CI](https://github.com/melank/heic_ready/actions/workflows/ci.yml/badge.svg)](https://github.com/melank/heic_ready/actions/workflows/ci.yml)
[![Rust: 1.77+](https://img.shields.io/badge/Rust-1.77%2B-000000?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Tauri: 2.x](https://img.shields.io/badge/Tauri-2.x-24C8D8?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![Image stack: macOS sips](https://img.shields.io/badge/Image%20Stack-macOS%20sips-5B5BD6?style=flat-square)](https://ss64.com/mac/sips.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-2ea44f?style=flat-square)](./LICENSE)

HEIC Ready は、設定したフォルダを監視し、追加された `*.heic` / `*.heif` を自動で `*.jpg` に変換する macOS 向けバックグラウンドユーティリティです。

トレイ常駐を前提にした設計で、通常利用にフォアグラウンドの操作は不要です。

## スコープ

このプロジェクトが行うこと:

- フォルダ監視による HEIC/HEIF の検出
- JPEG の自動生成
- 元ファイルの扱い（ゴミ箱へ移動する `replace`、または残す `coexist`）
- トレイと設定ウィンドウによる軽い状態表示・操作

意図的に行わないこと:

- 写真管理 UI
- 画像編集
- クラウドアップロード
- Web サービス連携

## はじめに（ソースからビルド）

本プロジェクトでは Apple Developer Program を利用していないため、公開 DMG は**未公証**です。一般利用では手元ビルドを推奨します。

```bash
git clone https://github.com/melank/heic_ready.git
cd heic_ready
./scripts/build-and-install.sh
```

`/Applications/HEIC Ready.app` へインストールし、Gatekeeper の初回起動手順を表示します。

前提条件、手動ビルド、トラブルシューティングの詳細: [`docs/build-from-source.md`](./docs/build-from-source.md)

## ランタイム構成

- コア: Rust
- UI / トレイシェル: Tauri
- ファイル監視: `notify`
- デコード / エンコード: macOS `sips`（OS の画像スタック）

スレッドモデル:

- 監視ディスパッチャースレッドがファイルイベントを受信
- デバウンス後のパスをキューへ投入
- ワーカープールが変換を処理（最大 2 ワーカー）

## 変換の挙動

- 入力拡張子: `.heic`, `.heif`
- 出力拡張子: `.jpg`
- アトミックな出力書き込み:
  1. `*.tmp` に書き込む
  2. 最終の `*.jpg` へリネームする
  3. 元ファイルポリシー（`coexist` または `replace`）を適用する
- 同名衝突: 既存 JPEG は上書きしない
  - 例: `IMG_0001.heic` → `IMG_0001.jpg`
  - 既に存在する場合: `IMG_0001 (1).jpg`, `IMG_0001 (2).jpg`, ...

安定化ガード（書き込み途中のファイルを処理しないため）:

- ファイルサイズが 300ms 変化しないことを待つ
- 最大 3 回リトライ
- 安定化に失敗した場合は理由付きでスキップ

## 権限と安全性

- `replace` モードには、監視フォルダへの書き込み権限と `~/.Trash` への書き込み権限が必要
- 設定保存時に権限チェックが失敗した場合、`replace` は `coexist` にフォールバックする
- 変換結果およびスキップ / 失敗理由は直近ログバッファに保持する（最新 10 件）

## 設定

保存先:

- `app_config_dir/heic-ready/config.json`

主なフィールド:

- `watch_folders`
- `recursive_watch`
- `output_policy`（`coexist` / `replace`）
- `jpeg_quality`（`0..=100`）
- `rescan_interval_secs`（`15..=3600`）
- `paused`

## UI

- トレイメニュー:
  - 実行中 / 一時停止の状態
  - Pause / Resume
  - Settings
  - Recent Logs
  - 言語（EN / JA）
  - Quit
- 設定ウィンドウ:
  - 監視フォルダ
  - 再帰監視
  - 元 HEIC の置換
  - JPEG 品質
  - 再スキャン間隔
- Recent Logs ウィンドウ:
  - 直近 10 件（`success` / `failure` / `skip` / `info`）

## リリース

GitHub Releases では便宜上、アドホック署名の `.dmg` を公開することがありますが、**未公証**であり、一般利用者向けの推奨経路ではありません。

- 推奨: [ソースからビルド](./docs/build-from-source.md)
- 任意（自己責任）: `https://github.com/melank/heic_ready/releases/latest`
- 特定バージョン: `https://github.com/melank/heic_ready/releases/tag/vX.Y.Z`

リリースノート方針:

- バージョン固有のノートは `docs/releases/` に置く（例: `docs/releases/v0.1.0.md`）
- 公開は `tag push`（`vX.Y.Z`）で起動する
- インストーラ（`.dmg`）は `.github/workflows/release.yml` がビルド、アドホック署名、アップロードする
- 未署名 / アドホックビルドの初回起動は、右クリック → 開く（または「プライバシーとセキュリティ」）が必要

## ランディングページ

- ソース: `site/`
- デプロイ: GitHub Pages（`.github/workflows/pages.yml`）
- i18n: EN / JA（ブラウザ言語で自動切替、既定は JA）
- 運用メモ: `docs/github-pages.md`
- 品質ガイド: `docs/landing-page-quality.md`

ローカルプレビュー:

```bash
npx serve site
```

## システム要件

- macOS
- v0.1.0 インストーラ対象: Apple Silicon（`aarch64`）
- 入力ファイル種別: `*.heic`, `*.heif`

## 開発

開発要件:

- macOS
- Rust ツールチェーン
- Tauri フロントエンド作業用の Node.js 環境

開発実行:

```bash
npm run tauri dev
```

または

```bash
cargo tauri dev
```

## テスト

- 自動テストは `cargo test` で実行
- pre-commit フックがテストを自動実行（`.githooks/pre-commit`）
- 手動 E2E チェックリスト: `docs/e2e-manual-test.md`
- CI 運用ガイド: `docs/ci-operations.md`
- リリースチェックリスト: `docs/release-checklist.md`
- リリースノートテンプレート: `docs/release-notes-template.md`

フックのインストール（初回）:

```bash
./scripts/setup-githooks.sh
```

## ライセンス

MIT
