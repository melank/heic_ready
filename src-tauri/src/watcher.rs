use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, Sender};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::AppConfig;

const STABLE_WINDOW: Duration = Duration::from_millis(300);
const MAX_STABILIZE_RETRIES: usize = 3;
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(400);
const WORKER_COUNT: usize = 2;

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
    let worker_handles = spawn_workers(job_rx, stop_rx.clone());

    let mut last_enqueued: HashMap<PathBuf, Instant> = HashMap::new();

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        match event_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(event)) => {
                for path in event.paths {
                    if !is_target_file(&path) {
                        continue;
                    }

                    let now = Instant::now();
                    if let Some(last_seen) = last_enqueued.get(&path) {
                        if now.duration_since(*last_seen) < DEBOUNCE_WINDOW {
                            continue;
                        }
                    }

                    last_enqueued.insert(path.clone(), now);
                    if let Err(err) = job_tx.send(path.clone()) {
                        log::error!("failed to enqueue path {}: {err}", path.display());
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

fn spawn_workers(job_rx: Receiver<PathBuf>, stop_rx: Receiver<()>) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(WORKER_COUNT);
    for worker_id in 0..WORKER_COUNT {
        let worker_job_rx = job_rx.clone();
        let worker_stop_rx = stop_rx.clone();
        let builder = thread::Builder::new().name(format!("watch-worker-{worker_id}"));
        let handle = builder
            .spawn(move || worker_loop(worker_id, worker_job_rx, worker_stop_rx))
            .expect("spawn worker thread");
        handles.push(handle);
    }
    handles
}

fn worker_loop(worker_id: usize, job_rx: Receiver<PathBuf>, stop_rx: Receiver<()>) {
    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        match job_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(path) => {
                if is_lock_file(&path) {
                    log::info!("[worker {worker_id}] skipped lock file: {}", path.display());
                    continue;
                }

                match wait_for_stable_file(&path) {
                    Ok(true) => {
                        log::info!(
                            "[worker {worker_id}] file is stable and queued for conversion: {}",
                            path.display()
                        );
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
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
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

    fn unique_temp_file_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("heic_ready_{stamp}_{name}"))
    }
}
