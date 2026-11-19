//! M24 §8.6.1 STEP-0628: source-level `sico:script/ui@1.0.0` emission
//! end-to-end. The fixture compiles `tests/end-to-end/script-ui-window.sico`
//! — pure Sico source calling the 13 `sico.ui.*` intrinsics — into a Component
//! that imports the ui host surface, and runs it through the real runner
//! against a deterministic [`ScriptedUi`] backend.
//!
//! This is the clean-room-consumer counterpart of the STEP-0554/0555/0556
//! `ui_d2d_source.rs` arc: here the guest is a real Sico source program, so the
//! whole chain — source → IR → codegen → Component → runner → UI backend — is
//! exercised. A granted guest receives the backend's node ids, the typed
//! event kind, and the backend's booleans; an ungranted guest surfaces the
//! typed `authority-denied` case name without touching the backend. The real
//! Direct2D frame and OS window remain named residuals (ADR-0018 #4).
//!
//! `poll-event` is RFC-0056 D5/D7 (STEP-0632): the frozen WIT keeps
//! `option<ui-event>` on the host side, and the guest binds it into the
//! typed `Option[ui_event]` — the program declares the pinned-shape
//! `enum ui_event` and matches `some(ev)` / `none`, with handle/index
//! attribution (`|poll=click:5`, `|poll=select:6:2`).

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
    ui::{UiError, UiEvent, UiGrants, UiHandle, UiRenderer, UiSelection},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const UI_SOURCE: &str = include_str!("../../../tests/end-to-end/script-ui-window.sico");

fn compile_source() -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0628-ui-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("ui-window.sico");
    let component_path = directory.join("ui-window.component.wasm");
    std::fs::write(&source_path, UI_SOURCE).unwrap();
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

/// Deterministic UI backend: the node ids are the closed fixture vocabulary
/// (`window` 1, `column` 2, `row` 3, `text` 4, `button` 5, `dropdown` 6,
/// `image-preview` 7, `progress` 8) and the backend counts how many times it
/// was reached. The backend itself asserts nothing about authority — that is
/// the binding's job — so the count is the proof a denied call never reached
/// it.
struct ScriptedUi {
    event: Option<UiEvent>,
    reached: AtomicUsize,
}

impl ScriptedUi {
    fn with_event(event: Option<UiEvent>) -> Arc<Self> {
        Arc::new(Self {
            event,
            reached: AtomicUsize::new(0),
        })
    }

    fn reached(&self) -> usize {
        self.reached.load(Ordering::Relaxed)
    }

    fn hit(&self) {
        self.reached.fetch_add(1, Ordering::Relaxed);
    }
}

impl UiRenderer for ScriptedUi {
    fn window(&self, _title: &str, _width: i64, _height: i64) -> Result<u64, UiError> {
        self.hit();
        Ok(1)
    }
    fn column(&self, _parent: u64) -> Result<u64, UiError> {
        self.hit();
        Ok(2)
    }
    fn row(&self, _parent: u64) -> Result<u64, UiError> {
        self.hit();
        Ok(3)
    }
    fn text(&self, _parent: u64, _content: &str) -> Result<u64, UiError> {
        self.hit();
        Ok(4)
    }
    fn button(&self, _parent: u64, _label: &str) -> Result<u64, UiError> {
        self.hit();
        Ok(5)
    }
    fn dropdown(&self, _parent: u64, _options: &[String]) -> Result<u64, UiError> {
        self.hit();
        Ok(6)
    }
    fn image_preview(&self, _parent: u64, _w: u64, _h: u64) -> Result<u64, UiError> {
        self.hit();
        Ok(7)
    }
    fn progress(&self, _parent: u64) -> Result<u64, UiError> {
        self.hit();
        Ok(8)
    }
    fn set_text(&self, _handle: u64, _content: &str) -> Result<bool, UiError> {
        self.hit();
        Ok(true)
    }
    fn set_progress(&self, _handle: u64, _done: u64, _of: u64) -> Result<bool, UiError> {
        self.hit();
        Ok(true)
    }
    fn draw_image(&self, _handle: u64, _w: u64, _h: u64, _pixels: &[u8]) -> Result<bool, UiError> {
        self.hit();
        Ok(true)
    }
    fn poll_event(&self) -> Option<UiEvent> {
        self.hit();
        self.event
    }
    fn run(&self) -> Result<bool, UiError> {
        self.hit();
        Ok(true)
    }
}

