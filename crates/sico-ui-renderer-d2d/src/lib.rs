//! The M24 §8.6.1 native Fluent-subset Direct2D renderer adapter.
//!
//! The crate has two halves:
//!
//! - the offscreen, headless Direct2D renderer (STEP-0338): the UI tree plus
//!   the frozen token table produce one deterministic frame, hashed
//!   byte-stably in `tests/frame_corpus.rs`;
//! - the OS window shell and event pump (STEP-0577): [`shell`] registers a
//!   window class, creates an `HWND`, pumps the message queue and presents
//!   frames through a GDI DIB, and [`events`] is the safe, platform-neutral
//!   message→event mapping it feeds.
//!
//! Platform split: the unsafe Win32 FFI lives in exactly two modules — the
//! renderer module and the shell module — and every other module in the crate
//! stays unsafe-free. On a non-Windows host the renderer and the shell refuse
//! with the typed [`UiBackendError::UnsupportedPlatform`] instead of
//! pretending to work (ADR-0018 §1/§2).
//!
//! Scope honesty: this crate exposes **no guest-visible UI capability**. The
//! runner-side `ui@1` wiring is a later slice; here a click is captured as
//! client coordinates and no `click(handle)` is delivered. No M24 §3 gate is
//! flipped by this crate.

pub mod events;
pub mod layout;
pub mod shell;
pub mod tokens;
pub mod tree;
pub mod windowed;

mod backend;
mod error;
mod frame;
mod render;

pub use backend::D2dUiBackend;
pub use error::UiBackendError;
pub use frame::Frame;
pub use layout::{Hit, Layout, Rect};
pub use windowed::WindowedUiBackend;
