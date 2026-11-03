use crate::sha256;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub(super) fn hash(bytes: &[u8]) -> String {
    sha256::hex(&sha256::sha256(bytes))
}
pub(super) fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .is_file()
    {
        return Err(format!(
            "{}: expected regular file, not symlink",
            path.display()
        ));
    }
    let mut data = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > limit {
        return Err(format!("{}: input exceeds limit", path.display()));
    }
    Ok(data)
}
pub(super) fn real_dir(path: &Path) -> Result<PathBuf, String> {
    path.to_str().ok_or("runtime paths must be UTF-8")?;
    // Reject links in the entire caller-supplied absolute path, not only its leaf.
    if !path.is_absolute() {
        return Err("runtime paths must be absolute".into());
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir | Component::CurDir) {
            return Err("runtime paths must be normalized".into());
        }
        current.push(part);
        if !fs::symlink_metadata(&current)
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            return Err(format!(
                "{}: expected directory, not symlink",
                current.display()
            ));
        }
    }
    Ok(path.to_path_buf())
}
pub(super) fn tool(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("tool paths must be absolute".into());
    }
    // Explicit tool aliases (e.g. clang -> clang-18) resolve once, then identity
    // includes the resolved executable, its bytes and invocation output.
    let p = path.canonicalize().map_err(|e| e.to_string())?;
    file_identity(&p)?;
    Ok(p)
}
// Validate native components without separator replacement, lexical folding or
// lossy Unicode conversion. Reject non-canonical spelling as well as traversal.
fn relative_key(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("empty snapshot key".into());
    }
    let mut rebuilt = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Normal(name) => {
                name.to_str().ok_or("snapshot name not UTF-8")?;
                rebuilt.push(name);
            }
            _ => return Err("snapshot key must contain only relative normal components".into()),
        }
    }
    if rebuilt.as_os_str() != path.as_os_str() {
        return Err("noncanonical snapshot key".into());
    }
    Ok(())
}
fn destination_root(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("snapshot destination must be absolute".into());
    }
    let mut rebuilt = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir | Component::CurDir) {
            return Err("snapshot destination must be normalized".into());
        }
        rebuilt.push(part);
    }
    if rebuilt.as_os_str() != path.as_os_str() {
        return Err("noncanonical snapshot root".into());
    }
    Ok(())
}
fn create_directories(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("snapshot root must be absolute".into());
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir | Component::CurDir) {
            return Err("snapshot destination must be normalized".into());
        }
        current.push(part);
        match fs::create_dir(&current) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
        if !fs::symlink_metadata(&current)
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            return Err("snapshot destination directory must not be a symlink/file".into());
        }
    }
    Ok(())
}
#[derive(Clone)]
pub(super) struct Tree(pub BTreeMap<PathBuf, Vec<u8>>);
impl Tree {
    pub fn load(root: &Path) -> Result<Self, String> {
        real_dir(root)?;
        let mut map = BTreeMap::new();
        let mut total = 0;
        fn visit(
            root: &Path,
            dir: &Path,
            map: &mut BTreeMap<PathBuf, Vec<u8>>,
            total: &mut usize,
            depth: usize,
        ) -> Result<(), String> {
            if depth > 32 {
                return Err("header tree depth exceeds 32".into());
            }
            for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let ty = entry.file_type().map_err(|e| e.to_string())?;
                if ty.is_dir() {
                    visit(root, &entry.path(), map, total, depth + 1)?;
                } else if ty.is_file() {
                    let path = entry.path();
                    let rel = path
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_path_buf();
                    relative_key(&rel)?;
                    if map.len() >= 10000 {
                        return Err("header tree exceeds 10000 files".into());
                    }
                    let data = read(&path, 8 * 1024 * 1024)?;
                    *total += data.len();
                    if *total > 64 * 1024 * 1024 {
                        return Err("header tree exceeds 64 MiB".into());
                    }
                    if map.insert(rel, data).is_some() {
                        return Err("duplicate native snapshot key".into());
                    }
                } else {
                    return Err(
                        "header trees require regular files/directories, no symlinks".into(),
                    );
                }
            }
            Ok(())
        }
        visit(root, root, &mut map, &mut total, 0)?;
        if map.is_empty() {
            return Err("empty header tree".into());
        }
        Ok(Self(map))
    }
    pub fn identity(&self) -> String {
        // Native component bytes are length-prefixed; literal Unix backslashes
        // remain filename bytes and can never become separators during replay.
        let mut data = b"header-tree-native-components-v1\n".to_vec();
        for (path, bytes) in &self.0 {
            for component in path.components() {
                let b = component.as_os_str().as_encoded_bytes();
                data.extend_from_slice(format!("{}:", b.len()).as_bytes());
                data.extend_from_slice(b);
            }
            data.extend_from_slice(format!("={}:{}\n", bytes.len(), hash(bytes)).as_bytes());
        }
        hash(&data)
    }
    pub fn write(&self, root: &Path) -> Result<(), String> {
        destination_root(root)?;
        // Preflight the complete inventory before creating even the root. Never
        // normalize an input key or overwrite a colliding destination.
        let mut seen = BTreeSet::new();
        for path in self.0.keys() {
            relative_key(path)?;
            let dest = root.join(path);
            if !dest.starts_with(root) || !seen.insert(dest) {
                return Err("snapshot destination collision/escape".into());
            }
            for parent in path.ancestors().skip(1) {
                if self.0.contains_key(parent) {
                    return Err("snapshot file/directory collision".into());
                }
            }
        }
        create_directories(root)?;
        for (path, bytes) in &self.0 {
            let dest = root.join(path);
            let parent = dest.parent().ok_or("snapshot parent")?;
            create_directories(parent)?;
            // Existing regular files, case-fold aliases and symlinks all refuse.
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dest)
                .and_then(|mut f| f.write_all(bytes))
                .map_err(|e| format!("{}: {e}", dest.display()))?;
        }
        Ok(())
    }
}

