# Producer 0.6.0 release preparation

## Scope and status

Prepared at the maintainer's request, 2026-11-08 UTC. **Not published.** This is
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

Pending exact committed candidate: workspace/default and root no-default tests,
fmt/Clippy/docs, Rust 1.90, metadata/lock coherence, all-five verified packages and
payload/license/VCS audits. Repeat the bounded real source-provider process gate
for 0.6.0 and check version-key refresh/import behavior. Reuse unchanged raw-data,
kernel, broad provider and clock evidence with its original scope; do not invent
new container or runtime results.

## Publication and downstream plan

No merge, publication, tag or GitHub Release is performed by preparation.
After independent release review and explicit publication authorization:

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
