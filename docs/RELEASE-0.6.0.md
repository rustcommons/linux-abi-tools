# Producer 0.6.0 release record

## Scope and status

**Published 2026-11-08 UTC** from `f409ae8e2b379e414d94809c8f4e6627d29eca20`.
All five archives, the annotated tag, docs.rs and a fresh registry installation
are verified below. GitHub Release publication and full published-docs
verification are complete; the final closeout follows the publication record.

Preparation began at the maintainer's request on the same UTC date. This is
the five-crate linux-abi-tools workspace release, not the already released
cargo-libc 0.6.0. Implementation PR14 merged normally as
`baf407084ea0d6a6b8fdfeb7d0ef9f79956429f9`; its independently approved tree
is preserved. This candidate changes only version metadata and excluded docs.
Source, tests, scripts, workflows, data, examples, package allowlists and licenses
must remain identical to that base.

## Version and dependency contract

- All five crates are 0.6.0, with all seven internal path/caret requirements at
  0.6.0. Public `cache::current` and the new physical cache layout/refusal contract
  warrant minor under the lockstep policy.
- The repository lock changes only the five workspace version entries. Both
  upstream glibc dependencies stay locked at 0.1.0 with identical checksums;
  manifest requirements, MSRV 1.90 and native-tool floors are unchanged.
- Root version is a provider input: preparation may use a new key. Generation
  retention is separate from cross-version/import discovery; no automatic offline
  migration or whole-toolchain hermeticity is promised.

## Carried implementation evidence

