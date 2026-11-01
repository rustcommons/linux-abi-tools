![kernel-abi-tools: an orange gear and image-to-kernel diagram](https://github.com/rustcommons/linux-abi-tools/blob/main/crates/kernel-abi-tools/docs/assets/banner.svg)

# kernel-abi-tools

**Test against the kernel you ship to.**

[![CI](https://github.com/rustcommons/linux-abi-tools/actions/workflows/kernel-abi-tools.yml/badge.svg)](https://github.com/rustcommons/linux-abi-tools/actions/workflows/kernel-abi-tools.yml)
![Rust 1.90+](https://img.shields.io/badge/Rust-1.90%2B-orange)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](https://github.com/rustcommons/linux-abi-tools/blob/main/crates/kernel-abi-tools/LICENSE)
![Status: prototype](https://img.shields.io/badge/status-prototype-yellow)
[![docs.rs](https://img.shields.io/docsrs/kernel-abi-tools)](https://docs.rs/kernel-abi-tools)

Linux kernel ABI data and seccomp filters that make a modern host kernel
answer like an older one. Run a Rust binary on the host under a local filter
(no container, no root), or in a container of the target distribution under
the equivalent profile, and:

- **Syscalls the kernel lacks** return `ENOSYS`, as on the real machine.
- **`madvise` advice values it does not know** return `EINVAL` (for example
  `MADV_POPULATE_READ` on 4.12, the kernel of SLES 12 SP5).

Part of the [linux-abi-tools](https://github.com/rustcommons/linux-abi-tools/blob/main/README.md) workspace, beside
`libc-abi-tools` (glibc ABI data) and [`linux-targets`](https://github.com/rustcommons/linux-abi-tools/blob/main/crates/linux-targets/README.md)
(distribution presets), and used by [cargo-libc](https://github.com/rustcommons/cargo-libc).
The glibc side answers "will it link and load?"; this crate answers "will its
syscalls work on that kernel?". Initial scope: **x86_64 Linux**. Std-only
Rust; its only dependency is `linux-targets`. Data queries and local simulation
work offline; container execution may need network access to obtain the image.

As a library, `local::run` and `container::run` run a program under the
simulation, `Runner::choose` picks between them, `filter::program` returns the
BPF program, and `seccomp_profile` returns the profile JSON, so a consumer such
as cargo-libc does not need the `kernel-seccomp` binary. See
[WORKFLOWS.md](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/WORKFLOWS.md) for every workflow and how it maps onto
cargo-libc.

**Status: prototype.** Every profile is verified in CI (see
[Verification](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-KERNEL-ABI.md#verification)); vendor kernel backports and most flag-level
differences are not modelled yet. See [docs/ROADMAP.md](https://github.com/rustcommons/linux-abi-tools/blob/main/crates/kernel-abi-tools/docs/ROADMAP.md).

## Contents

| Path | Purpose |
| --- | --- |
| `data/linux/syscalls-x86_64.tsv` | Every x86_64 syscall and the first mainline release whose table lists it (v3.3 to v7.2, from 81 kernel tags) |
| `data/linux/madvise-x86_64.tsv` | Every `madvise` advice value and the first release defining it |
| `../linux-targets` (dependency) | 28 distribution presets: kernel floor, glibc, container image |
| `profiles/kernel/linux-X.Y.json` | 19 generic kernel profiles |
| `profiles/distro/ID.json` | One profile per distribution preset |
| `kernel-seccomp` | Generates profiles and runs programs under the local filter or a container |
| `syscall-probe` | Proves a profile simulates its kernel on your host and runtime |
| `scripts/self-test.sh` | Runs the probe against any set of profiles |
| `scripts/run-under-kernel.sh` | Thin wrapper over `kernel-seccomp run` |
| `scripts/install-crun.sh` | Pinned, checksum-verified static crun for hosts with an old libseccomp |

## Usage

```bash
cargo install kernel-abi-tools --locked
kernel-seccomp distros
kernel-seccomp missing --distro sles-12-sp5
kernel-seccomp profile --kernel 4.12 --output linux-4.12.json

# On the host, as Linux 4.12 would answer it (local filter, no root)
kernel-seccomp run --kernel 4.12 -- ./my-app args

# In SLES 12 SP5 userland, as its kernel would answer it (container)
kernel-seccomp run --distro sles-12-sp5 -- ./my-app args
```

## Library use

```toml
[dependencies]
kernel-abi-tools = "0.3"
```

See [the API reference](https://docs.rs/kernel-abi-tools) for local/container
runners, BPF generation and syscall queries.

## Guides and limits

[Using kernel ABI tools](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-KERNEL-ABI.md) covers runner selection,
Cargo runner configuration, profiles, probes, runtime requirements and limitations.
The [workflow guide](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/WORKFLOWS.md) explains target selection and testing tiers.
Only syscall availability and madvise advice are simulated; vendor backports and
other behavior are not generally modeled. Native x86_64 rules share data; OCI
foreign-ABI errno/status parity is not qualified. Real old-system qualification
is deferred, as described in the guide.
For regeneration and validation, use [the maintainer guide](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/MAINTAINING.md).
