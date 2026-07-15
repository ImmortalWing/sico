//! Typed, bounded mobile bridge over the shared Desktop/Mobile Host core.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};

use serde::{Deserialize, Serialize};
use sico_host_core::{HostStore, InstallOptions};

pub const BRIDGE_SCHEMA: &str = "sico.android.bridge.v0";
pub const MAX_BRIDGE_BYTES: usize = 64 * 1024;
pub const MAX_REQUEST_ID_BYTES: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeRequest {
    pub schema: String,
    pub request_id: String,
    pub operation: BridgeOperation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum BridgeOperation {
    Probe,
    Install,
    Open {
        app_identity: String,
        revision_digest: String,
        host_grants: BTreeSet<String>,
    },
    Uninstall {
        app_identity: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeResponse {
    pub schema: String,
    pub request_id: String,
    pub status: ResponseStatus,
    pub code: String,
    pub data: serde_json::Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResponseStatus {
    Ok,
    Error,
}

pub struct MobileHostCore {
    store: HostStore,
    trusted_keys: BTreeSet<[u8; 32]>,
}

impl MobileHostCore {
    /// Opens the shared Host store used by the mobile bridge.
    ///
    /// # Errors
    ///
    /// Returns shared Host path and safety failures.
    pub fn open(
        root: &Path,
        trusted_keys: BTreeSet<[u8; 32]>,
    ) -> Result<Self, sico_host_core::HostError> {
        Ok(Self {
            store: HostStore::open(root)?,
            trusted_keys,
        })
    }

    /// Handles one typed envelope plus optional separately-owned package bytes.
    /// Panics are caught before they can cross the platform bridge.
    #[must_use]
    pub fn dispatch(&self, envelope: &[u8], package: Option<&[u8]>) -> Vec<u8> {
        let request_id = extract_request_id(envelope).unwrap_or_default();
        let response = catch_unwind(AssertUnwindSafe(|| self.handle(envelope, package)))
            .unwrap_or_else(|_| error(&request_id, "NATIVE_PANIC", "native bridge panicked"));
        encode_response(&response)
    }

    fn handle(&self, envelope: &[u8], package: Option<&[u8]>) -> BridgeResponse {
        if envelope.len() > MAX_BRIDGE_BYTES {
            return error("", "REQUEST_TOO_LARGE", "bridge envelope exceeds 64 KiB");
        }
        let request: BridgeRequest = match serde_json::from_slice(envelope) {
            Ok(request) => request,
            Err(_) => return error("", "INVALID_REQUEST", "invalid bridge envelope"),
        };
        if request.schema != BRIDGE_SCHEMA || !valid_request_id(&request.request_id) {
            return error(
                &request.request_id,
                "INVALID_REQUEST",
                "unknown schema or request id",
            );
        }
        let result = match request.operation {
            BridgeOperation::Probe => Ok(serde_json::json!({
                "bridge": BRIDGE_SCHEMA,
                "trust_owner": "sico-host-core",
                "package_transport": "separate-bytes"
            })),
            BridgeOperation::Install => package.map_or_else(
                || Err("PACKAGE_REQUIRED"),
                |bytes| {
                    self.store
                        .install_bytes(bytes, &self.trusted_keys, InstallOptions::default())
                        .map(|installed| {
                            serde_json::to_value(installed).expect("metadata serializes")
                        })
                        .map_err(|_| "INSTALL_REJECTED")
                },
            ),
            BridgeOperation::Open {
                app_identity,
                revision_digest,
                host_grants,
            } => self
                .store
                .open_installed(
                    &app_identity,
                    &revision_digest,
                    &self.trusted_keys,
                    &host_grants,
                )
                .map(|opened| serde_json::to_value(opened.metadata).expect("metadata serializes"))
                .map_err(|_| "OPEN_REJECTED"),
            BridgeOperation::Uninstall { app_identity } => self
                .store
                .uninstall(&app_identity)
                .map(|removed| serde_json::json!({"removed": removed}))
                .map_err(|_| "UNINSTALL_REJECTED"),
        };
        match result {
            Ok(data) => ok(&request.request_id, data),
            Err(code) => error(&request.request_id, code, "operation rejected"),
        }
    }
}

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REQUEST_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn extract_request_id(envelope: &[u8]) -> Option<String> {
    if envelope.len() > MAX_BRIDGE_BYTES {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(envelope).ok()?;
    value.get("request_id")?.as_str().map(str::to_owned)
}

fn ok(request_id: &str, data: serde_json::Value) -> BridgeResponse {
    BridgeResponse {
        schema: BRIDGE_SCHEMA.to_owned(),
        request_id: request_id.to_owned(),
        status: ResponseStatus::Ok,
        code: "OK".to_owned(),
        data,
    }
}

fn error(request_id: &str, code: &str, message: &str) -> BridgeResponse {
    BridgeResponse {
        schema: BRIDGE_SCHEMA.to_owned(),
        request_id: request_id.to_owned(),
        status: ResponseStatus::Error,
        code: code.to_owned(),
        data: serde_json::json!({"message": message}),
    }
}

fn encode_response(response: &BridgeResponse) -> Vec<u8> {
    serde_json::to_vec(&response).unwrap_or_else(|_| {
        br#"{"schema":"sico.android.bridge.v0","request_id":"","status":"error","code":"ENCODE_ERROR","data":{}}"#.to_vec()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_is_mapped_to_stable_response() {
        let response = catch_unwind(AssertUnwindSafe(|| -> BridgeResponse { panic!("test") }))
            .unwrap_or_else(|_| error("panic-test", "NATIVE_PANIC", "native bridge panicked"));
        assert_eq!(response.code, "NATIVE_PANIC");
        assert_eq!(response.request_id, "panic-test");
    }
}
