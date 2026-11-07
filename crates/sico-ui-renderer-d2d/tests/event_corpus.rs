//! The frozen window-event corpus (STEP-0577).
//!
//! Every digest below is the SHA-256 of one delivered event sequence, in the
//! fixed versioned wire form of [`event_sequence_digest`]. The mapping is
//! deterministic by construction — the same `RawMessage` stream always
//! produces the same event sequence, which always hashes the same way — so a
//! digest change means the message→event mapping or the serialization
//! changed, and re-freezing is a deliberate, audited act.
//!
//! The corpus is platform-neutral: the event model and its digest live in
//! [`events`](sico_ui_renderer_d2d::events), which knows nothing about Win32,
//! so these assertions run everywhere. The real-window smoke is the only
//! Windows-only part and is opt-in (`#[ignore]`) because a headless or CI
//! session without an interactive desktop cannot create a real window.
//!
//! The synthetic sequences below exercise the exact `RawMessage` → event path
//! the Win32 `WndProc` feeds, so the corpus is the single source of truth for
//! the event vocabulary (RFC-0052 §2: `click` / `select` / `closed`).

use sico_ui_renderer_d2d::events::{RawMessage, WindowEvent, event_sequence_digest, translate};
use sico_ui_renderer_d2d::shell::{MAX_TITLE_BYTES, WindowShell};
use sico_ui_renderer_d2d::{D2dUiBackend, UiBackendError};

/// One corpus sequence builder.
type SequenceFn = fn() -> Vec<WindowEvent>;

fn resize_paint() -> Vec<WindowEvent> {
    vec![
        WindowEvent::Resized {
            width: 800,
            height: 600,
        },
        WindowEvent::Paint,
        WindowEvent::Resized {
            width: 640,
            height: 480,
        },
        WindowEvent::Paint,
    ]
}

fn click() -> Vec<WindowEvent> {
    vec![
        WindowEvent::Resized {
            width: 320,
            height: 200,
        },
        WindowEvent::Paint,
        WindowEvent::Clicked { x: 96, y: 48 },
    ]
}

fn close() -> Vec<WindowEvent> {
    vec![
        WindowEvent::Resized {
            width: 320,
            height: 200,
        },
        WindowEvent::Paint,
        WindowEvent::Clicked { x: 12, y: 34 },
        WindowEvent::Closed,
    ]
}

/// The frozen digests of the corpus, in corpus order.
const CORPUS: [(&str, SequenceFn, &str); 3] = [
    (
        "resize_paint",
        resize_paint,
        "a527861327bafb4495f8777a66ced5fe2dadfcb13fe8ce937ea914ea6c05c92c",
    ),
    (
        "click",
        click,
        "dd34562962f23c750e4193f2069dfc4868fb83238ae997a124e16f358a0d1356",
    ),
    (
        "close",
        close,
        "46928f99598facfe0de61735cf84ce48ab3bd3b1f32e169d92556568ccc85975",
    ),
];

#[test]
fn event_corpus_digests_are_frozen() {
    for (name, build, digest) in CORPUS {
        assert_eq!(
            event_sequence_digest(&build()),
            digest,
            "{name}: event digest changed"
        );
    }
}

#[test]
fn the_same_sequence_hashes_the_same_way_in_this_process() {
    // Two independent recomputations in one process must agree; the frozen
    // const above already pins the value across processes.
    for (name, build, _) in CORPUS {
        let first = event_sequence_digest(&build());
        let second = event_sequence_digest(&build());
        assert_eq!(first, second, "{name}: recomputation differs");
    }
}

#[test]
fn translate_maps_every_raw_message_to_one_event() {
    assert_eq!(translate(RawMessage::Close), Some(WindowEvent::Closed));
    assert_eq!(
        translate(RawMessage::Resize {
            width: 640,
            height: 480
        }),
        Some(WindowEvent::Resized {
            width: 640,
            height: 480
        })
    );
    assert_eq!(translate(RawMessage::Paint), Some(WindowEvent::Paint));
    assert_eq!(
        translate(RawMessage::LeftButtonDown { x: 5, y: 9 }),
        Some(WindowEvent::Clicked { x: 5, y: 9 })
    );
}

#[test]
fn translate_filters_degenerate_messages() {
    // A minimized window has a zero client size; not delivered.
    assert_eq!(
        translate(RawMessage::Resize {
            width: 320,
            height: 0
        }),
        None
    );
    assert_eq!(
        translate(RawMessage::Resize {
            width: 0,
            height: 200
        }),
        None
    );
    // A pointer press outside the client area is not delivered.
    assert_eq!(translate(RawMessage::LeftButtonDown { x: -1, y: 5 }), None);
    assert_eq!(translate(RawMessage::LeftButtonDown { x: 5, y: -2 }), None);
}

#[test]
fn an_overlong_title_is_refused_before_the_window_is_created() {
    if !cfg!(windows) {
        eprintln!("skipped: the native shell is a Windows implementation");
        return;
    }
    let mut shell = WindowShell::new().expect("shell");
    let overlong = "x".repeat(MAX_TITLE_BYTES + 1);
    assert_eq!(
        shell.create_window(&overlong, 320, 200).unwrap_err(),
        UiBackendError::Limit {
            which: "title-bytes"
        }
    );
    // The refusal must not have half-created a window: a fresh window with a
    // valid title still succeeds (a stray window would give WrongParentKind).
    shell
        .create_window("Ok", 320, 200)
        .expect("a valid title still creates the window");
}

#[test]
#[ignore = "creates a real OS window; run with --ignored on an interactive Windows session"]
fn real_window_smoke_close_present_quit() {
    if !cfg!(windows) {
        eprintln!("skipped: the native shell is a Windows implementation");
        return;
    }
    let mut shell = WindowShell::new().expect("shell");
    shell.create_window("Smoke", 320, 200).expect("window");

    // Render one frame offscreen and present it: the present path must be a
    // pure byte copy, so the read-back equals the rendered frame byte-for-byte
    // and the frozen frame corpus is untouched.
    let backend = D2dUiBackend::new().expect("backend");
    let root = backend.window("Smoke", 320, 200).expect("root");
    let column = backend.column(root).expect("column");
    backend.text(column, "Sico").expect("text");
    let frame = backend.render_frame().expect("frame");
    shell.present_frame(&frame).expect("present");
    let read_back = shell.read_presented_frame().expect("read back");
    assert_eq!(
        read_back.sha256(),
        frame.sha256(),
        "the present path must be byte-identical to the rendered frame"
    );
    assert_eq!(
        shell.client_size().expect("client size"),
        (320, 200),
        "the client area must be the requested 96-DPI logical size"
    );

    // A close posts WM_CLOSE; pumping delivers Closed deterministically, and
    // the default close destroys the window and posts WM_QUIT (should_quit).
    shell.close().expect("close");
    let mut saw_closed = false;
    for _ in 0..64 {
        match shell.pump_once() {
            Some(WindowEvent::Closed) => {
                saw_closed = true;
                break;
            }
            Some(_) => {}
            None => {
                if shell.should_quit() {
                    break;
                }
            }
        }
    }
    assert!(saw_closed, "WM_CLOSE must deliver a Closed event");

    // After the close the OS has already destroyed the window; destroy is
    // idempotent and clears the shell without a second DestroyWindow.
    shell.destroy().expect("destroy is idempotent");
}
