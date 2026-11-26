# Security checks, fuzzing and CI inputs

This is the maintained security-tooling guide (LAT-REQ-033). These checks support
review; they do not establish application compatibility, SLSA Build L3, target
runtime floors or binary distribution clearance. Tier 3 qualification remains
deferred, and the prebuilt-provider legal gate is unchanged.

## SAST and workflow policy

CodeQL default setup remains managed by the coordinator. Never put CI-skip
directives in commit or merge messages: they suppress CodeQL and other CI checks.
Historical commits and their dated evidence are not rewritten. Required LF,
PR-only main, immutable release tags, read-only default permissions and scoped
job permissions remain in force. Dependabot version updates remain monthly,
grouped minor/patch, with limit two and a seven-day cooldown; the actor/author
guards skip expensive qualification and fuzzing for bot PRs. Core checks stay on.

Every external Action reference uses a full SHA with its version comment. The
fixture reusable workflow uses a local caller-commit reference, not a mutable
remote ref. Its two narrow zizmor `self-repository` exceptions preserve syntax
supported by the pinned actionlint and Scorecard parsers.

## Integration cache isolation

The glibc integration workflow runs on main/tag pushes, pull requests and manual
dispatches. Manual `companion_ref` may select unreviewed cargo-libc code, so
[glibc-2.17.yml](../.github/workflows/glibc-2.17.yml) sets `cache-mode: none` for
both `prepare` and `application`, on every trigger. This denies cache reads and
writes in the job's cache token, including direct service access; read-only
`contents` permission alone does not remove cache authority. Do not override
this with write-capable job settings or introduce cross-run caches here.

`ABI_PROVIDER_CACHE` is a directory under `$RUNNER_TEMP` on an ephemeral hosted
runner. The prepare jobs upload provider inputs as run-scoped artifacts, and
application jobs download them without a token or run ID selecting another run.
Those directories/artifacts remain available within the run; they are separate
from GitHub's repository cache service. Existing CodeQL default-setup caches
are managed separately and cannot be read or written by this integration
workflow. Action pins and the input-selected companion checkout are preserved.

## Runners, Actions and Rust toolchains

- Jobs use explicit runner labels: `ubuntu-24.04` and `windows-2025`. Moving
  labels such as `ubuntu-latest` are not used: by 2026-11-21 GitHub had started
  moving `ubuntu-latest` jobs onto Ubuntu 26.04. Moving to Ubuntu 26.04 (or a
  newer Windows image) must be a deliberate, reviewed change to every label.
- Every JavaScript Action runs on Node 24; Docker and composite Actions have no
  Node runtime of their own. Version comments name the exact release tag
  (`# vX.Y.Z`) of the pinned SHA, except ClusterFuzzLite, which publishes only `v1`.
- `dtolnay/rust-toolchain` is pinned to the commit of the branch named after the
  intended toolchain (`1.99.0` for regular CI, `1.90.0` for MSRV and publishing),
  never with a `toolchain:` input, which a SHA pin ignores. Each job's log
  prints the `rustc --version` it actually used.
- Pull requests that change the release workflow, release scripts, changelog,
  release notes or crate manifests/lock dry-run
  [trusted-publishing.yml](../.github/workflows/trusted-publishing.yml): its
  `gate` and `build` jobs package the release set with Rust 1.90, check each
  archive's clean VCS identity and produce SBOMs and `SHA256SUMS`, without the
  `crates-io` environment, a token or an upload.

## Installation and image pins

- Ubuntu packages come from apt with its signed metadata and are not version
  pinned: Scorecard does not score apt, and exact pins (notably the daily Clang24
  snapshot, which apt.llvm.org deletes) would break CI within days. The Clang
  installer still rejects any major other than the one requested.
- [.github/install-clang.sh](../.github/install-clang.sh) verifies the LLVM signing
  key's pinned SHA-256 before importing it.
- The SLES 12 SP5 CI image is fixed at the multi-platform manifest digest
  `sha256:51280921c24d1aeb6fe2d396e92d43912ba5b03c006407de20ef21592425c21a`.
  Generated distro presets/raw ABI inputs are not edited. Provider builder
  images/archives already carry their recorded digest/hash pins; crun 1.28 already
  verifies a literal SHA-256 before execution. Those contracts are preserved.
