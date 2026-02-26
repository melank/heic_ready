use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime},
};

use crossbeam_channel::{Receiver, Sender};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::{AppConfig, OutputPolicy};

const STABLE_WINDOW: Duration = Duration::from_millis(300);
const MAX_STABILIZE_RETRIES: usize = 3;
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(400);
const WORKER_COUNT: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSignature {
    len: u64,
    modified: Option<SystemTime>,
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
    let worker_handles = spawn_workers(job_rx, done_tx, stop_rx.clone(), config.clone());

    let mut last_enqueued: HashMap<PathBuf, Instant> = HashMap::new();
    let mut last_signature: HashMap<PathBuf, FileSignature> = HashMap::new();
    let mut in_flight: HashSet<PathBuf> = HashSet::new();
    enqueue_initial_pending_files(
        &config,
        &job_tx,
        &mut last_enqueued,
        &mut last_signature,
        &mut in_flight,
    );

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        drain_completed_jobs(&done_rx, &mut in_flight);

        match event_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(event)) => {
                for path in event.paths {
                    if !is_target_file(&path) {
                        continue;
                    }

                    let now = Instant::now();
                    let Some(signature) = file_signature(&path) else {
                        continue;
                    };
                    if !should_enqueue_path(
                        &path,
                        &signature,
                        now,
                        &last_enqueued,
                        &last_signature,
                        &in_flight,
                    ) {
                        continue;
                    }

                    last_enqueued.insert(path.clone(), now);
                    last_signature.insert(path.clone(), signature);
                    in_flight.insert(path.clone());
                    if let Err(err) = job_tx.send(path.clone()) {
                        log::error!("failed to enqueue path {}: {err}", path.display());
                        in_flight.remove(&path);
                    }
                }
            }
            Ok(Err(err)) => log::warn!("watch event error: {err}"),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
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
    last_enqueued: &mut HashMap<PathBuf, Instant>,
    last_signature: &mut HashMap<PathBuf, FileSignature>,
    in_flight: &mut HashSet<PathBuf>,
) {
    for root in &config.watch_folders {
        let files = collect_pending_files(root, config.recursive_watch);
        for path in files {
            let now = Instant::now();
            let Some(signature) = file_signature(&path) else {
                continue;
            };
            if !should_enqueue_path(
                &path,
                &signature,
                now,
                last_enqueued,
                last_signature,
                in_flight,
            ) {
                continue;
            }

            last_enqueued.insert(path.clone(), now);
            last_signature.insert(path.clone(), signature);
            in_flight.insert(path.clone());
            if let Err(err) = job_tx.send(path.clone()) {
                log::error!("failed to enqueue initial path {}: {err}", path.display());
                in_flight.remove(&path);
            } else {
                log::info!("initial scan queued for conversion: {}", path.display());
            }
        }
    }
}

fn spawn_workers(
    job_rx: Receiver<PathBuf>,
    done_tx: Sender<PathBuf>,
    stop_rx: Receiver<()>,
    config: AppConfig,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(WORKER_COUNT);
    for worker_id in 0..WORKER_COUNT {
        let worker_job_rx = job_rx.clone();
        let worker_done_tx = done_tx.clone();
        let worker_stop_rx = stop_rx.clone();
        let worker_config = config.clone();
        let builder = thread::Builder::new().name(format!("watch-worker-{worker_id}"));
        let handle = builder
            .spawn(move || {
                worker_loop(
                    worker_id,
                    worker_job_rx,
                    worker_done_tx,
                    worker_stop_rx,
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
    stop_rx: Receiver<()>,
    config: AppConfig,
) {
    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        match job_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(path) => {
                if is_lock_file(&path) {
                    log::info!("[worker {worker_id}] skipped lock file: {}", path.display());
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
                            }
                            Err(err) => {
                                log::error!(
                                    "[worker {worker_id}] failed converting {}: {err}",
                                    path.display()
                                );
                            }
                        }
                    }
                    Ok(false) => {
                        log::warn!(
                            "[worker {worker_id}] file did not stabilize within retry limit: {}",
                            path.display()
                        );
                    }
                    Err(err) => {
                        log::warn!(
                            "[worker {worker_id}] skipped file due to access error {}: {err}",
                            path.display()
                        );
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

    if let Some(previous) = last_signature.get(path) {
        if previous == signature {
            return false;
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
        if entry_path.is_dir() {
            if recursive {
                collect_pending_files_impl(&entry_path, true, out);
            }
            continue;
        }

        if !is_target_file(&entry_path) {
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

    if matches!(config.output_policy, OutputPolicy::Replace) && config.trash_on_replace {
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

fn move_file_to_trash(path: &Path) -> Result<(), String> {
    let Some(path_text) = path.to_str() else {
        return Err("source path is not valid UTF-8".to_string());
    };

    let output = Command::new("osascript")
        .arg("-e")
        .arg("on run argv")
        .arg("-e")
        .arg("tell application \"Finder\" to delete POSIX file (item 1 of argv)")
        .arg("-e")
        .arg("end run")
        .arg(path_text)
        .output()
        .map_err(|err| format!("failed to run osascript: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!(
        "failed to move file to trash (status {}): {}",
        output.status,
        if stderr.is_empty() {
            "no stderr output"
        } else {
            &stderr
        }
    ))
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
