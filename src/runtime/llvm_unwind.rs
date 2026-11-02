//! Policy-neutral, offline pinned LLVM libunwind archive preparation.
//!
//! Neither profile establishes application interoperability or a safe default.
//! No linker argument, provider descriptor or raw evidence changes. This builds
//! libunwind ONLY, without a C cleanup personality or general compiler helpers.
use super::{
    archive,
    inputs::{self, hash, read, real_dir, Tree},
    sources,
};
use crate::provider::json::{self, Value};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// Exact upstream source revision, separate from compiler provenance.
pub const SOURCE_COMMIT: &str = "7a941dfb2877a0b8cf30323fefa85ad019e0b0f5";
/// Conventional SHA-256 of the original official source archive.
pub const SOURCE_ARCHIVE_SHA256: &str =
    "819ecf7351f1a66f76b2d701d8942faaf5faeb324cb90c9d5b090de142434bb2";
/// Official source acquisition reference; preparation itself never downloads.
pub const SOURCE_ARCHIVE_URL:&str="https://github.com/llvm/llvm-project/releases/download/llvmorg-23.1.4/llvm-project-23.1.4.src.tar.xz";
const POLICY: &str = "llvm-unwind-23.1.4-archive-v1";
const UNITS: [&str; 9] = [
    "libunwind.cpp",
    "Unwind-EHABI.cpp",
    "Unwind-seh.cpp",
    "UnwindLevel1.c",
    "UnwindLevel1-gcc-ext.c",
    "Unwind-sjlj.c",
    "Unwind-wasm.c",
    "UnwindRegistersRestore.S",
    "UnwindRegistersSave.S",
];
/// Supported preparation target, not an oldest-system qualification claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// ELF64 little-endian GNU/Linux x86_64.
    X86_64LinuxGnu,
}
impl Target {
    /// Exact compiler target triple.
    pub fn triple(self) -> &'static str {
        "x86_64-unknown-linux-gnu"
    }
}
/// Experimental compile configuration, not a public binding/link policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveProfile {
    /// PIC hidden symbols, for isolated experiments.
    PrivatePicV1,
    /// PIC default visibility, for binding diagnostics.
    ExportablePicV1,
}
impl ArchiveProfile {
    /// Stable cache/fingerprint configuration identity.
    pub fn id(self) -> &'static str {
        match self {
            Self::PrivatePicV1 => "private-pic-v1",
            Self::ExportablePicV1 => "exportable-pic-v1",
        }
    }
}
/// Explicit trusted tools. No shell, PATH search or host archive fallback.
#[derive(Clone, Debug)]
pub struct Toolchain {
    /// Absolute Clang C driver path; explicit aliases resolve once.
    pub clang: PathBuf,
    /// Absolute Clang C++ driver. No target C++ standard library is used.
    pub clangxx: PathBuf,
    /// Absolute deterministic GNU-compatible `ar` (`rcsD`).
    pub archiver: PathBuf,
    /// Actual tool support files (including shared compiler libraries). Caller
    /// must declare the complete relevant closure; dlopen dependencies cannot
    /// be discovered or certified from version strings by this API.
    pub dependency_files: Vec<PathBuf>,
}
/// One actual selected input with an explicit independent containment root.
/// Separate header/library/descriptor roots are supported; no common provider
/// root is required. Caller owns semantic selection, not merely containment.
#[derive(Clone, Debug)]
pub struct SelectedFile {
    /// Existing normalized absolute directory, without symlink components.
    pub root: PathBuf,
    /// Normalized absolute regular file beneath `root`, without symlinks.
    pub path: PathBuf,
}
impl SelectedFile {
    fn validate(&self) -> Result<(), String> {
        real_dir(&self.root)?;
        if !self.path.starts_with(&self.root) {
            return Err("selected file escapes its declared root".into());
        }
        real_dir(self.path.parent().ok_or("selected file parent")?)?;
        Ok(())
    }
}
/// Inputs for archive preparation, independent of output-kind/default decisions.
#[derive(Clone, Debug)]
pub struct PrepareOptions {
    /// Actual selected libc, with its independently validated library root.
    /// Must be x86_64 ELF64 with SONAME `libc.so.6`, not a linker script.
    pub libc: SelectedFile,
    /// Supported archive target.
    pub target: Target,
    /// Explicit experimental configuration, never inferred from output kind.
    pub profile: ArchiveProfile,
    /// Selected sysroot: complete `usr/include` is snapshotted and hashed.
    /// Roots/trees must use normalized absolute paths with no symlinks.
    pub sysroot: PathBuf,
    /// Additional actual provider/sysroot identity files, e.g. a descriptor.
    /// Caller owns correct selection/containment. Files remain unchanged.
    pub provider_files: Vec<SelectedFile>,
    /// Explicit compilers/archiver and declared support dependency files.
    pub toolchain: Toolchain,
    /// Existing locally controlled absolute cache directory, not a symlink.
    pub cache_root: PathBuf,
}
/// Validated archive/provenance, NOT a link recipe or interoperability certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedArchive {
    /// Absolute ordinary target archive path.
    pub archive: PathBuf,
    /// Immutable manifest, including unresolved-symbol inventory.
    pub manifest: PathBuf,
    /// Source/configuration/header/provider/tool content identity.
    pub identity: String,
    /// Conventional SHA-256 of the entire archive.
    pub archive_sha256: String,
    /// Compile profile; conveys no application binding policy.
    pub profile: ArchiveProfile,
    /// Exact original source revision.
    pub source_commit: &'static str,
}
fn text(s: impl Into<String>) -> Value {
    Value::String(s.into())
}
fn obj(v: Vec<(&str, Value)>) -> Value {
    Value::Object(v.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn write(p: &Path, b: &[u8]) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)
        .map_err(|e| e.to_string())?;
    f.write_all(b)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}
