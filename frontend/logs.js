const invoke =
  window.__TAURI__?.core?.invoke ||
  window.__TAURI_INTERNALS__?.invoke?.bind(window.__TAURI_INTERNALS__);

const ui = {
  recentLogs: document.getElementById("recentLogs"),
  refreshLogsButton: document.getElementById("refreshLogsButton")
};

const RECENT_LOG_AUTO_REFRESH_MS = 10000;
let refreshTimer = null;

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
    ui.recentLogs.innerHTML = "<li>No logs yet.</li>";
    return;
  }

  ui.recentLogs.innerHTML = logs
    .map((entry) => {
      const result = String(entry.result || "skip");
      const time = formatLogTime(entry.timestamp_unix_ms);
      return `<li><span class=\"log-result-${escapeHtml(result)}\">${escapeHtml(result.toUpperCase())}</span> ${escapeHtml(time)} - ${escapeHtml(entry.reason)}<br><span>${escapeHtml(entry.path)}</span></li>`;
    })
    .join("");
}

async function refreshRecentLogs() {
  if (!invoke) {
    ui.recentLogs.innerHTML = "<li>Tauri API is not available.</li>";
    return;
  }
  try {
    const logs = await invoke("get_recent_logs");
    renderRecentLogs(logs);
  } catch (error) {
    ui.recentLogs.innerHTML = `<li>Failed to load logs: ${escapeHtml(error)}</li>`;
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

ui.refreshLogsButton.addEventListener("click", refreshRecentLogs);
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    refreshRecentLogs();
  }
});

refreshRecentLogs();
startAutoRefresh();
