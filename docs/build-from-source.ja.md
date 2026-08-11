# ソースからビルド

**Language:** [English](./build-from-source.md) | [日本語](./build-from-source.ja.md)

HEIC Ready はオープンソースです。本プロジェクトは Apple Developer Program に未加入のため、配布用 DMG は**未公証**です。一般利用では、Mac 上で手元ビルドしてインストールしてください。

## 要件

- macOS（主対象は Apple Silicon / `aarch64`）
- [Xcode Command Line Tools](https://developer.apple.com/xcode/resources/)
- [Rust ツールチェーン](https://rustup.rs/)（`rustc` / `cargo`）
- Tauri CLI 2.x（`cargo tauri`）

確認:

```bash
xcode-select -p
rustc --version
cargo --version
cargo tauri --version
```

Command Line Tools がない場合:

```bash
xcode-select --install
```

Rust がない場合は rustup で導入してください: https://rustup.rs/

Tauri CLI がない場合:

```bash
cargo install tauri-cli --version "^2"
```

## 推奨: ワンショットインストール

リポジトリのルートで:

```bash
./scripts/build-and-install.sh
```

スクリプトの内容:

1. 前提ツールを確認する（**自動インストールはしない**）
2. `cargo tauri build --bundles app` で `HEIC Ready.app` をビルドする
3. アドホック署名する
4. `/Applications/HEIC Ready.app` へコピーする（上書き時は確認する）
5. 初回起動の案内を表示する（macOS にブロックされた場合）

## 手動ビルド（スクリプトを使わない場合）

```bash
cd src-tauri
cargo tauri build --bundles app
```

`.app` は次のいずれか配下にあります:

- `src-tauri/target/release/bundle/macos/`
- `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/`

`CARGO_TARGET_DIR` が設定されている場合（一部 IDE のサンドボックスなど）、バンドルはそのディレクトリ配下に出力されます。`$CARGO_TARGET_DIR/release/bundle/macos/`（または `aarch64-apple-darwin` / `x86_64-apple-darwin` のサブパス）を確認してください。

リポジトリのルートから:

```bash
APP="$(find src-tauri/target -path '*/release/bundle/macos/*.app' -maxdepth 6 -print -quit)"
# CARGO_TARGET_DIR が設定されていて上記で見つからない場合:
# APP="$(find "$CARGO_TARGET_DIR" -path '*/release/bundle/macos/*.app' -maxdepth 6 -print -quit)"
codesign --force --deep -s - "$APP"
codesign --verify --deep --strict "$APP"
cp -R "$APP" "/Applications/HEIC Ready.app"
```

## 初回起動（macOS にブロックされた場合）

手元ビルドはアドホック署名のみで、公証されていません。macOS がアプリをブロックした場合、「開発元を確認できない」と表示されることがあります。

1. Finder で **アプリケーション** を開く
2. **HEIC Ready** を Control-クリック（右クリック）→ **開く**
3. ダイアログで **開く** を確認する

または **システム設定 → プライバシーとセキュリティ** からブロックされたアプリを許可します。

公証済み DMG のような体験は期待しないでください。

## インストール後

1. HEIC Ready を起動する（メニューバー / トレイアイコン）
2. **Settings** を開く
3. 監視フォルダを追加する
4. そのフォルダに `.heic` / `.heif` を入れ、`.jpg` が生成されることを確認する

## トラブルシューティング

| 症状 | 対処 |
|---|---|
| `Xcode Command Line Tools not found` | `xcode-select --install` を実行する |
| `rustc` / `cargo` が見つからない | https://rustup.rs/ から Rust を入れる |
| `cargo-tauri not found` | `cargo install tauri-cli --version "^2"` |
| `src-tauri` でビルド失敗 | macOS と CLT が入っていることを確認し、ログ先頭のエラーから直して再実行する |
| アプリが開かない / macOS にブロックされる | Control-クリック → 開く、または「プライバシーとセキュリティ」 |
| `/Applications` への書き込み権限がない | ディスクアクセス / 管理者権限を確認する。または自分のフォルダへ `.app` を手動コピーする |

## 関連

- English guide: [`docs/build-from-source.md`](./build-from-source.md)
- 開発（`tauri dev`）: README.ja.md の **開発** を参照
- 未公証の GitHub Release DMG（自己責任）: README.ja.md の **リリース** を参照
