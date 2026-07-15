use std::io::Read;

use serde::{Deserialize, Serialize};
use sico_package::MAX_PACKAGE_BYTES;

const SAPP_MIME: &str = "application/vnd.sico.sapp";
const MAX_URI_BYTES: usize = 4_096;
const MAX_DISPLAY_NAME_BYTES: usize = 255;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AndroidIntentAction {
    View,
    Send,
    PickerResult,
    DeepLink,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidIntent {
    pub action: AndroidIntentAction,
    pub uri: Option<String>,
    pub mime_type: Option<String>,
    pub display_name: Option<String>,
    pub clip_items: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentDecision {
    CopyContentUri(String),
    OpenSystemPicker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentError {
    MissingUri,
    InvalidUri,
    InvalidMime,
    InvalidName,
    MultipleItems,
    StreamTooLarge,
    StreamRead,
}

impl std::fmt::Display for IntentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Android package intent rejected: {self:?}")
    }
}

impl std::error::Error for IntentError {}

/// Validates one Android package/open/share/deep-link event without opening it.
///
/// # Errors
///
/// Rejects non-content external URIs, MIME/name spoofing, missing data and
/// multi-item `ClipData`.
pub fn validate_intent(intent: &AndroidIntent) -> Result<IntentDecision, IntentError> {
    if intent.clip_items > 1 {
        return Err(IntentError::MultipleItems);
    }
    if intent.action == AndroidIntentAction::DeepLink {
        return match intent.uri.as_deref() {
            Some("sico://open") if intent.clip_items == 0 => Ok(IntentDecision::OpenSystemPicker),
            _ => Err(IntentError::InvalidUri),
        };
    }
    if intent.clip_items != 1 {
        return Err(IntentError::MissingUri);
    }
    let uri = intent.uri.as_deref().ok_or(IntentError::MissingUri)?;
    if uri.len() > MAX_URI_BYTES || !uri.starts_with("content://") {
        return Err(IntentError::InvalidUri);
    }
    if intent.mime_type.as_deref() != Some(SAPP_MIME) {
        return Err(IntentError::InvalidMime);
    }
    let name = intent
        .display_name
        .as_deref()
        .ok_or(IntentError::InvalidName)?;
    if name.is_empty()
        || name.len() > MAX_DISPLAY_NAME_BYTES
        || !name.to_ascii_lowercase().ends_with(".sapp")
        || name.chars().any(char::is_control)
    {
        return Err(IntentError::InvalidName);
    }
    Ok(IntentDecision::CopyContentUri(uri.to_owned()))
}

/// Copies a provider stream once into a bounded owned byte vector.
///
/// # Errors
///
/// Rejects provider I/O failures and the first byte beyond the package limit.
pub fn copy_package_stream(reader: &mut impl Read) -> Result<Vec<u8>, IntentError> {
    let mut bytes = Vec::new();
    reader
        .take(64_u64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| IntentError::StreamRead)?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(IntentError::StreamTooLarge);
    }
    Ok(bytes)
}
