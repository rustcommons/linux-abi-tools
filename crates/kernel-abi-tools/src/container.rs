//! Runs a program in a container whose seccomp profile simulates a kernel.

use crate::{seccomp_profile, Target};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// The container engine and the options added to its command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Engine {
    /// `podman` or `docker`.
    pub program: String,
    /// Options placed before `run`, e.g. `--runtime crun`.
    pub engine_args: Vec<String>,
    /// Options added to `run`, e.g. `--network none`.
    pub run_args: Vec<String>,
}

impl Engine {
    /// Reads `CONTAINER_ENGINE` (default `podman`), `KERNEL_ABI_ENGINE_ARGS`,
    /// and `KERNEL_ABI_RUN_ARGS` (whitespace-separated).
    pub fn from_env() -> Self {
        Engine {
            program: env::var("CONTAINER_ENGINE").unwrap_or_else(|_| "podman".to_string()),
            engine_args: env_words("KERNEL_ABI_ENGINE_ARGS"),
            run_args: env_words("KERNEL_ABI_RUN_ARGS"),
        }
    }
}

/// Runs `program` with `args` in `image` under the profile for `target` and
/// returns its exit status (128 if it was killed by a signal).
///
/// The current directory, and `program` if it is a host path outside it, are
/// mounted at their own absolute paths so relative paths and compile-time
/// `CARGO_MANIFEST_DIR` values keep working. A bare `program` name (no `/`)
/// runs from the image. A host program is mounted as a single file, never its
/// directory, which could hide the image's own `/bin` or `/usr/bin`.
/// `CARGO*`/`RUST*` variables are forwarded by name. Standard input stays
/// attached (including pipes), without allocating a TTY. Suitable for a Cargo runner.
///
/// # Errors
///
/// Returns a message when `image` is empty, the current directory cannot be
/// read, a host `program` is not a file, the profile cannot be written to the
/// temporary directory, or the engine cannot be started. The program's own
/// failure is its exit status, not an error.
pub fn run(
    engine: &Engine,
    target: &Target,
    image: &str,
    program: &str,
    args: &[String],
) -> Result<i32, String> {
    if image.is_empty() {
        return Err("no container image selected".to_string());
    }
    let cwd = env::current_dir().map_err(|e| format!("current directory: {e}"))?;
    let host_program = program
        .contains('/')
        .then(|| absolute(&cwd, Path::new(program)));
    if let Some(p) = &host_program {
        if !p.is_file() {
            return Err(format!("{} is not a file on this host", p.display()));
        }
    }

    let profile = TempFile::new("kernel-seccomp", ".json")?;
    fs::write(&profile.0, seccomp_profile(target))
        .map_err(|e| format!("{}: {e}", profile.0.display()))?;

    let mut cmd = container_command(
        engine,
        &cwd,
        host_program.as_deref(),
        &profile.0,
        env::vars_os().map(|(name, _)| name),
    );
    cmd.args(&engine.run_args);
    cmd.arg(image);
    match &host_program {
        Some(p) => cmd.arg(p),
        None => cmd.arg(program),
    };
    cmd.args(args);

    let status = cmd
        .status()
        .map_err(|e| format!("cannot start {}: {e}", engine.program))?;
    Ok(status.code().unwrap_or(128))
}

fn container_command(
    engine: &Engine,
    cwd: &Path,
    host_program: Option<&Path>,
    profile: &Path,
    environment: impl IntoIterator<Item = OsString>,
) -> Command {
    let mut cmd = Command::new(&engine.program);
    cmd.args(&engine.engine_args);
    cmd.args(["run", "--rm", "-i", "--security-opt"]);
    cmd.arg(format!("seccomp={}", profile.display()));
    cmd.arg("-v").arg(format!("{0}:{0}:z", cwd.display()));
    match host_program {
        Some(p) if !p.starts_with(cwd) => {
            cmd.arg("-v").arg(format!("{0}:{0}:ro,z", p.display()));
        }
        _ => {}
    }
    cmd.arg("-w").arg(cwd);
    // Docker treats CARGO* literally; enumerate names for both engines.
    // Values remain in the inherited environment, never in command arguments.
    let mut names: Vec<_> = environment
        .into_iter()
        .filter(|name| {
            let bytes = name.as_encoded_bytes();
            bytes.starts_with(b"CARGO") || bytes.starts_with(b"RUST")
        })
        .collect();
    names.sort();
    names.dedup();
    for name in names {
        cmd.arg("-e").arg(name);
    }
    cmd
}

fn env_words(name: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

/// A file in the temporary directory, removed when dropped.
struct TempFile(PathBuf);

impl TempFile {
    fn new(prefix: &str, suffix: &str) -> Result<Self, String> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let name = format!("{prefix}-{}-{nanos}{suffix}", std::process::id());
        let path = env::temp_dir().join(name);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(TempFile(path))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KernelVersion;

    #[test]
    fn docker_and_podman_attach_stdin_and_forward_only_matching_names() {
        for program in ["docker", "podman"] {
            let engine = Engine {
                program: program.into(),
                engine_args: vec!["--example".into()],
                run_args: vec![],
            };
            let command = container_command(
                &engine,
                Path::new("/work"),
                None,
                Path::new("/profile.json"),
                ["RUSTFLAGS", "CARGO_HOME", "OTHER_SECRET", "CARGO_HOME"].map(OsString::from),
            );
            let args: Vec<_> = command.get_args().map(|s| s.to_str().unwrap()).collect();
            assert_eq!(
                args,
                [
                    "--example",
                    "run",
                    "--rm",
                    "-i",
                    "--security-opt",
                    "seccomp=/profile.json",
                    "-v",
                    "/work:/work:z",
                    "-w",
                    "/work",
                    "-e",
                    "CARGO_HOME",
                    "-e",
                    "RUSTFLAGS"
                ]
            );
            assert!(!args.contains(&"-t"));
        }
    }

    #[test]
    fn rejects_missing_image_and_missing_host_program() {
        let engine = Engine {
            program: "definitely-not-a-container-engine".to_string(),
            engine_args: Vec::new(),
            run_args: Vec::new(),
        };
        let target = Target::kernel(KernelVersion::parse("4.12").unwrap());
        assert!(run(&engine, &target, "", "sh", &[]).is_err());
        let err = run(&engine, &target, "img", "./no/such/program", &[]).unwrap_err();
        assert!(err.contains("not a file on this host"), "{err}");
        let err = run(&engine, &target, "img", "sh", &[]).unwrap_err();
        assert!(err.contains("cannot start"), "{err}");
    }
}
