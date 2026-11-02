use crate::sha256;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
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
#[derive(Clone)]
pub(super) struct Tree(pub BTreeMap<String, Vec<u8>>);
impl Tree {
    pub fn load(root: &Path) -> Result<Self, String> {
        real_dir(root)?;
        let mut map = BTreeMap::new();
        let mut total = 0;
        fn visit(
            root: &Path,
            dir: &Path,
            map: &mut BTreeMap<String, Vec<u8>>,
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
                        .to_str()
                        .ok_or("header name not UTF-8")?
                        .replace('\\', "/");
                    if map.len() >= 10000 {
                        return Err("header tree exceeds 10000 files".into());
                    }
                    let data = read(&path, 8 * 1024 * 1024)?;
                    *total += data.len();
                    if *total > 64 * 1024 * 1024 {
                        return Err("header tree exceeds 64 MiB".into());
                    }
                    map.insert(rel, data);
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
        let mut data = String::new();
        for (p, b) in &self.0 {
            data.push_str(&format!("{}:{}:{}\n", p.len(), p, hash(b)));
        }
        hash(data.as_bytes())
    }
    pub fn write(&self, root: &Path) -> Result<(), String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        for (p, b) in &self.0 {
            let p = root.join(p);
            fs::create_dir_all(p.parent().ok_or("missing parent")?).map_err(|e| e.to_string())?;
            fs::write(p, b).map_err(|e| e.to_string())?;
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
}
