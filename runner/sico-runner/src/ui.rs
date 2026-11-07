//! The guest-visible `sico:script/ui@1.0.0` host-provider binding (RFC-0052
//! §2/§3 as re-homed by [Amendment A1], paired with ADR-0018).
//!
//! This module is the #5 **binding / authority-refusal layer**: it registers
//! the closed v0 UI surface into the component [`Linker`](wasmtime::component::Linker)
//! and gates *every* call on the default-deny UI capability before any
//! renderer effect. The granted window/frame/event behaviour is delegated to a
//! [`UiRenderer`] backend — the ADR-0018 isolated renderer adapter (#4), which
//! is **not** available on this host (the Direct2D/DirectWrite offline-binding
//! external door recorded in STEP-0330 §5). Consequently:
//!
//! - with no backend present, `ui@1` is not advertised at all (the runner does
//!   not call [`link_ui`]), so a guest importing it fails to instantiate — an
//!   honest "UI host backend unavailable" launch failure, never a faked frame;
//! - with a backend present but no grant, every `result`-returning call yields
//!   the typed `authority-denied` discriminant and `poll-event` yields `none`,
//!   and the renderer is never touched.
//!
//! Error texts never cross into the guest: every refusal is one typed
//! `ui-error` discriminant. Real render-frame evidence stays gated on #4.
//!
//! [Amendment A1]: ../../../docs/rfc/RFC-0052-amendment-A1-host-provider-ui-surface.md

use std::sync::Arc;

use sico_ui_renderer_d2d::Hit;
use sico_ui_renderer_d2d::UiBackendError;
use sico_ui_renderer_d2d::events::WindowEvent;
use sico_ui_renderer_d2d::layout::compute as layout_compute;
use sico_ui_renderer_d2d::tree::UiTree;
use sico_ui_renderer_d2d::windowed::WindowedUiBackend;
use wasmtime::component::{ComponentType, Lift, Lower};

use crate::RunState;

/// The versioned host-provider import this module links (RFC-0052 A1 §2).
pub const UI_INTERFACE: &str = "sico:script/ui@1.0.0";

/// RFC-0052 §3 typed refusal set, in WIT declaration order (append-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ComponentType, Lower)]
#[component(enum)]
#[repr(u8)]
pub enum UiError {
    /// The handle does not name a live node.
    #[component(name = "unknown-handle")]
    UnknownHandle,
    /// The handle names a node that cannot parent the requested child.
    #[component(name = "wrong-parent-kind")]
    WrongParentKind,
    /// A declared node-count / tree-depth / queue budget limit+1 case.
    #[component(name = "limit")]
    Limit,
    /// An operation that needs a window ran with none open.
    #[component(name = "no-window")]
    NoWindow,
    /// The event queue is at its declared cap (events coalesce, never drop).
    #[component(name = "event-overflow")]
    EventOverflow,
    /// The default-deny UI capability was not granted (ADR-0018 §4).
    #[component(name = "authority-denied")]
    AuthorityDenied,
}

/// One opaque UI node handle (RFC-0052 §2 `record UiHandle { id: U64 }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ComponentType, Lift, Lower)]
#[component(record)]
pub struct UiHandle {
    /// The Host-assigned node identity; opaque to the guest.
    #[component(name = "id")]
    pub id: u64,
}

/// The payload of a `select` event (RFC-0052 §2 `select(handle, index)`).
/// Wasmtime's `variant` derive permits at most one unnamed field per case, so
/// the two case payloads are packaged into this record — an ABI realization
/// detail; the event still carries exactly the handle and the selected index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ComponentType, Lower)]
#[component(record)]
pub struct UiSelection {
    /// The dropdown node that changed.
    #[component(name = "handle")]
    pub handle: UiHandle,
    /// The zero-based selected option index.
    #[component(name = "index")]
    pub index: u64,
}

/// One delivered UI event (RFC-0052 §2 `poll-event` variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ComponentType, Lower)]
#[component(variant)]
pub enum UiEvent {
    /// A pointer click on `handle`.
    #[component(name = "click")]
    Click(UiHandle),
    /// A dropdown selection carrying the handle and the selected index.
    #[component(name = "select")]
    Select(UiSelection),
    /// The window closed; terminates the app deterministically.
    #[component(name = "closed")]
    Closed,
}

