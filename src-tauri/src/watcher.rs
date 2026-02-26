use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime},
};

use crossbeam_channel::{Receiver, Sender};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

use crate::config::{AppConfig, OutputPolicy};

const STABLE_WINDOW: Duration = Duration::from_millis(300);
const MAX_STABILIZE_RETRIES: usize = 3;
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(400);
const WORKER_COUNT: usize = 2;
const RECENT_LOG_LIMIT: usize = 10;
const MIN_RESCAN_INTERVAL_SECS: u64 = 15;
const MAX_RESCAN_INTERVAL_SECS: u64 = 60 * 60;

static RECENT_LOGS: OnceLock<Mutex<VecDeque<RecentLogEntry>>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSignature {
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecentLogEntry {
    timestamp_unix_ms: u128,
    path: String,
    result: &'static str,
    reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecentLog {
    pub timestamp_unix_ms: u128,
    pub path: String,
    pub result: String,
    pub reason: String,
}

pub struct WatchService {
    stop_tx: Sender<()>,
    join_handle: thread::JoinHandle<()>,
}

impl WatchService {
    pub fn start(config: AppConfig) -> Result<Self, String> {
        let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);

        let join_handle = thread::Builder::new()
            .name("watch-dispatcher".to_string())
            .spawn(move || {
                if let Err(err) = run_dispatcher(config, stop_rx) {
                    log::error!("watch dispatcher stopped with error: {err}");
                }
            })
            .map_err(|err| format!("failed to spawn watch dispatcher: {err}"))?;

        Ok(Self {
            stop_tx,
            join_handle,
        })
    }

    pub fn stop(self) {
        let _ = self.stop_tx.send(());
        if let Err(err) = self.join_handle.join() {
            log::error!("failed to join watch dispatcher: {err:?}");
        }
    }
}

fn run_dispatcher(config: AppConfig, stop_rx: Receiver<()>) -> Result<(), String> {
    if config.watch_folders.is_empty() {
        return Ok(());
    }

    let (event_tx, event_rx) = crossbeam_channel::unbounded::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = event_tx.send(res);
        },
        notify::Config::default(),
    )
    .map_err(|err| format!("failed to create watcher: {err}"))?;

    let recursive_mode = if config.recursive_watch {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };

    for dir in &config.watch_folders {
        watcher
            .watch(dir, recursive_mode)
            .map_err(|err| format!("failed to watch {}: {err}", dir.display()))?;
        log::info!("watching folder: {}", dir.display());
    }

    let (job_tx, job_rx) = crossbeam_channel::unbounded::<PathBuf>();
    let (done_tx, done_rx) = crossbeam_channel::unbounded::<PathBuf>();
    let worker_handles = spawn_workers(job_rx, done_tx, config.clone());

    let mut last_enqueued: HashMap<PathBuf, Instant> = HashMap::new();
    let mut last_signature: HashMap<PathBuf, FileSignature> = HashMap::new();
    let mut in_flight: HashSet<PathBuf> = HashSet::new();
    enqueue_initial_pending_files(
        &config,
        &job_tx,
        false,
        &mut last_enqueued,
        &mut last_signature,
        &mut in_flight,
    );
    let rescan_interval = effective_rescan_interval_secs(config.rescan_interval_secs);
    let mut next_rescan_at = Instant::now() + Duration::from_secs(rescan_interval);

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        drain_completed_jobs(&done_rx, &mut in_flight);

        match event_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(event)) => {
                for path in event.paths {
                    if is_target_file(&path) {
                        enqueue_conversion_job(
                            &job_tx,
                            &path,
                            false,
                            &mut last_enqueued,
                            &mut last_signature,
                            &mut in_flight,
                        );
                    }
                }
            }
            Ok(Err(err)) => log::warn!("watch event error: {err}"),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }

        if Instant::now() >= next_rescan_at {
            enqueue_initial_pending_files(
                &config,
                &job_tx,
                true,
                &mut last_enqueued,
                &mut last_signature,
                &mut in_flight,
            );
            next_rescan_at = Instant::now() + Duration::from_secs(rescan_interval);
        }
    }

    drop(job_tx);
    for handle in worker_handles {
        if let Err(err) = handle.join() {
            log::error!("failed to join worker: {err:?}");
        }
    }

    Ok(())
}

