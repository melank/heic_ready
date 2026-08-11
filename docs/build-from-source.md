# Build from Source

**Language:** [English](./build-from-source.md) | [日本語](./build-from-source.ja.md)

HEIC Ready is open source. Because the project is not enrolled in the Apple Developer Program, distributed DMGs are **not notarized**. For general use, build and install locally on your Mac.

## Requirements

- macOS (Apple Silicon / `aarch64` is the primary target)
- [Xcode Command Line Tools](https://developer.apple.com/xcode/resources/)
- [Rust toolchain](https://rustup.rs/) (`rustc`, `cargo`)
- Tauri CLI 2.x (`cargo tauri`)

Verify:

```bash
xcode-select -p
rustc --version
cargo --version
cargo tauri --version
```

If Command Line Tools are missing:

```bash
xcode-select --install
```

If Rust is missing, install via rustup: https://rustup.rs/

If Tauri CLI is missing:

```bash
cargo install tauri-cli --version "^2"
```

## Recommended: one-shot install

From the repository root:

```bash
./scripts/build-and-install.sh
```

The script:

1. Checks prerequisites (does **not** auto-install tools)
2. Builds `HEIC Ready.app` with `cargo tauri build --bundles app`
3. Ad-hoc codesigns the app
4. Copies it to `/Applications/HEIC Ready.app` (asks before overwrite)
5. Prints first-launch instructions (if macOS blocks the app)

## Manual build (without the script)

```bash
cd src-tauri
cargo tauri build --bundles app
```

Find the app under one of:

- `src-tauri/target/release/bundle/macos/`
- `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/`

If `CARGO_TARGET_DIR` is set (e.g. by some IDE sandboxes), Tauri writes the bundle under that directory instead — look in `$CARGO_TARGET_DIR/release/bundle/macos/` (or the `aarch64-apple-darwin` / `x86_64-apple-darwin` subpaths).

From the repository root:

```bash
APP="$(find src-tauri/target -path '*/release/bundle/macos/*.app' -maxdepth 6 -print -quit)"
# If CARGO_TARGET_DIR is set and the above find returns nothing:
# APP="$(find "$CARGO_TARGET_DIR" -path '*/release/bundle/macos/*.app' -maxdepth 6 -print -quit)"
codesign --force --deep -s - "$APP"
codesign --verify --deep --strict "$APP"
cp -R "$APP" "/Applications/HEIC Ready.app"
```

## First launch (if macOS blocks the app)

Local builds are ad-hoc signed and not notarized. If macOS blocks the app, it may say the developer cannot be verified.

1. Open **Applications** in Finder
2. Control-click (right-click) **HEIC Ready** → **Open**
3. Confirm **Open** in the dialog

Or: **System Settings → Privacy & Security** and allow the blocked app.

Do **not** expect a notarized DMG experience.

## After install

1. Launch HEIC Ready (menu bar / tray icon)
2. Open **Settings**
3. Add a watch folder
4. Drop a `.heic` / `.heif` file into that folder and confirm a `.jpg` appears

## Troubleshooting

| Symptom | What to do |
|---|---|
| `Xcode Command Line Tools not found` | Run `xcode-select --install` |
| `rustc` / `cargo` not found | Install Rust via https://rustup.rs/ |
| `cargo-tauri not found` | `cargo install tauri-cli --version "^2"` |
| Build fails in `src-tauri` | Ensure you are on macOS with CLT installed; re-run after fixing the first error in the log |
| App won’t open / blocked by macOS | Use Control-click → Open, or Privacy & Security |
| Permission denied writing `/Applications` | Check Disk Access / admin rights; or copy the `.app` manually to a folder you own |

## Related

- Japanese guide: [`docs/build-from-source.ja.md`](./build-from-source.ja.md)
- Development (`tauri dev`): see README **Development**
- Unsigned GitHub Release DMGs (self-responsibility): see README **Releases**
