//! The default-deny `sico:script/path-picker@1.0.0` host-provider binding
//! (STEP-0627, RFC-0055). The picker is a pure path-selection surface: it
//! grants no read/write/capture/input/clipboard/credential/process authority
//! by itself, and it composes no new trust beyond the existing default-deny
//! file capability (RFC-0055 §4).
//!
//! The source-visible `open`/`save` return the flattened
//! `result<string, picker-error>`: the ok path carries the chosen path (an
//! empty string means the user cancelled); the error side carries the typed
//! `picker-error` case name. Zero grammar growth — the source imports the
//! host surface the same way it imports `ui@1` or `http@0.2.0`.

use std::sync::Arc;

use wasmtime::component::{ComponentType, Lower};

use crate::RunState;

/// The versioned host-provider import this module links (RFC-0055 §3).
pub const PICKER_INTERFACE: &str = "sico:script/path-picker@1.0.0";

/// RFC-0055 §3 typed refusal set, in WIT declaration order (append-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ComponentType, Lower)]
#[component(enum)]
#[repr(u8)]
pub enum PickerError {
    /// The user cancelled the dialog (the guest sees an empty ok path).
    #[component(name = "cancelled")]
    Cancelled,
    /// The platform cannot show a modal dialog (no usable window stack).
    #[component(name = "no-window")]
    NoWindow,
    /// A declared picker budget limit+1 case (future filters/roots).
    #[component(name = "limit")]
    Limit,
    /// The default-deny picker capability was not granted (RFC-0055 §4).
    #[component(name = "authority-denied")]
    AuthorityDenied,
}

/// One open/save filter entry: display name + one semicolon-separated
/// extension pattern (`("Text", "*.txt")`). The guest crosses the boundary as
/// the flattened `list<string>` (one `"name|pattern"` string per entry, the
/// STEP-0557 encoding); this record is the runner-internal shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerFilter {
    /// The human-readable filter name shown in the dialog dropdown.
    pub name: String,
    /// The semicolon-separated extension pattern (for example `*.txt`).
    pub pattern: String,
}

/// The default-deny picker capability grant (RFC-0055 §4, ADR-0018 §4).
///
/// The default (`open: false, save: false`) means the capability is not
/// granted and every call fails closed with `authority-denied` before any
/// native dialog is shown. The grant composes no capture, input, clipboard,
/// credential or process authority, and no read/write authority — file
/// authority stays under the existing default-deny `fs-read`/`fs-write`
/// capabilities.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PickerGrants {
    /// True only when the open dialog is explicitly granted for this run.
    pub open: bool,
    /// True only when the save dialog is explicitly granted for this run.
    pub save: bool,
}

impl PickerGrants {
    /// True when the open dialog is granted (calls may reach the backend).
    #[must_use]
    pub fn allows_open(&self) -> bool {
        self.open
    }

    /// True when the save dialog is granted (calls may reach the backend).
    #[must_use]
    pub fn allows_save(&self) -> bool {
        self.save
    }
}

/// The RFC-0055 path-picker backend seam: the granted dialog behaviour the
/// binding delegates to. The production implementation shows a real native
/// file dialog on the dedicated STA dialog thread; a deterministic test double
/// exercises the binding/authority wiring without showing a real dialog.
pub trait PathPicker: Send + Sync {
    /// Shows the open dialog; returns the chosen absolute path, or an empty
    /// string when the user cancelled.
    fn open(&self, title: &str, filters: &[PickerFilter]) -> Result<String, PickerError>;
    /// Shows the save dialog seeded with `suggested_name`; returns the chosen
    /// absolute path, or an empty string when the user cancelled.
    fn save(
        &self,
        title: &str,
        suggested_name: &str,
        filters: &[PickerFilter],
    ) -> Result<String, PickerError>;
}

/// Runner-level picker host capability: the frozen grants plus the optional
/// backend. The default carries no backend, so `path-picker@1.0.0` is not
/// linked and a guest importing it fails to instantiate (an honest "picker
/// backend unavailable" outcome).
#[derive(Clone, Default)]
pub struct PickerHost {
    grants: PickerGrants,
    backend: Option<Arc<dyn PathPicker>>,
}

impl PickerHost {
    /// Builds a host capability with a backend and its grants.
    #[must_use]
    pub fn with_backend(grants: PickerGrants, backend: Arc<dyn PathPicker>) -> Self {
        Self {
            grants,
            backend: Some(backend),
        }
    }

    /// True when a backend exists (the precondition for linking).
    #[must_use]
    pub(crate) fn has_backend(&self) -> bool {
        self.backend.is_some()
    }

    /// Builds the per-Store state for one run generation.
    #[must_use]
    pub(crate) fn state(&self) -> PickerState {
        PickerState {
            grants: self.grants.clone(),
            backend: self.backend.clone(),
        }
    }
}

/// Per-Store picker state: the frozen grants and the shared backend. It dies
/// with the Store; no dialog or chosen path crosses a run or generation.
pub struct PickerState {
    grants: PickerGrants,
    backend: Option<Arc<dyn PathPicker>>,
}

impl PickerState {
    /// True when the open dialog is granted for this Store.
    #[must_use]
    fn allows_open(&self) -> bool {
        self.grants.allows_open()
    }

    /// True when the save dialog is granted for this Store.
    #[must_use]
    fn allows_save(&self) -> bool {
        self.grants.allows_save()
    }

    /// The backend, when one is linked.
    fn backend(&self) -> Option<Arc<dyn PathPicker>> {
        self.backend.clone()
    }
}

type PickerOutcome<T> = Result<(Result<T, PickerError>,), wasmtime::Error>;

