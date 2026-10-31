# 0.3.2 release qualification

Published **2026-10-31** after review and explicit authorization. [PR #1](https://github.com/rustcommons/linux-abi-tools/pull/1)
was merged at `d404073e2329405e9e6f2dd1005fd8671e0d0f41`, tagged
[`v0.3.2`](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.2).
Its package sources, manifests, lock and workflow are identical to tested producer
`53d3c6f0fa0fee5739e4021eb335017533396f06`; intervening changes only updated this
qualification record. All five package archives identify the clean merged commit.
No GitHub binary bundles or release assets were created.

## Release notes and scope

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
Pre-release evidence-only follow-ups modified this record, not the package sources,
manifests, lockfile or workflow. Consumer integration used that exact revised source.

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

Registry verification after publication on 2026-10-31 confirmed all five 0.3.2
packages, unyanked, with matching API/index/download checksums and clean release
commit metadata. Both upstream lock resolutions remain at 0.1.0; no upstream
data/source package was released for this change.

- [x] Before/after early-consumer reproduction and deterministic targeted regressions.
- [x] Write/flush failure classification and buffered-flush tests.
- [x] Final formatting, workspace all-target tests and doctests.
- [x] Final no-default root tests, Clippy, warning-free docs and MSRV.
- [x] Final all-five package verification, contents/license review and lock consistency.
- [x] Focused hosted workflow on the revised exact SHA: Linux/Windows root and workspace tests, MSRV, local/container kernel profiles.
- [x] Fresh cargo-libc integration with revised source, including shared provider stdout and later-error regressions (run 41243277747).
- [x] Maintainer scope/package review, registry/tag recheck and explicit publication authorization.
- [x] All five packages published and verified; v0.3.2 tags the exact released commit.

Full provider rebuild/replay, Python consumer scenarios and real old-target
qualification are excluded from this bounded output-handling patch. Provider
input hashes/build logic did not change; shared provider CLI behavior was
qualified against the revised source in the combined run below. No universal runtime or distribution certification is claimed.

## Combined consumer qualification

[Integration run 41243277747](https://github.com/rustcommons/cargo-libc/actions/runs/41243277747)
completed **successfully**, confirmed through the GitHub API, for exact revisions:

- Producer: `53d3c6f0fa0fee5739e4021eb335017533396f06`.
- Consumer: `b8ff22a5b1cbb71f123b58c4efdd9be7614bfce4`.

The run passed shared-provider export regressions for closed stdout, a later
independent failure, and genuine non-BrokenPipe output failures. It also passed
native provider reuse/link/inspect/run/refusal checks, C/CMake integration, both
plain-Cargo configuration formats, and sysroot/host-target isolation checks.
This qualifies the revised source-patched combination for the stated scope;
it does not establish universal runtime compatibility or real-registry adoption.
The earlier consumer revision `5d5184c5d8521fba50191716d0f1e38724ba04f4` is not
the final qualifying consumer revision.

Real-registry cargo-libc dependency adoption and locked-install validation remain
deferred until the producer packages exist, as specified in the publication order
below. Those are downstream release gates, not an uncompleted source-integration
check for this producer candidate.

## Historical evidence only

Producer `d2f9abbfa56fc8352b4639f9dd35048e474bf79c` passed local checks and
[kernel run 41238220271](https://github.com/rustcommons/linux-abi-tools/actions/runs/41238220271).
[Consumer run 41238413717](https://github.com/rustcommons/cargo-libc/actions/runs/41238413717)
passed against that producer and consumer
`27d2a672b783d774637d4ea51bad38f1612e6a23`.
Those results covered the earlier kernel-only patch and **do not qualify the
new libc-abi/provider/scanner code**. Evidence-only commits following a newly
tested source must identify both source and documentation HEAD explicitly.

## Published packages and downstream adoption

Publication completed in this order, verifying availability before each dependent
package: linux-targets → kernel-abi-tools → linux-abi-scan → libc-abi-tools →
umbrella linux-abi-tools. The root libc package is independent of the first three
after its existing upstream inputs. No extra upstream glibc release was needed.

Official registry API, sparse-index entries and downloaded archives were checked
independently. Every archive's `.cargo_vcs_info.json` identifies clean commit
`d404073e2329405e9e6f2dd1005fd8671e0d0f41`.

| Package | Version | SHA-256 of published archive |
| --- | --- | --- |
| [linux-targets](https://crates.io/api/v1/crates/linux-targets/0.3.2) | 0.3.2 | `a9e3ec31c779ff773874ca7541dbabc85868aa965af2c54f951516ea5699c104` |
| [kernel-abi-tools](https://crates.io/api/v1/crates/kernel-abi-tools/0.3.2) | 0.3.2 | `a9da5a4b7dedfb152d6635d7de87efa22a92b1fd6f496ec351caa6d3bbdfd209` |
| [linux-abi-scan](https://crates.io/api/v1/crates/linux-abi-scan/0.3.2) | 0.3.2 | `d2ea34b1aa4ff30bc553fd4a7061da0a3f40914d6e75394a2c1391c6d947bff4` |
| [libc-abi-tools](https://crates.io/api/v1/crates/libc-abi-tools/0.3.2) | 0.3.2 | `079327b25d8d9089e4dc20dc09de4d3dcc0fb2762b55166854c7787bc33db9fc` |
| [linux-abi-tools](https://crates.io/api/v1/crates/linux-abi-tools/0.3.2) | 0.3.2 | `532043a3b5b8e7012cc0b0bbcb1380910a725b6a145837edda7865dbf2ce2b4f` |

## API documentation status

Checked after publication on 2026-10-31 via each version's docs.rs status endpoint:

- [linux-targets](https://docs.rs/crate/linux-targets/0.3.2/status.json): `doc_status: true`.
- [kernel-abi-tools](https://docs.rs/crate/kernel-abi-tools/0.3.2/status.json): `doc_status: true`.
- [linux-abi-scan](https://docs.rs/crate/linux-abi-scan/0.3.2/status.json): `doc_status: true`.
- [libc-abi-tools](https://docs.rs/crate/libc-abi-tools/0.3.2/status.json): `doc_status: true`.
- [linux-abi-tools](https://docs.rs/crate/linux-abi-tools/0.3.2/status.json): `doc_status: true` (confirmed on recheck after an initial HTTP 404).

## Consumer adoption

Only **after producer publication**, cargo-libc raises its `libc-abi-tools`
minimum to **0.3.2** if its complete provider pipe behavior depends on this fix,
refreshes its real registry lock, and validates unpatched integration plus a
locked install before its own authorized release. Other dependency floors follow
actual required behavior, not automatic lockstep with this workspace. Temporary
source patches are only for isolated pre-publication integration: never commit
them to production manifests or claim nonexistent registry versions were adopted.

## Release status

Producer publication and tagging are complete. Revised producer and scoped
source-patched consumer qualification passed. Real-registry cargo-libc adoption
and locked-install verification are separate downstream release work; they are
not claimed complete by this producer record. All five docs.rs status endpoints now report successful documentation builds,
as recorded above.
