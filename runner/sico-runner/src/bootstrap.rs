//! Native validation boundary for the M22 canonical bootstrap source bundle.
//!
//! The guest compiler receives data, never host paths. This module is the
//! permanent native decoder required by ADR-0015: ordering, normalized paths,
//! per-file/aggregate limits and SHA-256 are checked before a bundle can enter
//! a compiler generation.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"SICOBND0";

pub const MAX_SOURCE_BUNDLE_FILES: usize = 256;
pub const MAX_SOURCE_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SOURCE_BUNDLE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceBundleEntry {
    pub path: String,
    pub source: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceBundleError {
    BadMagic,
    Empty,
    TooManyFiles,
    FileTooLarge,
    BundleTooLarge,
    Truncated,
    TrailingData,
    InvalidPathUtf8,
    InvalidPath,
    DuplicatePath,
    UnsortedPath,
    DigestMismatch,
    IntegerOverflow,
}

/// Encodes entries that are already in strict normalized path order.
///
/// Refusing unsorted input keeps the caller-visible order authoritative; the
/// encoder never hides nondeterministic traversal by sorting on its behalf.
pub fn encode_source_bundle(entries: &[SourceBundleEntry]) -> Result<Vec<u8>, SourceBundleError> {
    validate_entries(entries)?;
    let mut encoded = Vec::new();
    encoded.extend_from_slice(MAGIC);
    encoded.extend_from_slice(
        &u32::try_from(entries.len())
            .map_err(|_| SourceBundleError::IntegerOverflow)?
            .to_le_bytes(),
    );
    for entry in entries {
        encoded.extend_from_slice(
            &u32::try_from(entry.path.len())
                .map_err(|_| SourceBundleError::IntegerOverflow)?
                .to_le_bytes(),
        );
        encoded.extend_from_slice(
            &u64::try_from(entry.source.len())
                .map_err(|_| SourceBundleError::IntegerOverflow)?
                .to_le_bytes(),
        );
        encoded.extend_from_slice(entry.path.as_bytes());
        encoded.extend_from_slice(&entry.source);
        encoded.extend_from_slice(&Sha256::digest(&entry.source));
    }
    Ok(encoded)
}

/// Decodes the exact ADR-0015 source-bundle form and rejects trailing bytes.
pub fn decode_source_bundle(bytes: &[u8]) -> Result<Vec<SourceBundleEntry>, SourceBundleError> {
    let mut cursor = 0usize;
    let magic = take(bytes, &mut cursor, MAGIC.len())?;
    if magic != MAGIC {
        return Err(SourceBundleError::BadMagic);
    }
    let count = usize::try_from(read_u32(bytes, &mut cursor)?)
        .map_err(|_| SourceBundleError::IntegerOverflow)?;
    if count == 0 {
        return Err(SourceBundleError::Empty);
    }
    if count > MAX_SOURCE_BUNDLE_FILES {
        return Err(SourceBundleError::TooManyFiles);
    }

    let mut entries = Vec::with_capacity(count);
    let mut total = 0usize;
    let mut previous: Option<String> = None;
    let mut seen = BTreeSet::new();
    for _ in 0..count {
        let path_len = usize::try_from(read_u32(bytes, &mut cursor)?)
            .map_err(|_| SourceBundleError::IntegerOverflow)?;
        let source_len = usize::try_from(read_u64(bytes, &mut cursor)?)
            .map_err(|_| SourceBundleError::IntegerOverflow)?;
        if source_len > MAX_SOURCE_FILE_BYTES {
            return Err(SourceBundleError::FileTooLarge);
        }
        total = total
            .checked_add(source_len)
            .ok_or(SourceBundleError::IntegerOverflow)?;
        if total > MAX_SOURCE_BUNDLE_BYTES {
            return Err(SourceBundleError::BundleTooLarge);
        }

        let path_bytes = take(bytes, &mut cursor, path_len)?;
        let path = std::str::from_utf8(path_bytes)
            .map_err(|_| SourceBundleError::InvalidPathUtf8)?
            .to_owned();
        validate_path(&path)?;
        if !seen.insert(path.clone()) {
            return Err(SourceBundleError::DuplicatePath);
        }
        if previous.as_ref().is_some_and(|value| value >= &path) {
            return Err(SourceBundleError::UnsortedPath);
        }
        previous = Some(path.clone());

        let source = take(bytes, &mut cursor, source_len)?.to_vec();
        let digest = take(bytes, &mut cursor, 32)?;
        if digest != Sha256::digest(&source).as_slice() {
            return Err(SourceBundleError::DigestMismatch);
        }
        entries.push(SourceBundleEntry { path, source });
    }
    if cursor != bytes.len() {
        return Err(SourceBundleError::TrailingData);
    }
    Ok(entries)
}

fn validate_entries(entries: &[SourceBundleEntry]) -> Result<(), SourceBundleError> {
    if entries.is_empty() {
        return Err(SourceBundleError::Empty);
    }
    if entries.len() > MAX_SOURCE_BUNDLE_FILES {
        return Err(SourceBundleError::TooManyFiles);
    }
    let mut total = 0usize;
    let mut previous: Option<&str> = None;
    let mut seen = BTreeSet::new();
    for entry in entries {
        validate_path(&entry.path)?;
        if !seen.insert(entry.path.as_str()) {
            return Err(SourceBundleError::DuplicatePath);
        }
        if previous.is_some_and(|value| value >= entry.path.as_str()) {
            return Err(SourceBundleError::UnsortedPath);
        }
        previous = Some(&entry.path);
        if entry.source.len() > MAX_SOURCE_FILE_BYTES {
            return Err(SourceBundleError::FileTooLarge);
        }
        total = total
            .checked_add(entry.source.len())
            .ok_or(SourceBundleError::IntegerOverflow)?;
        if total > MAX_SOURCE_BUNDLE_BYTES {
            return Err(SourceBundleError::BundleTooLarge);
        }
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), SourceBundleError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path.as_bytes().get(1).is_some_and(|byte| *byte == b':')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(SourceBundleError::InvalidPath);
    }
    Ok(())
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, SourceBundleError> {
    let raw: [u8; 4] = take(bytes, cursor, 4)?
        .try_into()
        .map_err(|_| SourceBundleError::Truncated)?;
    Ok(u32::from_le_bytes(raw))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, SourceBundleError> {
    let raw: [u8; 8] = take(bytes, cursor, 8)?
        .try_into()
        .map_err(|_| SourceBundleError::Truncated)?;
    Ok(u64::from_le_bytes(raw))
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], SourceBundleError> {
    let end = cursor
        .checked_add(length)
        .ok_or(SourceBundleError::IntegerOverflow)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(SourceBundleError::Truncated)?;
    *cursor = end;
    Ok(value)
}
