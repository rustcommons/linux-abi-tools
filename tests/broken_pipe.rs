//! Exercise real CLI stdout without relying on a race with `head`.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

const BIN: &str = env!("CARGO_BIN_EXE_libc-abi");
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "libc-abi-pipe-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
fn success(output: Output) {
    assert!(
        output.status.success(),
        "{:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn help_version_metadata_and_provider_list_accept_a_closed_reader() {
    let temp = Temp::new();
    for args in [
        vec![],
        vec!["--help"],
        vec!["--version"],
        vec!["glibc", "snapshot", "--help"],
    ] {
        success(closed(Command::new(BIN).args(args)));
    }
    success(closed(
        Command::new(BIN)
            .args(["provider", "list", "--cache"])
            .arg(&temp.0),
    ));
    success(closed(
        Command::new(BIN)
            .args(["glibc", "elf", "--input"])
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/elf/symbols.so")),
    ));
    #[cfg(feature = "bundled-glibc-data")]
    for args in [
        vec!["glibc", "snapshot", "--release", "2.22"], // much larger than a pipe buffer
        vec!["glibc", "matrix", "--symbol", "memcpy"],
        vec!["glibc", "diff", "--from", "2.17", "--to", "2.22"],
        vec!["glibc", "changes"],
        vec![
            "glibc",
            "expect",
            "--release",
            "2.22",
            "--library",
            "c",
            "--symbol",
            "memcpy",
            "--version",
            "2.14",
        ],
    ] {
        success(closed(Command::new(BIN).args(args)));
    }
}

#[test]
fn writes_complete_and_overwrite_errors_are_not_hidden() {
    let temp = Temp::new();
    let dir = temp.0.join("stub");
    success(closed(
        Command::new(BIN)
            .args(["libgcc", "stub", "--output"])
            .arg(&dir),
    ));
    assert!(dir.join("libgcc_s.S").is_file());
    assert!(dir.join("libgcc_s.map").is_file());
    let failed = closed(
        Command::new(BIN)
            .args(["libgcc", "stub", "--output"])
            .arg(&dir),
    );
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("cannot create new directory"));
    #[cfg(feature = "bundled-glibc-data")]
    {
        let data = temp.0.join("data");
        success(closed(
            Command::new(BIN)
                .args(["glibc", "export-data", "--output"])
                .arg(&data),
        ));
        assert!(data.join("2.22").is_dir());
    }
}

#[test]
fn provider_export_continues_after_closed_stdout_and_reports_a_later_failure() {
    let temp = Temp::new();
    let cache = temp.0.join("cache");
    // Export only copies these fixture entries; no compiler/provider builds needed.
    for hash in ["a", "b"] {
        let entry = cache
            .join("x86_64-unknown-linux-gnu/glibc-2.22")
            .join(format!("source-{hash}"));
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("manifest.json"), "{}\n").unwrap();
        if hash == "a" {
            fs::write(entry.join("provider.txt"), "fixture\n").unwrap();
        }
    }
    let to = temp.0.join("export");
    let out = closed(
        Command::new(BIN)
            .args(["provider", "export", "--glibc", "2.22", "--cache"])
            .arg(&cache)
            .arg("--to")
            .arg(&to),
    );
    // First export succeeds and prints to the closed pipe. Second fails removing
    // its missing descriptor. BrokenPipe must neither panic nor skip that failure.
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.starts_with("libc-abi:"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
    assert!(to.join("glibc-2.22/source-a/manifest.json").is_file());
    assert!(to.join("glibc-2.22/source-b/manifest.json").is_file());
}

#[test]
fn ordinary_output_and_command_errors_are_preserved() {
    let out = Command::new(BIN).arg("--version").output().unwrap();
    assert_eq!(
        out.stdout,
        format!("libc-abi {}\n", env!("CARGO_PKG_VERSION")).as_bytes()
    );
    success(out);
    let temp = Temp::new();
    let out = Command::new(BIN)
        .args(["provider", "list", "--cache"])
        .arg(&temp.0)
        .output()
        .unwrap();
    assert_eq!(out.stdout, b"glibc\tmode\thash\tdescriptor\n");
    success(out);
    let out = closed(Command::new(BIN).arg("not-a-command"));
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("libc-abi: expected"));
}

#[cfg(target_os = "linux")]
#[test]
fn genuine_stdout_failure_is_not_success() {
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
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot write to stdout"));
}

#[test]
fn collection_summary_accepts_closed_stdout_after_writing_files() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    let abi = source.join("sysdeps/unix/sysv/linux/x86_64/64");
    fs::create_dir_all(&abi).unwrap();
    for library in libc_abi_tools::glibc::LIBRARIES {
        let name = if library == "ld" {
            "ld.abilist".into()
        } else {
            format!("lib{library}.abilist")
        };
        fs::write(abi.join(name), "GLIBC_2.2.5 example F\n").unwrap();
    }
    fs::write(source.join("COPYING"), "fixture license\n").unwrap();
    fs::write(source.join("COPYING.LIB"), "fixture license\n").unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "fixture",
        ],
        vec!["tag", "glibc-2.22"],
    ] {
        let out = Command::new("git")
            .arg("-C")
            .arg(&source)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let dest = temp.0.join("collected");
    success(closed(
        Command::new(BIN)
            .args(["glibc", "collect", "--source"])
            .arg(&source)
            .arg("--output")
            .arg(&dest),
    ));
    assert!(dest.join("2.22/SOURCE.txt").is_file());
}
