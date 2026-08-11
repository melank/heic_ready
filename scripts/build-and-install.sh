#!/usr/bin/env bash
set -euo pipefail

APP_NAME="HEIC Ready"
INSTALL_PATH="/Applications/${APP_NAME}.app"
BUILD_FROM_SOURCE_DOC="docs/build-from-source.md"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "${repo_root}"

die() {
  echo "error: $*" >&2
  exit 1
}

need_cmd() {
  local cmd="$1"
  local hint="$2"
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    die "${cmd} not found.
${hint}
See ${BUILD_FROM_SOURCE_DOC} for details."
  fi
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  die "HEIC Ready can only be built on macOS."
fi

if ! xcode-select -p >/dev/null 2>&1; then
  die "Xcode Command Line Tools not found.
Install with: xcode-select --install
See ${BUILD_FROM_SOURCE_DOC} for details."
fi

need_cmd rustc "Install Rust from https://rustup.rs (recommended) or another toolchain."
need_cmd cargo "Install Rust from https://rustup.rs (recommended) or another toolchain."

if ! cargo tauri --version >/dev/null 2>&1; then
  die "cargo-tauri (Tauri CLI) not found.
Install with: cargo install tauri-cli --version \"^2\"
See ${BUILD_FROM_SOURCE_DOC} for details."
fi

echo "==> Building ${APP_NAME}.app (this may take several minutes)..."
(
  cd src-tauri
  cargo tauri build --bundles app
)

bundle_dir=""
for candidate in \
  "src-tauri/target/release/bundle/macos" \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/macos" \
  "src-tauri/target/x86_64-apple-darwin/release/bundle/macos"
do
  if [[ -d "${candidate}" ]]; then
    bundle_dir="${candidate}"
    break
  fi
done

[[ -n "${bundle_dir}" ]] || die "Could not find built .app under src-tauri/target/**/release/bundle/macos"

app_path="$(find "${bundle_dir}" -maxdepth 1 -name '*.app' -print -quit)"
[[ -n "${app_path}" ]] || die "No .app found in ${bundle_dir}"

echo "==> Ad-hoc codesigning ${app_path}"
codesign --force --deep -s - "${app_path}"
codesign --verify --deep --strict "${app_path}"

if [[ -e "${INSTALL_PATH}" ]]; then
  echo "Existing app found at ${INSTALL_PATH}"
  if [[ ! -t 0 ]]; then
    die "Overwrite requires interactive confirmation (stdin is not a terminal).
Built app left at: ${app_path}"
  fi
  read -r -p "Overwrite? [y/N] " reply || reply=""
  case "${reply}" in
    y|Y|yes|YES) ;;
    *)
      echo "Aborted. Built app left at: ${app_path}"
      exit 0
      ;;
  esac
  if pgrep -f "${INSTALL_PATH}" >/dev/null 2>&1; then
    die "HEIC Ready is running. Quit the app first, then re-run this script.
Built app left at: ${app_path}"
  fi
  rm -rf "${INSTALL_PATH}"
fi

echo "==> Installing to ${INSTALL_PATH}"
cp -R "${app_path}" "${INSTALL_PATH}"

cat <<EOF

Installed: ${INSTALL_PATH}

If macOS blocks the app on first launch:
  1. Finder で「アプリケーション」を開く
  2. 「${APP_NAME}」を Control-クリック（右クリック）→「開く」
  3. 確認ダイアログで「開く」を選ぶ

Or: System Settings → Privacy & Security → allow the blocked app.

The app is ad-hoc signed and not notarized. This is expected for local builds.
Details: ${BUILD_FROM_SOURCE_DOC}
EOF
