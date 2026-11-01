# Unreleased parser and container I/O fixes

Scope reviewed 2026-11-01 against published 0.3.2 (`d404073e2329405e9e6f2dd1005fd8671e0d0f41`).
This work does not change versions, dependency requirements, upstream data, profiles
or provider build inputs. It is not a release announcement.

## Implemented scope

- **F1:** ELF version requirements and auxiliaries must fit their section, make
  forward progress, match their counts, and not overlap or reuse records. The
  old 4,096-header silent truncation is removed. A total 65,536 parsed metadata
  record limit and an aggregate work/string-copy budget (eight times input
  size, capped at 512 MiB) bound expansion from aliased input. The byte budget
  charges lossy UTF-8 expansion and copied strings before allocation. These
  limits can reject unusually large otherwise valid metadata; rejection is a
  normal parse error, not a partial successful scan.
- **F3:** Validate header/section/segment ranges before using their offsets,
  check string and executable-address additions, keep note payloads inside
  their segment, and require sufficient version-table entries. Header counts
  are 16-bit and record arithmetic stays within validated input ranges.
  Heuristic relative-call targets use checked address arithmetic; a target
  outside `u64` is not interpreted as a call-site match.
- **F4/F5:** Both Docker and Podman receive explicit `-e NAME` options for
  existing `CARGO*`/`RUST*` names. Values stay in the inherited environment,
  not arguments/logs. `-i` attaches stdin without a TTY. Existing engine/run
  options and child numeric exit statuses remain unchanged.

## Reproduction and validation

The initial debug regressions ran with `ulimit -v 131072`, `ulimit -t 10`,
`timeout 15s`, and one test thread. A fixture with 64 records and an 8 KiB
shared string was accepted and expanded into more than 2 MiB of strings;
a truncated version section was accepted; an overflowing note range panicked.
No multi-gigabyte amplification fixture was run.

Regression coverage includes shared/overlapping version records, early chain
termination, metadata and byte budgets, a valid 4,097-header chain, overflowing
headers/string-table/code addresses, short version tables, note confinement,
and a real scanner invocation returning status 2 instead of panicking. Docker
and Podman command tests verify explicit names, exclusion of unrelated names,
and stdin without a TTY. The focused kernel workflow also runs a native x86_64
Docker/Podman stdin/environment/exit-status smoke test.

Local ARM64 checks passed with `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`,
and a fresh `CARGO_TARGET_DIR=target/review-fixes`:

- Rust 1.99: `cargo fmt --all -- --check`;
  `cargo test --workspace --all-targets --locked --offline` (110 tests);
  `cargo test --workspace --doc --locked --offline` (6 doctests).
- `cargo test -p libc-abi-tools --all-targets --no-default-features --locked --offline`
  (43 tests).
- `cargo +1.90.0 check --workspace --all-targets --locked --offline`.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`;
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked --offline`.
- Shell syntax, changed documentation links/anchors and `git diff --check`.

An initial workspace attempt reused a pre-fix root CLI executable from the
existing target directory and failed five existing pipe tests. The fresh build
passed all 110 tests. Clippy then identified a redundant borrow in the container
builder; after removing it, clippy, docs, the two container unit tests and MSRV
checks passed. No behavior was suppressed to make these checks pass.

The supplied note-overflow, string-table-overflow and two-header shared-auxiliary
fixtures also returned CLI status 2 without panic under a 128 MiB/10 CPU-second
limit and 15-second timeout (about 2 ms each; peak child RSS below 10 MiB).
Fixtures were generated temporarily, not committed.

The local environment is ARM64 without a Docker/Podman daemon. Native hosted
qualification passed in [focused run 41263960927](https://github.com/rustcommons/linux-abi-tools/actions/runs/41263960927)
on exact source `c04269a6ea864b3a6371e8813d14f0fa069eb006`: Windows and Ubuntu
workspace checks, MSRV, 47 local kernel profiles, 22 SLES container profiles,
and the new Docker/Podman stdin/environment/exit-status smoke test. Both engines
reported success. This is native x86_64 coverage on a modern kernel, not tier 3
or i386/x32 qualification.

Independent re-review and fresh combined cargo-libc integration remain pending.
Any later evidence-only documentation commit does not replace this tested source
identity. No provider rebuild or package release is needed for this source review.

## Separate dispositions and limits

- **U1:** Existing `:z` mounts can recursively relabel the working tree on
  SELinux hosts, as described in [Podman's volume documentation](https://docs.podman.io/en/latest/markdown/podman-run.1.html).
  No SELinux reproduction was available. Mount defaults remain unchanged;
  use a dedicated working directory. A narrower mount/relabel policy is deferred.
- **U2:** The runner supplies no user override. With a rootful engine and root
  image user, new bind-mounted files can be root-owned. No rootful ownership
  reproduction was available. User mapping remains configurable through run
  arguments; automatic mapping needs separate image/engine qualification.
- **U3:** The local BPF filter explicitly rejects i386/x32 with `ENOSYS`; OCI
  JSON specifies only x86_64 and no bad-architecture action. libseccomp's
  [bad-architecture attribute](https://github.com/seccomp/libseccomp/blob/main/doc/man/man3/seccomp_attr_set.3)
  is separate from its default syscall action. Universal errno/status parity
  is therefore not established. Current wording is limited to native x86_64;
  no filter/profile behavior changes are made here.
- **Signal death:** retain the documented `128` fallback for a directly
  launched process with no numeric exit status. It loses signal detail but
  remains nonzero. Container engine numeric statuses pass through unchanged.
- **Forced cache replacement:** source inspection confirms removal before
  rename. Concurrent readers can see missing files; rename failure after
  removal leaves the old entry unavailable. Avoid forcing an entry in active
  use. Reader/writer coordination and recovery are separate design work.

[Roadmap follow-ups](ROADMAP.md#review-follow-ups-2026-11-01) track these limits.
Real old-system qualification remains deferred; containers on a modern kernel
are not tier 3 evidence.

## Future publication and adoption

Published 0.3.2 is immutable. A future version/release needs separate review and
publication authorization, with the existing selective-versus-lockstep policy
in [RELEASING.md](RELEASING.md). Under lockstep, publish and verify registry/index
availability in order: `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`,
`libc-abi-tools`, then `linux-abi-tools`. Unchanged glibc inputs need no new
upstream release. No dependency floor or version bump is included here.

Before publication, integration must use the exact candidate revision through
isolated patches. After publication, cargo-libc can adopt real registry versions,
raise dependency minima needed for the fixes, refresh its registry lock, and
validate an unpatched locked install. A dependency patch minimum is not itself
a compiler/native-tool minimum change; cargo-libc's own maintained
[release policy](https://github.com/rustcommons/cargo-libc/blob/main/docs/RELEASING.md)
governs its version decision.
