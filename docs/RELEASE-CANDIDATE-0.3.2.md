# 0.3.2 candidate — not published

Updated 2026-10-31 on `release/0.3.2-candidate`, draft PR #1. Publication, merge,
tags and yanks are not authorized. See [the release plan](RELEASE-PLAN.md) and
[procedure](RELEASING.md). This record supersedes the narrower kernel-output-only
candidate; old evidence is historical, not qualification of the revised source.

## Proposed release notes and scope

- All three workspace CLIs handle a closed stdout reader without a panic:
  `kernel-seccomp` (existing fix `7fbd303`), `libc-abi`, and `linux-abi-scan`.
  This also fixes tool-owned stdout writes in the shared provider CLI used by
  both libc-abi and cargo-libc.
- Only `ErrorKind::BrokenPipe` at the write/flush boundary is ignored. No error
  strings are parsed, subprocess behavior changed, or library function exited
  early because a pipe closed. Provider operations continue and report later
  failures. Scanner findings keep exit 1; usage/read errors keep exit 2.
  Genuine stdout failures remain errors. Ordinary output bytes are unchanged.
- Helpers explicitly flush, covering failures that surface only from buffering.
  No stdout JSON/version option is added to the scanner (it supports text/TSV
  and help). Kernel profile JSON and root help/version paths retain their formats.
- CLI/library installation, feature choices and scope are clarified in packaged
  READMEs and separate user/maintainer guides.

All five package versions, seven internal dependency requirements and five local
lock entries remain 0.3.2. Both upstream glibc dependencies stay at 0.1.0 with
unchanged checksums. No new project dependency, public API, data, source pack,
provider build/replay algorithm, preset, license or MSRV change is introduced.
No extra glibc-abilists or glibc-provider release is required for unchanged inputs.

Since v0.3.1, runtime changes are limited to tool-owned stdout handling. The
shared provider CLI change requires fresh combined cargo-libc integration; the
published 0.3.0 provider CLI has not been fixed by a local source patch.
The focused kernel workflow now also tests the root crate on Linux/Windows,
workspace doctests, no-default root tests and workspace MSRV. Provider matrices
remain separate. Workers/test threads are capped at two.

## Reproduction and targeted checks

Before changes, both libc-abi help and a large scanner report panicked with
status 101 when a consumer read one byte then closed a small pipe. Deterministic
closed-reader regressions reproduced the failures without a scheduling race.
After changes, those user-facing reproductions return 0 without stderr.

Targeted regressions pass for help/version, large metadata/text/TSV output,
collection/export/stub side effects, ordinary byte-identical output, scan errors,
and later read failures. A provider fixture exports one entry, hits BrokenPipe
on its progress line, then still attempts the second entry and reports its real
export failure. `/dev/full` tests retain genuine stdout failures on Linux;
injected writer tests cover non-BrokenPipe write/flush failures on every host,
including buffered output. Existing kernel tests cover its JSON output path.

An initial targeted rerun used an old binary in the prior target directory; a
fresh `target/pipe-fixes` directory removed that stale artifact. A test command
also used the wrong numeric-version syntax and was corrected (`--version 2.14`,
not `GLIBC_2.14`), including the matching guide example. Neither failure is
counted as a pass. Final source checks use the fresh build directory.

## Revised qualification checklist

Tested source: **`53d3c6f0fa0fee5739e4021eb335017533396f06`**.
Evidence-only follow-ups modify this record, not the package sources, manifests,
lockfile or workflow. Consumer integration must record and use that exact revised source.

Final local checks passed on ARM64 Linux with Rust 1.99.0, Rust 1.90.0 for MSRV,
`CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`, and a fresh `target/pipe-fixes` directory:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets --locked --offline` | Passed: 100 tests |
| `cargo test --workspace --doc --locked --offline` | Passed: 6 doctests |
| `cargo test -p libc-abi-tools --all-targets --locked --offline --no-default-features` | Passed: 43 tests |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed |
| `cargo clippy -p libc-abi-tools --all-targets --locked --offline --no-default-features -- -D warnings` | Passed |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked --offline` | Passed |
| `cargo +1.90.0 check --workspace --all-targets --locked --offline` | Passed |
| `cargo package --workspace --locked` | All five archives packaged and verified |
| Archive contents, normalized manifests/locks, notices/licenses and unchanged upstream inputs | Passed |
| Markdown links/anchors and diff checks | Passed |