// Domain-separated SHA-256 over fixed 1 MiB chunk hashes, lengths and total size.
// This bounds memory for large compiler executables/dependencies; it is NOT the
// conventional SHA-256 of the full file, and manifests name it content_identity.
pub(super) fn file_identity(path: &Path) -> Result<String, String> {
    path.to_str()
        .ok_or("runtime identity paths must be UTF-8")?;
    if !fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("identity input must be regular, not symlink".into());
    }
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1024 * 1024];
    let mut all = String::from("runtime-file-chunks-v1\n");
    let mut total = 0u64;
    loop {
        let mut used = 0;
        while used < buf.len() {
            let n = f.read(&mut buf[used..]).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            used += n;
        }
        if used == 0 {
            break;
        }
        total += used as u64;
        if total > 1024 * 1024 * 1024 {
            return Err("tool identity file exceeds 1 GiB".into());
        }
        all.push_str(&format!("{}:{}\n", used, hash(&buf[..used])));
    }
    all.push_str(&format!("total:{total}\n"));
    Ok(hash(all.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "runtime-inputs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn contents_not_timestamp_bind_identity() {
        let p = temp();
        fs::write(p.join("x.h"), b"one").unwrap();
        let a = Tree::load(&p).unwrap().identity();
        let f = file_identity(&p.join("x.h")).unwrap();
        fs::write(p.join("x.h"), b"two").unwrap();
        assert_ne!(a, Tree::load(&p).unwrap().identity());
        assert_ne!(f, file_identity(&p.join("x.h")).unwrap());
        assert!(read(&p.join("x.h"), 2).is_err());
        fs::remove_dir_all(p).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn header_and_parent_symlinks_refuse() {
        let p = temp();
        fs::write(p.join("actual"), b"one").unwrap();
        std::os::unix::fs::symlink(p.join("actual"), p.join("alias")).unwrap();
        assert!(Tree::load(&p).is_err());
        assert!(read(&p.join("alias"), 10).is_err());
        fs::remove_file(p.join("alias")).unwrap();
        std::os::unix::fs::symlink(&p, p.join("loop")).unwrap();
        assert!(real_dir(&p.join("loop")).is_err());
        fs::remove_dir_all(p).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn unix_backslashes_preserve_native_names_in_header_and_resource_staging() {
        let p = temp();
        let input = p.join("input");
        fs::create_dir(&input).unwrap();
        fs::create_dir(input.join("a")).unwrap();
        for (name, bytes) in [
            (r"..\escaped.h", b"literal traversal spelling".as_slice()),
            (r"a\b.h", b"literal backslash".as_slice()),
            ("a/b.h", b"real separator".as_slice()),
            (r"\leading.h", b"literal leading backslash".as_slice()),
            (r"C:\drive.h", b"literal Unix drive spelling".as_slice()),
        ] {
            fs::write(input.join(name), bytes).unwrap();
        }
        let tree = Tree::load(&input).unwrap();
        assert_eq!(tree.0.len(), 5);
        assert!(tree.0.contains_key(Path::new(r"..\escaped.h")));
        let before = tree.identity();
        for dest in [p.join("headers/usr/include"), p.join("resources/include")] {
            tree.write(&dest).unwrap();
            assert_eq!(Tree::load(&dest).unwrap().identity(), before);
            assert_eq!(
                fs::read(dest.join(r"..\escaped.h")).unwrap(),
                b"literal traversal spelling"
            );
            assert_eq!(fs::read(dest.join(r"a\b.h")).unwrap(), b"literal backslash");
            assert_eq!(fs::read(dest.join("a/b.h")).unwrap(), b"real separator");
            assert!(!dest.parent().unwrap().join("escaped.h").exists());
        }
        assert!(!p.join("escaped.h").exists());
        assert_eq!(Tree::load(&input).unwrap().identity(), before);
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn forged_traversal_absolute_and_normalized_keys_refuse_before_writes() {
        let p = temp();
        let absolute = p.join("absolute-escape.h");
        let mut bad = vec![
            PathBuf::new(),
            PathBuf::from("."),
            PathBuf::from(".."),
            PathBuf::from("../escape.h"),
            PathBuf::from("a/../../escape.h"),
            PathBuf::from("a/./b.h"),
            PathBuf::from("a//b.h"),
            PathBuf::from("a/"),
            absolute.clone(),
        ];
        #[cfg(windows)]
        bad.extend([
            PathBuf::from(r"..\escape.h"),
            PathBuf::from(r"C:\absolute.h"),
            PathBuf::from(r"C:relative.h"),
            PathBuf::from(r"\\server\share\file.h"),
            PathBuf::from(r"\rooted.h"),
        ]);
        for (i, key) in bad.drain(..).enumerate() {
            let tree = Tree(BTreeMap::from([(key, b"must not write".to_vec())]));
            for kind in ["headers", "resources"] {
                let root = p.join(format!("{kind}-{i}"));
                assert!(tree.write(&root).is_err());
                assert!(!root.exists());
            }
        }
        let mixed = Tree(BTreeMap::from([
            (PathBuf::from("safe.h"), b"valid first".to_vec()),
            (PathBuf::from("z/../../escape.h"), b"invalid later".to_vec()),
        ]));
        let root = p.join("mixed");
        assert!(mixed.write(&root).is_err());
        assert!(!root.exists());
        assert!(!absolute.exists());
        assert_eq!(fs::read_dir(&p).unwrap().count(), 0);
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn file_directory_collisions_and_existing_destinations_never_overwrite() {
        let p = temp();
        let collision = Tree(BTreeMap::from([
            (PathBuf::from("a"), b"file".to_vec()),
            (PathBuf::from("a").join("b"), b"child".to_vec()),
        ]));
        let root = p.join("collision");
        assert!(collision.write(&root).is_err());
        assert!(!root.exists());
        let root = p.join("existing");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("header.h"), b"keep").unwrap();
        let tree = Tree(BTreeMap::from([(
            PathBuf::from("header.h"),
            b"replace".to_vec(),
        )]));
        assert!(tree.write(&root).is_err());
        assert_eq!(fs::read(root.join("header.h")).unwrap(), b"keep");
        let badroot = p.join("new").join("..").join("escape");
        assert!(tree.write(&badroot).is_err());
        assert!(!p.join("new").exists());
        fs::remove_dir_all(p).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn destination_symlinks_refuse_without_following_or_mutating_source() {
        let p = temp();
        let outside = p.join("outside");
        let root = p.join("snapshot");
        fs::create_dir(&outside).unwrap();
        fs::create_dir(&root).unwrap();
        fs::write(outside.join("sentinel.h"), b"keep").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("dir")).unwrap();
        let tree = Tree(BTreeMap::from([(
            PathBuf::from("dir").join("escape.h"),
            b"bad".to_vec(),
        )]));
        assert!(tree.write(&root).is_err());
        assert!(!outside.join("escape.h").exists());
        std::os::unix::fs::symlink(outside.join("sentinel.h"), root.join("alias.h")).unwrap();
        let tree = Tree(BTreeMap::from([(
            PathBuf::from("alias.h"),
            b"bad".to_vec(),
        )]));
        assert!(tree.write(&root).is_err());
        assert_eq!(fs::read(outside.join("sentinel.h")).unwrap(), b"keep");
        fs::remove_dir_all(p).unwrap();
    }
}