[Dated evidence](evals/2026-11-07-provider-cache-concurrency.md) records baseline
regressions, owner Windows/Linux checks, all-28 source/cache lifecycle, real
concurrent preparations/readers and old-descriptor cargo-libc C/CMake smoke.
Independent review verified all 120 proof hashes/five source archives and passed
15 focused Rust 1.90 checks. Final punctuation-only delta was approved before
merge. [Full published-docs CI42433405415](https://github.com/rustcommons/linux-abi-tools/actions/runs/42433405415)
passed on the merge. These are implementation evidence, not qualification of the
new 0.6.0 version/archive identities.

## Candidate gates

Qualified package-input commit: `b1f00979fa5c8f9695ee7e973397a639fcd4c5a0`.
Rust 1.90 Linux/WSL gates passed with two build/test workers:

- Workspace tests: 145 passes across unit, integration, process and doctest
  results; three existing ignored cases (two optional clock-profile fixtures and
  the official-toolchain ELF fixture) are not counted as passes.
- Root no-default tests: 72 passes, two optional clock fixtures ignored.
- All five `cargo package --workspace --locked --offline` packages verified;
  clean VCS names the exact commit. File lists are identical to the reviewed
  implementation archives at `7b1125b956b38d762b6f8510dcaa49317eb34bdc`.
  Only manifests, packaged locks and VCS metadata differ; source, README, data
  and license payloads are byte-identical.
- Real source-provider process gate: three normal and three forced writers;
  2,307 complete held-file read cycles, no errors. The five held files retained
  their hashes and the returned old descriptor remained usable by the gate.
- Import/version refresh: a real 0.5.0 export imports and verifies. It does not
  silently satisfy the new version's computed key (`provider path` returns 1).
  New preparation selects key `e68e4fb5c5354222` instead of old
  `672680f3be17393c`, while preserving the imported descriptor's bytes.
- Metadata and lock comparison: only five workspace version entries change;
  all seven internal requirements match 0.6.0 and both upstream inputs remain
  registry 0.1.0. Implementation, test, script and workflow trees match PR14's
  merge exactly.

Windows Rust 1.99 workspace fmt, all-target Clippy with warnings denied and
workspace rustdoc with warnings denied passed. These checks do not claim
Windows provider preparation. Unchanged broad provider/kernel/clock evidence
keeps its original scope; no new container or old-system result is claimed.

The preparation proof packet has 1,194 hashed files, including all five actual
archives, command outputs, metadata, payload comparisons and process evidence.
Packet SHA-256: `6cf9885db63fc017ae91d7a47204f3cb40818525e9dc1e6bc6ac7bd9caf48963`
(2,048,877 bytes). These are local candidate checks, not registry-only installs:
Cargo's workspace package verification uses its temporary internal registry.

| Candidate archive | SHA-256 | Bytes | Files |
| --- | --- | --- | --- |
| linux-targets 0.6.0 | `f90925ab48aa03ca89ec68ef2d52a1b2d8f41bbe86ab954d5bc0579a27bf21db` | 7,407 | 8 |
| kernel-abi-tools 0.6.0 | `d4839acefdd9b662ad74c16c2c8328631014101e3f625616807cc239101158a7` | 24,023 | 15 |
| linux-abi-scan 0.6.0 | `a916d2c5c305cb5deef5cbad21194b617a543f40606109a69326ab66e5233609` | 23,874 | 11 |
| libc-abi-tools 0.6.0 | `b75d0070a1c1f413df924900e2ce5f84c10bd57087a7520a18ef1fa15bda0b2d` | 84,543 | 84 |
| linux-abi-tools 0.6.0 | `d8c235b86e2dc293a097e46e836aca5eb79d2ea2310bf5c4a6dd64feeed75006` | 3,983 | 7 |

These hashes identify the clean qualified commit, not a future merge/archive.
Later excluded-doc changes do not change package inputs, but fresh VCS metadata
and temporary-registry lock checksums can change archive hashes. Independent
release review and exact-head release-doc CI remain required before merge.

## Original preparation plan and downstream boundary

The preparation-only candidate did not publish or merge. The maintainer later
authorized publication; the sequence below was the plan. Producer archive/tag
verification is now complete; consumer adoption remains pending:

1. Merge normally; compare the exact merge tree/package inputs to the qualified
   candidate. Repackage with clean merged VCS identity.
2. Publish linux-targets, kernel-abi-tools, linux-abi-scan, libc-abi-tools, then
   linux-abi-tools. Verify each official archive/index checksum, non-yanked state,
   source/VCS and docs.rs. Tag the exact released commit without replacing tags.
3. Only after real crates.io availability, adopt libc-abi-tools 0.6.0 as the
   cargo-libc minimum and refresh its real registry lock deliberately. Keep other
   tool/data minima and checksums unless separately justified. Validate unpatched
   package/install plus managed and explicit-provider, plain-Cargo, native C and
   cache generation/refresh workflows. Assess the consumer release version under
   its own policy; do not silently assign it the producer's version.
4. Complete source/registry/docs/tag checks and reviewed GitHub Release notes,
   latest/zero-assets and full published-docs verification. Do not attach new binary
   bundles. Record incomplete gates honestly.

## Limits

Old paths survive preparation, not explicit removal, manual edits or external
startup-file/tool changes. Removal needs stopped builds/cache operations.
Retained generations cost disk; no automatic GC, crash-durable storage,
distributed-filesystem or authenticity guarantee is added. Real-container and
old-system/tier 3/musl/static-host gaps remain explicit. LLVM research, prebuilt
legal gates and other review follow-ups remain separate and unchanged.

## Published archives and fresh installation (2026-11-08 UTC)

[PR15](https://github.com/rustcommons/linux-abi-tools/pull/15) merged normally
as `f409ae8e2b379e414d94809c8f4e6627d29eca20`, preserving the approved
`b9c5e2533d7931c4b20e2dd3ae2a3a683190d009` tree. Independent release review
verified all 1,194 candidate proof hashes. Exact candidate
[docs CI42441937650](https://github.com/rustcommons/linux-abi-tools/actions/runs/42441937650)
and merged [docs CI42448883941](https://github.com/rustcommons/linux-abi-tools/actions/runs/42448883941)
passed. The separately triggered broad CI is not assumed passed here; unchanged
source/kernel/clock gates retain their independently reviewed scope.

The sole publisher used Rust/Cargo 1.90 `publish --locked -p CRATE` in dependency
order. All five official archives are non-yanked, match both crates.io API and
sparse-index SHA-256 values, and carry clean VCS at the exact merge. File lists
and source/README/license payloads match the qualified candidate. Differences
are VCS metadata and, in dependent packages, temporary-versus-official registry
checksums in generated locks; every packaged registry checksum was checked.

| Crate 0.6.0 | Official SHA-256 | Bytes | Files |
| --- | --- | ---: | ---: |
| [linux-targets](https://crates.io/crates/linux-targets/0.6.0) | `1ee91ba9349ba6303cdc3e4aa4789321bb3be739a2dd07d8ff3889fbf7f21980` | 7,407 | 8 |
| [kernel-abi-tools](https://crates.io/crates/kernel-abi-tools/0.6.0) | `9424376719765b7cafc3980c656216ee15704b177a81e146fb8d23f7799d667d` | 24,022 | 15 |
| [linux-abi-scan](https://crates.io/crates/linux-abi-scan/0.6.0) | `6a36d0e3d933ad5fae54b70e8f1e6caa6419cbebcf020e7e5e3c0635b91c3504` | 23,877 | 11 |
| [libc-abi-tools](https://crates.io/crates/libc-abi-tools/0.6.0) | `368579c7f6401ead63390c8c273da9d9840a905438b62e603917bb169e084d6e` | 84,541 | 84 |
| [linux-abi-tools](https://crates.io/crates/linux-abi-tools/0.6.0) | `08b9f17a2f8486d5d8d892e428901e7e5753da9d94e593f903228f487437e715` | 3,989 | 7 |

Annotated [v0.6.0](https://github.com/rustcommons/linux-abi-tools/tree/v0.6.0)
peels to the release merge; no old tag moved. All five docs.rs builds succeeded
and all five versioned API pages were retrieved, including the new
[cache::current](https://docs.rs/libc-abi-tools/0.6.0/libc_abi_tools/provider/cache/fn.current.html).

A fresh Cargo home and isolated install root ran
`cargo install libc-abi-tools --version 0.6.0 --locked` against crates.io on
x86_64 Linux/WSL with Rust 1.90. Installed version/help and the permanent real
source-provider process gate passed: three normal writers, three forced writers,
2,331 complete held-file cycles and no errors.
Installed executable SHA-256:
`a415ace470ce7de0df158e7cf52feaef703edb3d49bfe1af0e2845d0a75ea094`.
This is producer CLI/source preparation evidence, not Cargo adoption or
container/old-system qualification.

The published proof packet includes official archives, exact commands/logs,
the installed executable and process evidence: 1,205 hashed files,
7,878,287 bytes, SHA-256
`e56ce0c4729f2347c4ec60042a202df6140561ea170a7748520a72ecc36ee398`.

The notes candidate initially awaited GitHub Release publication and full
published-documentation verification; both are complete below. No crate upload
or tag was repeated during bookkeeping. Cargo's minimum, registry lock and
release remain unchanged here.

## Final Release and documentation closeout

Completed 2026-11-08 UTC. [PR16](https://github.com/rustcommons/linux-abi-tools/pull/16)
merged normally as `cc3c55ac858c6d302b23d3206f4c934732f88177`, with exactly the
independently approved `4392ff77f673b84cda3be0324a6130b620cbdd9b` tree. Its
[exact-head docs CI42449194081](https://github.com/rustcommons/linux-abi-tools/actions/runs/42449194081)
passed before merge and note publication. The reviewer separately verified all
1,205 published-proof hashes, actual registry installation/binary identity and
2,331 held-file cycles; no runtime test was repeated for the docs.

The [live v0.6.0 Release](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.6.0)
uses the exact committed notes, is latest and stable, and has zero assets.
The annotated tag remains at exact release source `f409ae8e2b379e414d94809c8f4e6627d29eca20`.
The committed note SHA-256 is `6ff0884b41f186fad3368314ce26026d550b39f00881d96384fe99952daed8c4`.
Both the authenticated local `--published` checker and
[full nine-version published CI42449316473](https://github.com/rustcommons/linux-abi-tools/actions/runs/42449316473)
passed. The [release-event check42449316132](https://github.com/rustcommons/linux-abi-tools/actions/runs/42449316132)
also passed. Exact live body/latest/assets and merge/tag identities were checked
independently. This closes the producer release, not Cargo minimum adoption or
new runtime/platform qualification. All limits above still apply.