fn enqueue_initial_pending_files(
    config: &AppConfig,
    job_tx: &Sender<PathBuf>,
    allow_same_signature: bool,
    last_enqueued: &mut HashMap<PathBuf, Instant>,
    last_signature: &mut HashMap<PathBuf, FileSignature>,
    in_flight: &mut HashSet<PathBuf>,
) {
    for root in &config.watch_folders {
        let files = collect_pending_files(root, config.recursive_watch);
        for path in files {
            enqueue_conversion_job(
                job_tx,
                &path,
                allow_same_signature,
                last_enqueued,
                last_signature,
                in_flight,
            );
        }
    }
}

fn enqueue_conversion_job(
    job_tx: &Sender<PathBuf>,
    path: &Path,
    allow_same_signature: bool,
    last_enqueued: &mut HashMap<PathBuf, Instant>,
    last_signature: &mut HashMap<PathBuf, FileSignature>,
    in_flight: &mut HashSet<PathBuf>,
) {
    let now = Instant::now();
    let Some(signature) = file_signature(path) else {
        return;
    };
    if !should_enqueue_path(
        path,
        &signature,
        now,
        allow_same_signature,
        last_enqueued,
        last_signature,
        in_flight,
    ) {
        return;
    }

    last_enqueued.insert(path.to_path_buf(), now);
    last_signature.insert(path.to_path_buf(), signature);
    in_flight.insert(path.to_path_buf());
    if let Err(err) = job_tx.send(path.to_path_buf()) {
        log::error!("failed to enqueue path {}: {err}", path.display());
        in_flight.remove(path);
    }
}

fn spawn_workers(
    job_rx: Receiver<PathBuf>,
    done_tx: Sender<PathBuf>,
    config: AppConfig,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(WORKER_COUNT);
    for worker_id in 0..WORKER_COUNT {
        let worker_job_rx = job_rx.clone();
        let worker_done_tx = done_tx.clone();
        let worker_config = config.clone();
        let builder = thread::Builder::new().name(format!("watch-worker-{worker_id}"));
        let handle = builder
            .spawn(move || {
                worker_loop(
                    worker_id,
                    worker_job_rx,
                    worker_done_tx,
                    worker_config,
                )
            })
            .expect("spawn worker thread");
        handles.push(handle);
    }
    handles
}

