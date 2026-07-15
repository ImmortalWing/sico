//! Deterministic `.sapp` v0 construction and strict verification.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use wasmparser::{Parser, Payload, ValidPayload, Validator, WasmFeatures};

pub const FORMAT_VERSION: u16 = 1;
pub const MAX_PACKAGE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 1_024;
pub const MAX_PATH_BYTES: usize = 240;
pub const MAX_PATH_DEPTH: usize = 8;
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_COMPONENT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_RESOURCE_BYTES: usize = 8 * 1024 * 1024;

const MAGIC: &[u8; 8] = b"SAPP\r\n\x1a\n";
const MANIFEST_PATH: &str = "manifest.json";
const COMPONENT_PATH: &str = "app.component.wasm";
const SIGNATURE_PATH: &str = "signature.json";
const SIGNATURE_DOMAIN: &[u8] = b"SICO-SAPP-DEV-SIGNATURE-V0\0";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub format_version: u16,
    pub app: AppIdentity,
    pub component: Artifact,
    pub resources: Vec<Artifact>,
    pub source_effects: Vec<String>,
    pub capabilities: Vec<String>,
    pub limits: RuntimeLimits,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppIdentity {
    pub id: String,
    pub version: String,
    pub entry: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLimits {
    pub fuel: u64,
    pub timeout_ms: u64,
    pub memory_bytes: u64,
    pub table_elements: u32,
    pub instances: u32,
    pub tables: u32,
    pub memories: u32,
    pub wasi_resources: u32,
    pub hostcall_fuel: u64,
    pub random_bytes: u64,
    pub body_bytes: u64,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            fuel: 1_000_000,
            timeout_ms: 5_000,
            memory_bytes: 64 * 1024 * 1024,
            table_elements: 10_000,
            instances: 16,
            tables: 16,
            memories: 8,
            wasi_resources: 128,
            hostcall_fuel: 100_000,
            random_bytes: 64 * 1024,
            body_bytes: 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceInput {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildInput {
    pub app_id: String,
    pub app_version: String,
    pub component: Vec<u8>,
    pub resources: Vec<ResourceInput>,
    pub source_effects: Vec<String>,
    pub capabilities: Vec<String>,
    pub limits: RuntimeLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPackage {
    pub manifest: Manifest,
    pub component: Vec<u8>,
    pub resources: BTreeMap<String, Vec<u8>>,
    pub component_imports: Vec<String>,
    pub signature: Option<Vec<u8>>,
    unsigned_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedPackage {
    pub package: VerifiedPackage,
    pub trust: TrustStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrustStatus {
    UnsignedDevelopment,
    Development { public_key: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrustPolicy {
    RequireDevelopment(BTreeSet<[u8; 32]>),
    AllowUnsignedDevelopment,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureRecord {
    schema: String,
    scheme: String,
    public_key: String,
    signature: String,
}

impl VerifiedPackage {
    #[must_use]
    pub fn unsigned_bytes(&self) -> &[u8] {
        &self.unsigned_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageError {
    TooLarge(&'static str),
    Truncated,
    BadMagic,
    UnsupportedVersion(u16),
    EntryCount(usize),
    InvalidPath(String),
    DuplicatePath(String),
    NonCanonicalOrder,
    TrailingBytes,
    MissingEntry(&'static str),
    UnexpectedEntry(String),
    Manifest(String),
    NonCanonicalManifest,
    InvalidIdentity(&'static str),
    InvalidComponent(String),
    DigestMismatch(String),
    LengthMismatch(String),
    ResourceSetMismatch,
    AlreadySigned,
    MissingSignature,
    InvalidSignature(String),
    UntrustedKey,
}

impl std::fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge(part) => write!(formatter, "{part} exceeds the v0 limit"),
            Self::Truncated => formatter.write_str("package is truncated"),
            Self::BadMagic => formatter.write_str("package magic is invalid"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "package format version {version} is unsupported")
            }
            Self::EntryCount(count) => write!(formatter, "package entry count {count} is invalid"),
            Self::InvalidPath(path) => write!(formatter, "package entry path is invalid: {path}"),
            Self::DuplicatePath(path) => {
                write!(formatter, "package entry path is duplicated: {path}")
            }
            Self::NonCanonicalOrder => {
                formatter.write_str("package entries are not in canonical order")
            }
            Self::TrailingBytes => formatter.write_str("package has trailing bytes"),
            Self::MissingEntry(path) => write!(formatter, "package is missing {path}"),
            Self::UnexpectedEntry(path) => {
                write!(formatter, "package entry is not allowed: {path}")
            }
            Self::Manifest(message) => write!(formatter, "manifest is invalid: {message}"),
            Self::NonCanonicalManifest => formatter.write_str("manifest JSON is not canonical"),
            Self::InvalidIdentity(field) => write!(formatter, "manifest {field} is invalid"),
            Self::InvalidComponent(message) => write!(formatter, "Component is invalid: {message}"),
            Self::DigestMismatch(path) => {
                write!(formatter, "content digest does not match: {path}")
            }
            Self::LengthMismatch(path) => {
                write!(formatter, "content length does not match: {path}")
            }
            Self::ResourceSetMismatch => {
                formatter.write_str("manifest and archive resource sets differ")
            }
            Self::AlreadySigned => formatter.write_str("package already has a signature"),
            Self::MissingSignature => {
                formatter.write_str("package requires a development signature")
            }
            Self::InvalidSignature(message) => {
                write!(formatter, "development signature is invalid: {message}")
            }
            Self::UntrustedKey => formatter.write_str("development signing key is not trusted"),
        }
    }
}

impl std::error::Error for PackageError {}

#[derive(Clone, Debug)]
struct Entry {
    path: String,
    bytes: Vec<u8>,
}

/// Builds a canonical unsigned `.sapp` v0 package.
///
/// # Errors
///
/// Rejects invalid identity, Component, resource paths, capability lists, limits,
/// or inputs that exceed the package ceilings.
pub fn build_unsigned(mut input: BuildInput) -> Result<Vec<u8>, PackageError> {
    validate_app_id(&input.app_id)?;
    validate_app_version(&input.app_version)?;
    validate_limits(&input.limits)?;
    if input.component.len() > MAX_COMPONENT_BYTES {
        return Err(PackageError::TooLarge("Component"));
    }
    component_imports(&input.component)?;
    normalize_sorted_unique(&mut input.source_effects, "source effect")?;
    normalize_sorted_unique(&mut input.capabilities, "capability")?;

    input
        .resources
        .sort_by(|left, right| left.path.cmp(&right.path));
    let mut resource_entries = Vec::with_capacity(input.resources.len());
    let mut resource_artifacts = Vec::with_capacity(input.resources.len());
    let mut previous = None;
    for resource in input.resources {
        let path = format!("resources/{}", resource.path);
        validate_path(&path)?;
        if resource.bytes.len() > MAX_RESOURCE_BYTES {
            return Err(PackageError::TooLarge("resource"));
        }
        let folded = path.to_ascii_lowercase();
        if previous.as_ref() == Some(&folded) {
            return Err(PackageError::DuplicatePath(path));
        }
        previous = Some(folded);
        resource_artifacts.push(artifact(&path, &resource.bytes));
        resource_entries.push(Entry {
            path,
            bytes: resource.bytes,
        });
    }

    let manifest = Manifest {
        schema: "sico.sapp.manifest.v0".to_owned(),
        format_version: FORMAT_VERSION,
        app: AppIdentity {
            id: input.app_id,
            version: input.app_version,
            entry: "main()".to_owned(),
        },
        component: artifact(COMPONENT_PATH, &input.component),
        resources: resource_artifacts,
        source_effects: input.source_effects,
        capabilities: input.capabilities,
        limits: input.limits,
    };
    let manifest_bytes = canonical_manifest(&manifest)?;
    let mut entries = Vec::with_capacity(resource_entries.len() + 2);
    entries.push(Entry {
        path: MANIFEST_PATH.to_owned(),
        bytes: manifest_bytes,
    });
    entries.push(Entry {
        path: COMPONENT_PATH.to_owned(),
        bytes: input.component,
    });
    entries.extend(resource_entries);
    encode_archive(&entries)
}

/// Strictly verifies and extracts a `.sapp` v0 package without executing it.
///
/// # Errors
///
/// Rejects malformed framing, non-canonical JSON, invalid Components, and all
/// content length/hash/resource-set mismatches.
pub fn verify(bytes: &[u8]) -> Result<VerifiedPackage, PackageError> {
    let entries = parse_archive(bytes)?;
    let manifest_entry = entries
        .first()
        .filter(|entry| entry.path == MANIFEST_PATH)
        .ok_or(PackageError::MissingEntry(MANIFEST_PATH))?;
    let manifest: Manifest = serde_json::from_slice(&manifest_entry.bytes)
        .map_err(|error| PackageError::Manifest(error.to_string()))?;
    if canonical_manifest(&manifest)? != manifest_entry.bytes {
        return Err(PackageError::NonCanonicalManifest);
    }
    validate_manifest(&manifest)?;

    let component_entry = entries
        .get(1)
        .filter(|entry| entry.path == COMPONENT_PATH)
        .ok_or(PackageError::MissingEntry(COMPONENT_PATH))?;
    verify_artifact(&manifest.component, component_entry)?;
    let imports = component_imports(&component_entry.bytes)?;

    let mut resources = BTreeMap::new();
    let mut actual_resource_paths = Vec::new();
    let mut signature = None;
    for entry in entries.iter().skip(2) {
        if entry.path == SIGNATURE_PATH {
            signature = Some(entry.bytes.clone());
            continue;
        }
        if !entry.path.starts_with("resources/") {
            return Err(PackageError::UnexpectedEntry(entry.path.clone()));
        }
        actual_resource_paths.push(entry.path.clone());
        resources.insert(entry.path.clone(), entry.bytes.clone());
    }
    let expected_resource_paths: Vec<_> = manifest
        .resources
        .iter()
        .map(|resource| resource.path.clone())
        .collect();
    if actual_resource_paths != expected_resource_paths {
        return Err(PackageError::ResourceSetMismatch);
    }
    for resource in &manifest.resources {
        let bytes = resources
            .get(&resource.path)
            .ok_or(PackageError::ResourceSetMismatch)?;
        verify_artifact(
            resource,
            &Entry {
                path: resource.path.clone(),
                bytes: bytes.clone(),
            },
        )?;
    }
    let unsigned_entries: Vec<_> = entries
        .iter()
        .filter(|entry| entry.path != SIGNATURE_PATH)
        .cloned()
        .collect();
    Ok(VerifiedPackage {
        manifest,
        component: component_entry.bytes.clone(),
        resources,
        component_imports: imports,
        signature,
        unsigned_bytes: encode_archive(&unsigned_entries)?,
    })
}

/// Adds a deterministic domain-separated Ed25519 development signature.
///
/// The 32-byte signing seed remains caller-owned. Only its public key and the
/// signature are written to the package.
///
/// # Errors
///
/// Rejects malformed or already signed packages.
pub fn sign_development(unsigned: &[u8], signing_seed: &[u8; 32]) -> Result<Vec<u8>, PackageError> {
    let verified = verify(unsigned)?;
    if verified.signature.is_some() {
        return Err(PackageError::AlreadySigned);
    }
    let signing_key = SigningKey::from_bytes(signing_seed);
    let mut message = Vec::with_capacity(SIGNATURE_DOMAIN.len() + unsigned.len());
    message.extend_from_slice(SIGNATURE_DOMAIN);
    message.extend_from_slice(unsigned);
    let signature = signing_key.sign(&message);
    let record = SignatureRecord {
        schema: "sico.sapp.signature.v0".to_owned(),
        scheme: "ed25519-dev-v0".to_owned(),
        public_key: encode_hex(signing_key.verifying_key().as_bytes()),
        signature: encode_hex(&signature.to_bytes()),
    };
    let signature_bytes = serde_json::to_vec(&record)
        .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
    let mut entries = parse_archive(unsigned)?;
    entries.push(Entry {
        path: SIGNATURE_PATH.to_owned(),
        bytes: signature_bytes,
    });
    encode_archive(&entries)
}

/// Applies an explicit development trust policy after structural verification.
///
/// # Errors
///
/// Rejects missing, malformed, non-canonical, cryptographically invalid, or
/// untrusted signatures according to `policy`.
pub fn verify_trusted(bytes: &[u8], policy: &TrustPolicy) -> Result<TrustedPackage, PackageError> {
    let package = verify(bytes)?;
    let Some(signature_bytes) = package.signature.as_deref() else {
        return match policy {
            TrustPolicy::AllowUnsignedDevelopment => Ok(TrustedPackage {
                package,
                trust: TrustStatus::UnsignedDevelopment,
            }),
            TrustPolicy::RequireDevelopment(_) => Err(PackageError::MissingSignature),
        };
    };
    let record: SignatureRecord = serde_json::from_slice(signature_bytes)
        .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
    let canonical = serde_json::to_vec(&record)
        .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
    if canonical != signature_bytes {
        return Err(PackageError::InvalidSignature(
            "signature JSON is not canonical".to_owned(),
        ));
    }
    if record.schema != "sico.sapp.signature.v0" || record.scheme != "ed25519-dev-v0" {
        return Err(PackageError::InvalidSignature(
            "unknown schema or scheme".to_owned(),
        ));
    }
    let public_key = decode_hex::<32>(&record.public_key)?;
    let signature = Signature::from_bytes(&decode_hex::<64>(&record.signature)?);
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
    if verifying_key.is_weak() {
        return Err(PackageError::InvalidSignature("weak public key".to_owned()));
    }
    let mut message = Vec::with_capacity(SIGNATURE_DOMAIN.len() + package.unsigned_bytes.len());
    message.extend_from_slice(SIGNATURE_DOMAIN);
    message.extend_from_slice(&package.unsigned_bytes);
    verifying_key
        .verify_strict(&message, &signature)
        .map_err(|error| PackageError::InvalidSignature(error.to_string()))?;
    if let TrustPolicy::RequireDevelopment(keys) = policy
        && !keys.contains(&public_key)
    {
        return Err(PackageError::UntrustedKey);
    }
    Ok(TrustedPackage {
        package,
        trust: TrustStatus::Development {
            public_key: record.public_key,
        },
    })
}

fn validate_manifest(manifest: &Manifest) -> Result<(), PackageError> {
    if manifest.schema != "sico.sapp.manifest.v0" {
        return Err(PackageError::Manifest("unknown schema".to_owned()));
    }
    if manifest.format_version != FORMAT_VERSION {
        return Err(PackageError::UnsupportedVersion(manifest.format_version));
    }
    validate_app_id(&manifest.app.id)?;
    validate_app_version(&manifest.app.version)?;
    if manifest.app.entry != "main()" {
        return Err(PackageError::InvalidIdentity("entry"));
    }
    if manifest.component.path != COMPONENT_PATH {
        return Err(PackageError::Manifest(
            "unexpected Component path".to_owned(),
        ));
    }
    validate_limits(&manifest.limits)?;
    validate_sorted_unique(&manifest.source_effects, "source effect")?;
    validate_sorted_unique(&manifest.capabilities, "capability")?;
    let paths: Vec<_> = manifest
        .resources
        .iter()
        .map(|resource| resource.path.as_str())
        .collect();
    if !paths.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(PackageError::ResourceSetMismatch);
    }
    for resource in &manifest.resources {
        validate_path(&resource.path)?;
        if !resource.path.starts_with("resources/") {
            return Err(PackageError::ResourceSetMismatch);
        }
    }
    Ok(())
}

fn validate_limits(limits: &RuntimeLimits) -> Result<(), PackageError> {
    let valid = limits.fuel > 0
        && limits.timeout_ms > 0
        && limits.memory_bytes > 0
        && limits.memory_bytes <= 4 * 1024 * 1024 * 1024
        && limits.table_elements > 0
        && limits.instances > 0
        && limits.tables > 0
        && limits.memories > 0
        && limits.wasi_resources > 0
        && limits.hostcall_fuel > 0
        && limits.random_bytes > 0
        && limits.body_bytes > 0;
    if valid {
        Ok(())
    } else {
        Err(PackageError::Manifest(
            "Runtime limits are invalid".to_owned(),
        ))
    }
}

fn canonical_manifest(manifest: &Manifest) -> Result<Vec<u8>, PackageError> {
    let bytes =
        serde_json::to_vec(manifest).map_err(|error| PackageError::Manifest(error.to_string()))?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        Err(PackageError::TooLarge("manifest"))
    } else {
        Ok(bytes)
    }
}

fn artifact(path: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        path: path.to_owned(),
        sha256: sha256_hex(bytes),
        bytes: u64::try_from(bytes.len()).expect("usize always fits u64"),
    }
}

fn verify_artifact(artifact: &Artifact, entry: &Entry) -> Result<(), PackageError> {
    if artifact.path != entry.path {
        return Err(PackageError::ResourceSetMismatch);
    }
    if artifact.bytes != u64::try_from(entry.bytes.len()).expect("usize always fits u64") {
        return Err(PackageError::LengthMismatch(entry.path.clone()));
    }
    if artifact.sha256 != sha256_hex(&entry.bytes) {
        return Err(PackageError::DigestMismatch(entry.path.clone()));
    }
    Ok(())
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn decode_hex<const N: usize>(text: &str) -> Result<[u8; N], PackageError> {
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PackageError::InvalidSignature(
            "hex length or alphabet is invalid".to_owned(),
        ));
    }
    let mut output = [0_u8; N];
    for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Ok(output)
}

const fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => 0,
    }
}

fn encode_archive(entries: &[Entry]) -> Result<Vec<u8>, PackageError> {
    if entries.len() > MAX_ENTRIES {
        return Err(PackageError::EntryCount(entries.len()));
    }
    validate_entry_order(entries)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(entries.len())
            .expect("entry ceiling fits u32")
            .to_le_bytes(),
    );
    for entry in entries {
        validate_path(&entry.path)?;
        let path = entry.path.as_bytes();
        bytes.extend_from_slice(
            &u16::try_from(path.len())
                .map_err(|_| PackageError::TooLarge("entry path"))?
                .to_le_bytes(),
        );
        bytes.extend_from_slice(
            &u64::try_from(entry.bytes.len())
                .expect("usize always fits u64")
                .to_le_bytes(),
        );
        bytes.extend_from_slice(path);
        bytes.extend_from_slice(&entry.bytes);
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(PackageError::TooLarge("package"));
        }
    }
    Ok(bytes)
}

fn parse_archive(bytes: &[u8]) -> Result<Vec<Entry>, PackageError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(PackageError::TooLarge("package"));
    }
    let mut cursor = Cursor::new(bytes);
    if cursor.take(8)? != MAGIC {
        return Err(PackageError::BadMagic);
    }
    let version = cursor.u16()?;
    if version != FORMAT_VERSION {
        return Err(PackageError::UnsupportedVersion(version));
    }
    let count = usize::try_from(cursor.u32()?).expect("u32 fits usize on supported targets");
    if !(2..=MAX_ENTRIES).contains(&count) {
        return Err(PackageError::EntryCount(count));
    }
    let mut entries = Vec::with_capacity(count);
    let mut folded_paths = BTreeSet::new();
    for _ in 0..count {
        let path_len = usize::from(cursor.u16()?);
        if path_len == 0 || path_len > MAX_PATH_BYTES {
            return Err(PackageError::TooLarge("entry path"));
        }
        let content_len =
            usize::try_from(cursor.u64()?).map_err(|_| PackageError::TooLarge("entry"))?;
        let path = std::str::from_utf8(cursor.take(path_len)?)
            .map_err(|_| PackageError::InvalidPath("non-UTF-8".to_owned()))?
            .to_owned();
        validate_path(&path)?;
        validate_entry_size(&path, content_len)?;
        if !folded_paths.insert(path.to_ascii_lowercase()) {
            return Err(PackageError::DuplicatePath(path));
        }
        entries.push(Entry {
            path,
            bytes: cursor.take(content_len)?.to_vec(),
        });
    }
    if cursor.remaining() != 0 {
        return Err(PackageError::TrailingBytes);
    }
    validate_entry_order(&entries)?;
    Ok(entries)
}

