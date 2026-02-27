# CLI版 HEIC→JPEG 変換処理 実装計画（M2先行）

## Summary
macOS の ImageIO を使い、**単一 HEIC/HEIF ファイル入力**を JPEG へ変換する CLI を先に実装する。  
品質要件は **JPEG品質100固定**、表示要件は **orientationを画素に反映して出力**、安全性要件は **`.tmp` への原子的書き込み + rename** を必須とする。  
将来の常駐アプリ統合に備え、**coreクレート + CLI bin** の2層構成で作る。

## 仕様確定（今回の意思決定）
- 入力: 単一ファイルのみ（`.heic` / `.heif`）
- 出力: 同一ディレクトリに `.jpg`
- 品質: `100` 固定
- メタ情報: orientation反映 + 基本メタ保持
- 配置: `coreクレート + CLI bin`
- デフォルト動作: coexist（元HEICは残す）

## 変更する公開インターフェース / 構成
- 追加クレート
- `crates/heic-ready-core`（変換ロジック）
- `crates/heic-ready-cli`（CLIエントリ）
- 追加コマンド
- `cargo run -p heic-ready-cli -- <input.heic>`
- core API（公開）
- `ConvertOptions { jpeg_quality: u8 /*=100*/ }`
- `ConvertResult { input_path, output_path, bytes_written, elapsed_ms }`
- `convert_heic_to_jpeg(input: &Path, opts: &ConvertOptions) -> Result<ConvertResult, ConvertError>`
- エラー型（公開）
- `InvalidInput`, `UnsupportedExtension`, `DecodeFailed`, `EncodeFailed`, `PermissionDenied`, `AtomicWriteFailed`, `MetadataReadFailed`

## 実装詳細（decision complete）

### 1. プロジェクト構造
- ルートに Cargo workspace を作成し、`src-tauri`, `crates/heic-ready-core`, `crates/heic-ready-cli` を member 化
- `heic-ready-cli` は `clap` で最小引数を受ける
- `heic-ready-cli` は I/O と終了コードのみ担当、変換処理は全て `heic-ready-core` に委譲

### 2. CLI仕様（固定）
- 形式: `heic-ready-cli <input_path>`
- 挙動:
- 拡張子が `.heic/.heif` 以外なら終了コード `2`
- 変換成功で終了コード `0`
- 変換失敗は終了コード `1`
- 標準出力:
- 成功: `Converted: <input> -> <output>`
- 失敗: `Error: <reason>`

### 3. 変換パイプライン（core）
- 入力検証:
- ファイル存在、通常ファイル、拡張子検証
- 出力先決定:
- 同名 `.jpg` が無ければ `<stem>.jpg`
- 存在時は `<stem> (1).jpg`, `<stem> (2).jpg`…（上書き禁止）
- デコード（ImageIO）:
- `CGImageSourceCreateWithURL` で source 作成
- `CGImageSourceCreateImageAtIndex` でフル解像度 `CGImage` を取得
- orientation / メタ取得:
- source properties から orientation と基本メタを取得
- orientation は「タグ保存」ではなく「画素へ適用」方針
- orientation反映:
- orientation 値に応じて `CGAffineTransform` を適用して新規 `CGContext` へ描画
- 出力画像の orientation は実質 `1` 相当になるようにする
- エンコード（JPEG）:
- `CGImageDestinationCreateWithURL` で `.tmp` パスへ出力
- `kCGImageDestinationLossyCompressionQuality = 1.0`
- 基本メタ（日時等）を引き継ぎつつ orientation は正規化値で書き込む
- 原子的書き込み:
- `<output>.tmp` へ書く
- `Finalize` 成功後に `rename(tmp, final)`
- 失敗時は tmp を削除してエラー返却

### 4. 色空間と品質の扱い
- 色空間:
- source の color profile / color space を優先して維持
- 取得不可時のみ sRGB フォールバック
- 品質:
- JPEG 仕様上ロスレスではないため、要件「品質を落とさず」は **品質100固定 + リサイズなし + 1回変換** で満たす

### 5. ログ / 観測（CLI段階）
- core は `log` crate で `info/warn/error`
- CLI は標準出力/標準エラーで結果を明示
- 変換時間（ms）と出力サイズを成功時に出す（将来常駐の性能検証に流用）

## テスト計画

### 単体テスト（core）
- 出力名衝突:
- `IMG_0001.heic` -> `IMG_0001.jpg`
- 既存時 -> `IMG_0001 (1).jpg`
- 拡張子検証:
- `.jpg` 入力は `UnsupportedExtension`
- 原子的書き込み:
- tmp名生成と rename 成功/失敗の分岐を検証（FSモックまたは一時ディレクトリ）

### 結合テスト（macOS限定）
- 正常:
- fixture HEIC 1枚を変換し JPEG 生成を確認
- 出力画像の縦横/見た目向きが orientation 通りであること
- メタ:
- 主要メタ（少なくとも作成日時系）引き継ぎを確認
- 異常:
- 読み取り権限なしファイルで失敗
- 壊れたHEICで `DecodeFailed`
- 既存同名JPEGがある状態で非上書き

### 受け入れ基準
- CLI 1コマンドで HEIC が JPEG に変換される
- 出力ファイル破損ゼロ（tmp→rename を常に使用）
- orientation が画素に反映され、Webアップロード時に向き崩れしない
- 同名衝突で既存JPEGを上書きしない

## 実装順序
1. workspace化 + `heic-ready-core` / `heic-ready-cli` 追加  
2. core の I/Oバリデーション・出力命名・原子的書き込み  
3. ImageIO decode/encode（品質100）  
4. orientation画素反映 + メタ引き継ぎ  
5. CLI配線（終了コード/メッセージ）  
6. 単体・結合テスト整備  
7. README に CLI利用例を追記

## 明示的な前提・デフォルト
- 対象OSは macOS のみ
- 入力は単一ファイルのみ（ディレクトリ処理は今回スコープ外）
- 品質オプションは公開しない（100固定）
- 出力ポリシーは coexist 固定（replace は後続マイルストーン）
- 変換エンジンは OS ImageIO を使用し、外部HEICデコーダは使わない
