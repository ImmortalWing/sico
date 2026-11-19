//! The frozen frame corpus (ADR-0018 §3).
//!
//! Every digest below is the SHA-256 of the rendered BGRA8 pixels of one
//! fixed scene. The renderer is deterministic by construction — a frozen
//! token table, a fixed client size, no timers, no animation and no
//! environment-derived value — so a digest change means the rendering
//! changed, and a token change is a new renderer version.
//!
//! The digests are **evidence-host local**: they are produced by the
//! DirectWrite glyph rasteriser of the host that froze them, exactly as
//! ADR-0018 §3 scopes the frame corpus ("hashes rendered output byte-stably
//! on the evidence host"). A different Windows build may rasterise glyphs
//! differently; that is a re-freeze, not a regression.
//!
//! The corpus is platform-scoped: the native renderer exists on Windows
//! only, so the assertions are skipped elsewhere instead of pretending the
//! frame exists.
//!
//! # Re-frozen under owner authorization (2026-11-18, audit item D2)
//!
//! The `controls` and `row` digests were re-frozen because the D2 layout
//! unification made the renderer consume [`sico_ui_renderer_d2d::layout::compute`]
//! (the same deterministic token-proxy layout the runner hit-tests a click
//! with) instead of its own DirectWrite-glyph-measured layout. Control
//! positions shifted to the token-proxy widths, so the rendered pixels of the
//! two scenes that contain width-dependent drawing changed; `empty` and
//! `text` are unchanged (a vertical stack of leading-aligned text does not
//! depend on the measured width). Per ADR-0018 §3 a digest change means a new
//! renderer version; this re-freeze is deliberate and owner-authorized.

use sico_ui_renderer_d2d::{D2dUiBackend, UiBackendError};

/// The frozen digests of the corpus, in corpus order.
const CORPUS: [(&str, u32, u32, &str); 4] = [
    (
        "empty",
        200,
        120,
        "039b877a6cfe7bc6bf44253430aa452f71865b0ce47e2f4d5e15ba7bfc95d503",
    ),
    (
        "text",
        320,
        160,
        "0bd324c87c0f4e41c3f61d62d7a10bd8f3b2bc15b7443a791b234673ef592868",
    ),
    (
        "controls",
        360,
        280,
        "a797c72fa884bff8bee6ad53292eea84791c31597f130c20851bca60a4d0c8c0",
    ),
    (
        "row",
        320,
        120,
        "03a089b7726bd5c8c2f99d7c50fc208b31cf0a011da5cee7cf8ff3bce8224b45",
    ),
];

fn scene_empty(backend: &D2dUiBackend) -> Result<(), UiBackendError> {
    backend.window("Empty", 200, 120)?;
    Ok(())
}

fn scene_text(backend: &D2dUiBackend) -> Result<(), UiBackendError> {
    let root = backend.window("Text", 320, 160)?;
    let column = backend.column(root)?;
    backend.text(column, "Sico")?;
    backend.text(column, "Fluent-subset renderer")?;
    Ok(())
}

fn scene_controls(backend: &D2dUiBackend) -> Result<(), UiBackendError> {
    let root = backend.window("Controls", 360, 280)?;
    let column = backend.column(root)?;
    backend.text(column, "Convert")?;
    backend.button(column, "Start")?;
    backend.dropdown(column, &["PNG".to_owned(), "JPEG".to_owned()])?;
    let progress = backend.progress(column)?;
    backend.set_progress(progress, 3, 8)?;
    let image = backend.image_preview(column, 32, 24)?;
    let mut pixels = Vec::with_capacity(32 * 24 * 4);
    for index in 0..(32 * 24) {
        let value = u8::try_from(index % 256).unwrap_or(0);
        pixels.extend_from_slice(&[value, value / 2, 0xff - value / 2, 0xff]);
    }
    backend.draw_image(image, 32, 24, &pixels)?;
    Ok(())
}