fn validate_entry_size(path: &str, length: usize) -> Result<(), PackageError> {
    let limit = match path {
        MANIFEST_PATH | SIGNATURE_PATH => MAX_MANIFEST_BYTES,
        COMPONENT_PATH => MAX_COMPONENT_BYTES,
        _ if path.starts_with("resources/") => MAX_RESOURCE_BYTES,
        _ => return Err(PackageError::UnexpectedEntry(path.to_owned())),
    };
    if length > limit {
        Err(PackageError::TooLarge("entry"))
    } else {
        Ok(())
    }
}

fn validate_entry_order(entries: &[Entry]) -> Result<(), PackageError> {
    if entries.len() < 2 || entries[0].path != MANIFEST_PATH || entries[1].path != COMPONENT_PATH {
        return Err(PackageError::NonCanonicalOrder);
    }
    let mut previous_resource: Option<&str> = None;
    let mut saw_signature = false;
    for entry in entries.iter().skip(2) {
        if entry.path == SIGNATURE_PATH {
            if saw_signature || entry.path != entries.last().expect("entries is nonempty").path {
                return Err(PackageError::NonCanonicalOrder);
            }
            saw_signature = true;
            continue;
        }
        if saw_signature || !entry.path.starts_with("resources/") {
            return Err(PackageError::UnexpectedEntry(entry.path.clone()));
        }
        if previous_resource.is_some_and(|previous| previous >= entry.path.as_str()) {
            return Err(PackageError::NonCanonicalOrder);
        }
        previous_resource = Some(&entry.path);
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), PackageError> {
    if path.len() > MAX_PATH_BYTES
        || path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', ':', '\0'])
        || path.nfc().collect::<String>() != path
    {
        return Err(PackageError::InvalidPath(path.to_owned()));
    }
    let segments: Vec<_> = path.split('/').collect();
    if segments.len() > MAX_PATH_DEPTH
        || segments.iter().any(|segment| {
            segment.is_empty() || *segment == "." || *segment == ".." || is_windows_device(segment)
        })
    {
        return Err(PackageError::InvalidPath(path.to_owned()));
    }
    Ok(())
}

fn is_windows_device(segment: &str) -> bool {
    let stem = segment
        .split_once('.')
        .map_or(segment, |(before, _)| before)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

fn validate_app_id(id: &str) -> Result<(), PackageError> {
    if id.len() > 128 {
        return Err(PackageError::InvalidIdentity("app.id"));
    }
    let labels: Vec<_> = id.split('.').collect();
    if !(2..=8).contains(&labels.len())
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 32
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                || label.starts_with('-')
                || label.ends_with('-')
        })
    {
        return Err(PackageError::InvalidIdentity("app.id"));
    }
    Ok(())
}

