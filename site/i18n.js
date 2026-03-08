const i18n = {
  en: {
    metaTitle: "HEIC Ready — Auto HEIC Conversion on Mac | HEIC to JPEG Converter",
    metaDescription: "Automate HEIC conversion on Mac. HEIC Ready watches folders and converts HEIC/HEIF files to JPEG automatically.",
    eyebrow: "macOS Background Utility",
    heroTitle: "HEIC In. JPEG Out. No\u00A0Extra\u00A0Steps.",
    lede: 'Watch folders, auto-convert <code>.heic</code>/<code>.heif</code>, and keep your upload workflow frictionless.',
    ctaDownload: "Download DMG",
    ctaSource: "View Source",
    cardAutoConvertTitle: "Zero Conversion Effort",
    cardAutoConvertDesc: "Set a watch folder and HEIC files are converted to JPEG automatically. No more manual conversions.",
    cardBackgroundTitle: "Zero Overhead While Idle",
    cardBackgroundDesc: "Sits quietly in the menu bar. Event-driven design means virtually no CPU or memory usage while waiting.",
    cardAtomicTitle: "Atomic Output",
    cardAtomicDesc: "Writes to temporary files first, then renames, so incomplete JPEG files are not surfaced.",
    stepsTitle: "How to Use",
    step1: "AirDrop HEIC photos to your watch folder.",
    step2: "HEIC Ready detects and converts in the background.",
    step3: "Attach and upload the generated JPEG to your favorite web service.",
    releaseTitle: "Release",
    faqTitle: "FAQ",
    faqQ1: "How do I convert HEIC on Mac?",
    faqA1: "Just install HEIC Ready and set a watch folder. Every HEIC file added is automatically converted to JPEG.",
    faqQ2: "Is HEIC Ready open source?",
    faqA2: "Yes. HEIC Ready is open-source software available on GitHub.",
    faqQ3: "What happens to the original HEIC files?",
    faqA3: "You choose. In \"Coexist\" mode the originals stay; in \"Replace\" mode they are moved to the Trash.",
    releaseSummary_v0_1_0: "Initial release. Folder watch with auto HEIC-to-JPEG conversion, atomic output, tray-resident UI, and bilingual support (EN/JA).",
  },
  ja: {
    metaTitle: "HEIC Ready — Mac で HEIC 変換を自動化 | HEIC→JPEG 変換アプリ",
    metaDescription: "HEIC 変換を Mac で自動化。HEIC Ready はフォルダを監視し HEIC/HEIF ファイルを自動で JPEG に変換するアプリです。",
    eyebrow: "macOS バックグラウンドユーティリティ",
    heroTitle: "iPhone の写真を JPEG に。\nPCに移したら、すぐにアップロード",
    lede: 'フォルダを監視し、<code>.heic</code>/<code>.heif</code> を自動変換。アップロード作業をスムーズに。',
    ctaDownload: "DMG をダウンロード",
    ctaSource: "ソースを見る",
    cardAutoConvertTitle: "HEIC 変換の手間をゼロに",
    cardAutoConvertDesc: "監視フォルダを設定すると、以降はそこに HEIC をコピーするだけ。毎回の変換作業はもう不要です。",
    cardBackgroundTitle: "常駐しても負荷ゼロ",
    cardBackgroundDesc: "ファイルイベント駆動で動作するため待機中の CPU・メモリ消費はほぼありません。",
    cardAtomicTitle: "アトミック出力",
    cardAtomicDesc: "一時ファイルに書き込み後にリネーム。不完全な JPEG ファイルが生成されることはありません。",
    stepsTitle: "使いかた・利用の流れ",
    step1: "AirDrop で HEIC 写真を監視フォルダに入れる。",
    step2: "HEIC Ready がバックグラウンドで検知・変換。",
    step3: "生成された JPEG を好きな Web サービスにすぐ添付、アップロード。",
    releaseTitle: "リリース",
    faqTitle: "よくある質問",
    faqQ1: "Mac で HEIC 変換するにはどうすればいいですか？",
    faqA1: "HEIC Ready をインストールして監視フォルダを設定するだけです。HEIC ファイルが追加されるたびに自動で JPEG に変換されます。",
    faqQ2: "HEIC Ready はオープンソースですか？",
    faqA2: "はい。HEIC Ready はオープンソースソフトウェアです。GitHub からダウンロードできます。",
    faqQ3: "変換元の HEIC ファイルはどうなりますか？",
    faqA3: "設定で選べます。「共存」モードでは元ファイルをそのまま残し、「置換」モードでは元ファイルをゴミ箱に移動します。",
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
