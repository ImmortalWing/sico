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
}
