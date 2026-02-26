# heic_ready 手動E2Eテスト

このドキュメントは、リリース前に実施する手動E2Eテスト項目を固定化したもの。
毎回同じ手順で確認し、回帰を検出することを目的とする。

## 前提

- macOS 環境
- `heic_ready` が起動していること
- 設定画面で `watch_folders` に検証用フォルダ（例: `~/Downloads`）が設定済みであること
- テストで使う `*.heic` / `*.heif` サンプルを用意していること

## 合否ルール

- 1件でも期待結果を満たさない場合は `FAIL`
- `FAIL` 時は `Recent logs` とターミナルログ（`app_lib::watcher`）を保存する

## ケース

### 1. coexist 基本変換

1. `Replace source HEIC (move to Trash)` を OFF
2. `Save`
3. 監視フォルダへ `IMG_A.heic` を配置

期待:
- `IMG_A.jpg` が生成される
- `IMG_A.heic` は残る
- `Recent logs` に `SUCCESS` が記録される

### 2. replace 変換 + Trash移動

1. `Replace source HEIC (move to Trash)` を ON
2. `Save`
3. 監視フォルダへ `IMG_B.heic` を配置

期待:
- `IMG_B.jpg` が生成される
- 元 `IMG_B.heic` は監視フォルダから消える
- `~/.Trash` に元ファイルが移動される
- `Recent logs` に `SUCCESS` が記録される

### 3. 安定化判定（書き込み中ファイルを即処理しない）

1. 大きめファイルをコピー中に監視フォルダへ投入
2. コピー中のログを確認

期待:
- コピー途中で破損した `jpg` が作られない
- コピー完了後に変換される

### 4. 再スキャン再生成

1. `Replace` を OFF（coexist）にする
2. `Rescan interval (sec)` を `15` に設定して `Save`
3. `IMG_C.heic` を配置し `IMG_C.jpg` 生成を確認
4. `IMG_C.jpg` だけ削除

期待:
- 削除直後には再生成されない
- 最大15秒程度で `IMG_C.jpg` が再生成される

### 5. Pause/Resume

1. トレイで `Pause`
2. `IMG_D.heic` を配置
3. トレイで `Resume`

期待:
- Pause中は変換されない
- Resume後に変換される
- トレイ表示が `🔴 Paused` / `🟢 Ready` で切り替わる

### 6. 入力バリデーション

1. `JPEG quality` に `101` を入力して `Save`
2. `Rescan interval (sec)` に `10` を入力して `Save`

期待:
- どちらも保存失敗になる
- エラーメッセージが表示される
- 既存設定は保持される

### 7. 同名衝突

1. `IMG_E.heic` を配置して `IMG_E.jpg` を生成
2. 同じ `IMG_E.heic`（別内容）を再配置

期待:
- 既存 `IMG_E.jpg` は上書きされない
- `IMG_E (1).jpg` 形式で新規生成される

## 実施記録テンプレート

```text
Date:
Build/Commit:
Tester:

[1] coexist 基本変換: PASS / FAIL
[2] replace + Trash移動: PASS / FAIL
[3] 安定化判定: PASS / FAIL
[4] 再スキャン再生成: PASS / FAIL
[5] Pause/Resume: PASS / FAIL
[6] 入力バリデーション: PASS / FAIL
[7] 同名衝突: PASS / FAIL

Notes:
```