Cargo 1.99 verified the unpublished internal 0.3.2 packages using its temporary
registry. No registry-ordering failure or publication workaround occurred. Archive
file counts are linux-targets 8, kernel-abi-tools 15, linux-abi-scan 11,
libc-abi-tools 82, umbrella linux-abi-tools 7. The new private stdout helpers are
included; validation outputs/instructions are excluded. Root libgcc input tables,
provider scripts/manifests and required notices remain present. Upstream registry
checksums, data, features and license declarations are unchanged.

[Focused hosted run 41243021989](https://github.com/rustcommons/linux-abi-tools/actions/runs/41243021989)
completed **successfully** on 2026-10-31 for exact
`53d3c6f0fa0fee5739e4021eb335017533396f06`:

- Windows and Ubuntu: whole-workspace all-target tests/Clippy, root CLI regressions,
  doctests, no-default root tests and unchanged generated-profile comparison.
- MSRV: whole-workspace all-target check with Rust 1.90.0.
- x86_64 kernel simulation: 47 local profiles and 22 SLES 12 SP5 container profiles.

Registry APIs rechecked on 2026-10-31: the five latest published workspace versions
remain 0.3.1, unyanked; **0.3.2 is not published**. Upstream latest versions remain
0.1.1 (glibc-abilists) and 0.1.0 (glibc-provider-sources), unyanked. The candidate
intentionally retains both upstream lock resolutions at 0.1.0.

- [x] Before/after early-consumer reproduction and deterministic targeted regressions.
- [x] Write/flush failure classification and buffered-flush tests.
- [x] Final formatting, workspace all-target tests and doctests.
- [x] Final no-default root tests, Clippy, warning-free docs and MSRV.
- [x] Final all-five package verification, contents/license review and lock consistency.
- [x] Focused hosted workflow on the revised exact SHA: Linux/Windows root and workspace tests, MSRV, local/container kernel profiles.
- [ ] Fresh cargo-libc integration with revised source, including shared provider stdout and later-error regressions.
- [ ] Maintainer scope/package review, registry/tag recheck and explicit publication authorization.

Full provider rebuild/replay, Python consumer scenarios and real old-target
qualification are excluded from this bounded output-handling patch. Provider
input hashes/build logic did not change, but shared provider CLI behavior must
be tested anew. No universal runtime or distribution certification is claimed.

## Historical evidence only

Producer `d2f9abbfa56fc8352b4639f9dd35048e474bf79c` passed local checks and
[kernel run 41238220271](https://github.com/rustcommons/linux-abi-tools/actions/runs/41238220271).
[Consumer run 41238413717](https://github.com/rustcommons/cargo-libc/actions/runs/41238413717)
passed against that producer and consumer
`27d2a672b783d774637d4ea51bad38f1612e6a23`.
Those results covered the earlier kernel-only patch and **do not qualify the
new libc-abi/provider/scanner code**. Evidence-only commits following a newly
tested source must identify both source and documentation HEAD explicitly.

## Future publication and downstream adoption order

No publish command is to be executed in this preparation task. After explicit
future approval, publish one package at a time in this order:

1. `linux-targets` 0.3.2.
2. `kernel-abi-tools` 0.3.2.
3. `linux-abi-scan` 0.3.2.
4. `libc-abi-tools` 0.3.2 (independent of steps 1–3 after its existing upstream inputs).
5. Umbrella `linux-abi-tools` 0.3.2.

After **each** publication, verify the exact version and unyanked status through
the crates.io API, registry index and fetched package manifest/checksum before
starting the next dependent publication. A successful upload alone is not proof
that downstream resolution can see it. Stop on discrepancies; do not overwrite
versions or move tags. Reuse existing upstream glibc packages; there is no data
change that calls for publishing them again.

Only **after producer publication**, cargo-libc raises its `libc-abi-tools`
minimum to **0.3.2** if its complete provider pipe behavior depends on this fix,
refreshes its real registry lock, and validates unpatched integration plus a
locked install before its own authorized release. Other dependency floors follow
actual required behavior, not automatic lockstep with this workspace. Temporary
source patches are only for isolated pre-publication integration: never commit
them to production manifests or claim nonexistent registry versions were adopted.

## Decision

**No-go for publication:** revised producer checks passed; fresh combined consumer
integration is pending, followed by maintainer review, registry/tag recheck and
explicit publication authorization.
No release, merge, tag, yank or downstream registry adoption occurs in this task.