/// The default-deny UI capability grant (ADR-0018 §4, RFC-0052 A1 §2).
///
/// The default (`window: false`) means the capability is not granted and every
/// call fails closed with `authority-denied`. It composes no capture, input,
/// clipboard, credential or process authority; file open/save stay separate
/// least-privilege file capabilities.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiGrants {
    /// True only when the UI capability is explicitly granted for this run.
    pub window: bool,
}

impl UiGrants {
    /// True when the UI capability is granted (calls may reach the backend).
    #[must_use]
    pub fn allows(&self) -> bool {
        self.window
    }
}

/// The ADR-0018 renderer-backend seam (#4): the granted window/frame/event
/// behaviour the binding delegates to. The production implementation is the
/// isolated Direct2D/DirectWrite adapter crate, which is externally gated on
/// this host; a deterministic test double exercises the binding/authority
/// wiring without claiming any real frame support.
pub trait UiRenderer: Send + Sync {
    /// Opens a top-level window; returns its node handle.
    fn window(&self, title: &str, width: i64, height: i64) -> Result<u64, UiError>;
    /// Adds a column layout panel under `parent`.
    fn column(&self, parent: u64) -> Result<u64, UiError>;
    /// Adds a row layout panel under `parent`.
    fn row(&self, parent: u64) -> Result<u64, UiError>;
    /// Adds a text node under `parent`.
    fn text(&self, parent: u64, content: &str) -> Result<u64, UiError>;
    /// Adds a button under `parent`.
    fn button(&self, parent: u64, label: &str) -> Result<u64, UiError>;
    /// Adds a dropdown with `options` under `parent`.
    fn dropdown(&self, parent: u64, options: &[String]) -> Result<u64, UiError>;
    /// Adds an image-preview node of `width`×`height` under `parent`.
    fn image_preview(&self, parent: u64, width: u64, height: u64) -> Result<u64, UiError>;
    /// Adds a progress node under `parent`.
    fn progress(&self, parent: u64) -> Result<u64, UiError>;
    /// Replaces a text node's content.
    fn set_text(&self, handle: u64, content: &str) -> Result<bool, UiError>;
    /// Sets a progress node's `done`/`of` position.
    fn set_progress(&self, handle: u64, done: u64, of: u64) -> Result<bool, UiError>;
    /// Blits an RFC-0041 BGRA8 buffer into an image-preview node.
    fn draw_image(
        &self,
        handle: u64,
        width: u64,
        height: u64,
        pixels: &[u8],
    ) -> Result<bool, UiError>;
    /// Pumps the OS event queue once; `None` when no event is pending.
    fn poll_event(&self) -> Option<UiEvent>;
    /// Presents one frame (a pure function of UI state, ADR-0018 §3).
    fn run(&self) -> Result<bool, UiError>;
}

/// The production [`UiRenderer`] backend: the isolated Direct2D renderer core
/// (STEP-0338) composed with the real OS window shell and event pump
/// (STEP-0577) through [`WindowedUiBackend`]. The runner holds it behind the
/// default-deny [`guarded`] gate, so it is **never touched without a grant**;
/// authority is not part of this type (ADR-0018 §4).
///
/// Error mapping: the renderer's [`UiBackendError`] is folded into the closed
/// guest `ui-error` set. The v0 `ui-error` enum has **no** render-failure
/// discriminant (RFC-0052 §3), so a window-creation / present / unsupported
/// failure collapses to the typed `no-window` refusal and an invalid argument
/// collapses to `limit` — never a panic, never a silently swallowed failure.
/// This collapse is the only honest option in the closed set; a dedicated
/// `render-failed` discriminant would need a WIT append (out of scope here).
pub struct D2dUiRenderer {
    backend: WindowedUiBackend,
    /// Number of coordinate clicks skipped because no node handle could be
    /// resolved (STEP-0585). This is the host-side observable for the honest
    /// "click happened but cannot map to a handle" gap: a skipped click is
    /// distinguishable from "no event was pending", even though the closed
    /// `ui@1` contract cannot carry it as a typed refusal.
    skipped_click_count: std::sync::atomic::AtomicU64,
}

