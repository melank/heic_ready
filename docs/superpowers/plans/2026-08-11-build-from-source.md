# Build-from-source distribution path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 一般ユーザーがクローン後に `./scripts/build-and-install.sh` 一本で `/Applications` へ入れ、README と GitHub Pages で手元ビルドを第一推奨にする。

**Architecture:** シェルスクリプトが前提チェック・Tauri `.app` ビルド・アドホック署名・`/Applications` コピー・Gatekeeper 案内まで担う。手順の詳細は `docs/build-from-source.md`、入口は README と `site/` の CTA。DMG Release は残すが推奨導線から外す。

**Tech Stack:** bash, Tauri CLI 2.x, macOS `codesign`, 静的 HTML/`site/i18n.js`

## Global Constraints

- macOS のみ。ドキュメント上は Apple Silicon（`aarch64`）中心
- 前提ツールは自動インストールしない（不足時は案内して `exit 1`）
- 公証・Developer ID 署名はしない（アドホック署名のみ）
- DMG 作成・アプリの自動 `open` はしない
- `release.yml` は削除しない
- LP の全面リデザインはしない（文言・CTA・FAQ・JSON-LD のみ）
- コミットメッセージは日本語
- LP 主 CTA / JSON-LD `downloadUrl` のリンク先は固定: `https://github.com/melank/heic_ready/blob/master/docs/build-from-source.md`
- アプリ名（インストール先）: `/Applications/HEIC Ready.app`（`tauri.conf.json` の `productName`）

---

## File map

| File | Responsibility |
|---|---|
| Create: `scripts/build-and-install.sh` | 前提チェック → build → ad-hoc sign → install → Gatekeeper 案内 |
| Create: `docs/build-from-source.md` | 要件・手順・手動ビルド・Gatekeeper・トラブルシュート |
| Modify: `README.md` | Get Started 追加、Releases 格下げ |
| Modify: `site/index.html` | CTA href、FAQ、JSON-LD、リリース注記 |
| Modify: `site/i18n.js` | EN/JA 文言（CTA・FAQ・リリース注記） |

---

### Task 1: `scripts/build-and-install.sh`

**Files:**
- Create: `scripts/build-and-install.sh`
- Reference: `scripts/setup-githooks.sh`（実行ビット・ルート解決の既存パターン）
- Reference: `src-tauri/tauri.conf.json`（`productName`: `HEIC Ready`）
- Reference: `.github/workflows/release.yml`（アドホック署名の既存手順）

**Interfaces:**
- Consumes: ホスト上の `xcode-select`, `rustc`, `cargo`, `cargo tauri`, `codesign`
- Produces: `/Applications/HEIC Ready.app`（アドホック署名済み）。stdout に Gatekeeper 案内

- [ ] **Step 1: スクリプトを作成する**

Create `scripts/build-and-install.sh` with exactly this content:

```bash
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

app_path="$(find "${bundle_dir}" -maxdepth 1 -name '*.app' -print | head -n 1)"
[[ -n "${app_path}" ]] || die "No .app found in ${bundle_dir}"

echo "==> Ad-hoc codesigning ${app_path}"
codesign --force --deep -s - "${app_path}"
codesign --verify --deep --strict "${app_path}"

if [[ -e "${INSTALL_PATH}" ]]; then
  echo "Existing app found at ${INSTALL_PATH}"
  read -r -p "Overwrite? [y/N] " reply
  case "${reply}" in
    y|Y|yes|YES) ;;
    *)
      echo "Aborted. Built app left at: ${app_path}"
      exit 0
      ;;
  esac
  rm -rf "${INSTALL_PATH}"
fi

echo "==> Installing to ${INSTALL_PATH}"
cp -R "${app_path}" "${INSTALL_PATH}"

cat <<EOF

Installed: ${INSTALL_PATH}

First launch (Gatekeeper):
  1. Finder で「アプリケーション」を開く
  2. 「${APP_NAME}」を Control-クリック（右クリック）→「開く」
  3. 確認ダイアログで「開く」を選ぶ

Or: System Settings → Privacy & Security → allow the blocked app.

The app is ad-hoc signed and not notarized. This is expected for local builds.
Details: ${BUILD_FROM_SOURCE_DOC}
EOF
```

