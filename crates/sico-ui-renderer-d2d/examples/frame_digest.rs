//! Prints the frame digests of the frozen corpus.
//!
//! Run it after a deliberate token change to see the new digests, then
//! update `tests/frame_corpus.rs` and the published evidence file:
//!
//! ```text
//! cargo run -p sico-ui-renderer-d2d --example frame_digest
//! ```

use sico_ui_renderer_d2d::{D2dUiBackend, UiBackendError};

/// One corpus scene builder.
type SceneFn = fn(&D2dUiBackend) -> Result<(), UiBackendError>;

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
    let button = backend.button(column, "Start")?;
    let _ = button;
    let dropdown = backend.dropdown(column, &["PNG".to_owned(), "JPEG".to_owned()])?;
    let _ = dropdown;
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

fn main() {
    let scenes: [(&str, SceneFn); 4] = [
        ("empty", scene_empty),
        ("text", scene_text),
        ("controls", scene_controls),
        ("row", scene_row),
    ];
    for (name, build) in scenes {
        let backend = D2dUiBackend::new().expect("backend");
        build(&backend).expect("scene");
        let frame = backend.render_frame().expect("frame");
        println!(
            "{name} {}x{} {}",
            frame.width(),
            frame.height(),
            frame.sha256()
        );
    }
}