fn worker_loop(
    worker_id: usize,
    job_rx: Receiver<PathBuf>,
    done_tx: Sender<PathBuf>,
    config: AppConfig,
) {
    loop {
        match job_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(path) => {
                if is_lock_file(&path) {
                    log::info!("[worker {worker_id}] skipped lock file: {}", path.display());
                    push_recent_log(&path, "skip", "lock file");
                    let _ = done_tx.send(path);
                    continue;
                }

                match wait_for_stable_file(&path) {
                    Ok(true) => {
                        log::info!("[worker {worker_id}] file is stable: {}", path.display());
                        match convert_heic_file(&path, &config) {
                            Ok(output_path) => {
                                log::info!(
                                    "[worker {worker_id}] converted to jpeg: {} -> {}",
                                    path.display(),
                                    output_path.display()
                                );
                                push_recent_log(&path, "success", "converted to jpeg");
                            }
                            Err(err) => {
                                let category = classify_conversion_error(err.as_str());
                                let detailed = format!("[{category}] {err}");
                                log::error!(
                                    "[worker {worker_id}] failed converting {}: {detailed}",
                                    path.display()
                                );
                                push_recent_log(&path, "failure", detailed.as_str());
                            }
                        }
                    }
                    Ok(false) => {
                        log::warn!(
                            "[worker {worker_id}] file did not stabilize within retry limit: {}",
                            path.display()
                        );
                        push_recent_log(&path, "skip", "did not stabilize within retry limit");
                    }
                    Err(err) => {
                        log::warn!(
                            "[worker {worker_id}] skipped file due to access error {}: {err}",
                            path.display()
                        );
                        push_recent_log(&path, "skip", &format!("access error: {err}"));
                    }
                }
                let _ = done_tx.send(path);
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn should_enqueue_path(
    path: &Path,
    signature: &FileSignature,
    now: Instant,
    allow_same_signature: bool,
    last_enqueued: &HashMap<PathBuf, Instant>,
    last_signature: &HashMap<PathBuf, FileSignature>,
    in_flight: &HashSet<PathBuf>,
) -> bool {
    if in_flight.contains(path) {
        return false;
    }

    if let Some(last_seen) = last_enqueued.get(path) {
        if now.duration_since(*last_seen) < DEBOUNCE_WINDOW {
            return false;
        }
    }

    if !allow_same_signature {
        if let Some(previous) = last_signature.get(path) {
            if previous == signature {
                return false;
            }
        }
    }

    true
}

fn file_signature(path: &Path) -> Option<FileSignature> {
    match fs::metadata(path) {
        Ok(metadata) => Some(FileSignature {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        }),
        Err(err) => {
            log::debug!(
                "skipping signature check for {} due to metadata error: {err}",
                path.display()
            );
            None
        }
    }
}

fn drain_completed_jobs(done_rx: &Receiver<PathBuf>, in_flight: &mut HashSet<PathBuf>) {
    while let Ok(path) = done_rx.try_recv() {
        in_flight.remove(&path);
    }
}

fn collect_pending_files(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut pending = Vec::new();
    collect_pending_files_impl(root, recursive, &mut pending);
    pending
}

fn collect_pending_files_impl(path: &Path, recursive: bool, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!("failed to read directory {}: {err}", path.display());
            return;
        }
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                log::debug!(
                    "failed to get file type for {}: {err}",
                    entry_path.display()
                );
                continue;
            }
        };

        if file_type.is_dir() {
            if recursive {
                collect_pending_files_impl(&entry_path, true, out);
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if !is_target_extension(&entry_path) {
            continue;
        }
        if has_jpeg_sibling(&entry_path) {
            continue;
        }

        out.push(entry_path);
    }
}

fn has_jpeg_sibling(path: &Path) -> bool {
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some(parent) = path.parent() else {
        return false;
    };
    parent.join(format!("{stem}.jpg")).exists()
}

fn convert_heic_file(input_path: &Path, config: &AppConfig) -> Result<PathBuf, String> {
    let output_path = resolve_output_path(input_path);
    let tmp_output_path = tmp_output_path_for(&output_path);
    run_sips_convert(input_path, &tmp_output_path, config.jpeg_quality)?;
    fs::rename(&tmp_output_path, &output_path).map_err(|err| {
        format!(
            "failed to finalize output {}: {err}",
            output_path.display()
        )
    })?;

    if matches!(config.output_policy, OutputPolicy::Replace) {
        move_file_to_trash(input_path)?;
    }

    Ok(output_path)
}

fn resolve_output_path(input_path: &Path) -> PathBuf {
    let Some(parent) = input_path.parent() else {
        return input_path.with_extension("jpg");
    };
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("converted");

    let mut candidate = parent.join(format!("{stem}.jpg"));
    if !candidate.exists() {
        return candidate;
    }

    let mut index = 1usize;
    loop {
        candidate = parent.join(format!("{stem} ({index}).jpg"));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn tmp_output_path_for(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output.jpg".to_string());
    output_path.with_file_name(format!("{file_name}.tmp"))
}

fn run_sips_convert(input_path: &Path, output_path: &Path, quality: u8) -> Result<(), String> {
    let quality_value = quality.to_string();
    let output = Command::new("sips")
        .arg("-s")
        .arg("format")
        .arg("jpeg")
        .arg("-s")
        .arg("formatOptions")
        .arg(&quality_value)
        .arg(input_path.as_os_str())
        .arg("--out")
        .arg(output_path.as_os_str())
        .output()
        .map_err(|err| format!("failed to run sips: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output_path.exists() {
        let _ = fs::remove_file(output_path);
    }
    Err(format!(
        "sips exited with status {}: {}",
        output.status,
        if stderr.is_empty() {
            "no stderr output"
        } else {
            &stderr
        }
    ))
}

fn classify_conversion_error(err: &str) -> &'static str {
    let lower = err.to_ascii_lowercase();
    if lower.contains("permission denied") || lower.contains("operation not permitted") {
        return "permission";
    }
    if lower.contains("sips exited") {
        return "decode";
    }
    "io"
}

fn move_file_to_trash(path: &Path) -> Result<(), String> {
    let trash_dir = user_trash_dir()?;
    fs::create_dir_all(&trash_dir)
        .map_err(|err| format!("failed to create trash dir {}: {err}", trash_dir.display()))?;

    let destination = unique_destination_path(&trash_dir, path);
    match fs::rename(path, &destination) {
        Ok(()) => Ok(()),
        Err(err) if err.raw_os_error() == Some(18) => {
            fs::copy(path, &destination).map_err(|copy_err| {
                format!(
                    "failed to copy file to trash {}: {copy_err}",
                    destination.display()
                )
            })?;
            fs::remove_file(path)
                .map_err(|remove_err| format!("failed to remove source after copy: {remove_err}"))
        }
        Err(err) => Err(format!(
            "failed to move file to trash {}: {err}",
            destination.display()
        )),
    }
}

fn user_trash_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".Trash"))
}

