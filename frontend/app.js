const invoke =
  window.__TAURI__?.core?.invoke ||
  window.__TAURI_INTERNALS__?.invoke?.bind(window.__TAURI_INTERNALS__);
const listen = window.__TAURI__?.event?.listen;

const ui = {
  watchFolders: document.getElementById("watchFolders"),
  addWatchFolderButton: document.getElementById("addWatchFolderButton"),
  recursiveWatch: document.getElementById("recursiveWatch"),
  replaceMode: document.getElementById("replaceMode"),
  jpegQuality: document.getElementById("jpegQuality"),
  rescanIntervalSecs: document.getElementById("rescanIntervalSecs"),
  saveButton: document.getElementById("saveButton"),
  status: document.getElementById("status")
};

let baselineConfig = null;
let isSaving = false;
let statusTimer = null;
let statusFxTimer = null;

function normalizeWatchFolderValue(value) {
  let path = String(value).trim();
  if (!path) {
    return "";
  }
  if (path.length > 1) {
    path = path.replace(/\/+$/g, "");
  }
  return path;
}

function normalizeWatchFolderList(values) {
  const seen = new Set();
  const folders = [];
  for (const value of values) {
    const path = normalizeWatchFolderValue(value);
    if (!path || seen.has(path)) {
      continue;
    }
    seen.add(path);
    folders.push(path);
  }
  return folders;
}

function normalizeConfig(raw) {
  return {
    watch_folders: normalizeWatchFolderList(raw.watch_folders || []),
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
    config.watch_folders.every((path) => path.startsWith("/")) &&
    Number.isFinite(config.jpeg_quality) &&
    config.jpeg_quality >= 0 &&
    config.jpeg_quality <= 100 &&
    Number.isFinite(config.rescan_interval_secs) &&
    config.rescan_interval_secs >= 15 &&
    config.rescan_interval_secs <= 3600
  );
}

async function addWatchFolder() {
  if (!invoke || isSaving) {
    return;
  }
  try {
    const picked = await invoke("pick_watch_folder");
    if (!picked) {
      return;
    }
    const merged = normalizeWatchFolderList([
      ...ui.watchFolders.value.split("\n"),
      String(picked)
    ]);
    ui.watchFolders.value = merged.join("\n");
    refreshFormState();
  } catch (error) {
    setStatus("error", `Folder pick failed: ${error}`, 5000);
  }
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
    setStatus("error", "watch_folders: absolute paths only\njpeg_quality: 0-100\nrescan_interval_secs: 15-3600", 4000);
    return;
  }

  isSaving = true;
  refreshFormState();

  try {
    const result = await invoke("update_config", { config });
    const actual = normalizeConfig(result?.config ?? config);
    baselineConfig = actual;
    writeConfigToForm(actual);

    if (result?.warning) {
      setStatus("error", String(result.warning), 5000);
    } else {
      setStatus("saved", "Saved", 2000);
    }
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
ui.addWatchFolderButton.addEventListener("click", addWatchFolder);

loadConfig();
