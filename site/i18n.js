const i18n = {
  en: {
    metaTitle: "HEIC Ready — Auto HEIC to JPEG Converter for macOS",
    metaDescription: "HEIC Ready converts incoming HEIC/HEIF files to JPEG automatically on macOS.",
    eyebrow: "macOS Background Utility",
    heroTitle: "HEIC In. JPEG Out. No\u00A0Extra\u00A0Steps.",
    lede: 'Watch folders, auto-convert <code>.heic</code>/<code>.heif</code>, and keep your upload workflow frictionless.',
    ctaDownload: "Download DMG",
    ctaSource: "View Source",
    cardWatchTitle: "Folder Watch",
    cardWatchDesc: "Monitors your folder and automatically converts HEIC to JPEG whenever new files appear.",
    cardAtomicTitle: "Atomic Output",
    cardAtomicDesc: "Writes to temporary files first, then renames, so incomplete JPEG files are not surfaced.",
    cardTrayTitle: "Fully Background",
    cardTrayDesc: "Just launch and forget. Sits quietly in the menu bar and is there when you need it.",
    stepsTitle: "How to Use",
    step1: "AirDrop HEIC photos to your watch folder.",
    step2: "HEIC Ready detects and converts in the background.",
    step3: "Attach and upload the generated JPEG to your favorite web service.",
    releaseTitle: "Release",
    releaseSummary_v0_1_0: "Initial release. Folder watch with auto HEIC-to-JPEG conversion, atomic output, tray-resident UI, and bilingual support (EN/JA).",
  },
  ja: {
    metaTitle: "HEIC Ready — macOS 向け HEIC→JPEG 自動変換ユーティリティ",
    metaDescription: "HEIC Ready は macOS 上で HEIC/HEIF ファイルを自動的に JPEG に変換します。",
    eyebrow: "macOS バックグラウンドユーティリティ",
    heroTitle: "iPhone の写真を JPEG に。\nPCに移したら、すぐにアップロード",
    lede: 'フォルダを監視し、<code>.heic</code>/<code>.heif</code> を自動変換。アップロード作業をスムーズに。',
    ctaDownload: "DMG をダウンロード",
    ctaSource: "ソースを見る",
    cardWatchTitle: "フォルダ監視",
    cardWatchDesc: "指定フォルダを常に見張り、HEIC が追加されるたびに自動で JPEG に変換します。",
    cardAtomicTitle: "アトミック出力",
    cardAtomicDesc: "一時ファイルに書き込み後にリネーム。不完全な JPEG ファイルが表示されることはありません。",
    cardTrayTitle: "完全バックグラウンド",
    cardTrayDesc: "起動したらあとはお任せ。邪魔にならずメニューバーに常駐し、必要なときだけ操作できます。",
    stepsTitle: "使いかた・利用の流れ",
    step1: "AirDrop で HEIC 写真を監視フォルダに入れる。",
    step2: "HEIC Ready がバックグラウンドで検知・変換。",
    step3: "生成された JPEG を好きな Web サービスにすぐ添付、アップロード。",
    releaseTitle: "リリース",
    releaseSummary_v0_1_0: "初回リリース。フォルダ監視による HEIC→JPEG 自動変換、アトミック出力、トレイ常駐 UI、日英2言語対応。",
  },
};

function applyLang(lang) {
  const dict = i18n[lang];
  if (!dict) return;

  document.documentElement.lang = lang;

  const metaDesc = document.querySelector('meta[name="description"]');
  if (metaDesc) metaDesc.setAttribute("content", dict.metaDescription);

  if (dict.metaTitle) document.title = dict.metaTitle;

  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (dict[key] !== undefined) {
      if (el.hasAttribute("data-i18n-html")) {
        el.innerHTML = dict[key];
      } else {
        el.textContent = dict[key];
      }
    }
  });

  document.querySelectorAll(".lang-toggle button").forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-lang") === lang);
  });

  localStorage.setItem("lang", lang);
}

function detectLang() {
  const stored = localStorage.getItem("lang");
  if (stored && i18n[stored]) return stored;
  const nav = (navigator.language || "").toLowerCase();
  return nav.startsWith("ja") ? "ja" : "en";
}

document.addEventListener("DOMContentLoaded", () => {
  applyLang(detectLang());

  document.querySelectorAll(".lang-toggle button").forEach((btn) => {
    btn.addEventListener("click", () => {
      const lang = btn.getAttribute("data-lang");
      if (lang) applyLang(lang);
    });
  });
});