fn validate_app_version(version: &str) -> Result<(), PackageError> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        Err(PackageError::InvalidIdentity("app.version"))
    } else {
        Ok(())
    }
}

fn normalize_sorted_unique(
    values: &mut Vec<String>,
    field: &'static str,
) -> Result<(), PackageError> {
    values.sort();
    values.dedup();
    validate_sorted_unique(values, field)
}

fn validate_sorted_unique(values: &[String], field: &'static str) -> Result<(), PackageError> {
    if !values.windows(2).all(|pair| pair[0] < pair[1])
        || values.iter().any(|value| {
            value.is_empty()
                || value.len() > 128
                || !value.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'-')
                })
        })
    {
        Err(PackageError::Manifest(format!("{field} list is invalid")))
    } else {
        Ok(())
    }
}

fn component_imports(bytes: &[u8]) -> Result<Vec<String>, PackageError> {
    let mut validator = Validator::new_with_features(WasmFeatures::all());
    let mut depth = 1_u32;
    let mut imports = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|error| PackageError::InvalidComponent(error.to_string()))?;
        match validator
            .payload(&payload)
            .map_err(|error| PackageError::InvalidComponent(error.to_string()))?
        {
            ValidPayload::Parser(_) => depth = depth.saturating_add(1),
            ValidPayload::End(_) => depth = depth.saturating_sub(1),
            ValidPayload::Ok | ValidPayload::Func(..) => {}
        }
        if let Payload::ComponentImportSection(section) = payload
            && depth == 1
        {
            for import in section {
                let import =
                    import.map_err(|error| PackageError::InvalidComponent(error.to_string()))?;
                imports.push(import.name.name.to_owned());
            }
        }
    }
    imports.sort();
    if !imports.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(PackageError::InvalidComponent(
            "duplicate Component import".to_owned(),
        ));
    }
    Ok(imports)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PackageError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(PackageError::TooLarge("entry"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(PackageError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, PackageError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("two bytes"),
        ))
    }

    fn u32(&mut self) -> Result<u32, PackageError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    fn u64(&mut self) -> Result<u64, PackageError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
}
