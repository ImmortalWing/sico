//! M24 G6 STEP-0635: the clean-room-consumer **Sico GUI PNG converter** pilot
//! (`案例项目/gui-converter/converter.sico`), exercised end to end.
//!
//! The pilot is a pure Sico source program: it declares `interface ImageCodec
//! version 1`, binds it with `use pkg image_codec version 1 expose ImageCodec`,
//! builds a `sico.ui.*` window tree, and drives the Convert action from a
//! `sico.ui.poll_event` `click(handle)` — `picker.open` -> `fs.read` ->
//! `image_codec.decode_png` — surfacing every outcome as a typed status Text.
//! This file compiles the source with `sico_cli::run` (the real frontend,
//! resolving the `use pkg` against the staged `sico-lock.json` + `.sapp`),
//! then runs the emitted Component through the real [`Runner`] against a
//! deterministic [`ScriptedUi`] backend, a [`ScriptedPicker`] backend and the
//! real package (`image_codec_package()`, the digest-pinned core).
//!
//! # Evidence class and honest scope
//!
//! This is `clean-room-consumer`: a real Sico source is compiled and run, the
//! whole chain source -> IR -> codegen -> Component -> runner -> host backends
//! is exercised, and the §8.5 fail-closed corpora are asserted at the source
//! status level. The Direct2D frame + OS window shell and the native Win32
//! dialog remain named residuals (ADR-0018 #4, RFC-0055 §7); the deterministic
//! [`ScriptedUi`] exercises the same guest logic the real backend would, with
//! the same typed `click(handle)` surface.
//!
//! **Re-encode (RFC-0057, batch 5):** the v0 gap (no bytes->U64 reader, no
//! I64->U64 cast) is closed by `sico.bytes.le_u64` + `sico.conv.u64_from_i64`.
//! The pilot now reads the 16-byte LE u64 width/height header, slices the
//! pixel payload, re-encodes via `image_codec.encode_png` and writes the file.
//! The §8.5 cases that need a write (save-path failure, byte-stable repeat,
//! long-run batch) are asserted here and pass with real re-encoded output.

use sico_codegen_wasm::package_builder::image_codec_package;
use sico_runner::{
    CancelToken, FsGrants, NetGrants, PackageBinary, RunOutcome, Runner, RunnerLimits, ScriptInput,
    picker::{PathPicker, PickerError, PickerFilter, PickerGrants},
    ui::{UiError, UiEvent, UiGrants, UiHandle, UiRenderer, UiSelection},
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

const PILOT_SOURCE: &str = include_str!("../../../案例项目/gui-converter/converter.sico");
/// The staged build artifact: the **wrapped + dev-signed** package binary
/// (the on-disk `.sapp`), which is what `sico build` resolves the `use pkg`
/// against via `sico-lock.json`. This differs from the raw `image_codec_package()`
/// component passed to the runner at instantiation time.
const IMAGE_CODEC_SAPP: &[u8] =
    include_bytes!("../../../案例项目/gui-converter/pkgs/image_codec.sapp");
const LOCK_JSON: &str = r#"{
  "schema": "sico:source-lock:v0",
  "packages": [
    {"name":"image_codec","version":1,"path":"pkgs/image_codec.sapp","sha256":"9ba7c2f0e25e3f5e56e040f0c20dc0a35c25913c71e520820dc921eb7093156c"}
  ]
}"#;

/// The corpus directory holding the frozen image-codec fixtures.
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/image-codec")
}

fn unique_temp(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-g6-converter-{tag}-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    directory
}

/// Compiles the pilot source once (the component bytes are fixture-independent)
/// and returns them. The `use pkg` is resolved against a staged
/// `sico-lock.json` + `pkgs/image_codec.sapp` beside the staged source.
fn compile_pilot() -> Vec<u8> {
    use std::sync::OnceLock;
    static COMPONENT: OnceLock<Vec<u8>> = OnceLock::new();
    COMPONENT
        .get_or_init(|| {
            let directory = unique_temp("build");
            std::fs::write(directory.join("converter.sico"), PILOT_SOURCE).unwrap();
            std::fs::write(directory.join("sico-lock.json"), LOCK_JSON).unwrap();
            let pkgs = directory.join("pkgs");
            std::fs::create_dir(&pkgs).unwrap();
            std::fs::write(pkgs.join("image_codec.sapp"), IMAGE_CODEC_SAPP).unwrap();
            let source_path = directory.join("converter.sico");
            let component_path = directory.join("converter.component.wasm");
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
                    source_path.as_os_str().to_owned(),
                ],
                &mut std::io::empty(),
                &mut stdout,
                &mut stderr,
            );
            assert_eq!(
                exit,
                0,
                "pilot build failed: {}",
                String::from_utf8_lossy(&stderr)
            );
            let component = std::fs::read(&component_path).unwrap();
            std::fs::remove_dir_all(directory).unwrap();
            component
        })
        .clone()
}