fn unique_destination_path(dir: &Path, source_path: &Path) -> PathBuf {
    let file_name = source_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "untitled".to_string());
    let mut candidate = dir.join(&file_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = source_path
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "untitled".to_string());
    let ext = source_path
        .extension()
        .map(|name| name.to_string_lossy().into_owned());

    let mut index = 1usize;
    loop {
        let file_name = match &ext {
            Some(ext) => format!("{stem} ({index}).{ext}"),
            None => format!("{stem} ({index})"),
        };
        candidate = dir.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn push_recent_log(path: &Path, result: &'static str, reason: &str) {
    let logs = RECENT_LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(RECENT_LOG_LIMIT)));
    let mut guard = match logs.lock() {
        Ok(guard) => guard,
        Err(err) => {
            log::error!("failed to lock recent log buffer: {err}");
            return;
        }
    };

    if guard.len() >= RECENT_LOG_LIMIT {
        guard.pop_front();
    }
    guard.push_back(RecentLogEntry {
        timestamp_unix_ms: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or(0),
        path: path.display().to_string(),
        result,
        reason: reason.to_string(),
    });
}

pub fn get_recent_logs() -> Vec<RecentLog> {
    let logs = RECENT_LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(RECENT_LOG_LIMIT)));
    let guard = match logs.lock() {
        Ok(guard) => guard,
        Err(err) => {
            log::error!("failed to lock recent log buffer: {err}");
            return Vec::new();
        }
    };

    guard
        .iter()
        .rev()
        .map(|entry| RecentLog {
            timestamp_unix_ms: entry.timestamp_unix_ms,
            path: entry.path.clone(),
            result: entry.result.to_string(),
            reason: entry.reason.clone(),
        })
        .collect()
}

fn effective_rescan_interval_secs(configured: u64) -> u64 {
    let clamped = configured.clamp(MIN_RESCAN_INTERVAL_SECS, MAX_RESCAN_INTERVAL_SECS);
    if clamped != configured {
        log::warn!(
            "rescan_interval_secs={} is out of range; clamped to {} (allowed {}..={})",
            configured,
            clamped,
            MIN_RESCAN_INTERVAL_SECS,
            MAX_RESCAN_INTERVAL_SECS
        );
    }
    clamped
}

#[cfg(test)]
fn recent_logs_len() -> usize {
    let logs = RECENT_LOGS.get_or_init(|| Mutex::new(VecDeque::with_capacity(RECENT_LOG_LIMIT)));
    match logs.lock() {
        Ok(guard) => guard.len(),
        Err(_) => 0,
    }
}