- [ ] **Step 2: 実行ビットを付与し、構文チェックする**

Run:

```bash
chmod +x scripts/build-and-install.sh
bash -n scripts/build-and-install.sh
```

Expected: no output, exit code 0.

- [ ] **Step 3: 非 macOS ガードを軽く確認する（任意・ローカル）**

Run:

```bash
# Darwin 上では uname を差し替えられないため、スクリプト先頭のガード文が存在することだけ確認
rg -n 'uname -s' scripts/build-and-install.sh
```

Expected: match showing Darwin check.

- [ ] **Step 4: Commit**

```bash
git add scripts/build-and-install.sh
git commit -m "$(cat <<'EOF'
手元ビルド用の build-and-install スクリプトを追加する

EOF
)"
```

---

### Task 2: `docs/build-from-source.md`

**Files:**
- Create: `docs/build-from-source.md`
- Reference: `scripts/build-and-install.sh`（Task 1 で確定したコマンドとパス）

**Interfaces:**
- Consumes: Task 1 のスクリプト名・インストール先・案内文言
- Produces: LP / README からリンクされる唯一の詳細手順書

- [ ] **Step 1: ドキュメントを作成する**

Create `docs/build-from-source.md` with exactly this content:

```markdown
# Build from Source

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
5. Prints first-launch / Gatekeeper instructions

## Manual build (without the script)

```bash
cd src-tauri
cargo tauri build --bundles app
```

Find the app under one of:

- `src-tauri/target/release/bundle/macos/`
- `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/`

Then:

```bash
APP="$(find src-tauri/target -path '*/release/bundle/macos/*.app' -maxdepth 6 -print | head -n 1)"
codesign --force --deep -s - "$APP"
codesign --verify --deep --strict "$APP"
cp -R "$APP" "/Applications/HEIC Ready.app"
```

## First launch (Gatekeeper)

Local builds are ad-hoc signed and not notarized. On first open macOS may say the developer cannot be verified.

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
| App won’t open / blocked | Use Control-click → Open, or Privacy & Security |
| Permission denied writing `/Applications` | Check Disk Access / admin rights; or copy the `.app` manually to a folder you own |

## Related

- Development (`tauri dev`): see README **Development**
- Unsigned GitHub Release DMGs (self-responsibility): see README **Releases**
```

- [ ] **Step 2: リンク切れがないか確認する**

Run:

```bash
test -f scripts/build-and-install.sh && test -f docs/build-from-source.md && rg -n 'build-and-install|Gatekeeper|/Applications/HEIC Ready' docs/build-from-source.md
```

Expected: file exists; matches for script name, Gatekeeper, and install path.

- [ ] **Step 3: Commit**

```bash
git add docs/build-from-source.md
git commit -m "$(cat <<'EOF'
手元ビルド手順ドキュメントを追加する

EOF
)"
```

---

### Task 3: README を手元ビルド第一推奨に再構成する

**Files:**
- Modify: `README.md`
- Reference: `docs/build-from-source.md`

**Interfaces:**
- Consumes: Task 1 スクリプト、Task 2 ドキュメント
- Produces: リポジトリ入口の Get Started 節

- [ ] **Step 1: Get Started 節を Scope の直後に挿入し、Releases を格下げする**

In `README.md`, insert this section **immediately after** the `## Scope` section (before `## Runtime Architecture`):

```markdown
## Get Started (Build from Source)

Apple Developer Program enrollment is not used for this project, so public DMGs are **not notarized**. For general use, build locally:

```bash
git clone https://github.com/melank/heic_ready.git
cd heic_ready
./scripts/build-and-install.sh
```

This installs `/Applications/HEIC Ready.app` and prints Gatekeeper first-launch steps.

Full prerequisites, manual build, and troubleshooting: [`docs/build-from-source.md`](./docs/build-from-source.md)
```

