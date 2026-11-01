# Unreleased parser and container I/O fixes

Scope reviewed 2026-11-01 against published 0.3.2 (`d404073e2329405e9e6f2dd1005fd8671e0d0f41`).
This work does not change versions, dependency requirements, upstream data, profiles
or provider build inputs. It is not a release announcement.

## Implemented scope

- **F1:** ELF version requirements and auxiliaries must fit their section, make
  forward progress, match their counts, and not overlap or reuse records. The
  old 4,096-header silent truncation is removed. A total 65,536 version-chain
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
  options and child numeric exit statuses remain unchanged. This also forwards
  credentials such as `CARGO_REGISTRY_TOKEN` to the engine/container, as Podman
  already did. Use trusted images/programs and unset unnecessary credentials;
  values are not logged, and no new exclusion contract is introduced.
- **R1:** Fixed-size symbol, relocation and dynamic tables use their validated
  file ranges and aggregate byte budget, not the version-chain count limit.
  This preserves acceptance of large legitimate tables in Rust toolchain ELF
  files. The version-chain cap, progress/overlap checks and string budgets remain.
- **R2:** Equivalent executable sections are coalesced only when file range,
  virtual address and PLT classification match. Different addresses and partial
  overlaps remain represented. Code scanning allows at most one site per two
  input bytes and 1,048,576 sites total (separately for PLT stubs and report
  sites), checked before building per-site findings. Exhaustion returns an
  explicit error, never a truncated report. Very heavily aliased mappings can
  exceed this resource limit despite being structurally valid ELF; distinct
  address-specific evidence is not silently dropped.

## R1/R2 follow-up validation

The former fixed-table record limit was reproduced on both official Rust 1.90
x86_64 artifacts. After the correction, full scans against glibc 2.17/kernel 3.10
accept `cargo` (41,437,528 bytes, 659 findings) and `librustc_driver` (153,389,440
bytes, 473 findings). The artifacts were downloaded from the official Rust
server and their archives matched the published SHA-256 files:

| Archive | SHA-256 |
| --- | --- |
| [cargo-1.90.0-x86_64-unknown-linux-gnu.tar.xz](https://static.rust-lang.org/dist/cargo-1.90.0-x86_64-unknown-linux-gnu.tar.xz) | `9853db03d68578a30972e2755c89c66aec035fec641cf8f3a7117c81eec2578d` |
| [rustc-1.90.0-x86_64-unknown-linux-gnu.tar.xz](https://static.rust-lang.org/dist/rustc-1.90.0-x86_64-unknown-linux-gnu.tar.xz) | `48c2a42de9e92fcae8c24568f5fe40d5734696a6f80e83cc6d46eef1a78f13c9` |

The opt-in `toolchain_elf` regression reads these files without executing them.
Set `LINUX_ABI_SCAN_CARGO_ELF` and `LINUX_ABI_SCAN_RUSTC_DRIVER_ELF`, then run
`cargo test -p linux-abi-scan --test toolchain_elf --locked --offline -- --ignored --nocapture`.
The hosted MSRV job runs it against its installed official Rust 1.90 toolchain.
Synthetic tests accept 150,000 relocations and 70,000 symbols, retain the
version-chain cap, preserve distinct virtual-address findings, coalesce exact
aliases and reject excessive alias-derived sites without a partial report.

Revised source: `6604ceea74152a2e8bb1b56e5605bd4feecd8365`.
Local final checks passed with two build/test workers: fmt, 113 workspace tests,
6 doctests, 43 no-default tests, Rust 1.90 all-target check, clippy with denied
warnings and docs with denied rustdoc warnings. The real-toolchain acceptance
test also passed separately; it is opt-in because it needs external ELF files.
The original malformed note/string/version fixtures still returned status 2
under the 128 MiB/10 CPU-second/15-second timeout bounds. The scaled alias-site
regression also passed under those limits.

[Focused run 41264765540](https://github.com/rustcommons/linux-abi-tools/actions/runs/41264765540)
passed on that exact revision: Windows/Ubuntu workspace checks, MSRV, local and
container kernel profiles, Docker/Podman I/O integration, and the official Rust
1.90 Cargo/compiler-file acceptance test. Hosted acceptance produced the same
659 and 473 findings as the local scans.

Independent re-review and fresh combined cargo-libc integration remain pending
for the revised source. All evidence below for `c04269a` is historical and does
not qualify this revision. Later evidence-only documentation changes do not
replace the tested source identity.

## Historical initial validation (`c04269a`)


The initial debug regressions ran with `ulimit -v 131072`, `ulimit -t 10`,
`timeout 15s`, and one test thread. A fixture with 64 records and an 8 KiB
shared string was accepted and expanded into more than 2 MiB of strings;
a truncated version section was accepted; an overflowing note range panicked.
No multi-gigabyte amplification fixture was run.

Regression coverage includes shared/overlapping version records, early chain
termination, version-chain and byte budgets, a valid 4,097-header chain, overflowing
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

## Planned next release and registry adoption

**Recommendation, not an enacted release:** publish the five workspace crates
as **0.3.3**, then adopt them in a **cargo-libc 0.2.2** patch release. These are
compatible bug-fix proposals under the maintained policies, conditional on final
independent review of the parser resource limits and preserved contracts. If
review identifies an intentional breaking contract change, reconsider the version
before preparing a release. No MSRV/native-tool floor increase is proposed.

**Published 0.3.2 lacks F1/F3 parser fixes and F4/F5 container fixes.** It is
immutable. Merging this source PR would change Git only: it neither updates
registry consumers nor authorizes publication. Keep source merge, a later
version/lock/package preparation change, and publication as separate steps.
No version, dependency or lock edits are part of this plan-only change.

On 2026-11-01, the crates.io API reported 0.3.2 as latest for all five producers
and 0.2.1 for cargo-libc; all proposed versions were unused. Recheck registry
versions, yanked status and tags immediately before any authorized release.

### Producer scope and order

[Workspace versioning](RELEASING.md#versioning) requires all five crates to share
one version while 0.x; it does not currently provide a selective-release
exception. Therefore publish all five at 0.3.3 even though only two contain the
runtime fixes. The later release preparation must set all five package versions
and all internal path dependency requirements to 0.3.3, and deliberately refresh
the workspace lock without unrelated dependency updates.

| Order | Planned package | Reason |
| --- | --- | --- |
| 1 | [linux-targets 0.3.3](https://crates.io/api/v1/crates/linux-targets) | Unchanged implementation; lockstep prerequisite for kernel and scanner |
| 2 | [kernel-abi-tools 0.3.3](https://crates.io/api/v1/crates/kernel-abi-tools) | F4/F5 environment/stdin fixes; requires linux-targets |
| 3 | [linux-abi-scan 0.3.3](https://crates.io/api/v1/crates/linux-abi-scan) | F1/F3 parser hardening and R1/R2 corrections; requires kernel-abi-tools and linux-targets |
| 4 | [libc-abi-tools 0.3.3](https://crates.io/api/v1/crates/libc-abi-tools) | Unchanged implementation; lockstep release, independent of steps 1–3 once existing glibc inputs are available |
| 5 | [linux-abi-tools 0.3.3](https://crates.io/api/v1/crates/linux-abi-tools) | Umbrella requires all four at the new minimum |

After each authorized publication, verify the registry/index version, non-yanked
status, downloaded archive checksum and clean source identity before proceeding
to the next package. Existing glibc data/source inputs are unchanged and already
published; no new glibc-abilists or glibc-provider release is needed.

Before publication, validate the final release tree: required tests, MSRV,
fmt/clippy/docs, package contents/builds, licenses/provenance, and applicable
native container/consumer integration. Account for version-dependent cache and
package behavior; source-candidate results do not alone validate new package
metadata. A package check blocked by unpublished internal requirements must be
recorded as such, not bypassed by publishing without authorization. Preserve
existing provider/data/pack/replay gates; unchanged inputs do not require an
unrelated full-provider rebuild. Verify final published archives and docs.rs,
and tag only the exact released commit under the normal release procedure.

### cargo-libc minima and adoption

Live cargo-libc main at
[`92bc2b7`](https://github.com/rustcommons/cargo-libc/blob/92bc2b7874a5ee23975bd04e2ea29eb306b53f00/Cargo.toml)
uses the requirements below; its registry lock resolves all four to 0.3.2.
After producer publication, propose these direct requirements:

| Dependency | Current requirement | Planned requirement | Required guarantee |
| --- | --- | --- | --- |
| `linux-abi-scan` | `"0.3.0"` | `"0.3.3"` | Exclude published versions lacking F1/F3 and the reviewed R1/R2 corrections |
| `kernel-abi-tools` | `"0.3.0"` | `"0.3.3"` | Exclude published versions lacking F4/F5 for direct container-runner calls |
| `libc-abi-tools` | `"0.3.2"` | Retain `"0.3.2"` | Preserve the existing shared-provider broken-pipe minimum; this patch adds no fix there |
| `linux-targets` | `"0.3.0"` | Retain `"0.3.0"` | No new target-data behavior is required; new producer transitive requirements still apply |

A Cargo requirement `"0.3.3"` means `>=0.3.3, <0.4.0`. Raising both scanner and
kernel direct minima prevents *any permitted resolution* from selecting their
known unfixed 0.3.2-or-earlier versions. Updating only Cargo.lock, relying on
latest-version selection, or raising only the shared-provider requirement does
not establish that guarantee. Do not raise unchanged direct minima merely for
cosmetic alignment; record the actual resolved graph, including transitive
linux-targets and any duplicate versions.

Once all required producer versions are verified in the registry:

1. Raise those two minima and deliberately refresh cargo-libc's **real registry**
   lock; preserve unrelated dependencies/checksums. Verify the resolved sources
   are crates.io, the fixed scanner/kernel versions are selected, and no path,
   git, patch or temporary source replacement remains.
2. Run applicable consumer tests on that graph: bounded malformed ELF errors,
   real large-ELF acceptance, alias-site behavior, Docker/Podman environment and
   redirected stdin, existing pipe/provider failures, and native workflow checks.
   Candidate integration against an exact source SHA is prerelease evidence;
   rerun the applicable registry qualification after adoption.
3. Prepare cargo-libc 0.2.2 under its own
   [release policy](https://github.com/rustcommons/cargo-libc/blob/main/docs/RELEASING.md),
   inspect the packaged normalized manifest and lock, and install the unpacked
   proposed package with `cargo install --path <unpacked-package> --locked` in
   an isolated root and clean Cargo home. Exercise all five installed helpers
   and affected workflows without temporary patches. A dependency patch-minimum
   increase is not an MSRV/native-tool floor increase and does not by itself
   require a minor version.
4. Only after review and explicit publication authorization, release cargo-libc.
   Verify its official archive, manifest/lock, source identity, non-yanked status
   and docs.rs. Then run `cargo install cargo-libc --version 0.2.2 --locked` in a
   clean Cargo home and isolated install root, and repeat installed smoke checks.
   An existing installation is not updated by editing Git or a different lock.

Real old-system tier 3 infrastructure remains unavailable and deferred, not an
ordinary-release blocker. Ship only with passing applicable automated gates and
an explicit coverage gap; do not add real-old-system compatibility claims without
actual evidence. External consumers are unknown, not absent. No published
archive or tag is replaced; failures require a reviewed corrective release,
with any yank separately authorized.
