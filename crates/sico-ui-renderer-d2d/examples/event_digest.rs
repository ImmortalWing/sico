//! Prints the event-sequence digests of the frozen window-event corpus.
//!
//! Run it after a deliberate change to the event model or the serialization
//! to see the new digests, then update `tests/event_corpus.rs` and the
//! published evidence file:
//!
//! ```text
//! cargo run -p sico-ui-renderer-d2d --example event_digest
//! ```

use sico_ui_renderer_d2d::events::{WindowEvent, event_sequence_digest};

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

fn main() {
    let sequences: [(&str, SequenceFn); 3] = [
        ("resize_paint", resize_paint),
        ("click", click),
        ("close", close),
    ];
    for (name, build) in sequences {
        let events = build();
        println!("{name} {}", event_sequence_digest(&events));
    }
}