fn scene_row(backend: &D2dUiBackend) -> Result<(), UiBackendError> {
    let root = backend.window("Row", 320, 120)?;
    let row = backend.row(root)?;
    backend.button(row, "OK")?;
    backend.button(row, "Cancel")?;
    Ok(())
}

fn build(name: &str) -> Option<Result<sico_ui_renderer_d2d::Frame, UiBackendError>> {
    let backend = D2dUiBackend::new().ok()?;
    let built = match name {
        "empty" => scene_empty(&backend),
        "text" => scene_text(&backend),
        "controls" => scene_controls(&backend),
        "row" => scene_row(&backend),
        _ => return None,
    };
    built.expect("scene builds");
    Some(backend.render_frame())
}

#[test]
fn frame_corpus_digests_are_frozen() {
    if !cfg!(windows) {
        eprintln!("skipped: the native renderer is a Windows implementation");
        return;
    }
    for (name, width, height, digest) in CORPUS {
        let frame = build(name)
            .expect("renderer available")
            .expect("frame renders");
        assert_eq!(frame.width(), width, "{name}: width");
        assert_eq!(frame.height(), height, "{name}: height");
        assert_eq!(frame.sha256(), digest, "{name}: frame digest changed");
    }
}

#[test]
fn rendering_the_same_tree_twice_is_byte_identical() {
    if !cfg!(windows) {
        eprintln!("skipped: the native renderer is a Windows implementation");
        return;
    }
    let first = build("controls")
        .expect("renderer available")
        .expect("frame");
    let second = build("controls")
        .expect("renderer available")
        .expect("frame");
    assert_eq!(first.sha256(), second.sha256());
    assert_eq!(first.pixels(), second.pixels());
}

#[test]
fn a_state_change_changes_the_frame() {
    if !cfg!(windows) {
        eprintln!("skipped: the native renderer is a Windows implementation");
        return;
    }
    let backend = D2dUiBackend::new().expect("backend");
    let root = backend.window("State", 240, 120).expect("window");
    let column = backend.column(root).expect("column");
    let progress = backend.progress(column).expect("progress");
    let before = backend.render_frame().expect("frame").sha256();
    backend.set_progress(progress, 1, 2).expect("progress");
    let after = backend.render_frame().expect("frame").sha256();
    assert_ne!(
        before, after,
        "the frame must be a function of the UI state"
    );
}

#[test]
fn typed_refusals_fail_closed() {
    let backend = D2dUiBackend::new().expect("backend");
    let root = backend.window("Refusals", 200, 120).expect("window");
    let column = backend.column(root).expect("column");
    let button = backend.button(column, "OK").expect("button");

    // Unknown handle.
    assert_eq!(
        backend.text(9_999, "x").unwrap_err(),
        UiBackendError::UnknownHandle
    );
    // A leaf control cannot hold children.
    assert_eq!(
        backend.column(button).unwrap_err(),
        UiBackendError::WrongParentKind
    );
    // A second window root is refused.
    assert_eq!(
        backend.window("Second", 100, 100).unwrap_err(),
        UiBackendError::WrongParentKind
    );
    // limit+1: one byte over the text budget.
    let oversized = "a".repeat(sico_ui_renderer_d2d::tree::MAX_TEXT_BYTES + 1);
    assert_eq!(
        backend.text(column, &oversized).unwrap_err(),
        UiBackendError::Limit {
            which: "text-bytes"
        }
    );
    // An image payload that does not match its declared size is refused.
    let image = backend.image_preview(column, 4, 4).expect("image");
    assert_eq!(
        backend.draw_image(image, 4, 4, &[0u8; 7]).unwrap_err(),
        UiBackendError::InvalidArgument {
            which: "image-payload"
        }
    );
}

#[test]
fn a_frame_without_a_window_is_refused() {
    let backend = D2dUiBackend::new().expect("backend");
    assert_eq!(
        backend.render_frame().unwrap_err(),
        UiBackendError::NoWindow
    );
}