/// Deterministic UI backend: assigns node handles 1..6 in call order
/// (window=1, column=2, status-text=3, convert-button=4, progress=5,
/// image-preview=6 — the pilot's build order), records every `set_text`
/// content, and drains a scripted event queue one event per `poll_event`.
struct ScriptedUi {
    events: Mutex<VecDeque<UiEvent>>,
    set_texts: Mutex<Vec<String>>,
    next_handle: AtomicUsize,
    window_title: Mutex<Option<String>>,
}

impl ScriptedUi {
    fn with_events(events: Vec<UiEvent>) -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(events.into_iter().collect()),
            set_texts: Mutex::new(Vec::new()),
            next_handle: AtomicUsize::new(1),
            window_title: Mutex::new(None),
        })
    }

    fn next(&self) -> u64 {
        self.next_handle.fetch_add(1, Ordering::SeqCst) as u64
    }

    fn set_texts(&self) -> Vec<String> {
        self.set_texts.lock().unwrap().clone()
    }
}

impl UiRenderer for ScriptedUi {
    fn window(&self, title: &str, _width: i64, _height: i64) -> Result<u64, UiError> {
        *self.window_title.lock().unwrap() = Some(title.to_owned());
        Ok(self.next())
    }
    fn column(&self, _parent: u64) -> Result<u64, UiError> {
        Ok(self.next())
    }
    fn row(&self, _parent: u64) -> Result<u64, UiError> {
        Ok(self.next())
    }
    fn text(&self, _parent: u64, _content: &str) -> Result<u64, UiError> {
        Ok(self.next())
    }
    fn button(&self, _parent: u64, _label: &str) -> Result<u64, UiError> {
        Ok(self.next())
    }
    fn dropdown(&self, _parent: u64, _options: &[String]) -> Result<u64, UiError> {
        Ok(self.next())
    }
    fn image_preview(&self, _parent: u64, _width: u64, _height: u64) -> Result<u64, UiError> {
        Ok(self.next())
    }
    fn progress(&self, _parent: u64) -> Result<u64, UiError> {
        Ok(self.next())
    }
    fn set_text(&self, _handle: u64, content: &str) -> Result<bool, UiError> {
        self.set_texts.lock().unwrap().push(content.to_owned());
        Ok(true)
    }
    fn set_progress(&self, _handle: u64, _done: u64, _of: u64) -> Result<bool, UiError> {
        Ok(true)
    }
    fn draw_image(
        &self,
        _handle: u64,
        _width: u64,
        _height: u64,
        _pixels: &[u8],
    ) -> Result<bool, UiError> {
        Ok(true)
    }
    fn poll_event(&self) -> Option<UiEvent> {
        self.events.lock().unwrap().pop_front()
    }
    fn run(&self) -> Result<bool, UiError> {
        Ok(true)
    }
}

/// Deterministic picker backend: returns the configured open/save strings and
/// counts how many times each dialog was reached. A `Cancelled` return is used
/// for the flattening regression (the guest must see the empty ok path).
struct ScriptedPicker {
    open: String,
    save: String,
    open_cancelled: std::sync::atomic::AtomicBool,
    save_cancelled: std::sync::atomic::AtomicBool,
    open_reached: AtomicUsize,
    save_reached: AtomicUsize,
}

impl ScriptedPicker {
    fn new(open: &str, save: &str) -> Arc<Self> {
        Arc::new(Self {
            open: open.to_owned(),
            save: save.to_owned(),
            open_cancelled: std::sync::atomic::AtomicBool::new(false),
            save_cancelled: std::sync::atomic::AtomicBool::new(false),
            open_reached: AtomicUsize::new(0),
            save_reached: AtomicUsize::new(0),
        })
    }

    fn save_reached(&self) -> usize {
        self.save_reached.load(Ordering::SeqCst)
    }
}