impl D2dUiRenderer {
    /// Builds the production Direct2D + window-shell backend.
    ///
    /// On a non-Windows host the renderer refuses (`UnsupportedPlatform`),
    /// folded to the typed `no-window`; the runner must then fall back to not
    /// advertising `ui@1` at all (an honest "UI backend unavailable").
    ///
    /// # Errors
    ///
    /// Returns [`UiError`] when the platform cannot provide a UI backend.
    pub fn new() -> Result<Self, UiError> {
        WindowedUiBackend::new()
            .map(|backend| Self {
                backend,
                skipped_click_count: std::sync::atomic::AtomicU64::new(0),
            })
            .map_err(fold_error)
    }

    /// The title of the open window, if a real window was created.
    ///
    /// This is the host-side audit affordance the e2e tests use to prove the
    /// guest's `window` call reached a real OS window; it is **not** a guest
    /// contract member.
    #[must_use]
    pub fn window_title(&self) -> Option<String> {
        self.backend.window_title()
    }

    /// The number of coordinate clicks that could not be resolved to a node
    /// handle and were therefore honestly skipped (STEP-0585).
    ///
    /// This is the host-side observable for the "click happened but cannot map
    /// to a handle" gap: a real click that was ignored increments this counter,
    /// so a caller (test, audit) can distinguish it from "no event was pending"
    /// (`poll_event` returned `None` with a zero count). It is **not** a guest
    /// contract member.
    #[must_use]
    pub fn skipped_click_count(&self) -> u64 {
        self.skipped_click_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Posts a `WM_CLOSE` so the window closes deterministically.
    ///
    /// Host-side lifecycle affordance (not guest-visible); the guest's app
    /// loop observes the resulting `closed` event through `poll-event`.
    ///
    /// # Errors
    ///
    /// Returns [`UiError::NoWindow`] when no window exists.
    pub fn close(&self) -> Result<(), UiError> {
        self.backend.close().map_err(fold_error)
    }

    /// Destroys the window; idempotent.
    ///
    /// # Errors
    ///
    /// Returns [`UiError::NoWindow`] when no window exists.
    pub fn destroy(&self) -> Result<(), UiError> {
        self.backend.destroy().map_err(fold_error)
    }
}

/// Folds the renderer's typed failures into the closed guest `ui-error` set.
fn fold_error(error: UiBackendError) -> UiError {
    match error {
        UiBackendError::UnknownHandle => UiError::UnknownHandle,
        UiBackendError::WrongParentKind => UiError::WrongParentKind,
        UiBackendError::Limit { .. } => UiError::Limit,
        // A range violation (e.g. an out-of-range window size) is a declared
        // constraint, folded onto the `limit` refusal.
        UiBackendError::InvalidArgument { .. } => UiError::Limit,
        // A window-creation / present / unsupported-platform failure has no
        // guest discriminant; it is an explicit typed `no-window` refusal
        // (documented collapse, never a panic or a silent swallow).
        UiBackendError::NoWindow
        | UiBackendError::UnsupportedPlatform
        | UiBackendError::RenderFailed { .. } => UiError::NoWindow,
    }
}

/// The honest classification of one pumped [`WindowEvent`] that is **not** a
/// coordinate click (STEP-0597).
///
/// Returns the `ui@1` event to deliver, if any. A `Closed` maps to the
/// `closed` variant. A `Resized`/`Paint` is a window-lifecycle notification
/// with no `ui@1` variant (it drives a re-render and never crosses the
/// boundary). A coordinate `Clicked` is **not** classified here: it needs the
/// layout to resolve a handle, so [`route_click_event`] owns it.
fn classify_window_event(event: WindowEvent) -> Option<UiEvent> {
    match event {
        WindowEvent::Closed => Some(UiEvent::Closed),
        WindowEvent::Clicked { .. } | WindowEvent::Resized { .. } | WindowEvent::Paint => None,
    }
}

/// Routes one client-coordinate click to a `ui@1` event (STEP-0597).
///
/// This is the host-side point→handle hit-testing seam: it lays the tree out
/// at the current client size and hit-tests the point. A click is **never
/// fabricated** into a `click(handle)` — the handle comes from the layout.
///
/// Return value:
/// - `Ok(Some(event))` delivers `click(handle)` for a node hit. The handle is
///   the **deepest** node under the point: a child/leaf resolves to itself, a
///   point in a container but in no child (e.g. the sibling gap) resolves to
///   the container, and a point in the window padding resolves to
///   `Hit::Node(root)`. `Node(root)` is the correct attribution when the point
///   is inside the window but under **no** child node — it is **not** the
///   STEP-0585 "attribute any click to the root" behaviour (which had no
///   layout and so could not distinguish the root from a child).
/// - `Ok(None)` is an explicit [`Hit::Miss`]: the point is outside the window
///   (or no node covers it). The caller counts it as a skipped click.
/// - `Err` is a typed layout failure ([`UiBackendError::NoWindow`] /
///   [`UiBackendError::InvalidArgument`] / a tree refusal). It is never turned
///   into an event.
pub fn route_click_event(
    x: i32,
    y: i32,
    tree: &UiTree,
    width: u32,
    height: u32,
) -> Result<Option<UiEvent>, UiBackendError> {
    let layout = layout_compute(tree, width, height)?;
    Ok(match layout.hit(x, y) {
        Hit::Node(handle) => Some(UiEvent::Click(UiHandle { id: handle })),
        Hit::Miss => None,
    })
}

impl UiRenderer for D2dUiRenderer {
    fn window(&self, title: &str, width: i64, height: i64) -> Result<u64, UiError> {
        self.backend
            .window(title, width, height)
            .map_err(fold_error)
    }