Replace the entire existing `## Releases` section with:

```markdown
## Releases

GitHub Releases may still publish an ad-hoc-signed `.dmg` for convenience, but it is **not notarized** and is **not** the recommended path for general users.

- Preferred: [Build from source](./docs/build-from-source.md)
- Optional (self-responsibility): `https://github.com/melank/heic_ready/releases/latest`
- Specific version: `https://github.com/melank/heic_ready/releases/tag/vX.Y.Z`

Release notes policy:

- Version-specific notes are stored under `docs/releases/` (example: `docs/releases/v0.1.0.md`)
- Release publication is triggered by `tag push` (`vX.Y.Z`)
- Installer (`.dmg`) is built, ad-hoc codesigned, and uploaded by `.github/workflows/release.yml`
- First launch of an unsigned/ad-hoc build requires right-click → Open (or Privacy & Security)
```

Keep `## Development` unchanged (developer `tauri dev` path).

- [ ] **Step 2: 案内の一貫性を確認する**

Run:

```bash
rg -n 'Get Started|build-and-install|not notarized|Build from source' README.md
```

Expected: Get Started section present; Releases points to build-from-source as preferred.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "$(cat <<'EOF'
README で手元ビルドを第一推奨に案内する

EOF
)"
```

---

### Task 4: GitHub Pages（`site/`）の CTA・FAQ・JSON-LD を更新する

**Files:**
- Modify: `site/index.html`
- Modify: `site/i18n.js`
- Reference: `docs/landing-page-quality.md`（品質ガイド。全面リデザイン禁止を再確認）

**Interfaces:**
- Consumes: 固定 URL `https://github.com/melank/heic_ready/blob/master/docs/build-from-source.md`
- Produces: LP 主 CTA・FAQ・JSON-LD・リリース注記が手元ビルド寄り

- [ ] **Step 1: `site/i18n.js` の文言キーを更新する**

In both `en` and `ja` objects:

1. Rename usage of download CTA to build CTA by changing keys and values as follows (replace `ctaDownload` with `ctaBuild`):

```js
// en
ctaBuild: "Build from Source",
releaseNote: "DMGs on GitHub Releases are ad-hoc signed and not notarized. Prefer building from source.",
faqQ4: "Why build from source instead of downloading a DMG?",
faqA4: "HEIC Ready is not notarized with an Apple Developer certificate. Local builds are the recommended way to install and run the app.",

// ja
ctaBuild: "ソースからビルド",
releaseNote: "GitHub Releases の DMG はアドホック署名のみで未公証です。一般利用はソースからのビルドを推奨します。",
faqQ4: "なぜ DMG ではなくソースからビルドするのですか？",
faqA4: "Apple Developer 証明書による公証を行っていないためです。手元でビルドして使う方法を推奨しています。",
```

Remove the old `ctaDownload` keys from both languages.

- [ ] **Step 2: `site/index.html` を更新する**

1. JSON-LD `downloadUrl` (around the SoftwareApplication block):

```json
"downloadUrl": "https://github.com/melank/heic_ready/blob/master/docs/build-from-source.md"
```

2. Add FAQ entity for Q4 inside the existing FAQPage `mainEntity` array (Japanese text is fine for the static JSON-LD, matching existing Q1/Q3 style):

```json
{
  "@type": "Question",
  "name": "なぜ DMG ではなくソースからビルドするのですか？",
  "acceptedAnswer": {
    "@type": "Answer",
    "text": "Apple Developer 証明書による公証を行っていないためです。手元でビルドして使う方法を推奨しています。"
  }
}
```

3. Hero CTA primary link — replace the DMG download anchor with:

