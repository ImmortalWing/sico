use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use serde::{Deserialize, Serialize};

use super::{ProductionIdentity, sha256_hex, validate_lower_hex, validate_name};

pub const LOCK_GRAPH_SCHEMA: &str = "sico.dependency.lock-graph.v0";
pub const STANDARD_LIBRARY_SCHEMA: &str = "sico.standard-library.contract.v0";
pub const MAX_DEPENDENCY_NODES: usize = 1_024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyError {
    Policy(super::PolicyError),
    Invalid(&'static str),
    InvalidValue(String),
    NotFound(DependencyIdentity),
    Ambiguous(DependencyIdentity),
    Conflict(DependencyIdentity),
    Cycle(DependencyIdentity),
    Incompatible(DependencyIdentity),
    Revoked(DependencyIdentity),
    CapabilityClosure,
    HostDenied(Vec<String>),
}

impl std::fmt::Display for DependencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(error) => {
                write!(formatter, "dependency policy validation failed: {error}")
            }
            Self::Invalid(field) => write!(formatter, "dependency input is invalid: {field}"),
            Self::InvalidValue(value) => write!(formatter, "dependency value is invalid: {value}"),
            Self::NotFound(identity) => write!(formatter, "dependency was not found: {identity}"),
            Self::Ambiguous(identity) => write!(formatter, "dependency is ambiguous: {identity}"),
            Self::Conflict(identity) => {
                write!(formatter, "dependency constraints conflict: {identity}")
            }
            Self::Cycle(identity) => write!(formatter, "dependency cycle includes: {identity}"),
            Self::Incompatible(identity) => {
                write!(formatter, "dependency is incompatible: {identity}")
            }
            Self::Revoked(identity) => write!(formatter, "dependency is revoked: {identity}"),
            Self::CapabilityClosure => {
                formatter.write_str("dependency capability closure differs from declaration")
            }
            Self::HostDenied(values) => write!(
                formatter,
                "Host denies dependency capabilities: {}",
                values.join(",")
            ),
        }
    }
}

impl std::error::Error for DependencyError {}

impl From<super::PolicyError> for DependencyError {
    fn from(error: super::PolicyError) -> Self {
        Self::Policy(error)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyIdentity {
    pub registry_id: String,
    pub namespace: String,
    pub package_name: String,
}

impl std::fmt::Display for DependencyIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}/{}/{}",
            self.registry_id, self.namespace, self.package_name
        )
    }
}