    fn column(&self, parent: u64) -> Result<u64, UiError> {
        self.backend.column(parent).map_err(fold_error)
    }

    fn row(&self, parent: u64) -> Result<u64, UiError> {
        self.backend.row(parent).map_err(fold_error)
    }

    fn text(&self, parent: u64, content: &str) -> Result<u64, UiError> {
        self.backend.text(parent, content).map_err(fold_error)
    }

    fn button(&self, parent: u64, label: &str) -> Result<u64, UiError> {
        self.backend.button(parent, label).map_err(fold_error)
    }

    fn dropdown(&self, parent: u64, options: &[String]) -> Result<u64, UiError> {
        self.backend.dropdown(parent, options).map_err(fold_error)
    }

    fn image_preview(&self, parent: u64, width: u64, height: u64) -> Result<u64, UiError> {
        self.backend
            .image_preview(parent, width, height)
            .map_err(fold_error)
    }

    fn progress(&self, parent: u64) -> Result<u64, UiError> {
        self.backend.progress(parent).map_err(fold_error)
    }

    fn set_text(&self, handle: u64, content: &str) -> Result<bool, UiError> {
        self.backend.set_text(handle, content).map_err(fold_error)
    }

    fn set_progress(&self, handle: u64, done: u64, of: u64) -> Result<bool, UiError> {
        self.backend
            .set_progress(handle, done, of)
            .map_err(fold_error)
    }

    fn draw_image(
        &self,
        handle: u64,
        width: u64,
        height: u64,
        pixels: &[u8],
    ) -> Result<bool, UiError> {
        self.backend
            .draw_image(handle, width, height, pixels)
            .map_err(fold_error)
    }

