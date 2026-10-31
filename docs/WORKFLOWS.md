# Workflows

How the linux-abi-tools crates fit together, which workflow to use for what, and
how each one maps onto [cargo-libc](https://github.com/rustcommons/cargo-libc).
The catalog distinguishes what is **implemented** from what is **planned**; a
planned row is a design, not a claim.

Shipping a Rust program to an older Linux system raises four separate questions:

| Question | When it matters | Answered by |
| --- | --- | --- |
| Which glibc symbol versions may the binary require? | Link time | `libc-abi-tools` (ABI data, stub libraries, providers) and `cargo libc build` |
| Which syscalls and `madvise` advice will the kernel accept? | Run time | `kernel-abi-tools` (syscall data, seccomp filter/profile, runners) |
| Which glibc and kernel does "the target" mean? | Both | `linux-targets` (distribution presets and selection rules) |
| Does a built file already require more than the target has? | After the build | `linux-abi-scan` (static inspection) |

The `linux-abi-tools` crate re-exports all four (`linux_abi_tools::{libc,
kernel, targets, scan}`).

## Workflow catalog

Every workflow, who runs it, and its status. **Implemented** means tested as
described in the linked section; **planned** and **long-term** rows are designs.

| Workflow | Who runs it | Status | Section |
| --- | --- | --- | --- |
| Choose a target (glibc and kernel floors) | Users | Implemented | [1](#1-choose-a-target) |
| Build against an older glibc with a provider | Users | Implemented (automatic preparation or an explicit provider) | [2](#2-build-against-an-older-glibc) |
| Type-check without linking (`cargo libc check`) | Users | Implemented | [6](#6-cargo-integration) |
| Inspect a built binary (tier 1) | Users | Implemented | [3](#3-inspect-a-built-binary-tier-1) |
| Run or test under an older kernel (tier 2) | Users | Implemented | [4](#4-run-under-an-older-kernel) |
| Check runners and inputs (`cargo libc doctor`) | Users | Implemented | [6](#6-cargo-integration) |
| Use the libraries directly | Developers | Implemented | [5](#5-use-the-libraries-directly) |
| Obtain a provider automatically (`source`, `container`) | Users | Implemented; `prebuilt` remains planned | [Using providers](USING-PROVIDERS.md) |
| Distribution-specific providers | Maintainers and users | Planned, later release | [10.2](#102-distribution-specific-providers) |
| Qualify on a real target host (tier 3) | Users | Outside these tools; required for any compatibility claim | [Testing tiers](#testing-tiers) |

Maintainer workflows: [collection, qualification, reference generation and releases](MAINTAINING.md).

### Testing tiers

Checks get more reliable, and more expensive, from tier to tier:

| Tier | How | Reliability | Tool |
| --- | --- | --- | --- |
| 1. Inspect | Read the built ELF files; nothing runs | Exact for symbols, versions, ELF notes; heuristic for syscalls in code | `linux-abi-scan`, `cargo libc inspect` |
| 2. Simulate | Run the tests under a seccomp filter (host userland) or in a container (target userland) | Catches what executes; syscall availability and `madvise` only | `kernel-seccomp run`, `cargo libc test`/`run` |
| 3. Qualify | Run on a real host at the target's minimums, e.g. a SLES 12 SP5 VM | The only qualification | Your target environment |

Tier 1 is documented as the least reliable: it reports what a file *could*
do. Tiers 1 and 2 use the same kernel data, so they cannot disagree about
what a kernel provides.

## 1. Choose a target

One selection drives both floors. The rules are implemented in
`linux_targets::Selection::resolve` and tested:

| User gives | glibc floor | Kernel floor | Runtime image |
| --- | --- | --- | --- |
| `--distro sles-12-sp5` | 2.22 | 4.12 (plus sourced backports) | the preset's |
| `--glibc 2.22` or `--target x86_64-unknown-linux-gnu.2.22` | 2.22 | none: host kernel, **not simulated** | none |
| `--kernel 4.12` | host | 4.12 | none |
| `--glibc 2.22 --kernel 4.12` | 2.22 | 4.12 | none |

`--distro` cannot be combined with `--glibc`, `--kernel`, or a versioned
`--target`. `kernel-seccomp distros` lists the 28 presets.

## 2. Build against an older glibc

**Status: implemented, experimental** (glibc 2.17 through 2.44 on x86_64; see
[PROVIDER-CONTRACT.md](PROVIDER-CONTRACT.md)).

cargo-libc prepares and caches a provider automatically. To prepare one directly,
run `libc-abi provider prepare --glibc 2.28`. Source mode is the default;
container mode is also available. See [Using glibc providers](USING-PROVIDERS.md)
for prerequisites, cache management, and offline use. Prebuilt mode is not available.

The stubs are used only for linking and never deployed. The
[provider pipeline](MAINTAINING.md#93-provider-pipeline) documents how maintainers qualify inputs.

## 3. Inspect a built binary (tier 1)

**Status: implemented in `linux-abi-scan`** (tested on synthetic ELF files and
real binaries on Linux 6.18; called by `cargo libc inspect --distro`/`--kernel`).

```sh
linux-abi-scan target/release/app --distro sles-12-sp5 --strict --verbose --tsv
```

| Check | Confidence | Fails when |
| --- | --- | --- |
| `GLIBC_*` version requirements against the glibc floor | Exact | A non-weak requirement is above the floor |
| `DT_RELR` relocations | Exact | The glibc floor is older than 2.36 |
| Imported syscall wrappers (`statx`, `getrandom`, `pidfd_open`, ...) against the kernel floor | Exact | A non-weak import's syscall is missing (weak: warning) |
| `NT_GNU_ABI_TAG` minimum kernel | Exact | Warning only |
| `syscall(N, ...)` calls and raw `syscall` instructions with a constant number | Heuristic | Warning only (error with `--strict`) |
| `madvise()` calls with constant advice | Heuristic | Warning only (error with `--strict`) |

For Cargo inspection options and output formats, use the
[cargo-libc user guide](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md).
A scan cannot see syscalls inside the C library, numbers computed
at run time, `dlopen`ed libraries, new flags on old syscalls, or whether a
fallback works; see the [crate README](../crates/linux-abi-scan/README.md).

## 4. Run under an older kernel

Both runners make the host kernel answer like the selected kernel:
syscalls it lacks return `ENOSYS`, `madvise` advice it lacks returns `EINVAL`,
and everything else behaves as on the host. Both use a **seccomp filter**, which
is a classic-BPF program. They differ in who builds the filter and which
userland the program sees.

| | Local filter (`--runner local`) | Container (`--runner container`) |
| --- | --- | --- |
| Status | Implemented; all 47 profiles verified on a 6.18 kernel | Implemented; generic kernels and three presets verified with crun in CI |
| Kernel simulated | Yes | Yes |
| Userland (glibc, `/etc`, tools) | **The host's** | The image's (e.g. real SLES 12 SP5) |
| Filter built by | This crate: BPF from compiled-in data | The runtime's libseccomp, from the generated OCI JSON profile |
| Needs | Linux x86_64 host with seccomp filters (3.5+) | podman or docker; crun with libseccomp 2.6+ to name the newest syscalls |
| Root | **No** (sets `no_new_privs`, then installs the filter) | No with rootless podman; docker needs access to its daemon |
| Catches | Code that needs a newer kernel | The same, plus the binary actually loading and running on the old glibc |
| Cost | Process start only | Image pull and container start |

Choose the local runner for fast tests of kernel behavior and when no container
runtime is available. Choose the container when you need the real target
userland. Of these two runners, only the container runner supplies the target's
userland; a real target host is another way to run against old glibc.

### Default runner

`kernel_abi_tools::Runner::choose` applies these rules (tested):

| Options | Runner |
| --- | --- |
| `--runner local` or `--runner container` | As requested |
| `--image IMAGE`, or `--distro ID` (the preset's image) | Container |
| `--kernel X.Y` alone | Local |
| `--runner local --image IMAGE` | Error: the local runner cannot use an image |
| `--runner container --kernel X.Y` without an image | Error: the container needs one |

`--runner local --distro ID` is allowed. It simulates the preset's kernel,
including backports, and reports that the userland is the host's.

### Commands

```sh
# Local filter: host userland, no container runtime, no root.
kernel-seccomp run --kernel 4.12 -- ./target/release/app
kernel-seccomp run --runner local --distro sles-12-sp5 -- ./target/release/app

# Container: the preset's real userland under the equivalent profile.
kernel-seccomp run --distro sles-12-sp5 -- ./target/release/app
kernel-seccomp run --kernel 4.12 --image registry.example/old:tag -- ./app
```

The local runner's filter cannot be removed and is inherited by every child.
Inside it, setuid programs such as `sudo` cannot gain privileges.

### Through Cargo

Cargo's runner setting applies the simulation to `cargo run`, `cargo test`, and
`cargo bench` target executables. Build scripts and procedural macros are not
affected. Tested with the local runner:

```sh
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="kernel-seccomp run --kernel 3.10 --"
cargo test
```

### Why not eBPF

seccomp's classic BPF is the right tool: an unprivileged process can install
it. eBPF programs (the `bpf()` syscall, tracing, BPF-LSM) need `CAP_BPF` or
`CAP_SYS_ADMIN`, so effectively root, and add nothing needed to answer
`ENOSYS`/`EINVAL`.

## 5. Use the libraries directly

| Need | API |
| --- | --- |
| Inspect a built file (tier 1) | `linux_abi_scan::scan_file`, `ScanTarget::from_selection`, `Report::{to_text, to_tsv, exit_code}` |
| Resolve user options to floors | `linux_targets::Selection::resolve`, `Selection::describe` |
| What a target lacks | `kernel_abi_tools::missing_syscalls`, `madvise_accepts` |
| OCI profile for a container runtime | `kernel_abi_tools::seccomp_profile` |
| BPF program for a target | `kernel_abi_tools::filter::program` (with `filter::evaluate` for tests) |
| Pick a runner | `kernel_abi_tools::Runner::choose` |
| Run a program | `kernel_abi_tools::local::run`, `kernel_abi_tools::container::run` |

With default features, the ABI lists and kernel/preset data are compiled in.
External ABI data is needed when bundled data is disabled. Container preparation
and image retrieval can require network access; see [offline provider use](USING-PROVIDERS.md#offline-and-air-gapped-machines).

## 6. Cargo integration

[cargo-libc's user guide](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md)
is the maintained reference for `build`, `check`, `inspect`, `run`, `test`,
`doctor`, and `provider` commands, including target options and runner diagnosis.
It prepares providers automatically and calls the libraries in this workspace.
Use its guide for exact command syntax rather than a second option table here.

For direct metadata commands, see [Using libc-abi](USING-LIBC-ABI.md); for
preparation/cache commands, see [Using providers](USING-PROVIDERS.md).

## 7. glibc ABI data

The ABI lists come from glibc's own signed release tags through the
[glibc-abilists](https://github.com/rustcommons/glibc-abilists) crate
(LGPL-2.1-or-later), which records each tag's signature check and each file's
blob id and SHA-256.

| Need | How |
| --- | --- |
| Default | `libc-abi-tools` compiles the lists in (`bundled-glibc-data` feature) |
| Leave the LGPL data out of a build | `default-features = false`; pass `--data DIR` (or `Snapshot::load`) |
| Files on disk for scripts or audits | `libc-abi glibc export-data --output DIR` writes the lists, `SOURCE.tsv`, and `COPYING.LIB` |
| cargo-libc | Uses the default libc-abi-tools features; no feature opt-out is exposed |

## 8. Open decisions

- **Provider delivery:** crate names, the source arrangement and the compiler runtime
  are decided ([PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md#decisions-2026-10-11));
  legal review of the `prebuilt` source arrangement is pending.
- **Distribution backports** remain unsourced for most presets; until sourced, a
  preset simulates its mainline base kernel.

## 9. Maintenance

Collection, provider qualification, generated references and release procedures
live in the [maintainer guide](MAINTAINING.md). These are not prerequisites for
ordinary use of the published tools.

## 10. Provider delivery and planned extensions

### 10.1 Provider delivery

Design: [PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md). `libc-abi provider` implements
steps 2–4 for `source` and `container` modes (2026-10-12), and cargo-libc runs them
automatically; `prebuilt` mode is still planned. The user runs
`cargo libc build --target x86_64-unknown-linux-gnu.2.28`; with no provider given:

1. **Resolve** the release and target, and look for a cached provider whose content
   hash matches.
2. **Prepare** one in the selected mode, if none is cached:
   - **`source` (default):**
     1. Read the release's headers, startup and nonshared sources, generated
        configuration headers and recorded compile commands from
        `glibc-provider-sources`, and check their hashes.
     2. Compile `crt1.o`, `crti.o`, `crtn.o` and `libc_nonshared.a` with the host's GCC,
        making the builder's recorded defaults explicit. Clang cannot compile these glibc
        sources faithfully ([why](PROVIDER-DELIVERY.md#why-source-mode-compiles-with-gcc-not-clang)).
     3. Verify every object against the container build's recorded interface;
        refuse on any unreviewed difference.
     4. Generate the stubs from the ABI lists and the recorded library table, and
        the `libgcc_s` stub.
   - **`prebuilt`:**
     1. Read the CI-built objects from `glibc-provider-binaries` and require their
        recorded SHA-256 and CI run.
     2. Take the headers from the sources crate.
     3. Generate the stubs.
   - **`container`:** run the [provider pipeline](MAINTAINING.md#93-provider-pipeline) steps 1–5
     locally with podman or docker.
3. **Assemble** the provider layout (sysroot, stubs, compiler runtime), write the
   descriptor and `manifest.json`, and audit the stubs as in the maintainer pipeline's stub-audit step.
4. **Cache** it under `$CARGO_HOME/libc-providers/...`, then build as in
   [section 2](#2-build-against-an-older-glibc).

`cargo libc provider prepare|list|path|verify|export|import|remove` (and
`libc-abi provider ...`) expose the same steps. Qualification requirements for
available modes and the planned prebuilt mode are:
- stubs and headers byte-identical across modes;
- `prebuilt` byte-identical to the qualified container build;
- `source` objects equivalent in symbols, sections and relocations;
- full suites against each mode's provider.

### 10.2 Distribution-specific providers

Later release ([roadmap](ROADMAP.md#future-release-distribution-specific-providers)).

1. Pin a redistributable distribution's binary and source packages, with checksums,
   signatures, licenses and terms.
2. Extract the same provider parts from them instead of building GNU glibc.
3. Measure the profile and compare it with the generic build, recording every
   difference in [DISTRIBUTION-DIFFERENCES.md](DISTRIBUTION-DIFFERENCES.md).
4. Run steps 4–9 of the [provider pipeline](MAINTAINING.md#93-provider-pipeline), plus a run in the distribution's own image.
5. Publish with the input's identity in the descriptor.

### 10.3 Source-built builder toolchains

Long-term ([roadmap](ROADMAP.md#long-term-source-built-builder-toolchains)).

1. Start from a minimal auditable seed.
2. Build era-matched binutils, GCC (with libgcc), make, bison, gawk and Python from
   signed upstream sources, and the UAPI headers from kernel.org.
3. Produce the builder images from committed Containerfiles, recording every input.
4. Build twice on different hosts, and require identical results.
5. Use these images in step 3 of the provider pipeline, and show the providers match
   today's vendor-built ones.
