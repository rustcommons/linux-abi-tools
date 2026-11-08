# Provider-cache concurrency repair

## Problem and chosen boundary

The reviewed 0.5.0 implementation removes the installed directory before
renaming a forced replacement. Readers holding its paths can see missing or
replaced objects; failed publication can leave no installed entry. Temporary
names also contain only the PID, so two attempts in one process delete each
other's unfinished directories.

Both regressions were reproduced before editing the implementation:
`stages_in_one_process_never_delete_each_other` observed identical staging paths;
`forced_preparation_preserves_paths_already_given_to_readers` read the new bytes
through the old reader path. Both tests failed on the baseline and pass with the
repair. These are actual filesystem tests, not an inference from source alone.

Complete immutable generations plus an atomic regular-file selector preserve
old returned paths without requiring compiler/build-script reader leases.
Writers only clean up their own unpublished generations. Normal writers retain
one winner; forced writers can each return a complete generation and the last
successful selection becomes current. Legacy directories remain untouched.

## Regression and compatibility coverage

- Same-process staging collision; legacy-reader preservation; eight simultaneous
  normal writers; four forced writers publishing sixteen generations each while
  a reader resolves one thousand complete selections.
- Open selection-file snapshot across replacement, failed publication into a
  competing directory, malformed/escaping/oversized/wrong-key selections and Unix
  selection/generation symlink refusal.
- Legacy discovery, one-row logical listing, export/import with descriptor v1,
  quiescent removal, interrupted orphan cleanup and corrupt-selection removal.
- Public current-lookup release/hash validation; existing broken-pipe export
  behavior; default and no-default builds. Cache `entry` constructs a logical
  base path; `current` resolves actual generation paths.
- A separate CLI regression reproduced unrelated malformed selections blocking a
  valid single-release export. Release/mode filtering now happens before selection
  resolution, avoiding repeated whole-cache manifest parsing. The regression
  passes on Windows and Linux; unfiltered listing still reports corrupt selections.

Windows/NTFS Rust 1.99 root default/no-default tests and Clippy passed, including
ten publication tests. Linux/WSL Rust 1.90 root tests passed, including eleven
publication tests (the additional Unix symlink case). Existing two optional
clock-profile native gates remain ignored; they are unrelated to this repair.
The final public argument guards and container-mode publication fixture also
passed 26 focused Windows and 27 Linux provider tests. Windows filesystem
testing is not support for preparing providers on Windows.

An initial real source-provider process gate on native x86-64 GNU/WSL passed
three normal writers, three forced writers and **15,522** complete old-path
observation cycles (five held files per cycle), plus verify/export/import/remove.
It used the new publication implementation before the later thin public-lookup
argument-validation guard; that guard has separate focused tests and is not on
the CLI path. The final all-release and process gates are recorded below.

## Permanent gates and scope

`scripts/check-provider-concurrency.py LIBC_ABI GCC NEW_EVIDENCE` records real
process stdout/stderr/statuses, held-file hashes and observation counts. The
existing source-provider CI runs it for each configured GCC after the 28-release
cache check. The all-release script now discovers actual paths through `provider
path` instead of assuming a physical cache directory name.

This fix is unreleased. Public `cache::current`, changed physical cache layout and
explicit selection-corruption refusal require a producer minor release and
deliberate downstream adoption. Published producer 0.5.0 and cargo-libc 0.6.0 are
unchanged. No version, dependency, data, descriptor schema, LLVM or link-policy
change is included here.

Retired generations consume disk until explicit removal with builds/cache
operations stopped. There is no automatic GC, destructive-removal lease,
authenticated delivery, crash-durable transaction or distributed-filesystem
guarantee. Older binaries and manual edits are not coordinated. Host startup
objects outside the provider remain independently managed files. Shared
publication is filesystem-tested for both preparation modes; real-container
builds and old-system/musl/static-host qualification are not newly claimed.

## Final native gates

The optimized Rust 1.90 source tool at `f1aef1cf1b09ba483825df5056a2562cafbd6815`
passed the complete existing `check-provider-cache.sh` with GCC **13.3.0**:
all 28 releases prepared and verified, cached reuse, forced rebuild, export/import,
tampering detection, removal and unsupported-input refusals. Exit 0, 587.32 seconds.
The following real multi-process gate also passed (14.35 seconds, 2,670 complete
held-path observation cycles). This covers the shared publication implementation;
the subsequent single-release filtering delta has its own Windows/Linux CLI test.

Final source `7b1125b956b38d762b6f8510dcaa49317eb34bdc` was rebuilt under
Rust 1.90 release mode and passed the real multi-process gate again: three normal
writers, three forced writers, **2,654** complete observation cycles over five
held files, and verify/export/import/remove. An actual installed cargo-libc
**0.6.0** then built and ran the C/CMake fixture using the **old** descriptor
after a forced preparation selected a different generation. Both C and CMake
reported glibc 2.17 headers. This is modern GNU/WSL host execution, not an old-OS
or old-kernel qualification; explicit descriptor interoperability is not registry
adoption of the unreleased producer API.

`cargo +1.90.0 package --workspace --locked --offline` verified all five source
archives at clean `7b1125b`, with exact VCS/source payloads. Root comparison to
official libc-abi-tools 0.5.0 finds only the provider implementation changes and
new `src/provider/cache_selection.rs`, plus generated metadata. These are
**unreleased source-verification archives**, not replacements for published 0.5.0.
No versions, dependencies or lock files changed.

The preliminary debug-build all-release run prepared all 28 releases and verified
2.17/2.18 before being deliberately interrupted during a path query (exit 143)
to switch expensive inventory checks to the optimized build. That partial run is
retained and is not counted as passing. Both later optimized gates above completed
normally; the final cargo compatibility smoke also completed normally.

Evidence packet: **120 verified member hashes**, 161,227 bytes, SHA256
`b6b43d0914453e3d55bf6703f14a28cb998c3f005c060c0aa567a4f1ae0e210d`.
The packet includes logs, summaries, source-package audits and all five source
archives. It is retained in coordinator thread storage as
`provider-cache-evidence/provider-cache-proof.tar.gz`, with the WSL originals
under `/tmp/rustcommons-provider-cache-concurrency`. Later excluded documentation
updates preserve this source and package evidence; a later VCS identity is not
the tested archive identity. Independent review is still required before merge.
