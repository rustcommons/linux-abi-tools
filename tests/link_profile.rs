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
        .as_chunks_mut::<2>()
        .0
        .iter_mut()
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

/// Qualified recorded providers and external linkers are intentionally supplied
/// explicitly. This gate never downloads tools or substitutes host libraries.
#[test]
#[ignore = "requires CLOCK_PROFILE_STUBS, CLOCK_PROFILE_AS, CLOCK_PROFILE_GNU_LD and CLOCK_PROFILE_LLD"]
fn recorded_stubs_and_mixed_version_linkers() {
    use std::process::Command;
    fn run(cmd: &mut Command) -> String {
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "{cmd:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }
    let env = |name| std::env::var_os(name).unwrap_or_else(|| panic!("missing {name}"));
    let root = PathBuf::from(env("CLOCK_PROFILE_STUBS"));
    let t = Temp::new();
    let mut release22 = None;
    for minor in 17..=44 {
        let src = root.join(format!("2.{minor}"));
        let out = prepare(
            &src.join("libc.so.6"),
            &src.join("librt.so.1"),
            &t.0.join("cache"),
            LinkProfile::LibcClocksV1,
        )
        .unwrap_or_else(|e| panic!("2.{minor}: {e}"));
        assert_eq!(out.search_dir.is_some(), minor < 30, "2.{minor}");
        if minor == 22 {
            release22 = out.search_dir;
        }
    }
    let overlay = release22.unwrap();
    let src = root.join("2.22");
    let clocks = [
        "clock_gettime",
        "clock_getres",
        "clock_settime",
        "clock_nanosleep",
        "clock_getcpuclockid",
    ];
    for calls in [false, true] {
        for legacy_calls in [false, true] {
            let mut asm = String::from(".text\n.globl _start\n_start:\n");
            let mut data = String::from(".data\n");
            for (i, clock) in clocks.iter().enumerate() {
                asm += &format!(".symver old{i},{clock}@GLIBC_2.2.5\n");
                if calls {
                    asm += &format!("call {clock}@PLT\n");
                } else {
                    data += &format!(".quad {clock}\n");
                }
                if legacy_calls {
                    asm += &format!("call old{i}@PLT\n");
                } else {
                    data += &format!(".quad old{i}\n");
                }
            }
            asm += "call timer_create@PLT\nret\n";
            asm += &data;
            asm += ".section .note.GNU-stack,\"\",@progbits\n";
            let input = t.0.join("input.S");
            let obj = t.0.join("input.o");
            fs::write(&input, asm).unwrap();
            run(Command::new(env("CLOCK_PROFILE_AS"))
                .arg("--64")
                .arg(&input)
                .arg("-o")
                .arg(&obj));
            let input_relocs = run(Command::new("readelf").args(["-Wr"]).arg(&obj));
            assert!(input_relocs.contains("clock_gettime@GLIBC_2.2.5"));
            for lld in [false, true] {
                for as_needed in [false, true] {
                    for pie in [false, true] {
                        let mut cmd = Command::new(env(if lld {
                            "CLOCK_PROFILE_LLD"
                        } else {
                            "CLOCK_PROFILE_GNU_LD"
                        }));
                        if lld {
                            cmd.args(["-flavor", "gnu", "--threads=1"]);
                        }
                        if pie {
                            cmd.arg("-pie");
                        }
                        let output = t.0.join("linked");
                        cmd.args([
                            "-m",
                            "elf_x86_64",
                            "-e",
                            "_start",
                            "--dynamic-linker",
                            "/lib64/ld-linux-x86-64.so.2",
                        ])
                        .arg(if as_needed {
                            "--as-needed"
                        } else {
                            "--no-as-needed"
                        })
                        .arg("-L")
                        .arg(&overlay)
                        .arg("-L")
                        .arg(&src)
                        .arg(&obj)
                        .args(["-lrt", "-lc", "-o"])
                        .arg(&output);
                        run(&mut cmd);
                        let syms = run(Command::new("readelf")
                            .args(["-W", "--dyn-syms"])
                            .arg(&output));
                        let relocs = run(Command::new("readelf").args(["-Wr"]).arg(&output));
                        let versions = run(Command::new("readelf").args(["-WV"]).arg(&output));
                        for clock in clocks {
                            for version in ["GLIBC_2.17", "GLIBC_2.2.5"] {
                                let target = format!("{clock}@{version}");
                                assert!(
                                    syms.contains(&target),
                                    "missing {target}: {cmd:?}\n{syms}"
                                );
                                assert!(
                                    relocs.contains(&target),
                                    "missing relocation {target}: {cmd:?}\n{relocs}"
                                );
                            }
                        }
                        assert!(syms.contains("timer_create@GLIBC_2.3.3"));
                        assert!(
                            versions.contains("File: libc.so.6")
                                && versions.contains("File: librt.so.1")
                        );
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "requires selected real x86_64 DSOs in CLOCK_PROFILE_REAL_C and CLOCK_PROFILE_REAL_RT"]
fn selected_real_dso_copy() {
    let t = Temp::new();
    let c = PathBuf::from(std::env::var_os("CLOCK_PROFILE_REAL_C").expect("CLOCK_PROFILE_REAL_C"));
    let rt =
        PathBuf::from(std::env::var_os("CLOCK_PROFILE_REAL_RT").expect("CLOCK_PROFILE_REAL_RT"));
    let original = fs::read(&rt).unwrap();
    let c_before = fs::read(&c).unwrap();
    let out = prepare(&c, &rt, &t.0, LinkProfile::LibcClocksV1).unwrap();
    let dir = out
        .search_dir
        .expect("this gate expects real pre-2.30 clock defaults");
    let copy = fs::read(dir.join("librt.so.1")).unwrap();
    assert_eq!(copy.len(), original.len());
    let diffs: Vec<_> = copy.iter().zip(&original).filter(|(a, b)| a != b).collect();
    assert_eq!(diffs.len(), 5);
    assert!(diffs.iter().all(|(a, b)| **a == **b | 0x80));
    let before = Metadata::parse(&original).unwrap();
    let after = Metadata::parse(&copy).unwrap();
    for (a, b) in before.symbols.iter().zip(&after.symbols) {
        let mut expected = a.clone();
        if [
            "clock_gettime",
            "clock_getres",
            "clock_settime",
            "clock_nanosleep",
            "clock_getcpuclockid",
        ]
        .contains(&a.name.as_str())
            && a.default_version == Some(true)
        {
            expected.default_version = Some(false);
        }
        assert_eq!(&expected, b);
    }
    assert_eq!(fs::read(c).unwrap(), c_before);
    assert_eq!(fs::read(rt).unwrap(), original);
}

#[test]
fn librt_content_identity_and_cache_inventory_are_enforced() {
    let t = Temp::new();
    t.inputs();
    let first = t.prepare().unwrap();
    let mut padded = RT.to_vec();
    padded.push(0);
    fs::write(t.0.join("rt"), padded).unwrap();
    let second = t.prepare().unwrap();
    assert_ne!(first.librt_sha256, second.librt_sha256);
    assert_ne!(first.search_dir, second.search_dir);
    fs::write(second.search_dir.unwrap().join("unexpected"), b"x").unwrap();
    assert!(t.prepare().unwrap_err().contains("inventory"));
}

#[test]
fn malformed_and_unsupported_elf_inputs_refuse_before_cache_creation() {
    let t = Temp::new();
    t.inputs();
    for n in [0, 4, 63, 64, 80, RT.len() - 1] {
        fs::write(t.0.join("rt"), &RT[..n]).unwrap();
        assert!(t.prepare().is_err(), "truncation {n}");
    }
    let mut object = RT.to_vec();
    let sym = section(RT, 11);
    let at = u64_at(RT, sym + 24) as usize;
    // All non-null symbols are version nodes, clocks or the RT control. Refuse
    // object/TLS clock definitions rather than making a function ABI assumption.
    for entry in object[at..at + u64_at(RT, sym + 32) as usize]
        .as_chunks_mut::<24>()
        .0
        .iter_mut()
        .skip(1)
    {
        entry[4] = (entry[4] & 0xf0) | 1;
    }
    fs::write(t.0.join("rt"), object).unwrap();
    assert!(t.prepare().is_err());
    let mut machine = RT.to_vec();
    machine[18] = 183;
    fs::write(t.0.join("rt"), machine).unwrap();
    assert!(t.prepare().is_err());
    assert!(!t.0.join("cache").exists());
}

#[cfg(unix)]
#[test]
fn cached_library_symlink_is_refused() {
    let t = Temp::new();
    t.inputs();
    let dir = t.prepare().unwrap().search_dir.unwrap();
    fs::remove_file(dir.join("librt.so")).unwrap();
    std::os::unix::fs::symlink(t.0.join("rt"), dir.join("librt.so")).unwrap();
    assert!(t.prepare().unwrap_err().contains("regular files"));
}

#[test]
fn refuses_loader_memory_extent_and_alignment_mismatches() {
    let t = Temp::new();
    t.inputs();
    let phoff = u64_at(RT, 32) as usize;
    let phnum = u16::from_le_bytes(RT[56..58].try_into().unwrap()) as usize;
    let load = (0..phnum)
        .map(|i| phoff + i * 56)
        .find(|&p| u32::from_le_bytes(RT[p..p + 4].try_into().unwrap()) == 1)
        .unwrap();
    for (field, value) in [(40, 0_u64), (16, u64::MAX), (48, 3)] {
        let mut broken = RT.to_vec();
        broken[load + field..load + field + 8].copy_from_slice(&value.to_le_bytes());
        fs::write(t.0.join("rt"), broken).unwrap();
        assert!(t.prepare().unwrap_err().contains("loader segment"));
    }
    assert!(!t.0.join("cache").exists());
}
