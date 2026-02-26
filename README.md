# heic_ready

AirDropしたら、もう送れる。

heic_ready は macOS 常駐ユーティリティです。
HEIC 画像を自動的に “どこでも使える形式” に準備します。

ファイルを開く必要はありません。
変換ボタンもありません。

届いた瞬間、使えます。

## なぜ必要？

iPhone の写真は HEIC 形式です。
しかし多くの Web サービスは HEIC を受け付けません。

よくある流れ：

1. AirDrop
2. アップロード失敗
3. 変換サイトを探す
4. ダウンロードし直す
5. もう一度添付

heic_ready はこの手順を消します。

## 何をするアプリ？

指定したフォルダに HEIC が入ると自動的に JPEG が生成されます。
ユーザー操作は不要です。

## 特徴

* 常駐・超軽量
* ローカル完結（アップロードしない）
* Finder を開く前に準備完了
* 元ファイルは保持可能
* 同名ファイルを壊さない

## 使い方

1. 起動（`npm run tauri dev` もしくは `cargo tauri dev`）
2. メニューバーのトレイアイコンを開く
3. `Settings` から設定を保存する
4. `Pause/Resume` で処理状態を切り替える

## 設定ファイル

- 保存場所: Tauri `app_config_dir` 配下の `heic_ready/config.json`
- 主な項目:
  - `watch_folders`
  - `recursive_watch`
  - `output_policy` (`coexist` / `replace`)
  - `jpeg_quality`
  - `paused`

## 対応環境

macOS

## コンセプト

heic_ready は画像変換ツールではありません。

Mac が HEIC を理解できるようにする小さな補助レイヤです。
変換を意識させないことを目標にしています。

## ライセンス

MIT
