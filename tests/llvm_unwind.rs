//! Explicit opt-in source-build gate; never executes the target archive.
use libc_abi_tools::runtime::llvm_unwind::*;
use std::{fs, path::PathBuf};
fn required(name: &str) -> PathBuf {
    std::env::var_os(name)
        .unwrap_or_else(|| panic!("set {name}"))
        .into()
}
#[test]
#[ignore = "requires explicitly selected Clang, archiver, headers/provider/tool identities; builds x86_64 objects only"]
fn selected_toolchain_archive_cache() {
    let cache = required("LLVM_UNWIND_TEST_CACHE");
    fs::create_dir(&cache).expect("fresh cache root required");
    let mut o = PrepareOptions {
        libc: SelectedFile {
            root: required("LLVM_UNWIND_TEST_LIBRARY_ROOT"),
            path: required("LLVM_UNWIND_TEST_PROVIDER"),
        },
        target: Target::X86_64LinuxGnu,
        profile: ArchiveProfile::PrivatePicV1,
        sysroot: required("LLVM_UNWIND_TEST_SYSROOT"),
        provider_files: vec![],
        toolchain: Toolchain {
            clang: required("LLVM_UNWIND_TEST_CLANG"),
            clangxx: required("LLVM_UNWIND_TEST_CLANGXX"),
            archiver: required("LLVM_UNWIND_TEST_AR"),
            dependency_files: std::env::var_os("LLVM_UNWIND_TEST_TOOL_FILES")
                .map(|v| std::env::split_paths(&v).collect())
                .unwrap_or_default(),
        },
        cache_root: cache.clone(),
    };
    // Simultaneous fresh writers must converge on one published generation.
    let a = o.clone();
    let b = o.clone();
    let a = std::thread::spawn(move || prepare(&a).unwrap());
    let b = std::thread::spawn(move || prepare(&b).unwrap());
    let a = a.join().unwrap();
    let b = b.join().unwrap();
    assert_eq!(a, b);
    assert_eq!(prepare(&o).unwrap(), a);
    let index = cache.join("llvm-unwind-archives").join(&a.identity);
    assert_eq!(fs::read(&index).unwrap(), fs::read(&a.manifest).unwrap());
    // A fresh independent cache must reproduce the archive, despite generation paths.
    let independent = cache.join("independent");
    fs::create_dir(&independent).unwrap();
    o.cache_root = independent;
    let repro = prepare(&o).unwrap();
    assert_eq!(a.identity, repro.identity);
    assert_eq!(a.archive_sha256, repro.archive_sha256);
    o.cache_root = cache.clone();
    o.profile = ArchiveProfile::ExportablePicV1;
    let exported = prepare(&o).unwrap();
    assert_ne!(a.identity, exported.identity);
    assert_ne!(a.archive_sha256, exported.archive_sha256);
    // Archive mutation refuses, rather than rebuilding or repairing.
    let bytes = fs::read(&a.archive).unwrap();
    let mut bad = bytes.clone();
    bad[8] ^= 1;
    fs::write(&a.archive, &bad).unwrap();
    o.profile = ArchiveProfile::PrivatePicV1;
    assert!(prepare(&o).is_err());
    assert_eq!(fs::read(&a.archive).unwrap(), bad);
    fs::write(&a.archive, bytes).unwrap();
    assert_eq!(prepare(&o).unwrap(), a);
    // Existing empty directory at the index must remain an empty directory.
    fs::remove_file(&index).unwrap();
    fs::create_dir(&index).unwrap();
    assert!(prepare(&o).is_err());
    assert_eq!(fs::read_dir(&index).unwrap().count(), 0);
    // Preserve output cache for package/source evidence; caller owns cleanup.
}