    fn poll_event(&self) -> Option<UiEvent> {
        // Pump until a deliverable event surfaces (STEP-0585, extended by
        // STEP-0597). `Closed` maps to the `closed` variant. A coordinate
        // `Clicked` is routed through the layout ([`route_click_event`]): a
        // node hit delivers `click(handle)`, a `Hit::Miss` is an honest skip
        // (counted, observable), and a layout failure is a host invariant
        // surfaced as `None` — never a fabricated event. A `Resized`/`Paint`
        // is a window-lifecycle notification with no `ui@1` variant (it drives
        // a re-render and never crosses the boundary). A lost shell thread
        // surfaces as `None` — the only honest signal the `option<ui-event>`
        // contract allows for a pump that cannot run.
        loop {
            match self.backend.poll_event() {
                Ok(Some(event)) => {
                    if let WindowEvent::Clicked { x, y } = event {
                        match self.backend.layout_input() {
                            Ok((width, height, tree)) => {
                                match route_click_event(x, y, &tree, width, height) {
                                    Ok(Some(delivered)) => return Some(delivered),
                                    Ok(None) => {
                                        // `Hit::Miss`: the point is outside the
                                        // window (or no node covers it). Honest
                                        // skip, observable (STEP-0585).
                                        self.skipped_click_count
                                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    }
                                    Err(_) => {
                                        // A layout failure is a host invariant
                                        // (the tree and the live client size
                                        // are always laid out-able in normal
                                        // operation). No event is fabricated;
                                        // the closed `option<ui-event>` has no
                                        // error channel, so the honest signal
                                        // is `none`.
                                        return None;
                                    }
                                }
                            }
                            // No window / a lost shell thread / a poisoned
                            // tree lock: the same honest `none` signal.
                            Err(_) => return None,
                        }
                    } else if let Some(delivered) = classify_window_event(event) {
                        return Some(delivered);
                    }
                }
                Ok(None) | Err(_) => return None,
            }
        }
    }

    fn run(&self) -> Result<bool, UiError> {
        self.backend.run().map_err(fold_error)
    }
}

/// Runner-level UI host capability: the frozen grants plus the optional
/// renderer backend. The default carries no backend, so `ui@1` is not linked.
#[derive(Clone, Default)]
pub struct UiHost {
    grants: UiGrants,
    renderer: Option<Arc<dyn UiRenderer>>,
}

impl UiHost {
    /// Builds a host capability with a renderer backend and its grants.
    #[must_use]
    pub fn with_backend(grants: UiGrants, renderer: Arc<dyn UiRenderer>) -> Self {
        Self {
            grants,
            renderer: Some(renderer),
        }
    }

    /// True when a renderer backend exists (the precondition for linking).
    #[must_use]
    pub(crate) fn has_backend(&self) -> bool {
        self.renderer.is_some()
    }

    /// Builds the per-Store state for one run generation.
    #[must_use]
    pub(crate) fn state(&self) -> Ui1State {
        Ui1State {
            grants: self.grants.clone(),
            renderer: self.renderer.clone(),
        }
    }
}

/// Per-Store UI state: the frozen grants and the shared renderer backend. It
/// dies with the Store; no window, frame or event crosses a run or generation.
pub struct Ui1State {
    grants: UiGrants,
    renderer: Option<Arc<dyn UiRenderer>>,
}

impl Ui1State {
    /// True when the UI capability is granted for this Store.
    #[must_use]
    fn allows(&self) -> bool {
        self.grants.allows()
    }

