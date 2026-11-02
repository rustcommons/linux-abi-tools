use libc_abi_tools::{
    elf::Metadata,
    provider::link_profile::{prepare, LinkProfile},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
const C: &[u8] = include_bytes!("fixtures/clock-profile/c.so");
const RT: &[u8] = include_bytes!("fixtures/clock-profile/rt.so");
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "clock-profile-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn inputs(&self) {
        fs::write(self.0.join("c"), C).unwrap();
        fs::write(self.0.join("rt"), RT).unwrap();
    }
    fn prepare(&self) -> Result<libc_abi_tools::provider::link_profile::Overlay, String> {
        prepare(
            &self.0.join("c"),
            &self.0.join("rt"),
            &self.0.join("cache"),
            LinkProfile::LibcClocksV1,
        )
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn u64_at(b: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}
fn section(b: &[u8], kind: u32) -> usize {
    let off = u64_at(b, 40) as usize;
    (0..u16::from_le_bytes(b[60..62].try_into().unwrap()) as usize)
        .map(|i| off + i * 64)
        .find(|&p| u32::from_le_bytes(b[p + 4..p + 8].try_into().unwrap()) == kind)
        .unwrap()
}
#[test]
fn only_five_version_bits_change_and_inputs_are_untouched() {
    let t = Temp::new();
    t.inputs();
    let out = t.prepare().unwrap();
    assert_eq!(out.profile.id(), "libc-clocks-v1");
    let dir = out.search_dir.unwrap();
    let derived = fs::read(dir.join("librt.so")).unwrap();
    let diffs: Vec<_> = RT
        .iter()
        .zip(&derived)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .collect();
    assert_eq!(diffs.len(), 5);
    let vs = section(RT, 0x6fffffff);
    let start = u64_at(RT, vs + 24) as usize;
    let end = start + u64_at(RT, vs + 32) as usize;
    for (i, (a, b)) in diffs {
        assert!(i >= start && i < end && (i - start) % 2 == 1);
        assert_eq!(*a ^ *b, 0x80);
    }
    let before = Metadata::parse(RT).unwrap();
    let after = Metadata::parse(&derived).unwrap();
    for (a, b) in before.symbols.iter().zip(&after.symbols) {
        let mut expected = a.clone();
        if a.name.starts_with("clock_") {
            expected.default_version = Some(false);
        }
        assert_eq!(&expected, b);
    }
    assert_eq!(fs::read(t.0.join("c")).unwrap(), C);
    assert_eq!(fs::read(t.0.join("rt")).unwrap(), RT);
    assert_eq!(fs::read(dir.join("librt.so.1")).unwrap(), derived);
    assert_eq!(t.prepare().unwrap().search_dir, Some(dir.clone()));
    fs::write(dir.join("librt.so"), b"corrupt").unwrap();
    assert!(t.prepare().unwrap_err().contains("content identity"));
    assert_eq!(fs::read(dir.join("librt.so")).unwrap(), b"corrupt");
}
#[test]
fn compatibility_only_input_is_noop_and_creates_no_cache() {
    let first = Temp::new();
    first.inputs();
    let dir = first.prepare().unwrap().search_dir.unwrap();
    let t = Temp::new();
    t.inputs();
    fs::copy(dir.join("librt.so"), t.0.join("rt")).unwrap();
    assert!(t.prepare().unwrap().search_dir.is_none());
    assert!(!t.0.join("cache").exists());
}
#[test]
fn input_identity_changes_cache_and_corrupt_manifest_refuses() {
    let t = Temp::new();
    t.inputs();
    let first = t.prepare().unwrap();
    let mut padded = C.to_vec();
    padded.extend_from_slice(b"padding");
    fs::write(t.0.join("c"), padded).unwrap();
    let second = t.prepare().unwrap();
    assert_ne!(first.libc_sha256, second.libc_sha256);
    assert_ne!(first.search_dir, second.search_dir);
    let dir = second.search_dir.unwrap();
    fs::write(dir.join("manifest.json"), b"{}").unwrap();
    assert!(t.prepare().is_err());
}
#[test]
fn rejects_scripts_wrong_soname_missing_defaults_and_loader_disagreement() {
    let t = Temp::new();
    t.inputs();
    fs::write(t.0.join("c"), b"GROUP ( /host/libc.so.6 )").unwrap();
    assert!(t.prepare().is_err());
    fs::write(t.0.join("c"), RT).unwrap();
    assert!(t.prepare().unwrap_err().contains("libc.so.6"));
    let mut missing = C.to_vec();
    let vs = section(C, 0x6fffffff);
    let at = u64_at(C, vs + 24) as usize;
    for b in missing[at..at + u64_at(C, vs + 32) as usize]
        .chunks_exact_mut(2)
        .skip(1)
    {
        b[1] |= 0x80;
    }
    fs::write(t.0.join("c"), missing).unwrap();
    assert!(t.prepare().unwrap_err().contains("requires libc"));
    t.inputs();
    let mut broken = RT.to_vec();
    let vs = section(RT, 0x6fffffff);
    broken[vs + 16] ^= 8;
    fs::write(t.0.join("rt"), broken).unwrap();
    assert!(t.prepare().is_err());
    assert!(!t.0.join("cache").exists());
}
#[test]
fn concurrent_writers_reuse_one_complete_entry() {
    let t = Temp::new();
    t.inputs();
    let results = std::thread::scope(|s| {
        let jobs: Vec<_> = (0..4).map(|_| s.spawn(|| t.prepare().unwrap())).collect();
        jobs.into_iter()
            .map(|j| j.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(results.iter().all(|r| r == &results[0]));
    let parent = results[0].search_dir.as_ref().unwrap().parent().unwrap();
    assert_eq!(fs::read_dir(parent).unwrap().count(), 1);
}
