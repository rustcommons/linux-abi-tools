# Next release plan

Date: **2026-10-31**. Proposal only; no version bump, package publication, tag,
release, or new compatibility claim is authorized by this document.

## Decision and scope

For GitHub-only instruction, guide, roadmap, and repository metadata cleanup,
**do not publish merely to synchronize GitHub**. README changes included in crate
archives, Rust API documentation on docs.rs, code fixes, and embedded data changes
need a new crate version to reach registry consumers. Existing published archives
and versioned docs.rs pages do not change when main changes.

If shipping the existing `kernel-seccomp` broken-pipe fix and approved packaged
documentation, recommend **0.3.2 for all five workspace crates**, subject to gates
below. This is a patch under the current [versioning policy](RELEASING.md#versioning).
Commit `7fbd303` implements the kernel fix after v0.3.1; fixes for `libc-abi` and
`linux-abi-scan` are still open in [the roadmap](ROADMAP.md). Either explicitly
exclude those two fixes from the candidate, or implement and validate them in a
separately authorized task before including them. This planning task adds no code.

New presets, kernels, rules, API/CLI additions, or breaking changes require review
for **0.4.0** under that policy. Do not disguise expanded simulation or provider
coverage as documentation. No new glibc releases, regenerated packs, prebuilt
provider mode, MSRV changes, or runtime qualification claims are in this candidate.

All five crates currently release in lockstep while 0.x. Selective releases could
reduce unrelated publications, but are **not the existing policy**: they need an
explicit policy decision and a review of internal version requirements first.
The upstream glibc repositories and cargo-libc have independent versions; do not
bump them just to match this workspace.

## Evidence snapshot

Checked live on 2026-10-31 using GitHub manifests, lockfiles, tags, and crates.io
API version records. These are observations, not predictions; recheck before release.
Every version listed below was unyanked. The API also listed no yanked older
versions for these packages at the time of review.

| Published package | Latest version | Registry evidence | Matching Git tag |
| --- | --- | --- | --- |
| `linux-targets` | 0.3.1 | [API](https://crates.io/api/v1/crates/linux-targets) | [v0.3.1](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.1) |
| `kernel-abi-tools` | 0.3.1 | [API](https://crates.io/api/v1/crates/kernel-abi-tools) | same workspace tag |
| `linux-abi-scan` | 0.3.1 | [API](https://crates.io/api/v1/crates/linux-abi-scan) | same workspace tag |
| `libc-abi-tools` | 0.3.1 | [API](https://crates.io/api/v1/crates/libc-abi-tools) | same workspace tag |
| `linux-abi-tools` | 0.3.1 | [API](https://crates.io/api/v1/crates/linux-abi-tools) | same workspace tag |
| `cargo-libc` | 0.2.0 | [API](https://crates.io/api/v1/crates/cargo-libc) | [v0.2.0](https://github.com/rustcommons/cargo-libc/tree/v0.2.0) |
| `glibc-abilists` | 0.1.1 | [API](https://crates.io/api/v1/crates/glibc-abilists) | [v0.1.1](https://github.com/rustcommons/glibc-abilists/tree/v0.1.1) |
| `glibc-provider-sources` | 0.1.0 | [API](https://crates.io/api/v1/crates/glibc-provider-sources) | [v0.1.0](https://github.com/rustcommons/glibc-provider/tree/v0.1.0) |

Git baselines inspected:

- linux-abi-tools main `6c67fa61481a516edd020291085457a1b864be4a`; v0.3.1
  `3cd97929a99280fca2f616dae63bd610a038e131`.
- cargo-libc main `ed7033ddd49f189d8b964cac510908ec56feffd2`; v0.2.0
  `3b283b044dca36d0e5b446d99874e0c46ea8c402`.
- glibc-abilists main and v0.1.1 `d7e5228dae5b2591fc7849a9b82607a7d6bc25bb`.
- glibc-provider main `dff7aae4f1b7e2c3f4151d88349e5812aa03092b`; v0.1.0
  `1ae578cf7123399b0c710f44defab93cc2021001`.

Tags alone do not prove package contents or successful qualification. Inspect the
candidate's packaged files and CI evidence before authorizing publication.

## Consumer requirements, locks, and embedded inputs

Sources: [workspace manifest](../Cargo.toml), [workspace lock](../Cargo.lock),
[Cargo consumer manifest](https://github.com/rustcommons/cargo-libc/blob/ed7033ddd49f189d8b964cac510908ec56feffd2/Cargo.toml),
[Cargo consumer lock](https://github.com/rustcommons/cargo-libc/blob/ed7033ddd49f189d8b964cac510908ec56feffd2/Cargo.lock).
External consumers are **unknown, not absent**.

| Consumer | Manifest requirement / features | Observed lock resolution |
| --- | --- | --- |
| root `libc-abi-tools` | Optional `glibc-abilists = "0.1.0"`, enabled by default `bundled-glibc-data` | 0.1.0, despite compatible 0.1.1 being available |
| root `libc-abi-tools` | Optional `glibc-provider-sources = "0.1.0"`, enabled by default `provider-sources` | 0.1.0 |
| `kernel-abi-tools` | `linux-targets`: path plus `version = "0.3.1"` | local 0.3.1 |
| `linux-abi-scan` | `kernel-abi-tools` and `linux-targets`: paths plus `"0.3.1"` | local 0.3.1 |
| umbrella `linux-abi-tools` | all four other workspace crates: paths plus `"0.3.1"`; default features enabled | local 0.3.1 |
| `cargo-libc` | `kernel-abi-tools`, `linux-abi-scan`, `linux-targets`, `libc-abi-tools`: each `"0.3.0"`, nonoptional; no feature opt-out exposed | each 0.3.0; transitive glibc crates each 0.1.0 |

These version strings are compatible ranges, not exact pins: `"0.3.0"` permits
0.3.x; `"0.1.0"` permits 0.1.x. Existing locks retain their selections until
refreshed. Library users have their own lock resolution, not this workspace's.

Default builds embed glibc ABI lists and provenance via `glibc-abilists`, and the
recorded glibc sources, generated headers, commands and installed headers via
`glibc-provider-sources`' pack. Provider qualification exceptions are packaged too.
`libc-abi-tools` also ships its libgcc interface table and selected container build
scripts/input manifests. `linux-targets` embeds distribution presets;
`kernel-abi-tools` embeds syscall/madvise tables, generating profiles from them.
`linux-abi-scan` uses those crates' facts. The umbrella re-exports the four crates;
it does not supply another runtime. cargo-libc compiles these dependencies into
its coordinator/helper tools; updating registry data does not update installed tools.

Disabling `libc-abi-tools` defaults removes both bundled ABI data and provider
sources: external ABI data must then be supplied where required, and source mode
is unavailable. Its features can be selected individually. The umbrella currently
enables defaults and does not forward an opt-out feature. Do not promise a
cargo-libc or umbrella opt-out that their manifests do not implement.

## What propagates, and what does not

| Change | Publication and consumer action |
| --- | --- |
| AGENTS/shim, GitHub description/topics, excluded guides/roadmaps | Git push or metadata update suffices; no package release needed |
| Packaged README or license/notice changes | New version of the owning crate to update its registry archive/README; downstream rebuild is unnecessary for prose alone |
| Rust API docs | New owning-crate version and successful docs.rs build; GitHub edits do not rewrite old API pages |
| Code, preset, syscall table, ABI data or source pack | New owning-crate version; consumer must resolve it and rebuild to obtain changed behavior/data |
| Compatible dependency patch | Manifest edit is not automatically required; refresh a consumer lock only when adopting that version |
| New dependency API/data floor or incompatible range | Raise the relevant manifest requirement, resolve the lock, then qualify the consumer |
| Published CLI's locked install must include newer inputs | Release that CLI with an updated lock after qualification; `cargo install --locked` of its existing release retains the old packaged lock |

A user building from a checkout can update a compatible lock and rebuild without
waiting for a new cargo-libc release. A new cargo-libc release is needed if the
maintainer wants its published locked installation to deliver that updated set.
An installed executable must be rebuilt/reinstalled to change embedded inputs;
refreshing a source lock alone does not change it. Provider cache identity includes
tool/input versions; do not treat an old cache entry as evidence for new inputs.

## Dependency order and upstream decisions

If required upstream versions already exist, reuse them. Documentation-only changes
to excluded glibc repository guides do not require upstream publication. Packaged
README/API-doc updates may justify an independent patch, without regenerating data.
For glibc-abilists, 0.1.1 is documented as a data-unchanged documentation release;
adopting it does not require a manifest change from `"0.1.0"`.

For regenerated ABI lists, rerun the upstream collector and its `--check`; review
tag objects/signatures (including recorded exceptions), mirror agreement, file
hashes, license evidence and semantic diffs. Never edit generated data by hand.
For changed provider inputs, collect from qualified pinned build trees, re-record
interfaces and library tables using the documented tools, regenerate the pack,
and require pack equality, blob integrity and replay qualification. Unchanged
inputs must not be regenerated merely for a README release.
See [ABI provenance](https://github.com/rustcommons/glibc-abilists/blob/main/docs/PROVENANCE.md),
[provider data](https://github.com/rustcommons/glibc-provider/blob/main/docs/DATA.md),
and [replay](https://github.com/rustcommons/glibc-provider/blob/main/docs/REPLAY.md).

For an authorized lockstep workspace release, publish in this valid order:

1. Any **changed and required** `glibc-abilists` and `glibc-provider-sources`
   versions, from their owning repositories (independent of each other).
2. `linux-targets` (no dependencies).
3. `kernel-abi-tools` (requires linux-targets).
4. `linux-abi-scan` (requires kernel-abi-tools and linux-targets).
5. `libc-abi-tools` (requires the two upstream glibc crates; independent of steps 2–4).
6. Umbrella `linux-abi-tools` (requires all four workspace crates).
7. cargo-libc **only if** its own approved scope or published locked dependency
   adoption calls for a release, after consumer qualification.

All internal version requirements and the workspace lock must agree with the
chosen lockstep version in the future release change. The separate musl runtime
probe is not a dependency in either inspected graph and is not certification;
it has no mandatory publication step here.

## Gates for the future candidate

These are future release gates, **not checks run for this documentation task**.
Use [AGENTS.md](../AGENTS.md) and [Releasing](RELEASING.md) for maintained commands.

- Confirm scope, exact commit, clean checkout, upstream versions/yanked state,
  and explicit maintainer release authorization. Inspect changes since v0.3.1;
  do not describe a candidate including the kernel fix as documentation-only.
- Run formatting, workspace tests (all targets plus doc tests), and warning-free
  Clippy on Windows/Linux, including root no-default-features coverage. CI uses
  Rust 1.99.0; the declared MSRV is 1.90. Check all five packages at the MSRV.
- Build workspace API docs with warnings denied. Verify packaged README links
  work on crates.io/docs.rs, not just inside the checkout.
- Run `cargo package --workspace --locked` and inspect each package with
  `cargo package --list -p <crate>`. Confirm all five packages, declared includes,
  licenses/notices, and no private artifacts. Keep the root's intentional
  provider-script exceptions; do not accidentally exclude them with other examples.
- Keep MIT/Zig attribution for applicable code and distinguish LGPL ABI/source
  inputs and the provider pack's full Linux UAPI license expression. Review
  provenance, hashes and corresponding-source obligations for changed inputs.
  Prebuilt mode remains outside scope pending its legal decision.
- For kernel changes, run the existing local and container self-tests and profile
  regeneration comparison; include the broken-pipe regression. Source-only
  changes do not establish old-kernel runtime qualification.
- For changed provider data, replay/build logic or dependencies, require the
  glibc-provider replay matrix and pack checks, plus this workspace's source
  cache checks (CI GCC 12/13/14), container/source comparisons and provider
  integration pipeline. Its full workflow covers each release with Clang 21/24.
  Review the current workflows rather than assuming old successful runs qualify
  changed inputs: [CI](../.github/workflows/ci.yml),
  [kernel](../.github/workflows/kernel-abi-tools.yml),
  [provider integration](../.github/workflows/glibc-2.17.yml),
  [upstream replay](https://github.com/rustcommons/glibc-provider/blob/main/.github/workflows/replay.yml).
- Qualify an intentionally updated cargo-libc dependency resolution with its
  build/check/inspect/run/test/doctor workflows, provider source/container modes,
  host/target separation, refusal cases and cache/import behavior as applicable.
  Record exact consumer/producer commits and resolved versions. For prose-only
  changes, verify commands/links; do not demand unrelated replay builds merely
  because a guide changed.
- A release claiming real old-system compatibility additionally needs tier 3
  evidence on real target kernels/userlands. Neither static scans, containers
  on modern kernels nor the separate musl probe supply that evidence.

## Go/no-go and recovery

- [ ] Candidate scope and patch/minor decision approved; deferred work named.
- [ ] Registry/tag evidence refreshed; dependency graph and locks reviewed.
- [ ] Applicable checks above pass on the exact candidate, with artifacts retained.
- [ ] Package contents, licensing, provenance and documentation reviewed.
- [ ] Consumer adoption decision explicit: none, lock/rebuild only, or new release.
- [ ] Maintainer explicitly authorizes publication; any failed gate means no-go.
- [ ] After publication, verify registry versions, docs.rs status and immutable
      tag/commit association before recording a completed release.

A published version cannot be overwritten. A partial multi-crate publication
cannot be rolled back atomically: stop, inspect which packages succeeded, and
resume only with compatible verified artifacts or prepare corrective new versions.
Yanking prevents new resolutions from selecting a version; existing lockfiles and
installed binaries remain. Restore users by pinning a known-good version and
rebuilding/reinstalling, or publishing a corrective version. Reverting Git does
not retract registry archives. Never move existing tags or force-push to simulate
rollback. Publishing, yanking or retagging is not authorized by this plan.

## Documentation cleanup status

The follow-up documentation cleanup implements the review recommendations:
separate CLI/library quickstarts, a concise root README, linked user and maintainer
guides, corrected provider/feature/scope claims, a build-before-regenerate kernel
command, smaller crate examples, and provider keep-work restrictions. Cargo CLI
syntax and doctor behavior are delegated to cargo-libc's maintained guide.
No new capability, data regeneration or compatibility qualification is implied.

Future documentation maintenance should keep these guides aligned with source,
check links from packaged README pages, and update this dated evidence only through
an explicit new observation rather than silently treating it as current forever.
