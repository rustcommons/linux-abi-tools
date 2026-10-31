# 0.3.2 candidate — not published

Prepared 2026-10-31 on `release/0.3.2-candidate`. This is a draft candidate,
not a completed release. Publication, tags, yanks and release assets remain
unauthorized. The [release plan](RELEASE-PLAN.md) and [procedure](RELEASING.md)
remain the release gates; this record tracks the narrower candidate scope.

## Proposed release notes

- `kernel-seccomp` now exits quietly with success when an output pipe's reader
  closes early, instead of panicking. The existing fix is `7fbd303`, with a
  regression covering missing/profile/distros/version/help output. Other output
  errors remain failures. Syscall data, filters and simulation behavior are unchanged.
- CLI/library installation, feature choices and scope are clarified. User guides
  and maintainer workflows are separated; packaged READMEs link to repository guides.
- All five packages move together from 0.3.1 to 0.3.2 under the current policy.
  No new public API, provider release, preset or compatibility claim is added.

Explicitly excluded: broken-pipe fixes for `libc-abi` and `linux-abi-scan` remain
unimplemented. Also excluded are new glibc data, source packs, prebuilt providers,
legal decisions, MSRV changes and real old-system qualification.

## Review against v0.3.1

The only Rust source/test differences are the kernel output fix and
`crates/kernel-abi-tools/tests/broken_pipe.rs`. Other intervening commits record
prior releases, simplify roadmaps, document existing CI, consolidate instructions,
and improve documentation. A separately authorized CI change adds only
`workflow_dispatch` to the existing kernel workflow; all triggers and jobs are
preserved. The candidate preparation adds version/internal-requirement updates,
the corresponding lock changes, and this record.

Package and lock changes:

- `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools` and
  umbrella `linux-abi-tools`: package versions 0.3.1 → 0.3.2.
- Seven internal path dependency requirements: 0.3.1 → 0.3.2 (one kernel,
  two scanner, four umbrella).
- Cargo.lock: only those five local package versions change.
  `glibc-abilists` and `glibc-provider-sources` remain at 0.1.0, with unchanged
  registry checksums. No feature, license, include-list, data or implementation
  changes are introduced by the candidate preparation.