    /// The renderer backend, when one is linked.
    fn backend(&self) -> Option<Arc<dyn UiRenderer>> {
        self.renderer.clone()
    }
}

type UiOutcome<T> = Result<(Result<T, UiError>,), wasmtime::Error>;

/// Runs `body` against the renderer only when the UI capability is granted and
/// a backend is linked; otherwise returns the typed `authority-denied`. The
/// backend is unreachable without the grant, so a denied call never touches a
/// window, frame or the OS event queue.
fn guarded<T>(
    store: &wasmtime::StoreContextMut<'_, RunState>,
    body: impl FnOnce(&Arc<dyn UiRenderer>) -> Result<T, UiError>,
) -> UiOutcome<T> {
    if !store.data().ui.allows() {
        return Ok((Err(UiError::AuthorityDenied),));
    }
    let Some(renderer) = store.data().ui.backend() else {
        // Unreachable: `link_ui` is only called when a backend exists. Fails
        // closed rather than fabricating a frame.
        return Ok((Err(UiError::AuthorityDenied),));
    };
    Ok((body(&renderer),))
}

/// Wires the default-deny `sico:script/ui@1.0.0` instance. The instance is
/// registered with the full RFC-0052 §2 closed surface; every call re-checks
/// the UI capability and fails closed with a typed discriminant. Only call this
/// when a renderer backend exists ([`UiHost::has_backend`]).
///
/// # Errors
///
/// Linker setup failures only; capability refusals are guest-visible results.
#[allow(clippy::too_many_lines)]
pub(crate) fn link_ui(
    linker: &mut wasmtime::component::Linker<RunState>,
) -> Result<(), wasmtime::Error> {
    let mut ui = linker.instance(UI_INTERFACE)?;

    ui.func_wrap(
        "window",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (title, width, height): (String, i64, i64)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| {
                renderer.window(&title, width, height).map(handle)
            })
        },
    )?;
    ui.func_wrap(
        "column",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (parent,): (UiHandle,)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| renderer.column(parent.id).map(handle))
        },
    )?;
    ui.func_wrap(
        "row",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (parent,): (UiHandle,)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| renderer.row(parent.id).map(handle))
        },
    )?;
    ui.func_wrap(
        "text",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (parent, content): (UiHandle, String)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| {
                renderer.text(parent.id, &content).map(handle)
            })
        },
    )?;
    ui.func_wrap(
        "button",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (parent, label): (UiHandle, String)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| {
                renderer.button(parent.id, &label).map(handle)
            })
        },
    )?;
    ui.func_wrap(
        "dropdown",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (parent, options): (UiHandle, Vec<String>)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| {
                renderer.dropdown(parent.id, &options).map(handle)
            })
        },
    )?;
    ui.func_wrap(
        "image-preview",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (parent, width, height): (UiHandle, u64, u64)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| {
                renderer.image_preview(parent.id, width, height).map(handle)
            })
        },
    )?;
    ui.func_wrap(
        "progress",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (parent,): (UiHandle,)|
         -> UiOutcome<UiHandle> {
            guarded(&store, |renderer| renderer.progress(parent.id).map(handle))
        },
    )?;
    ui.func_wrap(
        "set-text",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (node, content): (UiHandle, String)|
         -> UiOutcome<bool> {
            guarded(&store, |renderer| renderer.set_text(node.id, &content))
        },
    )?;
    ui.func_wrap(
        "set-progress",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (node, done, of): (UiHandle, u64, u64)|
         -> UiOutcome<bool> {
            guarded(&store, |renderer| renderer.set_progress(node.id, done, of))
        },
    )?;
    ui.func_wrap(
        "draw-image",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (node, width, height, pixels): (UiHandle, u64, u64, Vec<u8>)|
         -> UiOutcome<bool> {
            guarded(&store, |renderer| {
                renderer.draw_image(node.id, width, height, &pixels)
            })
        },
    )?;
    ui.func_wrap(
        "poll-event",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (): ()|
         -> Result<(Option<UiEvent>,), wasmtime::Error> {
            if !store.data().ui.allows() {
                return Ok((None,));
            }
            let Some(renderer) = store.data().ui.backend() else {
                return Ok((None,));
            };
            Ok((renderer.poll_event(),))
        },
    )?;
    ui.func_wrap(
        "run",
        |store: wasmtime::StoreContextMut<'_, RunState>, (): ()| -> UiOutcome<bool> {
            guarded(&store, |renderer| renderer.run())
        },
    )?;

    Ok(())
}