impl PathPicker for ScriptedPicker {
    fn open(&self, _title: &str, _filters: &[PickerFilter]) -> Result<String, PickerError> {
        self.open_reached.fetch_add(1, Ordering::SeqCst);
        if self.open_cancelled.load(Ordering::SeqCst) {
            Err(PickerError::Cancelled)
        } else {
            Ok(self.open.clone())
        }
    }
    fn save(
        &self,
        _title: &str,
        _suggested_name: &str,
        _filters: &[PickerFilter],
    ) -> Result<String, PickerError> {
        self.save_reached.fetch_add(1, Ordering::SeqCst);
        if self.save_cancelled.load(Ordering::SeqCst) {
            Err(PickerError::Cancelled)
        } else {
            Ok(self.save.clone())
        }
    }
}

/// The pilot's convert-button handle (ScriptedUi assigns window=1, column=2,
/// status=3, convert-button=4 in the pilot's build order).
const CONVERT_HANDLE: u64 = 4;

/// One pilot run outcome plus the backend records needed for assertions.
struct PilotRun {
    outcome: RunOutcome,
    ui: Arc<ScriptedUi>,
    picker: Arc<ScriptedPicker>,
    /// The run directory: the single read/write grant root. A successful
    /// re-encode writes `save` here, so tests read it back for byte checks.
    run_dir: PathBuf,
}

