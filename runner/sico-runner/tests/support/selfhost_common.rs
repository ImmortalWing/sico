//! Shared selfhost differential-support helpers (STEP-0317): an on-disk
//! component cache that removes the repeated in-process `sico build` of
//! unchanged selfhost sources, and a lazily-grown pool of engine-isolated
//! prepared programs for parallel local verification runs.
//!
//! Cache safety: the key binds a cached component to the exact selfhost
//! source texts plus a fingerprint of every compiler crate source file, so
//! any edit that could change build output rotates the key. Byte-exact
//! differentials still compare against the freshly computed Rust oracle on
//! every run, and a cold cache (CI) always builds fresh. A stale entry can
//! therefore only fail a differential, never pass one falsely.
//! `SICO_SELFHOST_COMPONENT_CACHE=0` disables reading and writing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};

use sha2::{Digest, Sha256};

use sico_runner::{FsGrants, NetGrants, PreparedProgram, Runner};

const CACHE_DIRECTORY: &str = "sico-selfhost-component-cache-v1";
const CACHE_CAPACITY: usize = 64;
/// Bound on fingerprint inputs; the repository tree is far below this.
const FINGERPRINT_FILE_LIMIT: usize = 20_000;

fn cache_enabled() -> bool {
    std::env::var_os("SICO_SELFHOST_COMPONENT_CACHE").is_none_or(|value| value != "0")
}

/// Builds `root_name` (plus its sibling `sources`) through the real
/// in-process `sico build --profile script-v0` CLI.
fn build_component(root_name: &str, sources: &[(&str, &str)]) -> Vec<u8> {
    static BUILD_ID: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-selfhost-build-{}-{}",
        std::process::id(),
        BUILD_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let root_path = directory.join(root_name);
    let component_path = directory.join("component.wasm");
    for (name, text) in sources {
        std::fs::write(directory.join(name), text).unwrap();
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("build"),
            std::ffi::OsString::from("--profile"),
            std::ffi::OsString::from("script-v0"),
            std::ffi::OsString::from("--output"),
            component_path.as_os_str().to_owned(),
            root_path.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

fn collect_rust_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    if files.len() >= FINGERPRINT_FILE_LIMIT {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
            if files.len() >= FINGERPRINT_FILE_LIMIT {
                return;
            }
        }
    }
}

/// Digest of the selfhost `sources` plus every compiler crate source file,
/// so cached components rotate whenever either side of the build changes.
fn component_key(root_name: &str, sources: &[(&str, &str)]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"sico-build-profile-script-v0");
    hasher.update(root_name.as_bytes());
    for (name, text) in sources {
        hasher.update(name.as_bytes());
        hasher.update((text.len() as u64).to_le_bytes());
        hasher.update(text.as_bytes());
    }
    let crates_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates");
    assert!(
        crates_root.is_dir(),
        "compiler crates root missing: {crates_root:?}"
    );
    let mut files = Vec::new();
    collect_rust_sources(&crates_root, &mut files);
    assert!(
        !files.is_empty(),
        "no compiler sources found under {crates_root:?}"
    );
    files.sort();
    for path in &files {
        let relative = path.strip_prefix(&crates_root).expect("under crates root");
        hasher.update(relative.to_string_lossy().as_bytes());
        let bytes = std::fs::read(path).expect("compiler source is readable");
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    hasher.finalize().into()
}

fn cache_path(key: &[u8; 32]) -> PathBuf {
    let hex: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
    std::env::temp_dir().join(CACHE_DIRECTORY).join(hex)
}

fn prune_cache(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut cached: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((entry.path(), modified))
        })
        .collect();
    if cached.len() <= CACHE_CAPACITY {
        return;
    }
    cached.sort_by_key(|(_, modified)| *modified);
    let excess = cached.len() - CACHE_CAPACITY;
    for (path, _) in cached.into_iter().take(excess) {
        let _ = std::fs::remove_file(path);
    }
}

fn store_component(key: &[u8; 32], component: &[u8]) {
    let directory = std::env::temp_dir().join(CACHE_DIRECTORY);
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let final_path = cache_path(key);
    let temp_path = directory.join(format!(
        "{}.tmp-{}",
        cache_path(key).display(),
        std::process::id()
    ));
    if std::fs::write(&temp_path, component).is_ok()
        && std::fs::rename(&temp_path, &final_path).is_ok()
    {
        prune_cache(&directory);
    } else {
        let _ = std::fs::remove_file(&temp_path);
    }
}

/// Returns the component for `root_name`, reusing a previous identical
/// build when one is cached.
pub fn cached_component(root_name: &str, sources: &[(&str, &str)]) -> Vec<u8> {
    let key = component_key(root_name, sources);
    if cache_enabled()
        && let Ok(bytes) = std::fs::read(cache_path(&key))
    {
        return bytes;
    }
    let component = build_component(root_name, sources);
    if cache_enabled() {
        store_component(&key, &component);
    }
    component
}

/// Default pool slot bound: enough parallelism to matter on a dev host,
/// small enough that concurrent 512 MiB guest stores stay bounded.
pub fn default_pool_slots() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .clamp(1, 4)
}

struct PoolState {
    idle: Vec<PreparedProgram>,
    created: usize,
}

/// A lazily-grown pool of engine-isolated prepared programs. Every slot has
/// its own `Runner`/Engine, so one slot's timeout watchdog cannot advance
/// another slot's epoch; every `PreparedProgram::run` still allocates a
/// fresh Store, resource table and fuel budget. Slots are compiled on
/// first demand so single-fixture invocations pay for exactly one.
pub struct PreparedPool {
    state: Mutex<PoolState>,
    signal: Condvar,
    max_slots: usize,
    build: Box<dyn Fn() -> PreparedProgram + Send + Sync>,
}

impl PreparedPool {
    pub fn new(
        max_slots: usize,
        build: impl Fn() -> PreparedProgram + Send + Sync + 'static,
    ) -> Self {
        Self {
            state: Mutex::new(PoolState {
                idle: Vec::new(),
                created: 0,
            }),
            signal: Condvar::new(),
            max_slots: max_slots.max(1),
            build: Box::new(build),
        }
    }

    pub fn acquire(&self) -> PreparedProgram {
        let mut state = self.state.lock().expect("selfhost pool lock is healthy");
        loop {
            if let Some(prepared) = state.idle.pop() {
                return prepared;
            }
            if state.created < self.max_slots {
                state.created += 1;
                drop(state);
                return (self.build)();
            }
            state = self
                .signal
                .wait(state)
                .expect("selfhost pool lock is healthy");
        }
    }

    pub fn release(&self, prepared: PreparedProgram) {
        let mut state = self.state.lock().expect("selfhost pool lock is healthy");
        state.idle.push(prepared);
        drop(state);
        self.signal.notify_one();
    }
}

/// Prepares one pool slot for a component through the default
/// (no-authority) grants.
pub fn prepare_slot(component: &[u8]) -> PreparedProgram {
    let runner = Runner::new().expect("runner builds");
    runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("selfhost component links")
}