/// Runs `body` against the backend only when the matching picker capability is
/// granted and a backend is linked; otherwise returns the typed
/// `authority-denied`. The backend is unreachable without the grant, so a
/// denied call never shows a native dialog.
fn guarded<T>(
    store: &wasmtime::StoreContextMut<'_, RunState>,
    granted: bool,
    body: impl FnOnce(&Arc<dyn PathPicker>) -> Result<T, PickerError>,
) -> PickerOutcome<T> {
    if !granted {
        return Ok((Err(PickerError::AuthorityDenied),));
    }
    let Some(backend) = store.data().picker.backend() else {
        // Unreachable: `link_picker` is only called when a backend exists.
        // Fails closed rather than fabricating a path.
        return Ok((Err(PickerError::AuthorityDenied),));
    };
    Ok((body(&backend),))
}

/// STEP-0557 flattening: `cancelled` is a non-error on the guest-visible
/// surface. A backend signalling cancellation (as the native Win32 dialog
/// does) is surfaced as `ok("")` — never as the `cancelled` error case name,
/// which stays a reserved wire discriminant.
fn flatten_cancelled(result: Result<String, PickerError>) -> Result<String, PickerError> {
    match result {
        Err(PickerError::Cancelled) => Ok(String::new()),
        other => other,
    }
}

/// Wires the default-deny `sico:script/path-picker@1.0.0` instance. The
/// instance is registered with the RFC-0055 §3 closed surface; every call
/// re-checks the matching picker capability and fails closed with a typed
/// discriminant. Only call this when a backend exists
/// ([`PickerHost::has_backend`]).
///
/// # Errors
///
/// Linker setup failures only; capability refusals are guest-visible results.
pub(crate) fn link_picker(
    linker: &mut wasmtime::component::Linker<RunState>,
) -> Result<(), wasmtime::Error> {
    let mut picker = linker.instance(PICKER_INTERFACE)?;

    picker.func_wrap(
        "open",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (title, filters): (String, Vec<String>)|
         -> PickerOutcome<String> {
            let granted = store.data().picker.allows_open();
            let filters: Vec<PickerFilter> = filters.into_iter().map(parse_filter).collect();
            guarded(&store, granted, |backend| {
                flatten_cancelled(backend.open(&title, &filters))
            })
        },
    )?;
    picker.func_wrap(
        "save",
        |store: wasmtime::StoreContextMut<'_, RunState>,
         (title, suggested_name, filters): (String, String, Vec<String>)|
         -> PickerOutcome<String> {
            let granted = store.data().picker.allows_save();
            let filters: Vec<PickerFilter> = filters.into_iter().map(parse_filter).collect();
            guarded(&store, granted, |backend| {
                flatten_cancelled(backend.save(&title, &suggested_name, &filters))
            })
        },
    )?;
    Ok(())
}

/// Parses one flattened `"name|pattern"` filter string (the guest-visible
/// encoding chosen in STEP-0557) into the runner-internal [`PickerFilter`].
/// A malformed entry becomes an empty name with the whole string as the
/// pattern, so a guest bug surfaces at the dialog rather than as a parse
/// refusal — the picker grants no authority and cannot fail the run.
fn parse_filter(entry: String) -> PickerFilter {
    match entry.split_once('|') {
        Some((name, pattern)) => PickerFilter {
            name: name.to_owned(),
            pattern: pattern.to_owned(),
        },
        None => PickerFilter {
            name: String::new(),
            pattern: entry,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_grants_default_deny() {
        assert!(!PickerGrants::default().allows_open());
        assert!(!PickerGrants::default().allows_save());
        assert!(
            PickerGrants {
                open: true,
                save: true,
            }
            .allows_open()
        );
        assert!(
            PickerGrants {
                open: true,
                save: true,
            }
            .allows_save()
        );
    }

    #[test]
    fn picker_error_discriminants_match_the_contract_order() {
        // RFC-0055 §3 closed set, in declaration order.
        assert_eq!(PickerError::Cancelled as u8, 0);
        assert_eq!(PickerError::NoWindow as u8, 1);
        assert_eq!(PickerError::Limit as u8, 2);
        assert_eq!(PickerError::AuthorityDenied as u8, 3);
    }

    #[test]
    fn host_without_backend_does_not_advertise_picker() {
        assert!(!PickerHost::default().has_backend());
        struct FakePicker;
        impl PathPicker for FakePicker {
            fn open(&self, _title: &str, _filters: &[PickerFilter]) -> Result<String, PickerError> {
                Ok(String::new())
            }
            fn save(
                &self,
                _title: &str,
                _suggested_name: &str,
                _filters: &[PickerFilter],
            ) -> Result<String, PickerError> {
                Ok(String::new())
            }
        }
        let host = PickerHost::with_backend(PickerGrants::default(), Arc::new(FakePicker));
        assert!(host.has_backend());
    }

    #[test]
    fn link_picker_registers_the_closed_surface() {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let mut linker = wasmtime::component::Linker::<RunState>::new(&engine);
        link_picker(&mut linker).expect("the path-picker surface registers without error");
    }

    #[test]
    fn cancellation_flattens_to_the_empty_ok_path() {
        // STEP-0557: `cancelled` is never a guest-visible error; a backend
        // signalling it surfaces as the empty ok path (exit 0).
        assert_eq!(
            flatten_cancelled(Err(PickerError::Cancelled)),
            Ok(String::new())
        );
        assert_eq!(
            flatten_cancelled(Ok("C:\\tmp\\chosen.txt".to_owned())),
            Ok("C:\\tmp\\chosen.txt".to_owned())
        );
        assert_eq!(
            flatten_cancelled(Err(PickerError::NoWindow)),
            Err(PickerError::NoWindow)
        );
        assert_eq!(
            flatten_cancelled(Err(PickerError::AuthorityDenied)),
            Err(PickerError::AuthorityDenied)
        );
    }
}
