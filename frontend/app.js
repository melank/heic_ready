const invoke =
  window.__TAURI__?.core?.invoke ||
  window.__TAURI_INTERNALS__?.invoke?.bind(window.__TAURI_INTERNALS__);
const listen = window.__TAURI__?.event?.listen;

const ui = {
  watchFolders: document.getElementById("watchFolders"),
  recursiveWatch: document.getElementById("recursiveWatch"),
  replaceMode: document.getElementById("replaceMode"),
  jpegQuality: document.getElementById("jpegQuality"),
  rescanIntervalSecs: document.getElementById("rescanIntervalSecs"),
  saveButton: document.getElementById("saveButton"),
  status: document.getElementById("status"),
  recentLogs: document.getElementById("recentLogs"),
  refreshLogsButton: document.getElementById("refreshLogsButton")
};
const RECENT_LOG_AUTO_REFRESH_MS = 45000;

let baselineConfig = null;
let isSaving = false;
let statusTimer = null;
let statusFxTimer = null;
let recentLogTimer = null;

function normalizeConfig(raw) {
  return {
    watch_folders: (raw.watch_folders || []).map((value) => String(value).trim()).filter((value) => value.length > 0),
    recursive_watch: Boolean(raw.recursive_watch),
    output_policy: raw.output_policy || "coexist",
    jpeg_quality: Number(raw.jpeg_quality ?? 92),
    rescan_interval_secs: Number(raw.rescan_interval_secs ?? 60),
    paused: Boolean(raw.paused)
  };
}

function setStatus(type, text, autoClearMs = 0) {
  if (statusFxTimer) {
    clearTimeout(statusFxTimer);
    statusFxTimer = null;
  }
  ui.status.className = `status-pill ${type}`.trim();
  ui.status.textContent = text;

  if (statusTimer) {
    clearTimeout(statusTimer);
    statusTimer = null;
  }
  if (autoClearMs > 0) {
    statusTimer = setTimeout(() => {
      if (isSaving) {
        return;
      }
      applyIdleStatus(isDirty(), true);
    }, autoClearMs);
  }
}

function pausedIdleStatus() {
  return baselineConfig?.paused ? { type: "paused", text: "Paused" } : { type: "ready", text: "Ready" };
}

function applyIdleStatus(dirty, withFade = false) {
  if (dirty) {
    setStatus("dirty", "Unsaved changes");
    return;
  }
  const idle = pausedIdleStatus();
  if (!withFade) {
    setStatus(idle.type, idle.text);
    return;
  }

  ui.status.classList.add("is-fading");
  statusFxTimer = setTimeout(() => {
    setStatus(idle.type, idle.text);
    requestAnimationFrame(() => ui.status.classList.remove("is-fading"));
    statusFxTimer = null;
  }, 180);
}

function readConfigFromForm() {
  return normalizeConfig({
    watch_folders: ui.watchFolders.value.split("\n"),
    recursive_watch: ui.recursiveWatch.checked,
    output_policy: ui.replaceMode.checked ? "replace" : "coexist",
    jpeg_quality: Number(ui.jpegQuality.value),
    rescan_interval_secs: Number(ui.rescanIntervalSecs.value),
    paused: baselineConfig?.paused ?? false
  });
}

function writeConfigToForm(config) {
  ui.watchFolders.value = (config.watch_folders || []).join("\n");
  ui.recursiveWatch.checked = Boolean(config.recursive_watch);
  ui.replaceMode.checked = (config.output_policy || "coexist") === "replace";
  ui.jpegQuality.value = Number(config.jpeg_quality ?? 92);
  ui.rescanIntervalSecs.value = Number(config.rescan_interval_secs ?? 60);
}

function isDirty() {
  if (!baselineConfig) {
    return false;
  }
  return JSON.stringify(readConfigFromForm()) !== JSON.stringify(baselineConfig);
}

function refreshFormState() {
  const dirty = isDirty();
  ui.saveButton.disabled = isSaving || !dirty;
  if (isSaving) return;

  if (!ui.status.classList.contains("error") && !ui.status.classList.contains("saved")) {
    applyIdleStatus(dirty);
  }
}

function handleFormEdited() {
  refreshFormState();
}

function validateConfig(config) {
  return (
    Number.isFinite(config.jpeg_quality) &&
    config.jpeg_quality >= 0 &&
    config.jpeg_quality <= 100 &&
    Number.isFinite(config.rescan_interval_secs) &&
    config.rescan_interval_secs >= 15 &&
    config.rescan_interval_secs <= 3600
  );
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
    return;
  }
  try {
    const logs = await invoke("get_recent_logs");
    renderRecentLogs(logs);
  } catch (error) {
    ui.recentLogs.innerHTML = `<li>Failed to load logs: ${escapeHtml(error)}</li>`;
  }
}

function startRecentLogAutoRefresh() {
  if (recentLogTimer) {
    clearInterval(recentLogTimer);
  }
  recentLogTimer = setInterval(() => {
    if (document.visibilityState !== "visible" || isSaving) {
      return;
    }
    refreshRecentLogs();
  }, RECENT_LOG_AUTO_REFRESH_MS);
}

async function loadConfig() {
  if (!invoke) {
    setStatus("error", "Tauri API is not available.");
    return;
  }

    setStatus("ready", "Loading...");
  try {
    const config = normalizeConfig(await invoke("get_config"));
    baselineConfig = config;
    writeConfigToForm(config);
    refreshFormState();
    await refreshRecentLogs();
    startRecentLogAutoRefresh();
  } catch (error) {
    setStatus("error", `Load failed: ${error}`);
  }
}

async function saveConfig() {
  if (!invoke || isSaving) {
    return;
  }

  const config = readConfigFromForm();
  if (!validateConfig(config)) {
    setStatus("error", "jpeg_quality: 0-100, rescan_interval_secs: 15-3600", 4000);
    return;
  }

  isSaving = true;
  refreshFormState();

  try {
    await invoke("update_config", { config });

    baselineConfig = config;
    setStatus("saved", "Saved", 2000);
    await refreshRecentLogs();
  } catch (error) {
    setStatus("error", `Save failed: ${error}`, 5000);
  } finally {
    isSaving = false;
    refreshFormState();
  }
}

if (listen) {
  listen("paused-changed", (event) => {
    const paused = Boolean(event.payload);
    if (baselineConfig) {
      baselineConfig = {
        ...baselineConfig,
        paused
      };
    }
    refreshFormState();
  });
}

[
  ui.watchFolders,
  ui.recursiveWatch,
  ui.replaceMode,
  ui.jpegQuality,
  ui.rescanIntervalSecs
].forEach((element) => {
  element.addEventListener("input", handleFormEdited);
  element.addEventListener("change", handleFormEdited);
});

ui.saveButton.addEventListener("click", saveConfig);
ui.refreshLogsButton.addEventListener("click", refreshRecentLogs);

loadConfig();
