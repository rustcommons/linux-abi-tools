//! Local immutable generations. Selection files change; returned paths do not.
use super::cache::{self, Entry};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const FORMAT: &str = "libc-abi-provider-selection-1";

/// Enumerate logical keys without trusting or following selection contents.
/// This also makes explicit removal of corrupt/orphaned selections possible.
pub(super) fn candidates(root: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for release in super::RELEASES {
        let parent = cache::release_dir(root, release);
        let read = match fs::read_dir(&parent) {
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        let mut keys = BTreeSet::new();
        for item in read {
            let item = item.map_err(|e| e.to_string())?;
            let file_name = item.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            let generation = name.strip_prefix(".gen-");
            let name = generation.unwrap_or_else(|| name.strip_prefix(".current-").unwrap_or(name));
            let Some((mode, tail)) = name.split_once('-') else {
                continue;
            };
            let hash = if generation.is_some() {
                tail.split('-').next().unwrap_or("")
            } else {
                tail
            };
            let Ok(mode) = mode.parse::<cache::Mode>() else {
                continue;
            };
            if !hash.is_empty()
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                keys.insert((mode, hash.to_owned()));
            }
        }
        entries.extend(
            keys.into_iter()
                .map(|(mode, hash)| cache::entry(root, release, mode, &hash)),
        );
    }
    Ok(entries)
}

fn name(entry: &Entry) -> String {
    format!("{}-{}", entry.mode, entry.hash)
}

fn index(entry: &Entry) -> PathBuf {
    entry
        .dir
        .with_file_name(format!(".current-{}", name(entry)))
}

pub(super) fn allocate(entry: &Entry) -> Result<PathBuf, String> {
    let parent = entry.dir.parent().ok_or("provider entry lacks parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..128 {
        let path = parent.join(format!(
            ".gen-{}-{}-{}",
            name(entry),
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Err("cannot allocate a unique provider generation".into())
}

fn checked(entry: Entry) -> Result<Entry, String> {
    let regular = |path: &Path, directory: bool| -> Result<(), String> {
        let kind = fs::symlink_metadata(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .file_type();
        if (directory && kind.is_dir()) || (!directory && kind.is_file()) {
            Ok(())
        } else {
            Err(format!(
                "{}: expected a regular {}",
                path.display(),
                if directory { "directory" } else { "file" }
            ))
        }
    };
    regular(&entry.dir, true)?;
    regular(&entry.descriptor(), false)?;
    regular(&entry.dir.join("manifest.json"), false)?;
    let manifest = entry.manifest()?;
    if manifest.str("format")? != cache::MANIFEST_FORMAT
        || manifest.str("target")? != cache::TARGET
        || manifest.str("glibc")? != entry.release
        || manifest.str("mode")? != entry.mode.name()
        || manifest.str("hash")? != entry.hash
    {
        return Err(format!(
            "{}: selection manifest identity mismatch",
            entry.dir.display()
        ));
    }
    Ok(entry)
}

pub(super) fn current(entry: &Entry) -> Result<Option<Entry>, String> {
    let path = index(entry);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Preserve legacy discovery semantics; full payload checking remains
            // the explicit verify operation. Never move an old directory.
            return Ok(entry
                .dir
                .join("manifest.json")
                .is_file()
                .then(|| entry.clone()));
        }
        Err(e) => return Err(e.to_string()),
        Ok(meta) if !meta.is_file() || meta.len() > 512 => {
            return Err(format!(
                "{}: invalid provider selection file",
                path.display()
            ))
        }
        Ok(_) => {}
    }
    // Opening once gives one old/new selection snapshot even if it is replaced.
    let mut text = String::new();
    File::open(&path)
        .map_err(|e| e.to_string())?
        .take(513)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    let lines: Vec<_> = text.lines().collect();
    if text.len() > 512 || lines.len() != 2 || lines[0] != FORMAT {
        return Err(format!(
            "{}: invalid provider selection format",
            path.display()
        ));
    }
    let generation = lines[1];
    let prefix = format!(".gen-{}-", name(entry));
    let valid = generation.strip_prefix(&prefix).is_some_and(|suffix| {
        let Some((pid, counter)) = suffix.split_once('-') else {
            return false;
        };
        !pid.is_empty()
            && !counter.is_empty()
            && pid.bytes().all(|b| b.is_ascii_digit())
            && counter.bytes().all(|b| b.is_ascii_digit())
    });
    if !valid {
        return Err(format!(
            "{}: invalid provider generation name",
            path.display()
        ));
    }
    let mut selected = entry.clone();
    selected.dir = entry.dir.with_file_name(generation);
    checked(selected).map(Some)
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub(super) fn publish(entry: &Entry, generation: &Path, force: bool) -> Result<Entry, String> {
    if !force {
        if let Some(existing) = current(entry)? {
            return Ok(existing);
        }
    }
    let mut result = entry.clone();
    result.dir = generation.to_path_buf();
    let result = checked(result)?;
    let component = generation
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid generation path")?;
    let temporary = generation.with_file_name(format!(".select-{component}"));
    // Never clean up or truncate a name we did not successfully create.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("{}: {e}", temporary.display()))?;
    let temporary = Temporary(temporary);
    file.write_all(format!("{FORMAT}\n{component}\n").as_bytes())
        .map_err(|e| e.to_string())?;
    drop(file);
    let path = index(entry);
    if force {
        // Atomic replacement of a file, never removal/replacement of a generation.
        fs::rename(&temporary.0, &path)
            .map_err(|e| format!("cannot select {}: {e}", path.display()))?;
    } else if let Err(e) = fs::hard_link(&temporary.0, &path) {
        // A winning writer may have published while this generation was built.
        return current(entry)?.ok_or_else(|| format!("cannot publish {}: {e}", path.display()));
    }
    Ok(result)
}

/// Explicit removal is destructive and requires quiescent readers/writers.
pub(super) fn remove(entry: &Entry) -> Result<(), String> {
    let parent = entry.dir.parent().ok_or("entry lacks parent")?;
    let logical = name(entry);
    let index = parent.join(format!(".current-{logical}"));
    let removed = match fs::symlink_metadata(&index) {
        Ok(meta) if meta.is_dir() => cache::remove_tree(&index).map_err(std::io::Error::other),
        _ => fs::remove_file(index),
    };
    match removed {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    cache::remove_tree(&parent.join(&logical))?;
    let prefix = format!(".gen-{logical}-");
    for item in fs::read_dir(parent).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        if item
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with(&prefix))
        {
            if item.file_type().map_err(|e| e.to_string())?.is_dir() {
                cache::remove_tree(&item.path())?;
            } else {
                fs::remove_file(item.path()).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
