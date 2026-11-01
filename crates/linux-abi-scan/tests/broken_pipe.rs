//! Real CLI coverage: output errors never erase scan findings or later read errors.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
const BIN: &str = env!("CARGO_BIN_EXE_linux-abi-scan");
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(relr: bool) -> Self {
        let path = std::env::temp_dir().join(format!(
            "linux-scan-pipe-{}-{}.elf",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // ELF64 x86_64 with one empty section and, optionally, a DT_RELR entry.
        let mut bytes = vec![0u8; 256];
        bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        bytes[16] = 2;
        bytes[18] = 62;
        bytes[40] = 192;
        bytes[58] = 64;
        bytes[60] = 1;
        if relr {
            bytes[32] = 64;
            bytes[54] = 56;
            bytes[56] = 1;
            bytes[64] = 2;
            bytes[72] = 120;
            bytes[96] = 16;
            bytes[120] = 36;
        }
        fs::write(&path, bytes).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn closed(command: &mut Command) -> Output {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    command
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}
fn check(out: Output, code: i32) {
    assert_eq!(
        out.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn help_and_large_text_or_tsv_scans_accept_closed_readers() {
    check(closed(Command::new(BIN).arg("--help")), 0);
    let file = Fixture::new(false);
    for flags in [vec![], vec!["--tsv"], vec!["--verbose"]] {
        let mut cmd = Command::new(BIN);
        // Many reports exceed pipe buffers; the deterministic closed-reader case
        // exercises the first write, with subsequent files still processed.
        for _ in 0..128 {
            cmd.arg(&file.0);
        }
        check(closed(cmd.args(["--glibc", "2.17"]).args(flags)), 0);
    }
}
#[test]
fn scan_findings_and_later_read_failures_remain_nonzero() {
    let clean = Fixture::new(false);
    let bad = Fixture::new(true);
    for flags in [vec![], vec!["--tsv"], vec!["--strict"]] {
        check(
            closed(
                Command::new(BIN)
                    .arg(&clean.0)
                    .arg(&bad.0)
                    .args(["--glibc", "2.17"])
                    .args(flags),
            ),
            1,
        );
    }
    let out = closed(
        Command::new(BIN)
            .arg(&clean.0)
            .arg(clean.0.with_extension("missing"))
            .args(["--glibc", "2.17"]),
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("linux-abi-scan:"));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    let out = closed(Command::new(BIN).arg("--unknown"));
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown option"));
}
#[test]
fn report_bytes_and_status_match_library_results() {
    let file = Fixture::new(true);
    let selection = linux_targets::Selection::resolve(&linux_targets::Request {
        glibc: Some("2.17"),
        ..Default::default()
    })
    .unwrap();
    let report = linux_abi_scan::scan_file(
        &file.0,
        &linux_abi_scan::ScanTarget::from_selection(&selection),
    )
    .unwrap();
    for tsv in [false, true] {
        let mut cmd = Command::new(BIN);
        cmd.arg(&file.0).args(["--glibc", "2.17"]);
        if tsv {
            cmd.arg("--tsv");
        }
        let out = cmd.output().unwrap();
        let expected = if tsv {
            report.to_tsv()
        } else {
            format!("== {}\n{}", file.0.display(), report.to_text(false))
        };
        assert_eq!(out.stdout, expected.as_bytes());
        check(out, 1);
    }
}
#[cfg(target_os = "linux")]
#[test]
fn genuine_stdout_failure_remains_an_error() {
    let out = Command::new(BIN)
        .arg("--help")
        .stdout(
            fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .unwrap(),
        )
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot write to stdout"));
}

#[test]
fn malformed_note_returns_cli_error_instead_of_panicking() {
    let file = Fixture::new(true);
    let mut bytes = fs::read(&file.0).unwrap();
    bytes[64] = 4; // PT_NOTE
    bytes[96..104].copy_from_slice(&u64::MAX.to_le_bytes());
    fs::write(&file.0, bytes).unwrap();
    let out = Command::new(BIN)
        .arg(&file.0)
        .args(["--kernel", "3.10"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("linux-abi-scan:"));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
}
