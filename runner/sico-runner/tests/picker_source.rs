//! M24 §8.6.1 STEP-0627: source-level `sico:script/path-picker@1.0.0`
//! emission end-to-end. The fixture compiles
//! `tests/end-to-end/script-picker-open.sico` — pure Sico source calling
//! `sico.picker.open(...)` — into a Component that imports the path-picker
//! host surface, and runs it through the real runner against a deterministic
//! [`ScriptedPicker`] backend.
//!
//! This is the clean-room-consumer counterpart of the STEP-0554/0555/0556
//! `ui_d2d_source.rs` arc: here the guest is a real Sico source program, so the
//! whole chain — source → IR → codegen → Component → runner → picker backend —
//! is exercised. A granted guest receives the backend's chosen path; a granted
//! cancellation surfaces as the empty ok string (the flattened binding); an
//! ungranted guest surfaces the typed `authority-denied` case name without
//! touching the backend. The native Win32 dialog itself is a named residual
//! (RFC-0055 §7), mirroring the Direct2D #4 evidence boundary.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
    picker::{PathPicker, PickerError, PickerFilter, PickerGrants},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const PICKER_SOURCE: &str = include_str!("../../../tests/end-to-end/script-picker-open.sico");

fn compile_source() -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0627-picker-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("picker-open.sico");
    let component_path = directory.join("picker-open.component.wasm");
    std::fs::write(&source_path, PICKER_SOURCE).unwrap();
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
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

/// Deterministic picker backend: returns a fixed path or cancellation, and
/// counts how many times either dialog was reached. The backend itself asserts
/// nothing about authority — that is the binding's job — so the count is the
/// proof a denied call never reached it.
struct ScriptedPicker {
    path: &'static str,
    chosen: AtomicUsize,
    save_reached: AtomicUsize,
}

impl ScriptedPicker {
    fn new(path: &'static str) -> Arc<Self> {
        Arc::new(Self {
            path,
            chosen: AtomicUsize::new(0),
            save_reached: AtomicUsize::new(0),
        })
    }

    fn chosen(&self) -> usize {
        self.chosen.load(Ordering::Relaxed)
    }

    fn save_reached(&self) -> usize {
        self.save_reached.load(Ordering::Relaxed)
    }
}

impl PathPicker for ScriptedPicker {
    fn open(&self, _title: &str, _filters: &[PickerFilter]) -> Result<String, PickerError> {
        self.chosen.fetch_add(1, Ordering::Relaxed);
        Ok(self.path.to_owned())
    }

    fn save(
        &self,
        _title: &str,
        _suggested_name: &str,
        _filters: &[PickerFilter],
    ) -> Result<String, PickerError> {
        self.save_reached.fetch_add(1, Ordering::Relaxed);
        Ok(self.path.to_owned())
    }
}

fn run_source(component: &[u8], grants: &PickerGrants) -> (RunOutcome, Arc<ScriptedPicker>) {
    let backend = ScriptedPicker::new("C:\\Users\\sun\\Downloads\\chosen.txt");
    let runner = Runner::new()
        .expect("runner builds")
        .with_picker_backend(grants.clone(), backend.clone());
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("source-emitted picker component links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("the run completes");
    (outcome, backend)
}

fn decode_stdout(outcome: &RunOutcome) -> String {
    let RunOutcome::Output(output) = outcome else {
        panic!("expected guest output, got {outcome:?}");
    };
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn assert_exit_code(outcome: &RunOutcome, expected: i64) {
    let RunOutcome::Output(output) = outcome else {
        unreachable!("completed above");
    };
    assert_eq!(output.exit_code, expected);
}

#[test]
fn a_granted_open_receives_the_backends_chosen_path() {
    let component = compile_source();
    let (outcome, backend) = run_source(
        &component,
        &PickerGrants {
            open: true,
            save: false,
        },
    );
    assert_eq!(
        decode_stdout(&outcome),
        "C:\\Users\\sun\\Downloads\\chosen.txt"
    );
    assert_exit_code(&outcome, 0);
    // The open dialog was reached exactly once; save was never offered.
    assert_eq!(backend.chosen(), 1);
    assert_eq!(backend.save_reached(), 0);
}

#[test]
fn a_granted_cancellation_surfaces_as_the_empty_ok_string() {
    let component = compile_source();
    let backend = ScriptedPicker::new("");
    let runner = Runner::new().expect("runner builds").with_picker_backend(
        PickerGrants {
            open: true,
            save: false,
        },
        backend.clone(),
    );
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("source-emitted picker component links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("the run completes");
    // The flattened binding: the guest sees an empty ok string, exit 0.
    assert_eq!(decode_stdout(&outcome), "");
    assert_exit_code(&outcome, 0);
    assert_eq!(backend.chosen(), 1);
}

#[test]
fn a_denied_open_never_reaches_the_backend() {
    let component = compile_source();
    let (outcome, backend) = run_source(
        &component,
        &PickerGrants {
            open: false,
            save: false,
        },
    );
    // Typed refusal surfaced as source-level Text, never a native dialog.
    assert_eq!(decode_stdout(&outcome), "error:authority-denied");
    assert_exit_code(&outcome, 1);
    // The backend was never reached — authority is enforced at the binding.
    assert_eq!(backend.chosen(), 0);
    assert_eq!(backend.save_reached(), 0);
}

#[test]
fn a_cancelled_backend_surfaces_as_the_empty_ok_string() {
    // STEP-0557 flattening: a backend signalling `cancelled` (exactly what the
    // native Win32 dialog does on user cancel) must surface as the empty ok
    // path at the guest — not as the `cancelled` error case name. This is the
    // regression that the `ok("")`-returning double cannot prove on its own.
    let component = compile_source();
    struct CancellingPicker;
    impl PathPicker for CancellingPicker {
        fn open(&self, _title: &str, _filters: &[PickerFilter]) -> Result<String, PickerError> {
            Err(PickerError::Cancelled)
        }
        fn save(
            &self,
            _title: &str,
            _suggested_name: &str,
            _filters: &[PickerFilter],
        ) -> Result<String, PickerError> {
            Err(PickerError::Cancelled)
        }
    }
    let runner = Runner::new().expect("runner builds").with_picker_backend(
        PickerGrants {
            open: true,
            save: false,
        },
        Arc::new(CancellingPicker),
    );
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("source-emitted picker component links");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("the run completes");
    // The flattening is applied at the host seam: cancelled → ok("") → exit 0.
    assert_eq!(decode_stdout(&outcome), "");
    assert_exit_code(&outcome, 0);
}

#[test]
fn a_guest_without_a_picker_backend_fails_to_instantiate() {
    let component = compile_source();
    // No `with_picker_backend`: the surface is not linked at all, so the
    // import fails to instantiate (an honest "picker backend unavailable").
    // The failure surfaces at the run's instantiation step, not at prepare.
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_net(&component, &FsGrants::default(), &NetGrants::default())
        .expect("prepare succeeds; the missing backend surfaces at instantiation");
    let outcome = prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("the run completes");
    let RunOutcome::Launch(message) = &outcome else {
        panic!("expected an instantiation failure, got {outcome:?}");
    };
    assert!(
        message.contains("picker"),
        "expected the picker import failure, got {message}"
    );
}
