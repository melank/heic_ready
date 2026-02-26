# HEIC Ready

HEIC Ready is a background macOS utility that watches configured folders and converts incoming `*.heic` / `*.heif` files to `*.jpg` automatically.

The app is designed as a tray-first daemon. No foreground workflow is required for normal operation.

## Scope

What this project does:

- Folder-based HEIC/HEIF detection
- Automatic JPEG generation
- Optional source replacement (`move to Trash`) or coexist mode
- Lightweight status/control via tray + settings window

What this project intentionally does not do:

- Photo management UI
- Image editing
- Cloud upload
- Web service integration

## Runtime Architecture

- Core: Rust
- UI/tray shell: Tauri
- File watching: `notify`
- Decode/encode path: macOS `sips` (OS image stack)

Thread model:

- Watch dispatcher thread receives file events
- Debounced paths are queued
- Worker pool processes conversions (max 2 workers)

## Conversion Behavior

- Input extensions: `.heic`, `.heif`
- Output extension: `.jpg`
- Atomic output write:
  1. Write to `*.tmp`
  2. Rename to final `*.jpg`
  3. Apply source policy (`coexist` or `replace`)
- Name collision policy: never overwrite existing JPEG
  - Example: `IMG_0001.heic` -> `IMG_0001.jpg`
  - If exists: `IMG_0001 (1).jpg`, `IMG_0001 (2).jpg`, ...

Stabilization guard (to avoid processing files still being written):

- Wait until file size is unchanged for 300ms
- Retry up to 3 times
- Skip with reason if stabilization fails

## Permissions and Safety

- `replace` mode requires writable watch folder and writable `~/.Trash`
- If permission checks fail while saving config, `replace` falls back to `coexist`
- Conversion and skip/failure reasons are kept in a recent log buffer (latest 10)

## Configuration

Stored at:

- `app_config_dir/heic_ready/config.json`

Main fields:

- `watch_folders`
- `recursive_watch`
- `output_policy` (`coexist` / `replace`)
- `jpeg_quality` (`0..=100`)
- `rescan_interval_secs` (`15..=3600`)
- `paused`

## UI Surfaces

- Tray menu:
  - Running/Paused status
  - Pause/Resume
  - Settings
  - Recent Logs
  - Quit
- Settings window:
  - Watch folders
  - Recursive watch
  - Replace source HEIC
  - JPEG quality
  - Rescan interval
- Recent Logs window:
  - Last 10 records (`success` / `failure` / `skip` / `info`)

## Development

Requirements:

- macOS
- Rust toolchain
- Node.js environment for Tauri frontend workflow

Run in development:

```bash
npm run tauri dev
```

or

```bash
cargo tauri dev
```

## Testing

- Automated tests run via `cargo test`
- Pre-commit hook runs tests automatically (`.githooks/pre-commit`)
- Manual E2E checklist: `docs/e2e-manual-test.md`

Install hooks (first time):

```bash
./scripts/setup-githooks.sh
```

## License

MIT