impl From<&ProductionIdentity> for DependencyIdentity {
    fn from(identity: &ProductionIdentity) -> Self {
        Self {
            registry_id: identity.registry_id.clone(),
            namespace: identity.namespace.clone(),
            package_name: identity.package_name.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityProfile {
    pub language_semantics: String,
    pub compiler_version: String,
    pub ir_schema: String,
    pub component_world: String,
    pub wasi_contract: String,
    pub package_format: u16,
    pub manifest_schema: String,
    pub publisher_policy_schema: String,
    pub release_schema: String,
    pub capability_contract: String,
    pub host_contract: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostCompatibility {
    pub language_semantics: BTreeSet<String>,
    pub compiler_versions: BTreeSet<String>,
    pub ir_schemas: BTreeSet<String>,
    pub component_worlds: BTreeSet<String>,
    pub wasi_contracts: BTreeSet<String>,
    pub package_formats: BTreeSet<u16>,
    pub manifest_schemas: BTreeSet<String>,
    pub publisher_policy_schemas: BTreeSet<String>,
    pub release_schemas: BTreeSet<String>,
    pub capability_contracts: BTreeSet<String>,
    pub host_contracts: BTreeSet<String>,
}

impl HostCompatibility {
    #[must_use]
    pub fn supports(&self, profile: &CompatibilityProfile) -> bool {
        self.language_semantics
            .contains(&profile.language_semantics)
            && self.compiler_versions.contains(&profile.compiler_version)
            && self.ir_schemas.contains(&profile.ir_schema)
            && self.component_worlds.contains(&profile.component_world)
            && self.wasi_contracts.contains(&profile.wasi_contract)
            && self.package_formats.contains(&profile.package_format)
            && self.manifest_schemas.contains(&profile.manifest_schema)
            && self
                .publisher_policy_schemas
                .contains(&profile.publisher_policy_schema)
            && self.release_schemas.contains(&profile.release_schema)
            && self
                .capability_contracts
                .contains(&profile.capability_contract)
            && self.host_contracts.contains(&profile.host_contract)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyRequirement {
    pub identity: DependencyIdentity,
    pub version_requirement: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCandidate {
    pub identity: DependencyIdentity,
    pub publisher_policy_id: String,
    pub version: String,
    pub release_record_sha256: String,
    pub package_sha256: String,
    pub compatibility: CompatibilityProfile,
    pub dependencies: Vec<DependencyRequirement>,
    pub capabilities: Vec<String>,
    pub yanked: bool,
    pub revoked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedDependencyNode {
    pub identity: DependencyIdentity,
    pub publisher_policy_id: String,
    pub version: String,
    pub release_record_sha256: String,
    pub package_sha256: String,
    pub compatibility: CompatibilityProfile,
    pub dependencies: Vec<DependencyRequirement>,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyLockGraph {
    pub schema: String,
    pub standard_library_sha256: String,
    pub roots: Vec<DependencyRequirement>,
    pub root_capabilities: Vec<String>,
    pub nodes: Vec<LockedDependencyNode>,
    pub capability_closure: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandardLibraryModule {
    pub name: String,
    pub api_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandardLibraryContract {
    pub schema: String,
    pub version: String,
    pub language_semantics: String,
    pub component_world: String,
    pub wasi_contract: String,
    pub capability_contract: String,
    pub modules: Vec<StandardLibraryModule>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevocationPolicy {
    Deny,
    AllowExact(BTreeSet<String>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SemVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<Identifier>,
    build: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Identifier {
    Numeric(u64),
    Text(String),
}

impl Ord for SemVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| compare_prerelease(&self.prerelease, &other.prerelease))
    }
}

impl PartialOrd for SemVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
enum VersionRequirement {
    Exact(SemVersion, String),
    Caret(SemVersion),
    Tilde(SemVersion),
    Interval {
        lower: SemVersion,
        upper: SemVersion,
    },
}

impl VersionRequirement {
    fn parse(value: &str) -> Result<Self, DependencyError> {
        if let Some(version) = value.strip_prefix('=') {
            return Ok(Self::Exact(SemVersion::parse(version)?, version.to_owned()));
        }
        if let Some(version) = value.strip_prefix('^') {
            return Ok(Self::Caret(SemVersion::parse(version)?));
        }
        if let Some(version) = value.strip_prefix('~') {
            return Ok(Self::Tilde(SemVersion::parse(version)?));
        }
        if let Some((lower, upper)) = value.split_once(',')
            && let (Some(lower), Some(upper)) = (lower.strip_prefix(">="), upper.strip_prefix('<'))
        {
            return Ok(Self::Interval {
                lower: SemVersion::parse(lower)?,
                upper: SemVersion::parse(upper)?,
            });
        }
        Err(DependencyError::Invalid("version requirement"))
    }

    fn matches(&self, original: &str, version: &SemVersion) -> bool {
        let permits_prerelease = match self {
            Self::Exact(_, text) => text.contains('-'),
            Self::Caret(value) | Self::Tilde(value) => !value.prerelease.is_empty(),
            Self::Interval { lower, upper } => {
                !lower.prerelease.is_empty() || !upper.prerelease.is_empty()
            }
        };
        if !version.prerelease.is_empty() && !permits_prerelease {
            return false;
        }
        match self {
            Self::Exact(expected, text) => expected == version && text == original,
            Self::Caret(lower) => version >= lower && version < &caret_upper(lower),
            Self::Tilde(lower) => version >= lower && version < &tilde_upper(lower),
            Self::Interval { lower, upper } => version >= lower && version < upper,
        }
    }
}

impl SemVersion {
    fn parse(value: &str) -> Result<Self, DependencyError> {
        if value.is_empty() || value.len() > 64 || value.starts_with('v') {
            return Err(DependencyError::Invalid("canonical SemVer"));
        }
        let mut build_parts = value.split('+');
        let version = build_parts.next().unwrap_or_default();
        let build = build_parts
            .next()
            .map(parse_text_identifiers)
            .transpose()?
            .unwrap_or_default();
        if build_parts.next().is_some() {
            return Err(DependencyError::Invalid("canonical SemVer"));
        }
        let (core, prerelease) = version
            .split_once('-')
            .map_or((version, None), |(core, pre)| (core, Some(pre)));
        let mut core_parts = core.split('.');
        let major = parse_core_number(core_parts.next())?;
        let minor = parse_core_number(core_parts.next())?;
        let patch = parse_core_number(core_parts.next())?;
        if core_parts.next().is_some() {
            return Err(DependencyError::Invalid("canonical SemVer"));
        }
        let prerelease = prerelease
            .map(parse_prerelease)
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
            build,
        })
    }
}

/// Resolves exact source identities to a canonical immutable dependency graph.
///
/// # Errors
///
/// Rejects missing, ambiguous, incompatible, yanked, revoked, cyclic or authority-expanding graphs.
pub fn resolve_dependencies(
    roots: &[DependencyRequirement],
    catalog: &[PackageCandidate],
    standard_library: &StandardLibraryContract,
    host: &HostCompatibility,
    root_capabilities: &BTreeSet<String>,
    declared_capabilities: &BTreeSet<String>,
    host_grants: &BTreeSet<String>,
) -> Result<DependencyLockGraph, DependencyError> {
    validate_standard_library(standard_library)?;
    if !host_supports_standard_library(host, standard_library) {
        return Err(DependencyError::Invalid(
            "standard library Host compatibility",
        ));
    }
    validate_requirements(roots)?;
    validate_catalog(catalog)?;
    let mut selected = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for requirement in roots {
        resolve_requirement(requirement, catalog, host, &mut selected, &mut visiting)?;
    }
    let closure: BTreeSet<_> = root_capabilities
        .iter()
        .cloned()
        .chain(
            selected
                .values()
                .flat_map(|candidate| candidate.capabilities.iter().cloned()),
        )
        .collect();
    if &closure != declared_capabilities {
        return Err(DependencyError::CapabilityClosure);
    }
    let denied: Vec<_> = closure.difference(host_grants).cloned().collect();
    if !denied.is_empty() {
        return Err(DependencyError::HostDenied(denied));
    }
    let nodes = selected.into_values().map(lock_node).collect();
    let mut roots = roots.to_vec();
    roots.sort();
    Ok(DependencyLockGraph {
        schema: LOCK_GRAPH_SCHEMA.to_owned(),
        standard_library_sha256: standard_library_digest(standard_library)?,
        roots,
        root_capabilities: root_capabilities.iter().cloned().collect(),
        nodes,
        capability_closure: closure.into_iter().collect(),
    })
}

/// Revalidates an immutable lock without allowing yank to rewrite it.
///
/// # Errors
///
/// Rejects digest drift, missing nodes, incompatible Hosts, graph mutation or unapproved revocation.
pub fn verify_dependency_lock(
    lock: &DependencyLockGraph,
    catalog: &[PackageCandidate],
    standard_library: &StandardLibraryContract,
    host: &HostCompatibility,
    host_grants: &BTreeSet<String>,
    revocation_policy: &RevocationPolicy,
) -> Result<(), DependencyError> {
    validate_lock_shape(lock)?;
    validate_catalog(catalog)?;
    if lock.standard_library_sha256 != standard_library_digest(standard_library)? {
        return Err(DependencyError::Invalid("standard library lock digest"));
    }
    if !host_supports_standard_library(host, standard_library) {
        return Err(DependencyError::Invalid(
            "standard library Host compatibility",
        ));
    }
    for node in &lock.nodes {
        let candidate = catalog
            .iter()
            .find(|candidate| candidate_matches_node(candidate, node))
            .ok_or_else(|| DependencyError::NotFound(node.identity.clone()))?;
        if !host.supports(&candidate.compatibility) {
            return Err(DependencyError::Incompatible(node.identity.clone()));
        }
        if candidate.revoked && !revocation_allowed(candidate, revocation_policy) {
            return Err(DependencyError::Revoked(node.identity.clone()));
        }
    }
    let closure: BTreeSet<_> = lock
        .root_capabilities
        .iter()
        .cloned()
        .chain(
            lock.nodes
                .iter()
                .flat_map(|node| node.capabilities.iter().cloned()),
        )
        .collect();
    if closure.into_iter().collect::<Vec<_>>() != lock.capability_closure {
        return Err(DependencyError::CapabilityClosure);
    }
    let denied: Vec<_> = lock
        .capability_closure
        .iter()
        .filter(|capability| !host_grants.contains(*capability))
        .cloned()
        .collect();
    if !denied.is_empty() {
        return Err(DependencyError::HostDenied(denied));
    }
    validate_locked_graph(lock)
}

/// Returns canonical compact JSON bytes for a lock graph.
///
/// # Errors
///
/// Rejects invalid graph shape or serialization failure.
pub fn canonical_lock_bytes(lock: &DependencyLockGraph) -> Result<Vec<u8>, DependencyError> {
    validate_lock_shape(lock)?;
    serde_json::to_vec(lock).map_err(|error| DependencyError::InvalidValue(error.to_string()))
}

/// Returns the immutable SHA-256 digest of a canonical lock graph.
///
/// # Errors
///
/// Rejects invalid graph shape or serialization failure.
pub fn dependency_lock_digest(lock: &DependencyLockGraph) -> Result<String, DependencyError> {
    Ok(sha256_hex(&canonical_lock_bytes(lock)?))
}

/// Parses canonical lock bytes and verifies their externally bound digest.
///
/// # Errors
///
/// Rejects oversized, non-canonical, unknown-field, malformed or digest-mismatched locks.
pub fn verify_dependency_lock_json(
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<DependencyLockGraph, DependencyError> {
    validate_lower_hex(expected_sha256, 64, "expected lock digest")?;
    if bytes.len() > 1024 * 1024 || sha256_hex(bytes) != expected_sha256 {
        return Err(DependencyError::Invalid("dependency lock bytes/digest"));
    }
    let lock: DependencyLockGraph = serde_json::from_slice(bytes)
        .map_err(|error| DependencyError::InvalidValue(error.to_string()))?;
    if serde_json::to_vec(&lock)
        .map_err(|error| DependencyError::InvalidValue(error.to_string()))?
        != bytes
    {
        return Err(DependencyError::Invalid("non-canonical dependency lock"));
    }
    validate_lock_shape(&lock)?;
    Ok(lock)
}

/// Returns the canonical standard-library contract digest.
///
/// # Errors
///
/// Rejects invalid contract shape or serialization failure.
pub fn standard_library_digest(
    contract: &StandardLibraryContract,
) -> Result<String, DependencyError> {
    validate_standard_library(contract)?;
    let bytes = serde_json::to_vec(contract)
        .map_err(|error| DependencyError::InvalidValue(error.to_string()))?;
    Ok(sha256_hex(&bytes))
}

fn resolve_requirement<'a>(
    requirement: &DependencyRequirement,
    catalog: &'a [PackageCandidate],
    host: &HostCompatibility,
    selected: &mut BTreeMap<DependencyIdentity, &'a PackageCandidate>,
    visiting: &mut BTreeSet<DependencyIdentity>,
) -> Result<(), DependencyError> {
    let parsed = VersionRequirement::parse(&requirement.version_requirement)?;
    if visiting.contains(&requirement.identity) {
        return Err(DependencyError::Cycle(requirement.identity.clone()));
    }
    if let Some(current) = selected.get(&requirement.identity) {
        return if parsed.matches(&current.version, &SemVersion::parse(&current.version)?) {
            Ok(())
        } else {
            Err(DependencyError::Conflict(requirement.identity.clone()))
        };
    }
    if !visiting.insert(requirement.identity.clone()) {
        return Err(DependencyError::Cycle(requirement.identity.clone()));
    }
    let mut matches: Vec<_> = catalog
        .iter()
        .filter(|candidate| {
            candidate.identity == requirement.identity && !candidate.yanked && !candidate.revoked
        })
        .filter_map(|candidate| {
            SemVersion::parse(&candidate.version)
                .ok()
                .map(|version| (candidate, version))
        })
        .filter(|(candidate, version)| {
            parsed.matches(&candidate.version, version) && host.supports(&candidate.compatibility)
        })
        .collect();
    matches.sort_by(|left, right| right.1.cmp(&left.1));
    let Some((chosen, chosen_version)) = matches.first() else {
        visiting.remove(&requirement.identity);
        return Err(DependencyError::NotFound(requirement.identity.clone()));
    };
    if matches
        .get(1)
        .is_some_and(|(_, version)| version.cmp(chosen_version) == Ordering::Equal)
    {
        visiting.remove(&requirement.identity);
        return Err(DependencyError::Ambiguous(requirement.identity.clone()));
    }
    selected.insert(requirement.identity.clone(), chosen);
    for dependency in &chosen.dependencies {
        resolve_requirement(dependency, catalog, host, selected, visiting)?;
    }
    visiting.remove(&requirement.identity);
    Ok(())
}

fn lock_node(candidate: &PackageCandidate) -> LockedDependencyNode {
    LockedDependencyNode {
        identity: candidate.identity.clone(),
        publisher_policy_id: candidate.publisher_policy_id.clone(),
        version: candidate.version.clone(),
        release_record_sha256: candidate.release_record_sha256.clone(),
        package_sha256: candidate.package_sha256.clone(),
        compatibility: candidate.compatibility.clone(),
        dependencies: candidate.dependencies.clone(),
        capabilities: candidate.capabilities.clone(),
    }
}

fn validate_catalog(catalog: &[PackageCandidate]) -> Result<(), DependencyError> {
    if catalog.len() > MAX_DEPENDENCY_NODES {
        return Err(DependencyError::Invalid("catalog size"));
    }
    let mut identities = BTreeSet::new();
    for candidate in catalog {
        validate_identity(&candidate.identity)?;
        validate_name(&candidate.publisher_policy_id, 64, "publisher policy id")?;
        SemVersion::parse(&candidate.version)?;
        validate_lower_hex(&candidate.release_record_sha256, 64, "release digest")?;
        validate_lower_hex(&candidate.package_sha256, 64, "package digest")?;
        validate_profile(&candidate.compatibility)?;
        validate_requirements(&candidate.dependencies)?;
        validate_sorted_strings(&candidate.capabilities, "candidate capabilities")?;
        let key = (
            candidate.identity.clone(),
            candidate.version.clone(),
            candidate.package_sha256.clone(),
        );
        if !identities.insert(key) {
            return Err(DependencyError::Invalid("duplicate catalog candidate"));
        }
    }
    Ok(())
}

fn validate_lock_shape(lock: &DependencyLockGraph) -> Result<(), DependencyError> {
    if lock.schema != LOCK_GRAPH_SCHEMA
        || lock.nodes.is_empty()
        || lock.nodes.len() > MAX_DEPENDENCY_NODES
    {
        return Err(DependencyError::Invalid("lock schema/node count"));
    }
    validate_lower_hex(&lock.standard_library_sha256, 64, "standard library digest")?;
    validate_requirements(&lock.roots)?;
    validate_sorted_strings(&lock.root_capabilities, "root capabilities")?;
    validate_sorted_strings(&lock.capability_closure, "lock capabilities")?;
    let identities: Vec<_> = lock.nodes.iter().map(|node| &node.identity).collect();
    if identities.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(DependencyError::Invalid("lock node order"));
    }
    for node in &lock.nodes {
        validate_identity(&node.identity)?;
        validate_name(&node.publisher_policy_id, 64, "publisher policy id")?;
        SemVersion::parse(&node.version)?;
        validate_lower_hex(&node.release_record_sha256, 64, "release digest")?;
        validate_lower_hex(&node.package_sha256, 64, "package digest")?;
        validate_profile(&node.compatibility)?;
        validate_requirements(&node.dependencies)?;
        validate_sorted_strings(&node.capabilities, "node capabilities")?;
    }
    Ok(())
}

fn validate_standard_library(contract: &StandardLibraryContract) -> Result<(), DependencyError> {
    if contract.schema != STANDARD_LIBRARY_SCHEMA
        || contract.modules.is_empty()
        || contract.modules.len() > 256
    {
        return Err(DependencyError::Invalid("standard library schema/modules"));
    }
    SemVersion::parse(&contract.version)?;
    for value in [
        &contract.language_semantics,
        &contract.component_world,
        &contract.wasi_contract,
        &contract.capability_contract,
    ] {
        validate_name(value, 64, "standard library compatibility")?;
    }
    let names: Vec<_> = contract
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    if names.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(DependencyError::Invalid("standard library module order"));
    }
    for module in &contract.modules {
        validate_name(&module.name, 64, "standard library module")?;
        validate_lower_hex(&module.api_sha256, 64, "standard library API digest")?;
    }
    Ok(())
}

fn validate_requirements(requirements: &[DependencyRequirement]) -> Result<(), DependencyError> {
    if requirements.len() > MAX_DEPENDENCY_NODES
        || requirements
            .windows(2)
            .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(DependencyError::Invalid("dependency requirement order"));
    }
    for requirement in requirements {
        validate_identity(&requirement.identity)?;
        VersionRequirement::parse(&requirement.version_requirement)?;
    }
    Ok(())
}

fn validate_identity(identity: &DependencyIdentity) -> Result<(), DependencyError> {
    validate_name(&identity.registry_id, 64, "dependency registry")?;
    validate_name(&identity.namespace, 128, "dependency namespace")?;
    validate_name(&identity.package_name, 64, "dependency package")?;
    Ok(())
}

fn validate_profile(profile: &CompatibilityProfile) -> Result<(), DependencyError> {
    if profile.package_format == 0 {
        return Err(DependencyError::Invalid("package format"));
    }
    for value in [
        &profile.language_semantics,
        &profile.compiler_version,
        &profile.ir_schema,
        &profile.component_world,
        &profile.wasi_contract,
        &profile.manifest_schema,
        &profile.publisher_policy_schema,
        &profile.release_schema,
        &profile.capability_contract,
        &profile.host_contract,
    ] {
        validate_name(value, 64, "compatibility identifier")?;
    }
    Ok(())
}

fn validate_sorted_strings(values: &[String], field: &'static str) -> Result<(), DependencyError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(DependencyError::Invalid(field));
    }
    for value in values {
        validate_name(value, 64, field)?;
    }
    Ok(())
}

fn candidate_matches_node(candidate: &PackageCandidate, node: &LockedDependencyNode) -> bool {
    candidate.identity == node.identity
        && candidate.publisher_policy_id == node.publisher_policy_id
        && candidate.version == node.version
        && candidate.release_record_sha256 == node.release_record_sha256
        && candidate.package_sha256 == node.package_sha256
        && candidate.compatibility == node.compatibility
        && candidate.dependencies == node.dependencies
        && candidate.capabilities == node.capabilities
}

fn validate_locked_graph(lock: &DependencyLockGraph) -> Result<(), DependencyError> {
    let nodes: BTreeMap<_, _> = lock
        .nodes
        .iter()
        .map(|node| (&node.identity, node))
        .collect();
    let mut reachable = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    for root in &lock.roots {
        visit_locked_requirement(root, &nodes, &mut reachable, &mut visiting)?;
    }
    if reachable.len() != lock.nodes.len() {
        return Err(DependencyError::Invalid("unreachable locked node"));
    }
    Ok(())
}

fn visit_locked_requirement(
    requirement: &DependencyRequirement,
    nodes: &BTreeMap<&DependencyIdentity, &LockedDependencyNode>,
    reachable: &mut BTreeSet<DependencyIdentity>,
    visiting: &mut BTreeSet<DependencyIdentity>,
) -> Result<(), DependencyError> {
    let node = nodes
        .get(&requirement.identity)
        .ok_or_else(|| DependencyError::NotFound(requirement.identity.clone()))?;
    if !VersionRequirement::parse(&requirement.version_requirement)?
        .matches(&node.version, &SemVersion::parse(&node.version)?)
    {
        return Err(DependencyError::Conflict(requirement.identity.clone()));
    }
    if reachable.contains(&requirement.identity) {
        return Ok(());
    }
    if !visiting.insert(requirement.identity.clone()) {
        return Err(DependencyError::Cycle(requirement.identity.clone()));
    }
    for dependency in &node.dependencies {
        visit_locked_requirement(dependency, nodes, reachable, visiting)?;
    }
    visiting.remove(&requirement.identity);
    reachable.insert(requirement.identity.clone());
    Ok(())
}

fn host_supports_standard_library(
    host: &HostCompatibility,
    contract: &StandardLibraryContract,
) -> bool {
    host.language_semantics
        .contains(&contract.language_semantics)
        && host.component_worlds.contains(&contract.component_world)
        && host.wasi_contracts.contains(&contract.wasi_contract)
        && host
            .capability_contracts
            .contains(&contract.capability_contract)
}

fn revocation_allowed(candidate: &PackageCandidate, policy: &RevocationPolicy) -> bool {
    match policy {
        RevocationPolicy::Deny => false,
        RevocationPolicy::AllowExact(digests) => digests.contains(&candidate.package_sha256),
    }
}

fn parse_core_number(value: Option<&str>) -> Result<u64, DependencyError> {
    let value = value.ok_or(DependencyError::Invalid("canonical SemVer"))?;
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DependencyError::Invalid("canonical SemVer"));
    }
    value
        .parse()
        .map_err(|_| DependencyError::Invalid("canonical SemVer numeric bound"))
}

fn parse_text_identifiers(value: &str) -> Result<Vec<String>, DependencyError> {
    if value.is_empty() {
        return Err(DependencyError::Invalid("canonical SemVer identifier"));
    }
    value
        .split('.')
        .map(|part| {
            if part.is_empty()
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            {
                Err(DependencyError::Invalid("canonical SemVer identifier"))
            } else {
                Ok(part.to_owned())
            }
        })
        .collect()
}

fn parse_prerelease(value: &str) -> Result<Vec<Identifier>, DependencyError> {
    parse_text_identifiers(value)?
        .into_iter()
        .map(|part| {
            if part.bytes().all(|byte| byte.is_ascii_digit()) {
                if part.len() > 1 && part.starts_with('0') {
                    return Err(DependencyError::Invalid("canonical SemVer prerelease"));
                }
                Ok(Identifier::Numeric(part.parse().map_err(|_| {
                    DependencyError::Invalid("canonical SemVer numeric bound")
                })?))
            } else {
                Ok(Identifier::Text(part))
            }
        })
        .collect()
}

fn compare_prerelease(left: &[Identifier], right: &[Identifier]) -> Ordering {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            for (left, right) in left.iter().zip(right) {
                let order = match (left, right) {
                    (Identifier::Numeric(left), Identifier::Numeric(right)) => left.cmp(right),
                    (Identifier::Numeric(_), Identifier::Text(_)) => Ordering::Less,
                    (Identifier::Text(_), Identifier::Numeric(_)) => Ordering::Greater,
                    (Identifier::Text(left), Identifier::Text(right)) => left.cmp(right),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
            left.len().cmp(&right.len())
        }
    }
}

fn caret_upper(version: &SemVersion) -> SemVersion {
    if version.major > 0 {
        plain_version(version.major.saturating_add(1), 0, 0)
    } else if version.minor > 0 {
        plain_version(0, version.minor.saturating_add(1), 0)
    } else {
        plain_version(0, 0, version.patch.saturating_add(1))
    }
}

fn tilde_upper(version: &SemVersion) -> SemVersion {
    plain_version(version.major, version.minor.saturating_add(1), 0)
}

fn plain_version(major: u64, minor: u64, patch: u64) -> SemVersion {
    SemVersion {
        major,
        minor,
        patch,
        prerelease: Vec::new(),
        build: Vec::new(),
    }
}
