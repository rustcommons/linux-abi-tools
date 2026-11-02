![linux-abi-tools: release-to-ABI diagram](https://raw.githubusercontent.com/rustcommons/linux-abi-tools/main/docs/assets/banner.svg)

# linux-abi-tools

Rust libraries and CLI tools for targeting older **x86_64 Linux** systems:
glibc ABI metadata and link providers, distribution presets, kernel syscall
simulation, and static ELF inspection. The companion
[cargo-libc](https://github.com/rustcommons/cargo-libc) coordinates these tools
for Cargo builds, tests, and runs.

| Crate | Location | Purpose |
| --- | --- | --- |
| `libc-abi-tools` | repository root | glibc ABI metadata, stubs and providers; `libc-abi` CLI |
| `kernel-abi-tools` | `crates/kernel-abi-tools` | Syscall/madvise data and local/container simulation; `kernel-seccomp` and `syscall-probe` CLIs |
| `linux-targets` | `crates/linux-targets` | Distribution presets and target selection |
| `linux-abi-scan` | `crates/linux-abi-scan` | Static ELF inspection; `linux-abi-scan` CLI |
| `linux-abi-tools` | `crates/linux-abi-tools` | Library re-exporting the other four crates |

The repository was renamed from `libc-abi-tools`; the root **package** still has
that name. Each crate uses std and only first-party RustCommons dependencies.

## Install and get started

Requires Rust/Cargo **1.90 or newer**. For Cargo integration, use
[cargo-libc's installation and user guide](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md).
For direct CLI use, install only the tools you need:

```sh
cargo install libc-abi-tools --locked
cargo install kernel-abi-tools --locked
cargo install linux-abi-scan --locked

libc-abi glibc snapshot --release 2.22 --symbol memcpy
```

The query prints both historical `memcpy` symbol versions as TSV. Unknown releases
fail; no newer snapshot is substituted. Metadata queries need no container or GCC.
Provider preparation needs GCC and GNU binutils; container mode has additional
requirements in [Using providers](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-PROVIDERS.md). Local syscall simulation
requires Linux x86_64; container simulation additionally needs a suitable engine
and runtime. See [runners and prerequisites](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-KERNEL-ABI.md).

For library consumers, add the umbrella or an individual crate:

```toml
[dependencies]
linux-abi-tools = "0.3"
```

The umbrella exposes `linux_abi_tools::{libc, kernel, targets, scan}`. It is not a
CLI package. See [library APIs](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/WORKFLOWS.md#5-use-the-libraries-directly)
and [docs.rs](https://docs.rs/linux-abi-tools). For fine-grained libc feature
selection, depend on `libc-abi-tools` directly; see [feature choices](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-LIBC-ABI.md#library-use).

## Guides

- [Workflows](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/WORKFLOWS.md): choose targets, inspect files, simulate kernels,
  and use the libraries; distinguishes inspection, simulation and qualification.
- [Using libc-abi](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-LIBC-ABI.md): snapshots, export, comparisons, symbol
  membership, ELF evidence, and link-only stubs.
- [Using glibc providers](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-PROVIDERS.md): modes, prerequisites, cache,
  export/import and offline use.
- [Using kernel tools](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-KERNEL-ABI.md): local/container runners, Cargo
  runner configuration, profiles, probes and runtime requirements.
- [Static scanning](https://github.com/rustcommons/linux-abi-tools/blob/main/crates/linux-abi-scan/README.md): checks, confidence and limits.
- [Maintainer guide](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/MAINTAINING.md): source checkout, collection, regeneration,
  validation and release procedures.

## Releases

- [Changelog](https://github.com/rustcommons/linux-abi-tools/blob/main/CHANGELOG.md): changes, upgrade impact and versioned notes.
- [GitHub Releases](https://github.com/rustcommons/linux-abi-tools/releases): user-facing release notes for existing source tags; historical backfill is in progress.
- [Release policy](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/RELEASING.md): validation, registry packages and release completion.

## Scope and provenance

The pinned glibc snapshots and providers cover 2.17–2.44; see the
[provider index](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/PROVIDERS.md) and [coverage](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/GLIBC-COVERAGE.md) for tested
stages. A target release is not a symbol-version filter: a 2.17 snapshot retains
`GLIBC_2.2.5`. Exact snapshots preserve library membership as symbols move between
libraries. [The metadata guide](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/USING-LIBC-ABI.md#why-exact-release-snapshots)
explains why newer ABI lists cannot substitute for older ones.

Generated stubs contain no runtime implementation. Default builds include upstream
ABI lists through `glibc-abilists` and glibc startup/nonshared source inputs through
`glibc-provider-sources`. Programs run with real target libraries, never the stubs.
Disabling defaults on `libc-abi-tools` removes both optional inputs; it does not
remove the need for appropriate external data when using the metadata tools.

A clean static scan does not establish compatibility. Seccomp simulation models
syscall availability and `madvise` advice, not all flags, behavior or vendor
backports. Actual qualification requires running on a real target system.

Code is MIT licensed; bundled glibc ABI data is LGPL-2.1-or-later, and the provider
sources retain glibc and Linux UAPI licenses. The design and some code derive from
Zig's libc ABI tools. See [NOTICE.md](https://github.com/rustcommons/linux-abi-tools/blob/main/NOTICE.md) for attribution and license details.
No Zig installation is required.

See the [roadmap](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/ROADMAP.md) for open work and the
[0.4.0 release record](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/RELEASING.md#published-040-archives-and-documentation) for published versions, checksums and validation evidence.
The crates are published to crates.io. The
[requirements and documentation map](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/REQUIREMENTS.md)
indexes confirmed constraints, bounded evidence and remaining gaps.