Registry rechecked on 2026-10-31: all five latest published versions remain
0.3.1, unyanked; **0.3.2 does not exist on crates.io**. Upstream latest versions
are glibc-abilists 0.1.1 and glibc-provider-sources 0.1.0, both unyanked.
The [release plan's registry links](RELEASE-PLAN.md#evidence-snapshot) provide evidence endpoints.

## Validation and integration checklist

Tested producer revision: **`d2f9abbfa56fc8352b4639f9dd35048e474bf79c`**.
The earlier version-bump commit is `711189a5be54dd9745c4275cac7f722fbd796f90`;
the tested revision also merges the manual-dispatch change from main (`5d79625`).
Subsequent edits to this record are evidence-only: package sources, manifests,
lockfile, features and workflows remain identical to the tested revision.

Local checks completed on Linux ARM64 with Rust 1.99.0, two Cargo jobs and two
test threads. Rust 1.90.0 was used separately for the declared MSRV. Only existing
locked dependencies were fetched; no new project dependency was added.

| Check | Result |
| --- | --- |
| `cargo +1.99.0 fmt --all -- --check` | Passed |
| `cargo +1.99.0 test --workspace --all-targets --locked --offline` | Passed: 84 tests |
| `cargo +1.99.0 test --workspace --doc --locked --offline` | Passed: 6 doctests |
| `cargo +1.99.0 clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed |
| `cargo +1.99.0 test -p libc-abi-tools --all-targets --locked --offline --no-default-features` | Passed: 33 tests |
| `cargo +1.99.0 clippy -p libc-abi-tools --all-targets --locked --offline --no-default-features -- -D warnings` | Passed |
| `RUSTDOCFLAGS="-D warnings" cargo +1.99.0 doc --workspace --no-deps --all-features --locked --offline` | Passed |
| `cargo +1.90.0 check --workspace --all-targets --locked --offline` | Passed for all five packages |
| `cargo +1.99.0 package --workspace --locked` | Passed: all five packaged and verified from archives |
| `cargo +1.99.0 package --list --locked -p <crate>` for each package | Passed; file lists and archive contents reviewed |
| `kernel-seccomp generate` into a temporary directory, compared with committed profiles | Passed: identical profiles |
| Markdown targets/anchors, command/source review, `git diff --check` | Passed |

Workspace packaging used Cargo 1.99's temporary registry for internal 0.3.2
packages. There was **no unpublished-dependency ordering failure**, no publication,
and no replacement of internal dependencies with registry 0.3.1. Normalized
archive manifests and locks select the candidate's internal versions. All five
archives have their README and license, retain expected notices/embedded data,
and exclude validation output and repository instructions. The root archive
retains the libgcc table, Zig notice/license and provider build scripts/input
manifests; guides stay on GitHub and README links are absolute repository URLs.
Reviewed archive file counts: linux-targets 8, kernel-abi-tools 15,
linux-abi-scan 10, libc-abi-tools 81, umbrella linux-abi-tools 7.

### Hosted x86_64 and Windows evidence

[Kernel workflow run 41238220271](https://github.com/rustcommons/linux-abi-tools/actions/runs/41238220271)
completed **successfully** for exact SHA
`d2f9abbfa56fc8352b4639f9dd35048e474bf79c` on 2026-10-31.
It was manually dispatched on `release/0.3.2-candidate` after the dispatch trigger
was added to main, preserving existing triggers and jobs.

- `kernel-simulation`: passed shell checks, static probe/generator builds,
  **47 local profiles** and **22 profiles in SLES 12 SP5 userland** with static crun.
- `rust (windows-latest)`: passed formatting, selected-crate all-target tests,
  Clippy and committed-profile comparison, including the broken-pipe regression.
- `rust (ubuntu-latest)`: passed the same checks on x86_64 Linux.
- `msrv`: passed selected-crate all-target checks on Rust 1.90.0.

The selected crates are kernel-abi-tools, linux-targets, linux-abi-scan and the
umbrella; this workflow does not run root libc-abi-tools' own test suite on Windows.
The root suite passed locally on ARM64; its implementation and upstream inputs
are unchanged from v0.3.1. No new root-platform qualification is claimed.

An unmarked merge commit inadvertently queued automatic push/PR workflows;
those six runs were cancelled (one needed force-cancellation). They are not
passing evidence. The focused dispatch above is the retained hosted evidence.
Later evidence-only commits use CI-skip markers. Full glibc provider/replay matrices
are not required for this candidate's unchanged inputs/build logic and were not
used as a substitute for the focused kernel checks.

### cargo-libc consumer integration

[Consumer workflow run 41238413717](https://github.com/rustcommons/cargo-libc/actions/runs/41238413717)
completed **successfully**, independently verified by the parent, against:

- Producer: `d2f9abbfa56fc8352b4639f9dd35048e474bf79c` (this candidate's tested package sources).
- Consumer: `27d2a672b783d774637d4ea51bad38f1612e6a23` (isolated cargo-libc integration revision).

All integration steps passed: source resolution of the four consumed packages at
0.3.2; portable tests; automatic provider preparation/reuse, inspection, run and
refusal cases; C/CMake dependencies and C++ refusal; command workflows with both
plain-Cargo configuration formats; existing sysroot and host/target isolation;
and the check that production files remained unchanged. The umbrella is not a
cargo-libc dependency; its packaging and local tests are covered separately above.
The source patches do not imply any 0.3.2 package exists on crates.io.

Container and Python consumer scenarios, the full provider matrix/replay, and real
old-target qualification were intentionally excluded for this unchanged-provider,
kernel-output patch. The focused kernel workflow separately exercised its SLES
container profiles. Neither run certifies universal runtime compatibility.
Root libc-abi-tools' Windows suite was not rerun; its unchanged implementation
has local portable coverage, as described above. No additional automated gate is
pending for this candidate's agreed scope; expanded scope would require a fresh
assessment of those excluded checks.

- [x] Local portable/root tests, lint/docs, MSRV and package verification.
- [x] Native x86_64 local/container kernel simulation and selected-crate Windows checks.
- [x] Version/internal-requirement/lock consistency and unchanged input review.
- [x] Parent-verified cargo-libc integration against the exact producer and consumer revisions above.
- [ ] Maintainer review of final scope and package contents.
- [ ] Explicit publication authorization and the pre-publication registry/tag recheck.

Evidence-only follow-ups change this record, not package sources, manifests,
locks or workflows. The tested producer revision remains the reference for both
hosted runs. No tests are rerun solely for these evidence updates.

## Decision

**Local, kernel-CI and scoped consumer qualification passed. No-go for publication
pending maintainer review, explicit publication authorization and the final
registry/tag recheck.**
The draft PR can be reviewed independently of release authorization. Actual
publication would follow dependency order and immutable-version constraints in
[Releasing](RELEASING.md); this candidate preparation publishes nothing.