- [.clusterfuzzlite/Dockerfile](../.clusterfuzzlite/Dockerfile) fixes the complete
  OSS-Fuzz Rust builder image by digest, selects its installed
  `nightly-2025-09-05`, and installs cargo-fuzz 0.13.2 with an exact version and
  `--locked`. ClusterFuzzLite Actions themselves are pinned to the `v1` commit.
  Upstream Action implementation/dependency closure is not independently rebuilt
  here; a pinned Action reference is not a universal hermeticity claim.
- No repository workflow installs Python packages with pip. Any future pip
  installation must use a reviewed hashed requirements file with `--require-hashes`.
  Future Cargo tool installs require an exact `--version` and `--locked`;
  downloaded executables/scripts require a literal pinned SHA-256 verified before
  execution. Container image references must use digests.

The image digests and key hash were resolved from official registries and
registry manifests on 2026-11-18. Review availability, signatures/checksums,
licenses and applicable compiler/provider qualification when changing them.
Do not rotate toolchain pins automatically: an Action SHA can select a different
compiler, as identified in the held toolchain update PR #21.

## Fuzz targets and oracles

[fuzz/Cargo.toml](../fuzz/Cargo.toml) is a separate, unpublished workspace,
explicitly excluded from the main workspace. Its own committed lock fixes
libfuzzer-sys 0.4.13 and fuzz-only transitive dependencies. Neither libfuzzer-sys
nor arbitrary enters the shipped crate graph. Root data/source dependency
requirements and the repository lock stay unchanged.

| Target | Actual implementation exercised | Oracle and boundaries |
| --- | --- | --- |
| `abi_snapshot` | `glibc::parse_abilist`, `Snapshot::to_tsv`, `Matrix::from_snapshots` | Grouped/modern lists, kinds, sizes, duplicates and numeric GLIBC/GCC versions; rejected text is normal. Snapshot filesystem containment/collection is not randomized: its byte parser is the same abilist decoder. |
| `metadata_tsv` | `elf::Metadata::from_tsv` and serialization | Parse, format and parse again without panics. Arbitrary accepted ELF names are not assumed to have stronger round-trip semantics than the table format specifies. |
| `provider_manifest` | `provider::json::parse`, `Provider::parse`, `build::read_interfaces` | JSON values must survive pretty-print/reparse; descriptors and interface TSV may reject. Does not open paths, compile inputs or execute subprocesses selected by fuzz data. |
| `kernel_data` | The same private syscall/madvise TSV decoder used by the embedded tables; `KernelVersion::parse`, `Runner::parse` | Fallible decoding accepts/rejects arbitrary rows without panics. Public APIs still read only embedded data; known-good table results and profile decisions are unchanged. |
| `elf_scan` | Root `elf::Metadata::parse`, scanner `elf::Elf::parse` and `scan` | Bounded ELF64 x86_64 metadata and static code inspection; parser/resource errors are normal. Parsed bytes are never executed. |
| `clock_profile` | The actual private `elf::libc_clock_copy` validator/deriver, compiled from its production source | Raw and structured mutations around the source-built libc/librt fixtures. Successful copies preserve length and can only set GNU nondefault bits; no runtime clock calls. |
| `recorded_libgcc` | The production private libgcc interface TSV decoder | Fuzzes the same decoder used for recorded input, without a public parser API or edits to the recorded GCC table. |
| `target_selection` | Numeric `Version`, release validation, versioned target parsing and `Selection::resolve` | Numeric versions survive canonical display/parse; invalid selections reject normally. No broader architecture/libc support is introduced. |

Private modules used only through internal APIs are compiled directly from their
production source into the fuzz-only crate. There are no copied parser bodies.
The kernel decoder returns errors for malformed external test input; its ordinary
callers still assert that their embedded tables are valid. Data/profiles were not
regenerated or edited. UTF-8 text targets reject non-UTF-8 bytes before decoding;
ELF targets accept arbitrary byte slices.

All targets use AddressSanitizer, debug/overflow checks, one fuzz worker, a 64 KiB
input limit, 512 MiB RSS limit and five-second per-input timeout. Those limits
bound this campaign, not the parser's entire supported input space. Existing
large-valid-ELF and explicit resource-budget regressions remain separate evidence.

## Local runs and CI campaigns

On Linux, install Clang/C++ build tools and GNU x86_64 binutils (cross-binutils
on aarch64). Tool prerequisites are for maintainer tests, not ordinary crate use.
For example, with the selected toolchain available:

