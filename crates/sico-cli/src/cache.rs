//! Content-addressed Script source cache (RFC-0029 cache identity).
//!
//! Entries are performance artifacts, never trust roots: the key is
//! recomputed from the frozen identity fields on every run, the cached
//! Component is revalidated before execution, and corrupt or ambiguous
//! entries fail closed and are never overwritten silently.

#![forbid(unsafe_code)]

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use sha2::{Digest, Sha256};

pub const CACHE_DOMAIN: &[u8] = b"SICO-SCRIPT-SOURCE-CACHE-V0\0";
pub const CACHE_SUBDIR: &str = "script-v0";

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// Frozen cache-key identity fields in their fixed order.
pub struct CacheKeyParts<'a> {
    pub compiler_digest: &'a [u8; 32],
    pub compiler_build_id: &'a str,
    pub semantics_id: &'a str,
    pub script_wit_version: &'a str,
    pub adapter_digest: &'a [u8; 32],
    pub source: &'a [u8],
    pub app_id: &'a str,
    pub app_version: &'a str,
    pub profile_id: &'a str,
    pub manifest_schema: &'a str,
    pub codegen_options: &'a [u8],
}

/// Computes the frozen cache key: domain, then fixed-order fields; fixed
/// digests raw, every variable field as u64 little-endian length + bytes.
#[must_use]
pub fn cache_key(parts: &CacheKeyParts<'_>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(CACHE_DOMAIN);
    hasher.update(parts.compiler_digest);
    field(&mut hasher, parts.compiler_build_id.as_bytes());
    field(&mut hasher, parts.semantics_id.as_bytes());
    field(&mut hasher, parts.script_wit_version.as_bytes());
    hasher.update(parts.adapter_digest);
    field(&mut hasher, parts.source);
    field(&mut hasher, parts.app_id.as_bytes());
    field(&mut hasher, parts.app_version.as_bytes());
    field(&mut hasher, parts.profile_id.as_bytes());
    field(&mut hasher, parts.manifest_schema.as_bytes());
    field(&mut hasher, parts.codegen_options);
    hasher.finalize().into()
}

fn field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CacheError {
    Io(String),
    /// The entry exists but fails structural revalidation; it is refused and
    /// never overwritten silently.
    Corrupt(PathBuf),
    /// A different artifact already occupies the key.
    Conflict(PathBuf),
}

pub struct SourceCache {
    dir: PathBuf,
}

impl SourceCache {
    /// Opens the cache directory: `SICO_CACHE_DIR`, else the platform user
    /// cache (`%LOCALAPPDATA%\sico\cache` or `$XDG_CACHE_HOME/sico` /
    /// `~/.cache/sico`).
    #[must_use]
    pub fn open() -> Option<Self> {
        let dir = std::env::var_os("SICO_CACHE_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("LOCALAPPDATA")
                    .map(|base| PathBuf::from(base).join("sico").join("cache"))
            })
            .or_else(|| {
                std::env::var_os("XDG_CACHE_HOME").map(|base| PathBuf::from(base).join("sico"))
            })
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache").join("sico"))
            })?;
        Some(Self { dir })
    }

    fn entry_path(&self, key: &[u8; 32]) -> PathBuf {
        let hex = key.iter().fold(String::with_capacity(64), |mut out, byte| {
            use std::fmt::Write as _;
            write!(out, "{byte:02x}").expect("writing to a String cannot fail");
            out
        });
        self.dir
            .join(CACHE_SUBDIR)
            .join(format!("{hex}.component.wasm"))
    }

    /// Reads and structurally revalidates a cached Component.
    pub fn get(&self, key: &[u8; 32]) -> Result<Option<Vec<u8>>, CacheError> {
        let path = self.entry_path(key);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CacheError::Io(format!("{}: {error}", path.display()))),
        };
        if wasmparser::Validator::new().validate_all(&bytes).is_err() {
            return Err(CacheError::Corrupt(path));
        }
        Ok(Some(bytes))
    }

    /// Commits a compiled Component: create-new temporary artifact, atomic
    /// rename where supported. A byte-identical completed entry is reused; a
    /// conflicting or corrupt entry is never overwritten.
    pub fn put(&self, key: &[u8; 32], bytes: &[u8]) -> Result<PathBuf, CacheError> {
        let path = self.entry_path(key);
        if let Ok(existing) = fs::read(&path) {
            return if existing == bytes {
                Ok(path)
            } else {
                Err(CacheError::Conflict(path))
            };
        }
        let parent = path.parent().expect("entry path has a parent");
        fs::create_dir_all(parent)
            .map_err(|error| CacheError::Io(format!("{}: {error}", parent.display())))?;
        let temporary = parent.join(format!(
            ".entry-{}-{}.tmp",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| CacheError::Io(format!("{}: {error}", temporary.display())))?;
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&temporary);
            return Err(CacheError::Io(format!("{}: {error}", temporary.display())));
        }
        drop(file);
        match fs::rename(&temporary, &path) {
            Ok(()) => Ok(path),
            Err(_) if path.exists() => {
                let _ = fs::remove_file(&temporary);
                self.put(key, bytes)
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(CacheError::Io(format!("{}: {error}", path.display())))
            }
        }
    }

    /// The on-disk path of an entry, for passing to the runner.
    #[must_use]
    pub fn entry_for(&self, key: &[u8; 32]) -> PathBuf {
        self.entry_path(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(source: &[u8]) -> CacheKeyParts<'_> {
        CacheKeyParts {
            compiler_digest: &[1; 32],
            compiler_build_id: "0.0.2-dev",
            semantics_id: "sico.ir.v0",
            script_wit_version: "sico:script@0.1.0",
            adapter_digest: &[2; 32],
            source,
            app_id: "sico",
            app_version: "0.0.2-dev",
            profile_id: "script-v0",
            manifest_schema: "sico.sapp.manifest.v1",
            codegen_options: &[],
        }
    }

    #[test]
    fn key_is_deterministic_and_source_sensitive() {
        assert_eq!(cache_key(&parts(b"a")), cache_key(&parts(b"a")));
        assert_ne!(cache_key(&parts(b"a")), cache_key(&parts(b"b")));
    }

    fn valid_component() -> Vec<u8> {
        let source = sico_source::SourceFile::from_text(
            sico_source::SourceId::new(1),
            "answer.sico",
            "function main() returns Int:\n  return 42\nend function\n",
        )
        .unwrap();
        sico_codegen_wasm::compile_component(&sico_ir::lower_core(&source).unwrap()).unwrap()
    }

    #[test]
    fn put_get_conflict_and_corruption() {
        let dir = std::env::temp_dir().join(format!("sico-cache-test-{}", std::process::id()));
        let cache = SourceCache { dir: dir.clone() };
        let key = cache_key(&parts(b"source"));
        let component = valid_component();
        assert_eq!(cache.get(&key).unwrap(), None);
        let path = cache.put(&key, &component).unwrap();
        assert_eq!(cache.get(&key).unwrap(), Some(component.clone()));
        // byte-identical entries are reused
        assert_eq!(cache.put(&key, &component).unwrap(), path);
        // conflicting content under the same key fails closed
        assert!(matches!(
            cache.put(&key, b"\0asm\x01"),
            Err(CacheError::Conflict(_))
        ));
        // corrupt entries fail closed and are never executed
        fs::write(&path, b"not-wasm").unwrap();
        assert!(matches!(cache.get(&key), Err(CacheError::Corrupt(_))));
        let _ = fs::remove_dir_all(&dir);
    }
}
