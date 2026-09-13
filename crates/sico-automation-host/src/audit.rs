//! Host-owned, append-only audit stream (RFC-0040 `audit`; threat T6).
//!
//! Guests can neither append nor read this stream. Every event is
//! redacted by construction: action class, surface identity, observation
//! revision, action digest, logical timestamps — never pixel or keystroke
//! content (threat T2/T5). The JSONL form is the v0 export format
//! (threat model F-2 ruling); retention is the embedding Host's policy
//! (keep-last-N-runs) and never guest-visible.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    /// Logical tick of the event.
    pub tick: u64,
    /// Event class: `capture`, `commit`, or `stop`.
    pub class: &'static str,
    /// Surface identity token (truncated rendering is the renderer's job).
    pub surface: String,
    /// Observation revision at the event, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    /// Action digest (sha256, hex) — binds the audit line to the exact
    /// committed action without revealing it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Coarse action class: move/drag versus press/release.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_click: Option<bool>,
    /// Monotonic capture sequence for the surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_seq: Option<u64>,
}

impl AuditEvent {
    pub(crate) fn capture(tick: u64, surface: &str, revision: u64, frame_seq: u64) -> Self {
        Self {
            tick,
            class: "capture",
            surface: surface.to_owned(),
            revision: Some(revision),
            digest: None,
            move_only: None,
            has_click: None,
            frame_seq: Some(frame_seq),
        }
    }

    pub(crate) fn commit(
        tick: u64,
        surface: &str,
        revision: u64,
        digest: &str,
        move_only: bool,
        has_click: bool,
    ) -> Self {
        Self {
            tick,
            class: "commit",
            surface: surface.to_owned(),
            revision: Some(revision),
            digest: Some(digest.to_owned()),
            move_only: Some(move_only),
            has_click: Some(has_click),
            frame_seq: None,
        }
    }

    pub(crate) fn stop(tick: u64) -> Self {
        Self {
            tick,
            class: "stop",
            surface: String::new(),
            revision: None,
            digest: None,
            move_only: None,
            has_click: None,
            frame_seq: None,
        }
    }

    /// The JSONL line for operator tooling (F-2 export format).
    #[must_use]
    pub fn jsonl(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"class\":\"render-failure\"}".into())
    }
}