fn run_source(component: &[u8], grants: &UiGrants, backend: Arc<ScriptedUi>) -> RunOutcome {
    let runner = Runner::new()
        .expect("runner builds")
        .with_ui_backend(grants.clone(), backend);
    let prepared = runner
        .prepare_program_with_net(component, &FsGrants::default(), &NetGrants::default())
        .expect("source-emitted ui component links");
    prepared
        .run(
            &ScriptInput {
                arguments: vec!["unused".to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("the run completes")
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

/// The transcript the source emits when every call succeeds and the backend
/// has a `closed` event pending: one segment per `sico.ui.*` intrinsic.
const GRANTED_TRANSCRIPT: &str = "window=1|column=2|text=4|button=5|row=3|progress=8\
     |dropdown=6|preview=7|set_text=1|set_progress=1|draw_image=1|run=1|poll=closed";

#[test]
fn a_granted_source_receives_the_backends_node_ids() {
    let component = compile_source();
    let backend = ScriptedUi::with_event(Some(UiEvent::Closed));
    let outcome = run_source(&component, &UiGrants { window: true }, backend.clone());
    assert_eq!(decode_stdout(&outcome), GRANTED_TRANSCRIPT);
    assert_exit_code(&outcome, 0);
    // Every one of the 13 intrinsics reached the backend.
    assert_eq!(backend.reached(), 13);
}

#[test]
fn a_granted_source_with_no_pending_event_sees_the_empty_poll_text() {
    let component = compile_source();
    let backend = ScriptedUi::with_event(None);
    let outcome = run_source(&component, &UiGrants { window: true }, backend.clone());
    assert_eq!(
        decode_stdout(&outcome),
        "window=1|column=2|text=4|button=5|row=3|progress=8\
         |dropdown=6|preview=7|set_text=1|set_progress=1|draw_image=1|run=1|poll=none"
    );
    assert_exit_code(&outcome, 0);
    assert_eq!(backend.reached(), 13);
}

#[test]
fn a_granted_source_decodes_the_click_event_kind() {
    // The STEP-0628 §1 residual: the host `option<ui-event>` is decoded to the
    // event-kind Text at the guest boundary, so a click reads `click`.
    let component = compile_source();
    let backend = ScriptedUi::with_event(Some(UiEvent::Click(UiHandle { id: 5 })));
    let outcome = run_source(&component, &UiGrants { window: true }, backend.clone());
    assert!(
        decode_stdout(&outcome).ends_with("|poll=click:5"),
        "expected the click event kind, got {}",
        decode_stdout(&outcome)
    );
    assert_exit_code(&outcome, 0);
}

#[test]
fn a_granted_source_decodes_the_select_event_kind() {
    let component = compile_source();
    let backend = ScriptedUi::with_event(Some(UiEvent::Select(UiSelection {
        handle: UiHandle { id: 6 },
        index: 2,
    })));
    let outcome = run_source(&component, &UiGrants { window: true }, backend.clone());
    assert!(
        decode_stdout(&outcome).ends_with("|poll=select:6:2"),
        "expected the select event kind, got {}",
        decode_stdout(&outcome)
    );
    assert_exit_code(&outcome, 0);
}

#[test]
fn a_denied_source_never_reaches_the_backend() {
    let component = compile_source();
    let backend = ScriptedUi::with_event(Some(UiEvent::Closed));
    let outcome = run_source(&component, &UiGrants::default(), backend.clone());
    // Typed refusal surfaced as source-level Text, never a native window.
    assert_eq!(decode_stdout(&outcome), "window:authority-denied");
    assert_exit_code(&outcome, 1);
    // The backend was never reached — authority is enforced at the binding.
    assert_eq!(backend.reached(), 0);
}

#[test]
fn a_guest_without_a_ui_backend_fails_to_instantiate() {
    let component = compile_source();
    // No `with_ui_backend`: the surface is not linked at all, so the import
    // fails to instantiate (an honest "ui backend unavailable").
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
        message.to_lowercase().contains("ui"),
        "expected the ui import failure, got {message}"
    );
}
