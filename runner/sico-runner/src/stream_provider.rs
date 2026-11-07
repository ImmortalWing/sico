//! Typed stream-provider transport seam (RFC-0030, STEP-0587).
//!
//! The runner's `sico:script/streams@0.1.0` host provider moves bytes to and
//! from the process stdio on bounded, cancel-aware worker threads. This module
//! exposes the narrow typed interface that provider implements — a
//! **dependency-injection seam**, not a production switch:
//!
//! - `Runner::new()` (and every existing constructor) is unchanged and always
//!   uses the real stdio transport through [`crate::RealStreamTransportFactory`];
//! - a caller may substitute a typed implementation through
//!   [`crate::Runner::with_stream_transport`]; the default construction and
//!   behavior of the run path are unaffected;
//! - a failing implementation lives entirely **outside** the production crate
//!   (the test), so no fault branch is ever compiled into the provider path.
//!
//! The seam lets a test inject a deterministic provider fault (a typed
//! [`crate::HostStreamError::Io`] from `write_output`) and prove the provider-
//! crash isolation property: the host process survives, the Store is reusable,
//! and the failure surfaces as the typed [`crate::RunOutcome::HostProviderFailure`].

use crate::{CancelToken, HostStreamError};

/// Per-run stream provider transport (one instance per Store).
///
/// Only `Send` is required (not `Sync`): the transport is owned by exactly one
/// Store, which lives on the single guest-worker thread, so `&self` methods
/// are never called concurrently.
pub trait StreamTransport: Send + 'static {
    /// Bounded, cancel-aware stdin read of at most `max` bytes.
    fn read_stdin(&self, max: u64, cancel: &CancelToken) -> Result<Vec<u8>, HostStreamError>;

    /// One bounded output write on the selected channel.
    fn write_output(
        &self,
        stderr: bool,
        bytes: Vec<u8>,
        cancel: &CancelToken,
    ) -> Result<(), HostStreamError>;

    /// Blocking flush of one channel.
    fn flush_output(&self, stderr: bool, cancel: &CancelToken) -> Result<(), HostStreamError>;

    /// Best-effort non-blocking flush (stream drop/close); failures are
    /// deliberately ignored (RFC-0030 close semantics).
    fn try_flush_output(&self, stderr: bool);
}

/// Factory producing one transport per Store — the seam's injection point.
///
/// The default is [`crate::RealStreamTransportFactory`]; a test supplies its
/// own failing implementation to [`crate::Runner::with_stream_transport`].
///
/// `Sync` is required (not just `Send`) because the factory is held behind an
/// `Arc` inside the [`crate::Runner`], which is cloned and moved into the
/// guest-worker thread; the spawned transport itself only needs `Send` because
/// each Store owns exactly one transport on one thread.
pub trait StreamTransportFactory: Send + Sync + 'static {
    /// Spawns one transport for one run/Store.
    fn spawn_transport(&self) -> Box<dyn StreamTransport>;
}
