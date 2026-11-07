//! The window event model and its deterministic message mapping.
//!
//! This module is the safe core of the event pump (STEP-0577). It is
//! platform-neutral: it knows nothing about `MSG`, `WndProc` or any other
//! Win32 type. The [`shell`](crate::shell) module reduces real OS messages to
//! [`RawMessage`] and feeds them through [`translate`], so the synthetic
//! corpus below exercises the exact same mapping as a real window.
//!
//! Event names align with the `ui@1` event vocabulary where a variant exists
//! (RFC-0052 §2: `click` / `select` / `closed`): [`WindowEvent::Closed`] is
//! the `closed` variant and [`WindowEvent::Clicked`] carries the client point
//! that a `click(ui-handle)` event would be built from. The `click`/`select`
//! **variants exist** in the contract, but a click is captured as client
//! **coordinates** here because mapping a point to the UI node under it needs
//! the renderer layout (next slice). [`WindowEvent::Resized`] and
//! [`WindowEvent::Paint`] are window-lifecycle notifications with **no**
//! `ui@1` variant; they drive a re-render and never cross the guest boundary
//! as events.
//!
//! This slice therefore claims no guest-visible `click(handle)` delivery — it
//! delivers only the coordinate event, honestly (STEP-0585 makes the skip of
//! an unmappable coordinate click observable on the runner side).

/// Maximum number of undelivered events one window queue holds.
///
/// The queue is pre-allocated to this cap and a push is rejected (and
/// reported via the overflow flag) before any new allocation, so a limit
/// overrun is a typed overflow signal, never a silently dropped event and
/// never an unbounded buffer.
pub const MAX_QUEUE_EVENTS: usize = 256;

/// The semantic content of one captured OS window message.
///
/// The Win32 `WndProc` reduces each interesting `WM_*` message into exactly
/// one of these; the synthetic event corpus feeds the same values, so the
/// message→event mapping is the single source of truth for both.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawMessage {
    /// The user asked to close the window (`WM_CLOSE`).
    Close,
    /// The client area was resized (`WM_SIZE`), in 96-DPI logical pixels.
    Resize {
        /// New client width.
        width: u32,
        /// New client height.
        height: u32,
    },
    /// The client area needs repainting (`WM_PAINT`).
    Paint,
    /// A primary pointer button was pressed (`WM_LBUTTONDOWN`).
    LeftButtonDown {
        /// Client x coordinate.
        x: i32,
        /// Client y coordinate.
        y: i32,
    },
}

/// One delivered window event.
///
/// `Closed` and `Clicked` mirror the `ui@1` `closed` and `click` variants;
/// `Resized` and `Paint` are window-lifecycle notifications with no guest
/// variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowEvent {
    /// The window was closed; terminates the app deterministically (`closed`).
    Closed,
    /// The client area was resized to the given size.
    Resized {
        /// New client width.
        width: u32,
        /// New client height.
        height: u32,
    },
    /// The client area needs repainting.
    Paint,
    /// A primary pointer press at client coordinates (`click` payload
    /// position; the handle mapping is the next slice).
    Clicked {
        /// Client x coordinate.
        x: i32,
        /// Client y coordinate.
        y: i32,
    },
}

/// Maps one raw OS message to the delivered event.
///
/// Returns `None` for a message with no meaning to deliver: a degenerate
/// resize (`width` or `height` is `0`, e.g. a minimized window) and a pointer
/// press outside the client area are filtered instead of delivered, so the
/// event sequence stays meaningful.
///
/// # Errors
///
/// Never errors; the `Option` is the filter, not a failure.
#[must_use]
pub fn translate(message: RawMessage) -> Option<WindowEvent> {
    match message {
        RawMessage::Close => Some(WindowEvent::Closed),
        RawMessage::Resize { width, height } => {
            if width == 0 || height == 0 {
                None
            } else {
                Some(WindowEvent::Resized { width, height })
            }
        }
        RawMessage::Paint => Some(WindowEvent::Paint),
        RawMessage::LeftButtonDown { x, y } => {
            if x < 0 || y < 0 {
                None
            } else {
                Some(WindowEvent::Clicked { x, y })
            }
        }
    }
}

/// The canonical SHA-256 digest of an event sequence.
///
/// The serialization is a fixed, versioned wire form (a tag byte per event
/// plus little-endian payloads). It is the identity of the frozen event
/// corpus: the same delivered sequence always hashes to the same value on
/// every process, which is what makes the corpus a cross-process check.
#[must_use]
pub fn event_sequence_digest(events: &[WindowEvent]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serialize(events));
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn serialize(events: &[WindowEvent]) -> Vec<u8> {
    let mut out = Vec::new();
    for event in events {
        match event {
            WindowEvent::Closed => out.push(0),
            WindowEvent::Resized { width, height } => {
                out.push(1);
                out.extend_from_slice(&width.to_le_bytes());
                out.extend_from_slice(&height.to_le_bytes());
            }
            WindowEvent::Paint => out.push(2),
            WindowEvent::Clicked { x, y } => {
                out.push(3);
                out.extend_from_slice(&x.to_le_bytes());
                out.extend_from_slice(&y.to_le_bytes());
            }
        }
    }
    out
}

use sha2::{Digest, Sha256};
use std::fmt::Write as _;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_filters_degenerate_messages() {
        assert_eq!(translate(RawMessage::Close), Some(WindowEvent::Closed));
        assert_eq!(
            translate(RawMessage::Resize {
                width: 320,
                height: 200
            }),
            Some(WindowEvent::Resized {
                width: 320,
                height: 200
            })
        );
        // A minimized window has a 0-height client; not delivered.
        assert_eq!(
            translate(RawMessage::Resize {
                width: 320,
                height: 0
            }),
            None
        );
        // A pointer press outside the client is not delivered.
        assert_eq!(translate(RawMessage::LeftButtonDown { x: -1, y: 5 }), None);
        assert_eq!(
            translate(RawMessage::LeftButtonDown { x: 10, y: 20 }),
            Some(WindowEvent::Clicked { x: 10, y: 20 })
        );
    }

    #[test]
    fn the_same_sequence_hashes_the_same_way_twice() {
        let events = [
            WindowEvent::Closed,
            WindowEvent::Resized {
                width: 320,
                height: 200,
            },
            WindowEvent::Paint,
            WindowEvent::Clicked { x: 4, y: 8 },
        ];
        assert_eq!(
            event_sequence_digest(&events),
            event_sequence_digest(&events)
        );
    }
}
