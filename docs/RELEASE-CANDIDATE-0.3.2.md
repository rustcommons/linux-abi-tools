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
and improve documentation. The candidate adds only version/internal-requirement
updates, the corresponding lock changes, and this record.

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

Checks are initially pending and will be updated with actual results. Local host:
Linux ARM64; two Cargo jobs and two test threads. Installed Rust 1.99.0 (CI version)
and 1.90.0 (declared MSRV), with only existing locked dependencies fetched.

- [ ] Rust formatting, workspace all-target tests and doctests, warning-free Clippy.
- [ ] Root no-default-features tests and Clippy.
- [ ] Workspace docs with warnings denied.
- [ ] Rust 1.90 workspace all-target MSRV check.
- [ ] All five package file lists and archive/verification checks.
- [ ] Profile regeneration equals committed reference data.
- [ ] Native x86_64 local/container simulation and Windows checks.
- [ ] Parent's cargo-libc integration against the exact candidate revision.

CI skip markers avoid automatically starting the existing unrestricted provider
workflows for unchanged provider inputs. No provider replay or all-release rebuild
is requested for this patch. The existing kernel workflow describes x86_64
simulation and Windows/MSRV gates, but it is not workflow-dispatch enabled.
Any skipped hosted checks remain pending, not passing. This record does not change
workflow definitions or repository policy.

For consumer tests, temporarily patch all five packages to this branch's exact
commit (or checkout paths) in an isolated cargo-libc tree and resolve its lock.
Its `"0.3.0"` requirements permit this patch version. Do not substitute registry
0.3.1 or pretend 0.3.2 is published. Keep temporary patches out of release manifests.
Record the exact consumer and producer revisions and resolution. A later update
only to this validation record does not change package sources or dependencies;
record both revisions if integration ran against the earlier candidate.

## Decision

**No-go for publication while required checks or consumer qualification are pending.**
The draft PR can be reviewed independently of release authorization. Actual
publication would follow dependency order and immutable-version constraints in
[Releasing](RELEASING.md); this candidate preparation publishes nothing.