fn handle(id: u64) -> UiHandle {
    UiHandle { id }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A deterministic backend double: it records call counts and returns
    /// fixed handles, proving the binding reaches the backend only when
    /// granted. It is **not** a renderer and claims no frame behaviour.
    struct FixtureRenderer {
        calls: AtomicUsize,
    }

    impl FixtureRenderer {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
            }
        }

        fn hit(&self) {
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl UiRenderer for FixtureRenderer {
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
        fn draw_image(
            &self,
            _handle: u64,
            _w: u64,
            _h: u64,
            _pixels: &[u8],
        ) -> Result<bool, UiError> {
            self.hit();
            Ok(true)
        }
        fn poll_event(&self) -> Option<UiEvent> {
            self.hit();
            Some(UiEvent::Closed)
        }
        fn run(&self) -> Result<bool, UiError> {
            self.hit();
            Ok(true)
        }
    }

    #[test]
    fn ui_grants_default_deny() {
        assert!(!UiGrants::default().allows());
        assert!(UiGrants { window: true }.allows());
    }

    #[test]
    fn ui_error_discriminants_match_the_contract_order() {
        // RFC-0052 §3 closed set, in declaration order.
        assert_eq!(UiError::UnknownHandle as u8, 0);
        assert_eq!(UiError::WrongParentKind as u8, 1);
        assert_eq!(UiError::Limit as u8, 2);
        assert_eq!(UiError::NoWindow as u8, 3);
        assert_eq!(UiError::EventOverflow as u8, 4);
        assert_eq!(UiError::AuthorityDenied as u8, 5);
    }

    #[test]
    fn host_without_backend_does_not_advertise_ui() {
        assert!(!UiHost::default().has_backend());
        let host = UiHost::with_backend(UiGrants::default(), Arc::new(FixtureRenderer::new()));
        assert!(host.has_backend());
    }

    #[test]
    fn link_ui_registers_the_full_closed_surface() {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let mut linker = wasmtime::component::Linker::<RunState>::new(&engine);
        link_ui(&mut linker).expect("the ui@1 surface registers without error");
    }

    #[test]
    fn classify_window_event_covers_the_non_click_events() {
        // STEP-0597: a `Closed` is delivered as the `closed` variant; a
        // `Resized`/`Paint` is a lifecycle notification with no `ui@1` variant
        // (it drives a re-render and never crosses the boundary); a coordinate
        // `Clicked` is NOT classified here — it needs the layout and is routed
        // by `route_click_event`.
        assert_eq!(
            classify_window_event(WindowEvent::Closed),
            Some(UiEvent::Closed)
        );
        assert_eq!(
            classify_window_event(WindowEvent::Clicked { x: 4, y: 8 }),
            None
        );
        assert_eq!(
            classify_window_event(WindowEvent::Resized {
                width: 1,
                height: 1,
            }),
            None
        );
        assert_eq!(classify_window_event(WindowEvent::Paint), None);
    }

    #[test]
    fn route_click_event_delivers_the_deepest_node_and_misses_explicitly() {
        use sico_ui_renderer_d2d::tree::NodeKind;

        // window(320x160) > column > button "OK".
        let mut tree = UiTree::new();
        let root = tree.insert(NodeKind::Root, None).unwrap();
        let column = tree.insert(NodeKind::Column, Some(root)).unwrap();
        let button = tree.insert(NodeKind::Button, Some(column)).unwrap();
        tree.set_text(button, "OK").unwrap();
        let layout = layout_compute(&tree, 320, 160).unwrap();

        // A click on the button resolves to the button (the deepest node), not
        // to its column or the root.
        let (cx, cy) = layout.rect(button).unwrap().center();
        assert_eq!(
            route_click_event(cx as i32, cy as i32, &tree, 320, 160),
            Ok(Some(UiEvent::Click(UiHandle { id: button })))
        );

        // A click in the window padding resolves to the root: the point is
        // inside the window but under NO child node. This is the correct
        // `Node(root)` attribution (STEP-0592), NOT the STEP-0585 "attribute
        // any click to the root" (which had no layout).
        assert_eq!(
            route_click_event(8, 8, &tree, 320, 160),
            Ok(Some(UiEvent::Click(UiHandle { id: root })))
        );

        // A click outside the window is an explicit `Hit::Miss` (`Ok(None)`),
        // never a fabricated event and never an error.
        assert_eq!(route_click_event(-1, -1, &tree, 320, 160), Ok(None));
        assert_eq!(route_click_event(400, 400, &tree, 320, 160), Ok(None));

        // A tree without a window root is a typed layout failure, never a
        // fabricated event.
        let empty = UiTree::new();
        assert_eq!(
            route_click_event(0, 0, &empty, 320, 160),
            Err(UiBackendError::NoWindow)
        );
        // A zero client size is a typed layout failure.
        assert_eq!(
            route_click_event(0, 0, &tree, 0, 160),
            Err(UiBackendError::InvalidArgument {
                which: "client-size"
            })
        );
    }
}
