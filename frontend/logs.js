const invoke =
  window.__TAURI__?.core?.invoke ||
  window.__TAURI_INTERNALS__?.invoke?.bind(window.__TAURI_INTERNALS__);
const listen = window.__TAURI__?.event?.listen;

const ui = {
  logsTitle: document.getElementById("logsTitle"),
  logsSubtitle: document.getElementById("logsSubtitle"),
  recentLogs: document.getElementById("recentLogs"),
  refreshLogsButton: document.getElementById("refreshLogsButton")
};

const I18N = window.HEIC_READY_I18N?.logs || {};

const RECENT_LOG_AUTO_REFRESH_MS = 10000;
let refreshTimer = null;
let locale = "en";

function normalizeLocale(value) {
  return value === "ja" ? "ja" : "en";
}

function t(key) {
  return I18N[locale]?.[key] ?? I18N.en[key] ?? key;
}

function tr(template, vars = {}) {
  return String(template).replace(/\{(\w+)\}/g, (_, key) => String(vars[key] ?? ""));
}

function applyStaticText() {
  document.title = t("pageTitle");
  ui.logsTitle.textContent = t("logsTitle");
  ui.logsSubtitle.textContent = t("logsSubtitle");
  ui.refreshLogsButton.textContent = t("refresh");
}

function formatLogTime(unixMs) {
  const d = new Date(Number(unixMs));
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function renderRecentLogs(logs) {
  if (!Array.isArray(logs) || logs.length === 0) {
    ui.recentLogs.innerHTML = `<li>${escapeHtml(t("noLogs"))}</li>`;
    return;
  }

  ui.recentLogs.innerHTML = logs
    .map((entry) => {
      const result = String(entry.result || "skip");
      const time = formatLogTime(entry.timestamp_unix_ms);
      return `<li><span class=\"log-result-${escapeHtml(result)}\">${escapeHtml(
        result.toUpperCase()
      )}</span> ${escapeHtml(time)} - ${escapeHtml(entry.reason)}<br><span>${escapeHtml(
        entry.path
      )}</span></li>`;
    })
    .join("");
}

async function refreshRecentLogs() {
  if (!invoke) {
    ui.recentLogs.innerHTML = `<li>${escapeHtml(t("tauriUnavailable"))}</li>`;
    return;
  }
  try {
    const logs = await invoke("get_recent_logs");
    renderRecentLogs(logs);
  } catch (error) {
    ui.recentLogs.innerHTML = `<li>${escapeHtml(tr(t("loadFailed"), { error }))}</li>`;
  }
}

function startAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer);
  }

  refreshTimer = setInterval(() => {
    if (document.visibilityState !== "visible") {
      return;
    }
    refreshRecentLogs();
  }, RECENT_LOG_AUTO_REFRESH_MS);
}

async function loadLocale() {
  if (!invoke) {
    return;
  }
  try {
    const config = await invoke("get_config");
    locale = normalizeLocale(config?.locale);
    applyStaticText();
  } catch (_) {
    // ignore and keep English defaults
  }
}

ui.refreshLogsButton.addEventListener("click", refreshRecentLogs);
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    refreshRecentLogs();
  }
});

if (listen) {
  listen("locale-changed", (event) => {
    locale = normalizeLocale(event.payload);
    applyStaticText();
    refreshRecentLogs();
  });
}

applyStaticText();
loadLocale().finally(() => {
  refreshRecentLogs();
  startAutoRefresh();
});