fn wait_for_stable_file(path: &Path) -> Result<bool, std::io::Error> {
    for _ in 0..MAX_STABILIZE_RETRIES {
        let first = fs::metadata(path)?.len();
        thread::sleep(STABLE_WINDOW);
        let second = fs::metadata(path)?.len();

        if first == second {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_target_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    is_target_extension(path)
}

fn is_target_extension(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    ext.eq_ignore_ascii_case("heic") || ext.eq_ignore_ascii_case("heif")
}

fn is_lock_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lock"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stable_file_returns_true() {
        let path = unique_temp_file_path("stable.heic");
        fs::write(&path, b"abc").expect("write stable file");

        let result = wait_for_stable_file(&path).expect("stable check");
        assert!(result);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn target_file_filter_accepts_heic_and_heif() {
        let heic = PathBuf::from("/tmp/a.heic");
        let heif = PathBuf::from("/tmp/a.heif");
        let jpg = PathBuf::from("/tmp/a.jpg");

        fs::write(&heic, b"x").expect("write heic");
        fs::write(&heif, b"x").expect("write heif");
        fs::write(&jpg, b"x").expect("write jpg");

        assert!(is_target_file(&heic));
        assert!(is_target_file(&heif));
        assert!(!is_target_file(&jpg));

        let _ = fs::remove_file(heic);
        let _ = fs::remove_file(heif);
        let _ = fs::remove_file(jpg);
    }

    #[test]
    fn duplicate_signature_is_not_enqueued() {
        let path = PathBuf::from("/tmp/sample.heic");
        let now = Instant::now();
        let signature = FileSignature {
            len: 123,
            modified: None,
        };
        let mut last_enqueued = HashMap::new();
        let mut last_signature = HashMap::new();
        let in_flight = HashSet::new();

        last_enqueued.insert(path.clone(), now - Duration::from_secs(2));
        last_signature.insert(path.clone(), signature.clone());

        assert!(!should_enqueue_path(
            &path,
            &signature,
            now,
            false,
            &last_enqueued,
            &last_signature,
            &in_flight
        ));
    }

    #[test]
    fn in_flight_path_is_not_enqueued() {
        let path = PathBuf::from("/tmp/sample.heic");
        let now = Instant::now();
        let signature = FileSignature {
            len: 123,
            modified: None,
        };
        let last_enqueued = HashMap::new();
        let last_signature = HashMap::new();
        let mut in_flight = HashSet::new();
        in_flight.insert(path.clone());

        assert!(!should_enqueue_path(
            &path,
            &signature,
            now,
            false,
            &last_enqueued,
            &last_signature,
            &in_flight
        ));
    }

    #[test]
    fn jpeg_sibling_check_detects_existing_converted_file() {
        let dir = unique_temp_dir_path("sibling");
        fs::create_dir_all(&dir).expect("create temp dir");
        let heic = dir.join("IMG_0001.heic");
        let jpg = dir.join("IMG_0001.jpg");
        fs::write(&heic, b"x").expect("write heic");

        assert!(!has_jpeg_sibling(&heic));
        fs::write(&jpg, b"y").expect("write jpg");
        assert!(has_jpeg_sibling(&heic));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_output_path_uses_increment_suffix_on_collision() {
        let dir = unique_temp_dir_path("collision");
        fs::create_dir_all(&dir).expect("create temp dir");
        let heic = dir.join("IMG_0002.heic");
        let jpg = dir.join("IMG_0002.jpg");
        let jpg1 = dir.join("IMG_0002 (1).jpg");
        fs::write(&heic, b"x").expect("write heic");
        fs::write(&jpg, b"y").expect("write jpg");
        fs::write(&jpg1, b"z").expect("write jpg1");

        let resolved = resolve_output_path(&heic);
        assert_eq!(resolved, dir.join("IMG_0002 (2).jpg"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recent_log_buffer_keeps_only_latest_ten_items() {
        let path = PathBuf::from("/tmp/recent.heic");
        for idx in 0..12 {
            push_recent_log(&path, "skip", &format!("reason-{idx}"));
        }

        assert_eq!(recent_logs_len(), 10);
    }

    #[test]
    fn rescan_interval_is_clamped_to_safe_range() {
        assert_eq!(effective_rescan_interval_secs(1), 15);
        assert_eq!(effective_rescan_interval_secs(60), 60);
        assert_eq!(effective_rescan_interval_secs(99999), 3600);
    }

    #[test]
    fn conversion_error_is_classified() {
        assert_eq!(classify_conversion_error("Permission denied"), "permission");
        assert_eq!(
            classify_conversion_error("sips exited with status 1"),
            "decode"
        );
        assert_eq!(classify_conversion_error("failed to finalize output"), "io");
    }

    #[test]
    fn unique_destination_path_adds_suffix_on_collision() {
        let dir = unique_temp_dir_path("trash_collision");
        fs::create_dir_all(&dir).expect("create temp dir");
        let source = dir.join("IMG_1000.HEIC");
        fs::write(&source, b"x").expect("write source");
        fs::write(dir.join("IMG_1000.HEIC"), b"x").expect("write collision");

        let candidate = unique_destination_path(&dir, &source);
        assert_eq!(candidate, dir.join("IMG_1000 (1).HEIC"));

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_temp_file_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("heic_ready_{stamp}_{name}"))
    }

    fn unique_temp_dir_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("heic_ready_dir_{stamp}_{name}"))
    }
}