```sh
rustup toolchain install nightly-2025-09-05 --profile minimal
cargo +nightly-2025-09-05 install cargo-fuzz --locked --version 0.13.2
python3 scripts/build-test-fixtures.py
python3 scripts/seed-fuzz-corpus.py
cargo fetch --locked --manifest-path fuzz/Cargo.toml
CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true cargo +nightly-2025-09-05 fuzz build -O --debug-assertions
cargo +nightly-2025-09-05 fuzz run elf_scan -- -max_total_time=300 -max_len=65536 -rss_limit_mb=512 -timeout=5
```

Choose any target from the table. Curated text seeds in `fuzz/seeds/` derive from
existing parser tests/table schemas. The seed script also copies the source-built
ELF fixtures into ignored, mutable `fuzz/corpus/`. Keep generated corpora, target
builds and raw crash artifacts out of Git. A crash is not an acceptable expected
outcome: retain the reproducer, minimize it, fix the production decoder with a
regression test, or escalate a genuine contract/product decision to the owner.
Do not add an assertion bypass or ignore a crash to make CI green.

[PR fuzzing](../.github/workflows/cflite_pr.yml) allocates **300 seconds total** to
code-change fuzzing when parser/fuzz sources or their integration change, and
skips Dependabot actors/authors. Its per-PR concurrency cancels stale campaigns.
[Weekly batch fuzzing](../.github/workflows/cflite_batch.yml) allocates **1800 seconds**
to batch mode. Both allow owner manual dispatch after workflow registration on
main. Crash SARIF is uploaded to code scanning when produced and permissions
allow it; fork PRs skip that upload and retain ordinary fuzz logs/artifacts.
No new secret or external corpus-storage repository is configured: campaigns
start with the reviewed seeds rather than claiming cross-run corpus persistence.

Native aarch64 smoke runs and hosted x86_64 ClusterFuzzLite execution are distinct
evidence. Neither establishes old-OS runtime compatibility or whole-parser
coverage. A weekly workflow definition is not evidence of a completed weekly run.

## Retired committed binary inventory

The 2026-11-18 tracked-file inventory found exactly these three binary files
(all original MIT test data; never shipped in crate archives):

| Former committed path | Original SHA-256 | Source and retained coverage |
| --- | --- | --- |
| `tests/fixtures/elf/symbols.so` | `5582c32814f3345be821e5cf9608eae55fa7f674f4bfb40f8ea7665459f4d89b` | `symbols.S`/`symbols.map`: symbol versions, weak/protected binding, aliases, IFUNC/TLS, imports and storage evidence. |
| `tests/fixtures/clock-profile/c.so` | `d5194aa9bc43548c72dcc27ed97c29f04cfa8010f89a4b4ff8d22b4dcd1a83ae` | `c.S`/`c.map`: the five weak libc clock defaults. |
| `tests/fixtures/clock-profile/rt.so` | `8b8aa201f5387d1d8507923670c22038af9114591a5379b0560068c68ee1c015` | `rt.S`/`rt.map`: legacy IFUNC clock defaults and genuine non-clock RT control. |

They are removed from Git. The reviewed sources remain, and
[scripts/build-test-fixtures.py](../scripts/build-test-fixtures.py) builds all
three into ignored paths. GNU 2.40 on the aarch64 maintainer host reproduces all
three original hashes; this is bounded tool-version evidence, not a claim that
all linkers emit identical bytes. Existing tests check semantic ELF properties.
Source-only `SHA256SUMS` retain assembly/map hashes; the generated manifest
records actual tool versions, source identities and output checksums.

The [reusable fixture job](../.github/workflows/test-fixtures.yml) builds once in
Ubuntu, and both Linux/Windows tests verify and consume the same workflow's
artifact. Verification checks the fixed inventory, hashes and matching source
inputs; it is not publisher authentication. Windows developers need GNU cross
binutils or the corresponding source-built CI artifact; `--verify-only` checks
its inventory before running tests. No encoded replacement binary is committed.

## Check evidence and limits

On the clean source snapshot, local OpenSSF Scorecard 5.6.0 detected ClusterFuzzLite
and all eight cargo-fuzz targets, reported Fuzzing 10, Binary-Artifacts 10, and
Pinned-Dependencies 10. The latter found 35/35 GitHub-owned Actions, 21/21 external
Actions and the builder image pinned. This is a local file-policy result; it is
not the public dashboard's score. Local SAST cannot inspect managed CodeQL runs
and reports no merged-PR metadata; it is not a live SAST measurement. The hosted
CodeQL configuration is unchanged, and all new commit/merge messages must keep
CI enabled. Public scores and the first scheduled batch require post-merge checks.
