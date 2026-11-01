# Parser and container I/O fixes: 0.3.3 release record

Scope reviewed 2026-11-01 against published 0.3.2 (`d404073e2329405e9e6f2dd1005fd8671e0d0f41`).
The reviewed source fixes did not change versions or dependency requirements.
The five-package 0.3.3 release was published on 2026-11-01 at
`d483fd5a58e1361d008f06db5989acef72a84e54`, tagged
[`v0.3.3`](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.3).
Release preparation updated package metadata only; upstream data, profiles and
provider build inputs remain unchanged. See the
[published archive verification](#published-033-archives-and-documentation) below.

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

Independent re-review approved the revised source, merged in PR #2 at
`c24ed06a9ac4e7014f9384b0692f22c098e21b99`.
[Combined run 41265226302](https://github.com/rustcommons/cargo-libc/actions/runs/41265226302)
passed with consumer `192e0157063f6b2c1ab868f075eb6897b61b15f8` against
producer `6604ceea74152a2e8bb1b56e5605bd4feecd8365`. This is source-patched
integration evidence, not registry adoption. All evidence below for `c04269a`
is historical and does not qualify this revision.

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

**Historical plan, superseded for producer preparation:** the recommendation
below predates the authorized 0.3.3 candidate. Its statements about no version,
dependency or lock edits apply only to that earlier plan-only change. Current
producer status and qualification are in
[Authorized 0.3.3 release preparation](#authorized-033-release-preparation-2026-11-01);
the downstream adoption gates below remain applicable.

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

## Authorized 0.3.3 release preparation (2026-11-01)

Historical preparation record: the coordinator alone published using existing
local Cargo credentials after independent approval and normal merge of PR #3.
All five 0.3.3 registry versions and v0.3.3 were absent at preparation start.
Publication and tagging are now complete, as recorded below. Published 0.3.2
remains unchanged and lacks these fixes.

### Release notes

- linux-abi-scan: F1/F3 bounded version-chain parsing, section/range validation
  and checked arithmetic return errors for malformed/resource-exhausting ELF.
  Large fixed-size tables remain accepted (R1); equivalent executable aliases
  coalesce while distinct mappings retain findings (R2). Resource limits can
  reject unusually large otherwise valid metadata; no partial report is emitted.
- kernel-abi-tools: F4/F5 explicitly forward existing CARGO*/RUST* names on
  Docker and Podman and attach redirected stdin without a TTY. Forwarding can
  include registry credentials; use trusted images and unset unnecessary values.
- linux-targets, libc-abi-tools and umbrella linux-abi-tools participate under
  the existing five-package lockstep policy. No new functionality, API, MSRV,
  upstream glibc dependency, generated data, profile, provider input or build
  algorithm change is included. Real Cargo/compiler scan output is unchanged.

Existing SELinux relabeling, rootful ownership, foreign-ABI OCI behavior, signal
status and forced-cache concurrency limitations remain as documented above.
The provider cache key includes the libc-abi-tools package version, so upgrading
to 0.3.3 can require fresh provider preparation despite unchanged build inputs.
Provider legal/provenance/pack/replay gates remain in force; unchanged provider
inputs do not require an unrelated full matrix rebuild. Tier 3 remains deferred
and unavailable, with no new real-old-system compatibility claim.

### Candidate gates and publication handoff

Five manifests, seven internal minima and five workspace lock entries move to
0.3.3. Both upstream glibc requirements/resolutions/checksums remain at 0.1.0.
Exact release candidate: `dc36fb26aaf06b3740322bc805b67d8b7ebfdfe5`,
[PR #3](https://github.com/rustcommons/linux-abi-tools/pull/3).
Reviewed implementation remains `6604ceea`; a checked diff against approved merge
`c24ed06a` confirms only the five manifests, lockfile and two release documents
changed. Later evidence-only documentation commits do not replace this identity.

Local ARM64 validation passed with fresh `target/release-033`, two build/test
workers, Cargo/Rust 1.99 and Rust 1.90 for MSRV:

- `cargo +1.99.0 fmt --all -- --check`.
- `cargo +1.90.0 check --workspace --all-targets --locked --offline`.
- Workspace all-target Clippy and root no-default-features all-target Clippy,
  both locked/offline with `-D warnings`.
- Workspace all-target tests (113 passed; external toolchain test opt-in),
  workspace doctests (6 passed), root no-default all-target tests (43 passed).
- All-feature workspace docs with `--no-deps` and `RUSTDOCFLAGS='-D warnings'`.
- `cargo +1.99.0 package --workspace --locked`: all five packaged sources
  verified using Cargo's temporary registry for unpublished internal versions.
  No publication workaround or dependency update was used.

All five archive audits passed: allowed file lists, byte-for-byte included source,
normalized manifests/internal minima, generated locks, clean candidate VCS SHA
and repository-relative paths, MIT/license/notice files, libgcc table, provider
scripts/input manifests, and unchanged upstream 0.1.0 requirements/checksums.
No credentials, build output, review docs, instruction files or integration-test directories are packaged.

Candidate archive SHA-256 values (not published checksums):

| Package | Files | SHA-256 |
| --- | --- | --- |
| linux-targets | 8 | `ee09d7ba465dfe103766265127694234d6376a810c9f7c6fa3f66d12cfe15288` |
| kernel-abi-tools | 15 | `64e176f3db8383c2781f119da3d9512c4ad899808bf366669b63acf7eb1a50a1` |
| linux-abi-scan | 11 | `8745c94d1b4f5e67a5251f7349abf166bf4f5bd740a2352b9464489535cc2a47` |
| libc-abi-tools | 82 | `ef8289d3d808347b2d02059fe6238f800e3ba2257836d3820e3c7b18bb7714dc` |
| linux-abi-tools | 7 | `3b343ed97f16bd4037fe36d864ad31d509dd0d97f52a608b718184bd3454d348` |

[Focused native run 41268289801](https://github.com/rustcommons/linux-abi-tools/actions/runs/41268289801)
passed on the exact release candidate: Windows/Ubuntu workspace checks, MSRV,
official Rust 1.90 Cargo/compiler ELF acceptance, local/container kernel profiles,
and Docker/Podman environment/stdin/status checks. This is modern-host native
x86_64 simulation, not tier 3 qualification. Independent release-preparation
review approved candidate `dc36fb26` and evidence head `eca532e`, then approved
final README/historical-plan corrections at
`bf61fe11e6697d9dd6937ad3cad0d7b7571ea0a8`. PR #3 merged normally at
`d483fd5a58e1361d008f06db5989acef72a84e54`, whose tree is identical to that
final reviewed head. All five packages were rebuilt and audited from the clean
merged commit; no unchanged full Rust/native suite or provider matrix was rerun. Source-patched consumer integration above remains applicable to unchanged
implementation; registry adoption and installs follow publication.

The sole publisher used the exact merged release commit and published in order:
linux-targets → kernel-abi-tools → linux-abi-scan → libc-abi-tools → linux-abi-tools.
Each official archive checksum, clean VCS identity, index availability and
non-yanked status was verified before proceeding. The coordinator reported
ALL_FIVE_PUBLISHED before the exact release commit was tagged. All five docs.rs
builds were then verified. Candidate archive hashes above are historical
preparation evidence; official published checksums follow.
Downstream minima, registry-lock adoption and unpatched locked-install gates
remain as specified above and are separate from producer source merge.

## Published 0.3.3 archives and documentation

Published on **2026-11-01**. Independent verification of the official crates.io
API, sparse index and downloaded archives matched the coordinator's evidence
and the clean merged-tree packages. Every archive identifies exact release
commit `d483fd5a58e1361d008f06db5989acef72a84e54` with no dirty marker, and every
version is non-yanked. Allowed files, normalized manifests, internal 0.3.3
minima, source payloads, locks and license/notices passed audit. Upstream glibc
requirements/resolutions/checksums remain at 0.1.0; no upstream release was needed.

| Package | Files | Published archive SHA-256 |
| --- | --- | --- |
| [linux-targets 0.3.3](https://crates.io/api/v1/crates/linux-targets/0.3.3) | 8 | `9138ccb80a4467c37c805b9ed7bfab8a574bd749a52fe42c4e00921e24affcb1` |
| [kernel-abi-tools 0.3.3](https://crates.io/api/v1/crates/kernel-abi-tools/0.3.3) | 15 | `0feddf7e5e33b15df1b279e3ba99c6a0743aae7edb91dd654e3a69ac01234dd3` |
| [linux-abi-scan 0.3.3](https://crates.io/api/v1/crates/linux-abi-scan/0.3.3) | 11 | `eedffc57c88f27dee9bd28c3f177f39a457a694ddbe2678d9ba9e6078702028e` |
| [libc-abi-tools 0.3.3](https://crates.io/api/v1/crates/libc-abi-tools/0.3.3) | 82 | `6a600e4bb78f84101a30c2658bd26a51ded0c6f79f73b74b99ad1168fa6b3bc7` |
| [linux-abi-tools 0.3.3](https://crates.io/api/v1/crates/linux-abi-tools/0.3.3) | 7 | `82e7bc3bc7fe5bea39f215c6489b83833fd3aaa722b8d7840a15a103fc7ec2e0` |

The annotated tag [`v0.3.3`](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.3)
points to the exact release commit above. No existing tag was moved and no
GitHub binary assets were created. This subsequent evidence update does not
change the immutable released tree or archives.

All five docs.rs status endpoints reported `doc_status: true`, version `0.3.3`,
on 2026-11-01. Each version-specific API page also loaded and displayed 0.3.3:

- [linux-targets API](https://docs.rs/linux-targets/0.3.3/linux_targets/): [successful build](https://docs.rs/crate/linux-targets/0.3.3/status.json).
- [kernel-abi-tools API](https://docs.rs/kernel-abi-tools/0.3.3/kernel_abi_tools/): [successful build](https://docs.rs/crate/kernel-abi-tools/0.3.3/status.json).
- [linux-abi-scan API](https://docs.rs/linux-abi-scan/0.3.3/linux_abi_scan/): [successful build](https://docs.rs/crate/linux-abi-scan/0.3.3/status.json).
- [libc-abi-tools API](https://docs.rs/libc-abi-tools/0.3.3/libc_abi_tools/): [successful build](https://docs.rs/crate/libc-abi-tools/0.3.3/status.json).
- [linux-abi-tools API](https://docs.rs/linux-abi-tools/0.3.3/linux_abi_tools/): [successful build](https://docs.rs/crate/linux-abi-tools/0.3.3/status.json).

### Released scope and remaining limits

0.3.3 ships F1/F3 parser hardening with R1 large-table acceptance and R2 alias/site
budget handling, plus F4/F5 container environment/stdin fixes. The detailed
resource limits and remaining SELinux, ownership, foreign-ABI, signal and cache
concurrency limitations above still apply. Version-keyed provider caches can
require fresh preparation after upgrading. No new real-old-system compatibility
claim is made: tier 3 remains unavailable and deferred, not completed. Existing
provider legal/provenance/pack/replay gates remain unchanged.

Producer publication, archive verification, tagging and docs.rs verification are
complete. Source-patched consumer integration passed as recorded above; real
registry adoption, raised scanner/kernel minima and unpatched locked installs
are separate downstream gates and are not claimed complete here. Published
0.3.2 remains immutable and lacks these fixes.