```html
<a class="primary" href="https://github.com/melank/heic_ready/blob/master/docs/build-from-source.md" data-i18n="ctaBuild">ソースからビルド</a>
```

Keep the secondary “View Source” link to the repo.

4. In the visible FAQ `<dl>`, after faqQ3/faqA3, add:

```html
<dt data-i18n="faqQ4">なぜ DMG ではなくソースからビルドするのですか？</dt>
<dd data-i18n="faqA4">Apple Developer 証明書による公証を行っていないためです。手元でビルドして使う方法を推奨しています。</dd>
```

5. In the release sidebar, immediately after `<h2 data-i18n="releaseTitle">リリース</h2>`, add:

```html
<p class="release-note" data-i18n="releaseNote">GitHub Releases の DMG はアドホック署名のみで未公証です。一般利用はソースからのビルドを推奨します。</p>
```

- [ ] **Step 3: リリース注記の最低限スタイルを足す（既存トーンに合わせる）**

In `site/styles.css`, near `.release-sidebar h2` / `.release-list`, add:

```css
.release-note {
  margin: 0 0 0.75rem;
  font-size: 0.85rem;
  line-height: 1.45;
  opacity: 0.85;
}
```

Do not redesign the page layout.

- [ ] **Step 4: 文言キーとリンク先を確認する**

Run:

```bash
rg -n 'ctaBuild|ctaDownload|build-from-source\.md|faqQ4|releaseNote|downloadUrl' site/index.html site/i18n.js site/styles.css
```

Expected:
- `ctaBuild` present in HTML and both languages
- no remaining `ctaDownload`
- blob URL to `docs/build-from-source.md` on CTA and JSON-LD
- `faqQ4` / `releaseNote` present

- [ ] **Step 5: ローカルプレビュー（任意）**

Run:

```bash
npx --yes serve site
```

Open the printed local URL, toggle EN/JA, confirm primary CTA label and FAQ Q4.

- [ ] **Step 6: Commit**

```bash
git add site/index.html site/i18n.js site/styles.css
git commit -m "$(cat <<'EOF'
ランディングの案内を手元ビルド推奨に更新する

EOF
)"
```

---

### Task 5: 通し確認

**Files:**
- Verify only (no new files required)

- [ ] **Step 1: 仕様の成果物がすべて存在することを確認する**

Run:

```bash
test -x scripts/build-and-install.sh
test -f docs/build-from-source.md
rg -n 'Get Started \(Build from Source\)' README.md
rg -n 'ctaBuild|faqQ4|releaseNote' site/i18n.js site/index.html
```

Expected: all checks succeed.

- [ ] **Step 2: 実ビルド（時間がかかる・前提が揃っているマシンでのみ）**

Run:

```bash
./scripts/build-and-install.sh
```

Expected:
- Prerequisites pass or clear error messages
- On success: `/Applications/HEIC Ready.app` exists
- Script prints Gatekeeper instructions
- Script does **not** auto-open the app

If a full build is too heavy in the agent environment, document that Step 2 was skipped and rely on `bash -n` + path checks from earlier tasks.

- [ ] **Step 3: 最終コミットは不要（各 Task で済み）。`git status` が clean であることを確認**

```bash
git status
git log --oneline -6
```

Expected: clean working tree; commits for script, docs, README, site present (plus earlier design/plan commits if any).

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|---|---|
| `scripts/build-and-install.sh` | Task 1 |
| 前提チェックのみ・自動 install なし | Task 1 |
| ad-hoc sign → `/Applications` → Gatekeeper 案内 | Task 1 |
| `docs/build-from-source.md` | Task 2 |
| README Get Started + Releases 格下げ | Task 3 |
| site CTA / FAQ / JSON-LD / 未公証注記 | Task 4 |
| 通し確認 | Task 5 |
| 非ゴール（公証・Homebrew・release.yml 削除・全面リデザイン） | 全 Task で触らない |

No TBD/TODO placeholders remain. CTA URL is fixed to the blob URL from the spec.
