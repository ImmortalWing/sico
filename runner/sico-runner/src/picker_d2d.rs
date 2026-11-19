//! The RFC-0055 file-dialog adapter behind the runner's guest-visible
//! `sico:script/path-picker@1.0.0` seam (STEP-0627).
//!
//! The two accepted contracts split ownership cleanly:
//!
//! - RFC-0055 §2 owns the **guest-visible surface**: the closed `open`/`save`
//!   functions, the typed `picker-error` refusals and the default-deny
//!   authority gate. The runner owns all of that ([`crate::picker`]).
//! - ADR-0018 owns the **platform dialog**: the Windows IFileOpenDialog /
//!   IFileSaveDialog STA thread. That lives in the root crate
//!   `sico-ui-renderer-d2d` (STEP-0627 extracted the adapter from the
//!   audit-refused `sico-ui-d2d`; the renderer crate is the only crate
//!   allowed to hold platform `unsafe` code).
//!
//! They meet exactly once, here. This runner-side wrapper implements the
//! runner's [`PathPicker`] seam over the adapter and maps the adapter's typed
//! refusals onto the runner's closed [`PickerError`] set. The wrapper grants
//! nothing: authority stays with [`crate::picker::PickerGrants`].

use sico_ui_renderer_d2d::picker::{
    PickerBackend, PickerError as AdapterError, PickerFilter as AdapterFilter,
};

use crate::picker::{PathPicker, PickerError, PickerFilter};

/// Maps one adapter refusal onto the runner's closed [`PickerError`] set.
///
/// The adapter cannot produce `AuthorityDenied`: that stays behind the
/// runner's fail-closed gate in [`crate::picker`].
#[must_use]
pub fn map_adapter_error(error: AdapterError) -> PickerError {
    match error {
        AdapterError::Cancelled => PickerError::Cancelled,
        AdapterError::NoWindow => PickerError::NoWindow,
        AdapterError::Limit => PickerError::Limit,
    }
}

/// Converts runner-side filters into the adapter's shape (same fields, but the
/// adapter type lives behind the `sico-ui-renderer-d2d` boundary).
fn to_adapter_filters(filters: &[PickerFilter]) -> Vec<AdapterFilter> {
    filters
        .iter()
        .map(|filter| AdapterFilter {
            name: filter.name.clone(),
            pattern: filter.pattern.clone(),
        })
        .collect()
}

/// The production [`PathPicker`] backend: the native Windows file dialogs,
/// reachable from the guest through the `path-picker@1.0.0` seam.
pub struct NativeFilePicker {
    inner: sico_ui_renderer_d2d::picker::NativePicker,
}

impl NativeFilePicker {
    /// Builds a picker over the native Windows dialogs.
    ///
    /// # Errors
    ///
    /// [`PickerError::NoWindow`] when the platform has no usable file-dialog
    /// stack — including every non-Windows host, where ADR-0018 §Consequences
    /// leaves v0 unimplemented. Nothing is faked in that case.
    pub fn native() -> Result<Self, PickerError> {
        Ok(Self {
            inner: sico_ui_renderer_d2d::picker::native_picker().map_err(map_adapter_error)?,
        })
    }
}

impl PathPicker for NativeFilePicker {
    fn open(&self, title: &str, filters: &[PickerFilter]) -> Result<String, PickerError> {
        self.inner
            .open(title, &to_adapter_filters(filters))
            .map_err(map_adapter_error)
    }

    fn save(
        &self,
        title: &str,
        suggested_name: &str,
        filters: &[PickerFilter],
    ) -> Result<String, PickerError> {
        self.inner
            .save(title, suggested_name, &to_adapter_filters(filters))
            .map_err(map_adapter_error)
    }
}
