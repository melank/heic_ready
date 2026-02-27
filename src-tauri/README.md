# src-tauri

This directory contains the Tauri backend and desktop app packaging configuration for HEIC Ready.

## Responsibilities

- Tray app lifecycle and menu handling
- Folder watch orchestration and worker execution
- HEIC/HEIF to JPEG conversion pipeline
- App config load/save and runtime updates
- Tauri command boundary exposed to frontend

## Directory Guide

- `src/main.rs`  
  Binary entrypoint.
- `src/lib.rs`  
  App bootstrap, tray menu, window lifecycle, state wiring.
- `src/commands.rs`  
  Tauri commands (`get_config`, `update_config`, `get_recent_logs`, etc.).
- `src/watcher.rs`  
  File watching, debounce/stabilization, conversion dispatch, recent logs buffer.
- `src/config.rs`  
  Config model and persistence (`app_config_dir/heic-ready/config.json`).
- `tauri.conf.json`  
  Tauri app metadata, bundle settings, window config.
- `capabilities/`  
  Capability definitions used by Tauri ACL.
- `gen/schemas/`  
  Generated schemas/manifests for capability validation (do not hand-edit).

## Development Commands

From repository root:

```bash
cargo tauri dev
```

From this directory:

```bash
cargo test --lib
cargo tauri build --bundles app
cargo tauri build --bundles dmg
```

DMG build note:

- `cargo tauri build --bundles dmg` may fail in restricted/sandboxed environments because `hdiutil` and Finder AppleScript steps require broader system access.
- If DMG creation fails while `.app` bundling succeeds, retry outside the restricted sandbox.

## Notes

- Conversion output must be atomic (`*.tmp` then rename).
- Stabilization check is required before conversion.
- Same-name collisions must never overwrite existing JPEG.
- `replace` mode requires writable watch folder and writable Trash.

## Editing Policy

- Prefer editing capability source files under `capabilities/`.
- Treat `gen/schemas/` as generated artifacts.
- Keep user-visible behavior aligned with `docs/e2e-manual-test.md`.
