//! Single-release operations must not resolve unrelated cached selections.
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..128 {
            let path = std::env::temp_dir().join(format!(
                "lat-provider-selection-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("{e}"),
            }
        }
        panic!("no unique fixture directory")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn export_filters_release_and_mode_before_resolving_selections() {
    for (release, mode) in [("2.28", "source"), ("2.17", "container")] {
        let temp = Scratch::new();
        let root = temp.0.join("cache");
        let parent = root.join("x86_64-unknown-linux-gnu/glibc-2.17");
        let entry = parent.join("source-a");
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("manifest.json"), b"{}\n").unwrap();
        fs::write(entry.join("provider.txt"), b"legacy export fixture\n").unwrap();
        let corrupt = root.join(format!(
            "x86_64-unknown-linux-gnu/glibc-{release}/.current-{mode}-b"
        ));
        fs::create_dir_all(corrupt.parent().unwrap()).unwrap();
        fs::write(&corrupt, b"invalid selection\n").unwrap();
        let export = temp.0.join("export");
        let result = Command::new(env!("CARGO_BIN_EXE_libc-abi"))
            .args([
                "provider", "export", "--glibc", "2.17", "--mode", "source", "--cache",
            ])
            .arg(&root)
            .arg("--to")
            .arg(&export)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(export.join("glibc-2.17/source-a/manifest.json").is_file());
        assert_eq!(fs::read(&corrupt).unwrap(), b"invalid selection\n");
    }
}
