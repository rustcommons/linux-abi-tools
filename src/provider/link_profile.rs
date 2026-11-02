//! Immutable one-library overlays derived from actual selected ELF inputs.
//!
//! The caller owns library selection and sysroot/provider containment checks.
//! This module never edits the inputs, changes descriptors, generates code or
//! reorders linker arguments. Unqualified or malformed inputs return errors.
use super::json::{self, Value};
use crate::{elf, sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// An explicitly requested, versioned derived link policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkProfile {
    /// Retain the five librt clock versions as compatibility-only, requiring
    /// libc to supply their GLIBC_2.17 defaults. No other ELF bytes change.
    LibcClocksV1,
}
impl LinkProfile {
    /// Stable identity for cache keys, fingerprints and exported configuration.
    pub fn id(self) -> &'static str {
        match self {
            Self::LibcClocksV1 => "libc-clocks-v1",
        }
    }
}

/// The result of validating selected libraries and preparing a derived overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Overlay {
    /// Search prefix containing only `librt.so` and `librt.so.1`, or `None` when
    /// no default clock flags need changing. A no-op creates no cache directory.
    pub search_dir: Option<PathBuf>,
    /// SHA-256 of the actual selected libc bytes.
    pub libc_sha256: String,
    /// SHA-256 of the original selected librt bytes, never of a mutated input.
    pub librt_sha256: String,
    /// The explicitly requested policy, including for a no-op.
    pub profile: LinkProfile,
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("{}: input exceeds limit", path.display()));
    }
    Ok(bytes)
}
fn hash(bytes: &[u8]) -> String {
    sha256::hex(&sha256::sha256(bytes))
}
fn manifest(profile: LinkProfile, c: &str, rt: &str, derived: &[u8], generation: &str) -> Value {
    Value::Object(vec![
        ("generation".into(), Value::String(generation.into())),
        ("policy".into(), Value::String(profile.id().into())),
        ("libc_sha256".into(), Value::String(c.into())),
        ("librt_sha256".into(), Value::String(rt.into())),
        ("overlay_sha256".into(), Value::String(hash(derived))),
    ])
}
fn verify(dir: &Path, expected: &Value, bytes: &[u8]) -> Result<(), String> {
    if !fs::symlink_metadata(dir)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_dir()
    {
        return Err("clock overlay entry must be a directory, not a link".into());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            return Err("clock overlay files must be regular files".into());
        }
        names.push(entry.file_name());
    }
    names.sort();
    if names != ["librt.so", "librt.so.1", "manifest.json"].map(std::ffi::OsString::from) {
        return Err("clock overlay inventory differs".into());
    }
    let text = read(&dir.join("manifest.json"), 4096)?;
    if json::parse(std::str::from_utf8(&text).map_err(|e| e.to_string())?)? != *expected
        || read(&dir.join("librt.so"), elf::MAX_FILE_BYTES)? != bytes
        || read(&dir.join("librt.so.1"), elf::MAX_FILE_BYTES)? != bytes
    {
        return Err("clock overlay does not match its content identity".into());
    }
    Ok(())
}
struct Generation {
    path: PathBuf,
    published: bool,
}
impl Drop for Generation {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn publish(generation: &mut Generation, index: &Path) -> Result<(), std::io::Error> {
    // hard_link never replaces an existing destination, including an empty dir.
    fs::hard_link(generation.path.join("manifest.json"), index)?;
    // Nothing fallible may run between publication and persistence ownership.
    generation.published = true;
    Ok(())
}

fn indexed(
    index: &Path,
    profile: LinkProfile,
    c: &str,
    rt: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    if !fs::symlink_metadata(index)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_file()
    {
        return Err("clock overlay index must be a regular file".into());
    }
    let text = read(index, 4096)?;
    let value = json::parse(std::str::from_utf8(&text).map_err(|e| e.to_string())?)?;
    let name = value.str("generation")?;
    let valid = name
        .strip_prefix(".gen-")
        .and_then(|s| s.split_once('-'))
        .is_some_and(|(pid, serial)| {
            !pid.is_empty()
                && !serial.is_empty()
                && pid.bytes().all(|c| c.is_ascii_digit())
                && serial.bytes().all(|c| c.is_ascii_digit())
        });
    if name.len() > 128 || !valid {
        return Err("invalid single-component clock generation name".into());
    }
    let expected = manifest(profile, c, rt, bytes, name);
    if value != expected {
        return Err("clock overlay index does not match its content identity".into());
    }
    let dir = index.parent().ok_or("index lacks parent")?.join(name);
    verify(&dir, &expected, bytes)?;
    Ok(dir)
}

/// Validates actual selected libc/librt ELF files and prepares an immutable copy.
///
/// Only GNU versym non-default bits for the five librt clock definitions may be
/// set; symbols, versions, code, aliases, dependencies and all other bytes stay
/// unchanged. Both files must be bounded x86_64 ELF64 DSOs with consistent loader
/// and section tables. libc must provide all five GLIBC_2.17 defaults.
///
/// Cache identity binds PROFILE and both original SHA-256s. Existing entries are
/// verified, never overwritten or repaired. A complete private generation is
/// published by hard-linking its manifest to a create-new content-key index.
/// Losing writers verify the winner and remove only their unpublished generation. Malformed input
/// or an inconsistent cache entry returns an error without fallback.
///
/// The caller must resolve actual ELF files (not linker scripts), prove selected
/// sysroot/provider containment, and prefix the returned directory for `-lrt` or
/// `-l:librt.so[.1]`. Absolute paths/scripts need identity-checked same-position
/// substitution or explicit refusal. Never reorder arguments or mutate user files.
pub fn prepare(
    libc: &Path,
    librt: &Path,
    cache_root: &Path,
    profile: LinkProfile,
) -> Result<Overlay, String> {
    let c = read(libc, elf::MAX_FILE_BYTES)?;
    let rt = read(librt, elf::MAX_FILE_BYTES)?;
    let mut result = Overlay {
        search_dir: None,
        libc_sha256: hash(&c),
        librt_sha256: hash(&rt),
        profile,
    };
    let derived = match profile {
        LinkProfile::LibcClocksV1 => elf::libc_clock_copy(&c, &rt)?,
    };
    let Some(derived) = derived else {
        return Ok(result);
    };
    let identity = format!(
        "{}\n{}\n{}\n",
        profile.id(),
        result.libc_sha256,
        result.librt_sha256
    );
    let parent = cache_root.join("clock-link-profiles").join(profile.id());
    let index = parent.join(hash(identity.as_bytes()));
    match fs::symlink_metadata(&index) {
        Ok(_) => {
            result.search_dir = Some(indexed(
                &index,
                profile,
                &result.libc_sha256,
                &result.librt_sha256,
                &derived,
            )?);
            return Ok(result);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut generation = None;
    for _ in 0..32 {
        let path = parent.join(format!(
            ".gen-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                generation = Some(Generation {
                    path,
                    published: false,
                });
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    let mut generation = generation.ok_or("cannot allocate clock overlay generation")?;
    let name = generation
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid generation name")?;
    let expected = manifest(
        profile,
        &result.libc_sha256,
        &result.librt_sha256,
        &derived,
        name,
    );
    let manifest_text = expected.to_text();
    for (name, bytes) in [
        ("librt.so", derived.as_slice()),
        ("librt.so.1", derived.as_slice()),
        ("manifest.json", manifest_text.as_bytes()),
    ] {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(generation.path.join(name))
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    verify(&generation.path, &expected, &derived)?;
    if let Err(error) = publish(&mut generation, &index) {
        if fs::symlink_metadata(&index).is_err() {
            return Err(error.to_string());
        }
    }
    result.search_dir = Some(indexed(
        &index,
        profile,
        &result.libc_sha256,
        &result.librt_sha256,
        &derived,
    )?);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temporary() -> Generation {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "clock-index-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Generation {
            path,
            published: false,
        }
    }
    fn complete(parent: &Path, name: &str) -> Generation {
        let path = parent.join(name);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("manifest.json"), b"complete").unwrap();
        Generation {
            path,
            published: false,
        }
    }
    #[test]
    fn racing_empty_directory_is_not_replaced_or_removed() {
        let root = temporary();
        let mut generation = complete(&root.path, ".gen-1-1");
        let index = root.path.join("index");
        assert!(!index.exists());
        // Equivalent to a concurrent mkdir after prepare's absent-index check.
        fs::create_dir(&index).unwrap();
        assert!(publish(&mut generation, &index).is_err());
        let own = generation.path.clone();
        drop(generation);
        assert!(!own.exists());
        assert!(index.is_dir());
        assert_eq!(fs::read_dir(index).unwrap().count(), 0);
    }
    #[test]
    fn existing_file_is_not_replaced_and_winner_survives_loser_cleanup() {
        let root = temporary();
        let index = root.path.join("index");
        let mut winner = complete(&root.path, ".gen-1-1");
        publish(&mut winner, &index).unwrap();
        assert!(winner.published);
        let winner_path = winner.path.clone();
        drop(winner);
        assert!(winner_path.is_dir());
        let mut loser = complete(&root.path, ".gen-1-2");
        assert!(publish(&mut loser, &index).is_err());
        let loser_path = loser.path.clone();
        drop(loser);
        assert!(!loser_path.exists());
        assert!(winner_path.is_dir());
        assert_eq!(fs::read(index).unwrap(), b"complete");
    }
}