struct Generation {
    path: PathBuf,
    published: bool,
}
impl Drop for Generation {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
fn generation(parent: &Path) -> Result<Generation, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..64 {
        let path = parent.join(format!(
            ".gen-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                return Ok(Generation {
                    path,
                    published: false,
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("cannot allocate runtime generation".into())
}
fn run(program: &Path, args: &[String], cwd: &Path) -> Result<Vec<u8>, String> {
    let log = cwd.join("command.log");
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&log)
        .map_err(|e| e.to_string())?;
    let mut c = Command::new(program);
    c.args(args)
        .current_dir(cwd)
        .env_clear()
        .env("LC_ALL", "C")
        .env("TMPDIR", cwd)
        .env("TMP", cwd)
        .env("TEMP", cwd);
    if let Some(v) = std::env::var_os("SystemRoot") {
        c.env("SystemRoot", v);
    }
    let mut child = c
        .stdout(Stdio::from(file.try_clone().map_err(|e| e.to_string())?))
        .stderr(Stdio::from(file))
        .spawn()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > Duration::from_secs(120)
            || fs::metadata(&log).map_err(|e| e.to_string())?.len() > 1024 * 1024
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err("runtime tool exceeded 120s/1MiB output limit".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let bytes = read(&log, 1024 * 1024)?;
    fs::remove_file(log).map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "{} failed ({status}): {}",
            program.display(),
            String::from_utf8_lossy(&bytes)
        ));
    }
    Ok(bytes)
}
fn query(p: &Path, args: &[&str], cwd: &Path) -> Result<String, String> {
    String::from_utf8(run(
        p,
        &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        cwd,
    )?)
    .map_err(|e| e.to_string())
}
fn files(paths: &[PathBuf]) -> Result<Value, String> {
    if paths.len() > 64 {
        return Err("at most 64 identity files per group".into());
    }
    let mut v = BTreeMap::new();
    for p in paths {
        real_dir(p.parent().ok_or("identity path parent")?)?;
        let id = inputs::file_identity(p)?;
        if v.insert(p.to_string_lossy().into_owned(), text(id))
            .is_some()
        {
            return Err("duplicate identity file".into());
        }
    }
    Ok(Value::Object(v.into_iter().collect()))
}
fn selected_files(paths: &[SelectedFile]) -> Result<Value, String> {
    if paths.len() > 64 {
        return Err("at most 64 provider identity files".into());
    }
    let mut result = BTreeMap::new();
    for p in paths {
        p.validate()?;
        let id = inputs::file_identity(&p.path)?;
        if result
            .insert(
                p.path.to_string_lossy().into_owned(),
                obj(vec![
                    ("root", text(p.root.to_string_lossy())),
                    ("content_identity", text(id)),
                ]),
            )
            .is_some()
        {
            return Err("duplicate provider identity file".into());
        }
    }
    Ok(Value::Object(result.into_iter().collect()))
}
struct Snapshot {
    headers: Tree,
    resource: Tree,
    tools: [PathBuf; 3],
    identity: Value,
}
fn snapshot(o: &PrepareOptions, work: &Path) -> Result<Snapshot, String> {
    o.libc.validate()?;
    let libc_bytes = read(&o.libc.path, crate::elf::MAX_FILE_BYTES)?;
    let libc = crate::elf::Metadata::parse(&libc_bytes)?;
    if libc.soname != "libc.so.6" {
        return Err("selected libc SONAME must be libc.so.6".into());
    }
    real_dir(&o.sysroot)?;
    let headers = Tree::load(&o.sysroot.join("usr/include"))?;
    for name in ["features.h", "stdint.h", "stdlib.h", "pthread.h", "dlfcn.h"] {
        if !headers.0.contains_key(name) {
            return Err(format!("selected headers lack {name}"));
        }
    }
    let tools = [
        inputs::tool(&o.toolchain.clang)?,
        inputs::tool(&o.toolchain.clangxx)?,
        inputs::tool(&o.toolchain.archiver)?,
    ];
    let mut ids = Vec::new();
    let mut resource: Option<(PathBuf, Tree)> = None;
    for (i, p) in tools.iter().enumerate() {
        let before = inputs::file_identity(p)?;
        let version = query(p, &["--version"], work)?;
        if i < 2 {
            if !version.to_lowercase().contains("clang") {
                return Err("runtime requires Clang drivers".into());
            }
            let triple = query(
                p,
                &[
                    "--no-default-config",
                    "--target=x86_64-unknown-linux-gnu",
                    "-dumpmachine",
                ],
                work,
            )?;
            if triple.trim() != o.target.triple() {
                return Err("compiler target mismatch".into());
            }
            let dir = PathBuf::from(
                query(p, &["--no-default-config", "-print-resource-dir"], work)?.trim(),
            );
            let tree = Tree::load(&dir.join("include"))?;
            if let Some((_, old)) = &resource {
                if old.identity() != tree.identity() {
                    return Err("C/C++ resource headers differ".into());
                }
            }
            resource = Some((dir, tree));
        }
        if inputs::file_identity(p)? != before {
            return Err("tool mutated during identity query".into());
        }
        ids.push(obj(vec![
            ("path", text(p.to_string_lossy())),
            ("content_identity", text(before)),
            ("version", text(version)),
        ]));
    }
    let (resource_dir, resource) = resource.ok_or("missing compiler resources")?;
    let identity = obj(vec![
        ("schema", text("libc-abi-llvm-archive-v1")),
        ("contract",text("archive construction only; no application/GLIBC-floor qualification or binary distribution clearance; no personality/helper completeness; declared tools and MD are not complete toolchain closure; caller owns compiler/provider provenance, header consistency and undeclared/unobservable loaded-library/subprocess/plugin/config dependencies; locally controlled roots required; not hermetic or an adversarial sandbox")),
        ("libc_root",text(o.libc.root.to_string_lossy())),
        ("libc",text(o.libc.path.to_string_lossy())),
        ("libc_sha256",text(hash(&libc_bytes))),
        ("policy", text(POLICY)),
        (
            "preparation_implementation_identity",
            text(hash(
                format!(
                    "{}{}{}",
                    hash(include_bytes!("llvm_unwind.rs")),
                    hash(include_bytes!("inputs.rs")),
                    hash(include_bytes!("archive.rs"))
                )
                .as_bytes(),
            )),
        ),
        ("profile", text(o.profile.id())),
        ("target", text(o.target.triple())),
        ("source_commit", text(SOURCE_COMMIT)),
        ("source_tag",text("llvmorg-23.1.4")),
        ("source_archive_url",text(SOURCE_ARCHIVE_URL)),
        ("license",text("Apache-2.0 WITH LLVM-exception; preserve full historical libunwind MIT/NCSA notices")),
        ("license_sha256",text(hash(include_bytes!("llvm_sources/LICENSE.TXT")))),
        ("source_archive_sha256", text(SOURCE_ARCHIVE_SHA256)),
        (
            "source_inventory_sha256",
            text(hash(include_bytes!("llvm_sources/SOURCES.tsv"))),
        ),
        ("headers", text(headers.identity())),
        ("sysroot", text(o.sysroot.to_string_lossy())),
        ("resource_dir", text(resource_dir.to_string_lossy())),
        ("resource_headers", text(resource.identity())),
        ("tools", Value::Array(ids)),
        ("tool_dependencies", files(&o.toolchain.dependency_files)?),
        ("provider_files", selected_files(&o.provider_files)?),
    ]);
    Ok(Snapshot {
        headers,
        resource,
        tools,
        identity,
    })
}
fn load_objects(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    real_dir(dir)?;
    let mut objects = BTreeMap::new();
    for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e
            .file_name()
            .into_string()
            .map_err(|_| "object name UTF-8")?;
        objects.insert(name, read(&e.path(), 8 * 1024 * 1024)?);
    }
    let mut expected = UNITS.map(|u| format!("{u}.o"));
    expected.sort();
    if objects.keys().cloned().collect::<Vec<_>>() != expected {
        return Err("runtime object inventory differs".into());
    }
    Ok(objects)
}
fn verified(
    index: &Path,
    input: &Value,
    key: &str,
    profile: ArchiveProfile,
) -> Result<PreparedArchive, String> {
    let pointer = read(index, 64 * 1024)?;
    let v = json::parse(std::str::from_utf8(&pointer).map_err(|e| e.to_string())?)?;
    let required = [
        "generation",
        "identity",
        "inputs",
        "archive_sha256",
        "object_inventory",
        "unresolved_symbols",
        "build_log_sha256",
    ];
    if !matches!(&v,Value::Object(fields) if fields.len()==required.len() && required.iter().all(|name|v.get(name).is_some()))
    {
        return Err("runtime manifest schema differs".into());
    }
    let name = v.str("generation")?;
    let valid = name
        .strip_prefix(".gen-")
        .and_then(|s| s.split_once('-'))
        .is_some_and(|(p, n)| {
            !p.is_empty()
                && !n.is_empty()
                && p.bytes().all(|c| c.is_ascii_digit())
                && n.bytes().all(|c| c.is_ascii_digit())
        });
    if name.len() > 128 || !valid {
        return Err("invalid runtime generation name".into());
    }
    let dir = index.parent().ok_or("index parent")?.join(name);
    real_dir(&dir)?;
    if read(&dir.join("manifest.json"), 64 * 1024)? != pointer
        || v.get("inputs") != Some(input)
        || v.str("identity")? != key
    {
        return Err("runtime cache identity differs".into());
    }
    let mut names = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.file_name()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    if names
        != [
            "LICENSE.TXT",
            "SOURCES.tsv",
            "build.log",
            "libunwind.a",
            "manifest.json",
            "objects",
        ]
        .map(std::ffi::OsString::from)
    {
        return Err("runtime cache inventory differs".into());
    }
    if read(&dir.join("LICENSE.TXT"), 64 * 1024)? != include_bytes!("llvm_sources/LICENSE.TXT")
        || read(&dir.join("SOURCES.tsv"), 64 * 1024)? != include_bytes!("llvm_sources/SOURCES.tsv")
    {
        return Err("runtime provenance differs".into());
    }
    let objects = load_objects(&dir.join("objects"))?;
    if v.str("object_inventory")? != archive::inventory(&objects) {
        return Err("runtime object hash differs".into());
    }
    let bytes = read(&dir.join("libunwind.a"), 32 * 1024 * 1024)?;
    if v.str("archive_sha256")? != hash(&bytes)
        || v.str("build_log_sha256")? != hash(&read(&dir.join("build.log"), 16 * 1024 * 1024)?)
    {
        return Err("runtime archive/log hash differs".into());
    }
    let unresolved = archive::validate(&bytes, &objects, profile == ArchiveProfile::PrivatePicV1)?;
    if v.get("unresolved_symbols")
        != Some(&Value::Array(unresolved.into_iter().map(text).collect()))
    {
        return Err("runtime unresolved symbols differ".into());
    }
    Ok(PreparedArchive {
        archive: dir.join("libunwind.a"),
        manifest: dir.join("manifest.json"),
        identity: key.into(),
        archive_sha256: hash(&bytes),
        profile,
        source_commit: SOURCE_COMMIT,
    })
}
/// Prepare an experimental archive offline from bundled unmodified sources.
///
/// The fixed recipe uses PIC, integrated assembly, selected headers/resources,
/// C++17 without exceptions/RTTI/C++ library headers and deterministic `ar rcsD`.
/// No target binary executes, no sources download and no runtime fallback occurs.
/// Sources, recipe/profile, target, actual headers/provider inputs and tool
/// bytes/version/resource/dependency identities bind the cache. Inputs are
/// snapshotted/rechecked. Complete generations publish via hard-link create-new;
/// corrupt entries refuse without repair. Losing writers verify the winner and
/// remove only their unpublished staging. Preparation is sequential; tool calls
/// are bounded individually to 120 seconds/1 MiB output.
///
/// Tools are trusted, not sandboxed. Caller must declare their relevant dependency
/// closure and control all roots against hostile concurrent replacement. Stable
/// symlinks refuse except explicit compiler aliases resolved once. These checks
/// are not an adversarial filesystem security boundary. Cache hits still inspect
/// current tools/inputs. General mixed-runtime/forced-unwind compatibility remains
/// blocked: an archive alone MUST NOT select an executable linking default.
pub fn prepare(o: &PrepareOptions) -> Result<PreparedArchive, String> {
    let root = real_dir(&o.cache_root)?;
    let parent = root.join("llvm-unwind-archives");
    match fs::create_dir(&parent) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    real_dir(&parent)?;
    let mut stage = generation(&parent)?;
    let s = snapshot(o, &stage.path)?;
    let key = hash(s.identity.to_text().as_bytes());
    let index = parent.join(&key);
    match fs::symlink_metadata(&index) {
        Ok(_) => return verified(&index, &s.identity, &key, o.profile),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    build(o, &s, &stage.path)?;
    if snapshot(o, &stage.path)?.identity != s.identity {
        return Err("runtime inputs mutated during build".into());
    }
    for n in ["inputs", "source"] {
        fs::remove_dir_all(stage.path.join(n)).map_err(|e| e.to_string())?;
    }
    let objects = load_objects(&stage.path.join("objects"))?;
    let bytes = read(&stage.path.join("libunwind.a"), 32 * 1024 * 1024)?;
    let unresolved =
        archive::validate(&bytes, &objects, o.profile == ArchiveProfile::PrivatePicV1)?;
    let v = obj(vec![
        (
            "generation",
            text(
                stage
                    .path
                    .file_name()
                    .ok_or("generation name")?
                    .to_string_lossy(),
            ),
        ),
        ("identity", text(&key)),
        ("inputs", s.identity.clone()),
        ("archive_sha256", text(hash(&bytes))),
        ("object_inventory", text(archive::inventory(&objects))),
        (
            "unresolved_symbols",
            Value::Array(unresolved.into_iter().map(text).collect()),
        ),
        (
            "build_log_sha256",
            text(hash(&read(
                &stage.path.join("build.log"),
                16 * 1024 * 1024,
            )?)),
        ),
    ]);
    write(&stage.path.join("manifest.json"), v.to_text().as_bytes())?;
    match fs::hard_link(stage.path.join("manifest.json"), &index) {
        Ok(()) => stage.published = true,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    verified(&index, &s.identity, &key, o.profile)
}
fn build(o: &PrepareOptions, s: &Snapshot, dir: &Path) -> Result<(), String> {
    let source = dir.join("source");
    fs::create_dir(&source).map_err(|e| e.to_string())?;
    for (name, expected, data) in sources::FILES {
        if hash(data) != *expected {
            return Err("bundled LLVM source hash mismatch".into());
        }
        let p = source.join(name);
        fs::create_dir_all(p.parent().ok_or("source parent")?).map_err(|e| e.to_string())?;
        write(&p, data)?;
    }
    let sysroot = dir.join("inputs/sysroot");
    let resources = dir.join("inputs/compiler-resource/include");
    s.headers.write(&sysroot.join("usr/include"))?;
    s.resource.write(&resources)?;
    let mut allowed = BTreeMap::new();
    for (name, _, bytes) in sources::FILES {
        allowed.insert(source.join(name), hash(bytes));
    }
    for (name, bytes) in &s.headers.0 {
        allowed.insert(sysroot.join("usr/include").join(name), hash(bytes));
    }
    for (name, bytes) in &s.resource.0 {
        allowed.insert(resources.join(name), hash(bytes));
    }
    let objects = dir.join("objects");
    fs::create_dir(&objects).map_err(|e| e.to_string())?;
    let mut log = String::new();
    for unit in UNITS {
        let cpp = unit.ends_with(".cpp");
        let mut args = vec![
            "--no-default-config".into(),
            format!("--target={}", o.target.triple()),
            format!("--sysroot={}", sysroot.display()),
            "-nostdinc".into(),
            format!(
                "-resource-dir={}",
                dir.join("inputs/compiler-resource").display()
            ),
            "-isystem".into(),
            resources.to_string_lossy().into_owned(),
            "-isystem".into(),
            sysroot.join("usr/include").to_string_lossy().into_owned(),
            format!("-I{}", source.join("include").display()),
            format!("-I{}", source.join("src").display()),
            "-O2".into(),
            "-g0".into(),
            "-fPIC".into(),
            "-fintegrated-as".into(),
            "-funwind-tables".into(),
            "-fno-stack-protector".into(),
            "-D__STDC_FORMAT_MACROS".into(),
            "-D__STDC_LIMIT_MACROS".into(),
            "-D_LIBUNWIND_IS_NATIVE_ONLY".into(),
            "-DNDEBUG".into(),
            "-ffunction-sections".into(),
            "-fdata-sections".into(),
            format!("-ffile-prefix-map={}=/libc-abi-llvm-unwind", dir.display()),
        ];
        if o.profile == ArchiveProfile::PrivatePicV1 {
            args.extend(["-fvisibility=hidden", "-D_LIBUNWIND_HIDE_SYMBOLS"].map(String::from));
        }
        if cpp {
            args.extend(
                [
                    "-std=c++17",
                    "-fno-exceptions",
                    "-fno-rtti",
                    "-nostdinc++",
                    "-fvisibility-inlines-hidden",
                ]
                .map(String::from),
            );
        } else if unit.ends_with(".c") {
            args.extend(["-std=c99", "-fexceptions"].map(String::from));
        }
        args.extend([
            "-MD".into(),
            "-MF".into(),
            dir.join("dependencies.d").to_string_lossy().into_owned(),
            "-c".into(),
            source.join("src").join(unit).to_string_lossy().into_owned(),
            "-o".into(),
            objects
                .join(format!("{unit}.o"))
                .to_string_lossy()
                .into_owned(),
        ]);
        let p = &s.tools[usize::from(cpp)];
        log.push_str(&format!("{p:?} {args:?}\n"));
        log.push_str(&String::from_utf8_lossy(&run(p, &args, dir)?));
        let dependencies = read(&dir.join("dependencies.d"), 1024 * 1024)?;
        audit_dependencies(&dependencies, &allowed)?;
        log.push_str(&String::from_utf8_lossy(&dependencies));
        fs::remove_file(dir.join("dependencies.d")).map_err(|e| e.to_string())?;
        archive::object(&read(&objects.join(format!("{unit}.o")), 8 * 1024 * 1024)?)?;
    }
    let mut args = vec![
        "rcsD".into(),
        dir.join("libunwind.a").to_string_lossy().into_owned(),
    ];
    args.extend(UNITS.iter().map(|u| {
        objects
            .join(format!("{u}.o"))
            .to_string_lossy()
            .into_owned()
    }));
    log.push_str(&format!("{:?} {args:?}\n", s.tools[2]));
    log.push_str(&String::from_utf8_lossy(&run(&s.tools[2], &args, dir)?));
    if Tree::load(&sysroot.join("usr/include"))?.identity() != s.headers.identity()
        || Tree::load(&resources)?.identity() != s.resource.identity()
    {
        return Err("compiler mutated copied headers".into());
    }
    for (name, expected, _) in sources::FILES {
        if hash(&read(&source.join(name), 8 * 1024 * 1024)?) != *expected {
            return Err("compiler mutated copied source".into());
        }
    }
    write(&dir.join("build.log"), log.as_bytes())?;
    write(
        &dir.join("LICENSE.TXT"),
        include_bytes!("llvm_sources/LICENSE.TXT"),
    )?;
    write(
        &dir.join("SOURCES.tsv"),
        include_bytes!("llvm_sources/SOURCES.tsv"),
    )?;
    Ok(())
}

// Parse Clang's one-rule make dependency output, including escaped spaces/newlines.
fn audit_dependencies(data: &[u8], allowed: &BTreeMap<PathBuf, String>) -> Result<(), String> {
    let text = std::str::from_utf8(data)
        .map_err(|e| e.to_string())?
        .replace("\\\r\n", "")
        .replace("\\\n", "");
    let (_, rhs) = text
        .split_once(": ")
        .ok_or("invalid compiler dependency rule")?;
    let mut names = Vec::new();
    let mut name = String::new();
    let mut chars = rhs.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            name.push(chars.next().ok_or("truncated dependency escape")?);
        } else if c.is_whitespace() {
            if !name.is_empty() {
                names.push(std::mem::take(&mut name));
            }
        } else if c == '$' {
            if chars.next() != Some('$') {
                return Err("unsupported dependency variable".into());
            }
            name.push('$');
        } else {
            name.push(c);
        }
    }
    if !name.is_empty() {
        names.push(name);
    }
    if names.is_empty() {
        return Err("empty compiler dependencies".into());
    }
    for n in names {
        let p = PathBuf::from(n);
        if !p.is_absolute() || !allowed.contains_key(&p) {
            return Err(format!("undeclared compiler dependency: {}", p.display()));
        }
        real_dir(p.parent().ok_or("dependency parent")?)?;
        if Some(&hash(&read(&p, 8 * 1024 * 1024)?)) != allowed.get(&p) {
            return Err("observed compiler dependency content differs".into());
        }
        if p.canonicalize().map_err(|e| e.to_string())? != p {
            return Err("compiler dependency not canonical".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "runtime-cache-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn bundled_source_inventory_is_exact() {
        let mut lines = String::new();
        for (name, expected, data) in sources::FILES {
            assert_eq!(hash(data), *expected);
            assert!(!Path::new(name).is_absolute());
            assert!(!name.contains(".."));
            lines.push_str(&format!("{expected}\t{name}\n"));
        }
        assert_eq!(lines.as_bytes(), include_bytes!("llvm_sources/SOURCES.tsv"));
        assert_ne!(
            ArchiveProfile::PrivatePicV1.id(),
            ArchiveProfile::ExportablePicV1.id()
        );
    }
    #[test]
    fn publication_never_clobbers_even_empty_directory() {
        let p = temp();
        let mut g = generation(&p).unwrap();
        write(&g.path.join("manifest.json"), b"complete").unwrap();
        let index = p.join("index");
        fs::create_dir(&index).unwrap();
        assert!(fs::hard_link(g.path.join("manifest.json"), &index).is_err());
        assert!(index.is_dir());
        fs::remove_dir(&index).unwrap();
        fs::hard_link(g.path.join("manifest.json"), &index).unwrap();
        g.published = true;
        let other = generation(&p).unwrap();
        write(&other.path.join("manifest.json"), b"different").unwrap();
        assert!(fs::hard_link(other.path.join("manifest.json"), &index).is_err());
        drop(other);
        assert_eq!(fs::read(&index).unwrap(), b"complete");
        drop(g);
        assert_eq!(fs::read(&index).unwrap(), b"complete");
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn malformed_cache_pointer_refuses_without_repair() {
        let p = temp();
        let index = p.join("index");
        write(&index, b"{\"generation\":\"../escape\"}").unwrap();
        assert!(verified(&index, &Value::Null, "x", ArchiveProfile::PrivatePicV1).is_err());
        assert_eq!(fs::read(index).unwrap(), b"{\"generation\":\"../escape\"}");
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn dependency_escape_and_injected_host_headers_refuse() {
        let p = temp();
        let path = p.join("selected header.h");
        fs::write(&path, b"header").unwrap();
        let rule = format!("object.o: {}\n", path.to_string_lossy().replace(' ', "\\ "));
        let allowed = BTreeMap::from([(path.clone(), hash(b"header"))]);
        assert!(audit_dependencies(rule.as_bytes(), &allowed).is_ok());
        assert!(audit_dependencies(b"object.o: /usr/include/stdio.h\n", &allowed).is_err());
        assert!(audit_dependencies(b"object.o:", &allowed).is_err());
        fs::write(&path, b"mutated").unwrap();
        assert!(audit_dependencies(rule.as_bytes(), &allowed).is_err());
        let unknown = p.join("undeclared.h");
        fs::write(&unknown, b"in staged directory but not inventory").unwrap();
        let rule = format!("object.o: {}\n", unknown.display());
        assert!(audit_dependencies(rule.as_bytes(), &allowed).is_err());
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn independently_selected_files_need_no_common_root() {
        let headers = temp();
        let libs = temp();
        let descriptor = temp();
        fs::write(libs.join("c.so"), b"identity").unwrap();
        fs::write(descriptor.join("provider.txt"), b"descriptor").unwrap();
        let a = SelectedFile {
            root: libs.clone(),
            path: libs.join("c.so"),
        };
        let b = SelectedFile {
            root: descriptor.clone(),
            path: descriptor.join("provider.txt"),
        };
        assert!(a.validate().is_ok());
        assert!(selected_files(&[a.clone(), b]).is_ok());
        let escape = SelectedFile {
            root: headers.clone(),
            path: a.path,
        };
        assert!(escape.validate().is_err());
        for p in [headers, libs, descriptor] {
            fs::remove_dir_all(p).unwrap();
        }
    }
}