/// Stages `fixture` (or nothing when empty) under a fresh read root, runs the
/// pilot against a scripted UI + picker + the real package, and returns the
/// outcome and backend records.
fn run_pilot(events: Vec<UiEvent>, open: &str, save: &str, fixture: Option<&str>) -> PilotRun {
    let dir = unique_temp("run");
    // The runner's fs contract: grant roots are canonicalized by the caller
    // (`capability_state_machine.rs`), so `resolve_read`'s
    // `canonical.starts_with(root)` containment check compares like with like.
    let read_root = dir.canonicalize().unwrap();
    if let Some(name) = fixture {
        let bytes = std::fs::read(corpus_dir().join(name))
            .unwrap_or_else(|error| panic!("read fixture {name}: {error}"));
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    let component = compile_pilot();
    let ui = ScriptedUi::with_events(events);
    let picker = ScriptedPicker::new(open, save);
    let runner = Runner::new()
        .expect("runner builds")
        .with_ui_backend(UiGrants { window: true }, ui.clone())
        .with_picker_backend(
            PickerGrants {
                open: true,
                save: true,
            },
            picker.clone(),
        );
    let fs_roots = FsGrants {
        read_roots: vec![read_root.clone()],
        write_roots: vec![read_root],
    };
    let packages = [PackageBinary {
        bytes: image_codec_package(),
    }];
    let prepared = runner
        .prepare_program_with_packages(&component, &fs_roots, &NetGrants::default(), &packages)
        .expect("the pilot component links against the prepared package");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold");
    PilotRun {
        outcome,
        ui,
        picker,
        run_dir: dir,
    }
}

fn status_of(run: &PilotRun) -> String {
    let RunOutcome::Output(output) = &run.outcome else {
        panic!("expected guest output, got {:?}", run.outcome);
    };
    assert_eq!(output.exit_code, 0, "the pilot exits 0 on a typed status");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn assert_status(run: &PilotRun, expected: &str) {
    assert_eq!(
        status_of(run),
        expected,
        "the pilot's last status must be the typed outcome"
    );
    assert!(
        run.ui.set_texts().iter().any(|text| text == expected),
        "the status must have been pushed to the status node, got {:?}",
        run.ui.set_texts()
    );
}

fn click_convert() -> Vec<UiEvent> {
    vec![
        UiEvent::Click(UiHandle { id: CONVERT_HANDLE }),
        UiEvent::Closed,
    ]
}

// ---------------------------------------------------------------------------
// §8.5 fail-closed decode corpora (PNG-completable): every case is a typed
// status at the source level; none traps, hangs, writes a partial file or
// falls back silently.
// ---------------------------------------------------------------------------

#[test]
fn bad_signature_is_a_typed_status() {
    let run = run_pilot(
        click_convert(),
        "bad-signature.png",
        "out.png",
        Some("bad-signature.png"),
    );
    assert_status(&run, "decode: bad-signature");
    // The decode failed before the save dialog; no write is ever attempted.
    assert_eq!(run.picker.save_reached(), 0);
}

#[test]
fn truncated_is_a_typed_status() {
    let run = run_pilot(
        click_convert(),
        "truncated.png",
        "out.png",
        Some("truncated.png"),
    );
    assert_status(&run, "decode: truncated");
    assert_eq!(run.picker.save_reached(), 0);
}

#[test]
fn trailing_data_is_a_typed_status() {
    let run = run_pilot(
        click_convert(),
        "trailing-data.png",
        "out.png",
        Some("trailing-data.png"),
    );
    assert_status(&run, "decode: trailing-data");
    assert_eq!(run.picker.save_reached(), 0);
}

#[test]
fn oversized_limit_plus_one_is_a_typed_status() {
    let run = run_pilot(
        click_convert(),
        "oversized.png",
        "out.png",
        Some("oversized.png"),
    );
    assert_status(&run, "decode: oversized");
    assert_eq!(run.picker.save_reached(), 0);
}

#[test]
fn unsupported_palette_is_a_typed_status() {
    let run = run_pilot(
        click_convert(),
        "palette.png",
        "out.png",
        Some("palette.png"),
    );
    assert_status(&run, "decode: unsupported-profile");
    assert_eq!(run.picker.save_reached(), 0);
}

#[test]
fn unsupported_depth_16_is_a_typed_status() {
    let run = run_pilot(
        click_convert(),
        "depth-16.png",
        "out.png",
        Some("depth-16.png"),
    );
    assert_status(&run, "decode: unsupported-profile");
    assert_eq!(run.picker.save_reached(), 0);
}

#[test]
fn unsupported_interlaced_is_a_typed_status() {
    let run = run_pilot(
        click_convert(),
        "interlaced.png",
        "out.png",
        Some("interlaced.png"),
    );
    assert_status(&run, "decode: unsupported-profile");
    assert_eq!(run.picker.save_reached(), 0);
}

// ---------------------------------------------------------------------------
// Happy path and cancellation (still never writes).
// ---------------------------------------------------------------------------

#[test]
fn decode_ok_reports_dimensions_then_save_cancelled_is_typed() {
    // The decoded 2x2 payload header carries width=2, height=2; the pilot
    // reports "decoded 2x2" then offers the save dialog, which returns the
    // empty ok path = cancelled. No write.
    let run = run_pilot(
        click_convert(),
        "ok-2x2-rgba.png",
        "",
        Some("ok-2x2-rgba.png"),
    );
    assert_status(&run, "cancelled");
    assert_eq!(
        run.picker.save_reached(),
        1,
        "the save dialog was offered once"
    );
}

#[test]
fn decode_ok_reencodes_and_writes_a_real_png() {
    // RFC-0057 closed the v0 gap: the 16-byte LE u64 width/height header is
    // read by `sico.bytes.le_u64` and re-fed to `encode-png` (which demands
    // U64) directly. The pilot now writes a real re-encoded PNG file.
    let run = run_pilot(
        click_convert(),
        "ok-2x2-rgba.png",
        "out.png",
        Some("ok-2x2-rgba.png"),
    );
    assert_status(&run, "re-encoded 2x2");
    assert_eq!(run.picker.save_reached(), 1);
    let written = run.run_dir.join("out.png");
    let bytes = std::fs::read(&written)
        .unwrap_or_else(|error| panic!("the re-encoded PNG must be written: {error}"));
    assert!(bytes.len() > 8, "re-encoded PNG must be non-trivial");
    assert_eq!(
        &bytes[0..8],
        b"\x89PNG\r\n\x1a\n",
        "the re-encoded file must carry the PNG signature"
    );
}

#[test]
fn byte_stable_repeat_produces_identical_png() {
    // RFC-0057 payoff: the re-encode is deterministic — two independent runs
    // over the same fixture write byte-identical PNG files (no embedded
    // timestamp, no non-deterministic chunk).
    let run_a = run_pilot(
        click_convert(),
        "ok-2x2-rgba.png",
        "out.png",
        Some("ok-2x2-rgba.png"),
    );
    assert_status(&run_a, "re-encoded 2x2");
    let bytes_a = std::fs::read(run_a.run_dir.join("out.png")).unwrap();
    let run_b = run_pilot(
        click_convert(),
        "ok-2x2-rgba.png",
        "out.png",
        Some("ok-2x2-rgba.png"),
    );
    assert_status(&run_b, "re-encoded 2x2");
    let bytes_b = std::fs::read(run_b.run_dir.join("out.png")).unwrap();
    assert_eq!(
        bytes_a, bytes_b,
        "re-encode must be byte-stable across independent runs"
    );
}

#[test]
fn save_path_failure_is_a_typed_status() {
    // The save dialog returns a path that escapes the granted write root;
    // the write must fail closed as a typed status (never a trap, hang or
    // partial write). `..` is rejected deterministically by `scope_path`.
    let run = run_pilot(
        click_convert(),
        "ok-2x2-rgba.png",
        "../escape.png",
        Some("ok-2x2-rgba.png"),
    );
    assert_status(&run, "write: storage.write: path escapes the granted scope");
    assert_eq!(run.picker.save_reached(), 1);
    assert!(
        !run.run_dir.parent().unwrap().join("escape.png").exists(),
        "no file may appear above the granted root"
    );
}

#[test]
fn long_run_batch_sampling_terminates_typed_every_case() {
    // RFC-0057 payoff: a batch over the frozen §8.5 corpus terminates with a
    // typed status on every case — decode failures stay typed, and the ok
    // case now completes the full decode -> re-encode -> write path.
    for name in [
        "bad-signature.png",
        "truncated.png",
        "trailing-data.png",
        "oversized.png",
        "palette.png",
        "depth-16.png",
        "interlaced.png",
        "ok-2x2-rgba.png",
    ] {
        let run = run_pilot(click_convert(), name, "out.png", Some(name));
        let status = status_of(&run);
        assert!(
            status.starts_with("decode: ") || status.starts_with("re-encoded "),
            "case {name} must surface a typed status, got {status}"
        );
    }
}

#[test]
fn cancelled_open_is_a_typed_status() {
    // The open dialog returns the empty ok path = cancelled, before any read.
    let run = run_pilot(click_convert(), "", "out.png", None);
    assert_status(&run, "cancelled");
    assert_eq!(run.picker.save_reached(), 0);
}

#[test]
fn pick_open_flattening_surfaces_the_empty_path() {
    // A backend `cancelled` (what the native Win32 dialog does on user cancel)
    // must surface as the empty ok path — the STEP-0557 flattening — and the
    // pilot reports "cancelled" without reaching the backend save.
    let component = compile_pilot();
    let ui = ScriptedUi::with_events(click_convert());
    let picker = ScriptedPicker::new("", "out.png");
    picker.open_cancelled.store(true, Ordering::SeqCst);
    let runner = Runner::new()
        .expect("runner builds")
        .with_ui_backend(UiGrants { window: true }, ui.clone())
        .with_picker_backend(
            PickerGrants {
                open: true,
                save: true,
            },
            picker.clone(),
        );
    let fs_roots = FsGrants {
        read_roots: vec![],
        write_roots: vec![],
    };
    let prepared = runner
        .prepare_program_with_packages(
            &component,
            &fs_roots,
            &NetGrants::default(),
            &[PackageBinary {
                bytes: image_codec_package(),
            }],
        )
        .expect("links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("runs");
    let RunOutcome::Output(output) = outcome else {
        panic!("expected output");
    };
    assert_eq!(output.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cancelled");
}

#[test]
fn select_event_is_a_typed_status_not_a_trap() {
    // A dropdown select (not deliverable by the real host surface in this
    // slice) must surface as a typed status, never trap or hang.
    let run = run_pilot(
        vec![
            UiEvent::Select(UiSelection {
                handle: UiHandle { id: 6 },
                index: 0,
            }),
            UiEvent::Closed,
        ],
        "",
        "out.png",
        None,
    );
    assert_status(&run, "select unsupported in this pilot");
}

// ---------------------------------------------------------------------------
// Authority: an ungranted UI never reaches the backend (default-deny).
// ---------------------------------------------------------------------------

#[test]
fn denied_ui_is_typed_authority_refusal_with_zero_backend_touch() {
    let component = compile_pilot();
    let ui = ScriptedUi::with_events(click_convert());
    let picker = ScriptedPicker::new("", "out.png");
    let runner = Runner::new()
        .expect("runner builds")
        .with_ui_backend(UiGrants::default(), ui.clone())
        .with_picker_backend(
            PickerGrants {
                open: true,
                save: true,
            },
            picker.clone(),
        );
    let fs_roots = FsGrants {
        read_roots: vec![],
        write_roots: vec![],
    };
    let prepared = runner
        .prepare_program_with_packages(
            &component,
            &fs_roots,
            &NetGrants::default(),
            &[PackageBinary {
                bytes: image_codec_package(),
            }],
        )
        .expect("links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: Vec::new(),
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("runs");
    let RunOutcome::Output(output) = outcome else {
        panic!("expected output");
    };
    // The first `sico.ui.window` fails closed as `authority-denied`; the guest
    // surfaces it as the typed status "build_tree: authority-denied" with a
    // non-zero exit (the pilot's typed-failure convention, not a guest error).
    assert_eq!(output.exit_code, 1);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "build_tree: authority-denied"
    );
    // The backend was never touched.
    assert_eq!(ui.window_title.lock().unwrap().as_deref(), None);
    assert!(ui.set_texts().is_empty());
}
