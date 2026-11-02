# Releasing

See the [0.4.0 release record](#published-040-archives-and-documentation)
for current published checksums, source identity and documentation status.
The [0.3.3 release record](REVIEW-FIXES.md#published-033-archives-and-documentation)
remains historical.
The [0.3.2 qualification record](RELEASE-CANDIDATE-0.3.2.md) remains historical. The
[2026-10-31 plan](RELEASE-PLAN.md) retains the dependency analysis and release gates.

The workspace publishes five crates to **crates.io only**, after
[glibc-abilists](https://github.com/rustcommons/glibc-abilists), which holds the
glibc ABI lists and is released from its own repository first. GitHub Releases document each existing source tag with reviewed release notes
and source/registry links. No standalone binary bundles or additional release
attachments are distributed. Nothing is published
without the maintainer's go-ahead.

| Crate | Depends on | Contains |
| --- | --- | --- |
| `linux-targets` | none | Library; distribution presets compiled in |
| `kernel-abi-tools` | `linux-targets` | Library and the `kernel-seccomp`/`syscall-probe` binaries; syscall and `madvise` data compiled in |
| `libc-abi-tools` | `glibc-abilists` (default `bundled-glibc-data` feature), `glibc-provider-sources` (default `provider-sources` feature) | Library and the `libc-abi` binary; no data of its own except the `libgcc_s` table and the container build scripts (`examples/glibc-*/build-inputs.sh`, `INPUTS.toml`) |
| `linux-abi-scan` | `kernel-abi-tools`, `linux-targets` | Library and the `linux-abi-scan` binary (tier 1 static inspection) |
| `linux-abi-tools` | the four above | Re-exports only |

Each manifest has an `include` list, so only sources, compiled-in data,
README, and license/notice files are packaged. `libc-abi-tools` leaves out
`data/`, `docs/`, and `tests/`, plus examples other than the explicitly included
provider build scripts and input manifests; its own crate is MIT. A default
build of it includes glibc-abilists' LGPL-2.1-or-later data; build with
`default-features = false` to leave that out.

`glibc-abilists` and `glibc-provider-sources` must be on crates.io in the
required versions before `libc-abi-tools` is published; both release from their own
repositories. A dependency that is still a git revision (with a `version` too, as
`glibc-provider-sources` was before 0.3.0) packages as its version, so `cargo package`
cannot verify `libc-abi-tools` until that version is on crates.io. Publish it first,
then drop the `git`/`rev` keys and update `Cargo.lock`.

The [0.3.3 preparation and qualification](REVIEW-FIXES.md#authorized-033-release-preparation-2026-11-01)
is preserved alongside the completed publication record.

## Release record

| Version | Date | Crates | Commit (tag) |
| --- | --- | --- | --- |
| 0.1.0 | 2026-10-11 | `glibc-abilists` | `4ffb9f698264d7d8bd834f6f2b5e03926bdc02fb` (glibc-abilists `v0.1.0`) |
| 0.1.0 | 2026-10-11 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `f691c048fc2b55d6c7d891ac0a8a5e03db5ccb22` (`v0.1.0`) |
| 0.2.0 | 2026-10-11 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `921cf79cc2780a88e917aa9542955a746214f6a1` (`v0.2.0`) |
| 0.1.0 | 2026-10-13 | `glibc-provider-sources` | `1ae578cf7123399b0c710f44defab93cc2021001` (glibc-provider `v0.1.0`) |
| 0.3.0 | 2026-10-13 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `6c3dd725e8b7ed9d8265dba360b7db021c3afc54` (`v0.3.0`) |
| 0.3.1 | 2026-10-13 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `3cd97929a99280fca2f616dae63bd610a038e131` (`v0.3.1`) |
| 0.1.1 | 2026-10-14 | `glibc-abilists` | `d7e5228dae5b2591fc7849a9b82607a7d6bc25bb` (glibc-abilists `v0.1.1`) |
| 0.3.2 | 2026-10-31 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `d404073e2329405e9e6f2dd1005fd8671e0d0f41` ([`v0.3.2`](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.2)) |
| 0.3.3 | 2026-11-01 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `d483fd5a58e1361d008f06db5989acef72a84e54` ([`v0.3.3`](https://github.com/rustcommons/linux-abi-tools/tree/v0.3.3)) |
| 0.4.0 | 2026-11-02 | `linux-targets`, `kernel-abi-tools`, `linux-abi-scan`, `libc-abi-tools`, `linux-abi-tools` | `7ce231df7128582a355c2dce47ae6436364d5e88` ([`v0.4.0`](https://github.com/rustcommons/linux-abi-tools/tree/v0.4.0)) |

**0.2.0:** `libc-abi-tools` accepts provider descriptors for every glibc release in the
pinned ABI lists, 2.17 through 2.44 (0.1.0 accepted 2.17, 2.18 and 2.22), exposes them as
`provider::RELEASES`, and requires `libpthread_nonshared.a` only before 2.28. The other
crates are unchanged apart from the shared version. The [provider index](PROVIDERS.md)
lists each release's qualification.

**0.3.0:** `libc-abi-tools` prepares providers itself: the `provider` module and the
`libc-abi provider` commands (`prepare`, `path`, `list`, `verify`, `export`, `import`, `remove`)
build one in `source` mode, from `glibc-provider-sources` with the host's GCC, or in
`container` mode, from glibc's release archive in a container, and cache it. Every
provider links against a GCC 4.8.5 `libgcc_s` stub. The host glibc check picks only
symbols newer than the host's glibc. Hosted CI qualifies both modes for every release
2.17 through 2.44 ([PROVIDERS.md](PROVIDERS.md)); `prebuilt` mode is still planned.
The other crates are unchanged apart from the shared version.

**0.3.1:** documentation only. Every public item of the five crates is documented
(`missing_docs` is warned and CI denies warnings), each crate overview has a runnable
example kept as a doc test, and the manifests carry docs.rs metadata. No API or
behaviour changes.

**0.3.2:** all three CLI stdout broken-pipe fixes, including reusable provider
command output, plus installation and guide cleanup. Broken output pipes do not
hide scanner findings or later provider failures; genuine output errors still
fail. Five-package lockstep release; upstream data/source inputs are unchanged.
The [qualification record](RELEASE-CANDIDATE-0.3.2.md) links Linux/Windows/MSRV,
package and scoped consumer checks and records published archive checksums.

**0.3.3:** bounded ELF parsing with large fixed tables accepted, equivalent
executable-alias coalescing and explicit site-budget errors; Docker/Podman
explicit environment forwarding and redirected stdin. All five packages share
the patch version. Upstream glibc inputs, API and MSRV remain unchanged. Parser
resource limits, runner/provider limitations and deferred tier 3 coverage remain
explicit in the [release record](REVIEW-FIXES.md#published-033-archives-and-documentation).

**glibc-abilists 0.1.1:** documentation only. Every public item is documented,
including when `release()` and `abilist()` return `None`; the README example runs as
a doc test. The data is unchanged. This workspace's lockfile still selects 0.1.0;
0.1.1 is semver-compatible.

crates.io limits how many new crate names one account can publish in a short
period; a first release of several crates may need a pause (the sixth crate
here waited a few minutes).

## Qualification scope for releases

Ordinary bugfix and documentation releases may ship when their applicable
automated gates pass and the release record explicitly states the tier 3 coverage
gap. Real old-system environments and user-provided testing are currently
unavailable; [tier 3 work is deferred](ROADMAP.md#deferred-qualification), not
marked completed or made a prerequisite for every ordinary release.

Any new real-old-system compatibility claim still requires actual evidence on
the claimed target kernel and userland. Static inspection, seccomp simulation,
and containers on a modern host do not provide that evidence. Report the exact
scope of successful automated checks without extending it to untested systems.

All applicable provider, data, pack/replay, provenance/license, package, MSRV and
consumer integration gates remain in force. This distinction does not relax the
legal review or prebuilt-provider gates, and publication still requires explicit
authorization. Preserve dated qualification evidence as historical records.

This policy applies to the five linux-abi-tools workspace packages. For
cargo-libc releases, follow its maintained
[release policy](https://github.com/rustcommons/cargo-libc/blob/main/docs/RELEASING.md)
rather than duplicating its policy here.

## Changelog and release notes

Maintain the root [CHANGELOG.md](../CHANGELOG.md) with `Unreleased` and dated
entries describing user impact, plus [versioned notes](releases/v0.4.0.md) at
`docs/releases/vX.Y.Z.md`. Each note has **Changes**, **Upgrade**, **Validation and
limits**, and **Links**. State the original source/tag, UTC registry-publication
date, package availability and exact validation scope. Proposals stay in
`Unreleased` or the roadmap, outside shipped-version changes.

A release is complete only when its source tag, applicable registry archives,
GitHub Release notes, changelog, documentation and verification agree. A tag or
crate upload alone does not meet that documentation-completion rule. The
[GitHub Releases index](https://github.com/rustcommons/linux-abi-tools/releases)
was empty in the initial 2026-11-02 history audit: all seven source tags and all 35
crate-version archives existed, but GitHub note publication was missing. This
backfill requires no new crate version, tag, archive or binary.

Older notes were reconstructed from exact tag/source diffs, registry metadata and
existing records. All 35 downloaded archives matched their registry checksums
and clean VCS tag commits on 2026-11-02. The 0.1.0–0.3.1 notes identify the missing
consolidated final-tag validation bundles and preserve the narrower historical
records. Later 0.3.2–0.4.0 qualification records keep their original scope.
No historical test run is inferred or fabricated from current source or a
configured workflow. The original dates in the changelog/notes are registry
publication dates; GitHub's posting dates are the actual backfill dates, not
the original registry dates. Backfilled documents did not exist at old source tags: use absolute `main`
or documentation-commit links, never `blob/vOLD/CHANGELOG.md`. Every release-body
link must be absolute, so it resolves on GitHub Releases too.

## Release-documentation gate and first-install bootstrap

Python 3.11+ and its standard library are sufficient; no project dependency is
added. Run the committed local gate:

```sh
python3 scripts/check-release-docs.py --repo rustcommons/linux-abi-tools
```

Routine merges require the registered [release-documentation workflow](../.github/workflows/release-docs.yml)
to pass. Its first installation is a bootstrap exception: independently review
local checker, link/render/diff and workflow syntax evidence; normal-merge the
reviewed installation to register it on the default branch; then require a green
workflow run before bootstrap completion or any GitHub Release publication.
There is no claimed candidate hosted pass before registration. Run it after merge:

```sh
gh workflow run release-docs.yml --ref main
```

The release-event path checks historical tag notes using default-branch docs,
because old tags predate this backfill. After all existing-tag notes are
published, verify the exact live bodies and current stable latest marker:

```sh
python3 scripts/check-release-docs.py --repo rustcommons/linux-abi-tools --published
gh workflow run release-docs.yml --ref main -f verify_published=true
```

`--published-tag vX.Y.Z` verifies one live release event. The local gate does not
query the network; neither checker mode publishes anything. Keep the shared
checker/workflow byte-identical to the reviewed shared versions. Markdown/CI-only
changes need these documentation checks, not unrelated full Rust/provider suites.

## Release-documentation backfill completed (2026-11-02)

The initial pre-registration workflow lookup returned HTTP 404 because the new
workflow was not yet on the default branch. That dated bootstrap observation and
the independently reviewed local checker/syntax evidence remain historical; no
candidate hosted pass was inferred from them. Normal bootstrap merge
[`6f1e5bdeb06f168bf29f1d7bddf937c028e5cb60`](https://github.com/rustcommons/linux-abi-tools/commit/6f1e5bdeb06f168bf29f1d7bddf937c028e5cb60)
preserved the approved `01d335246a3f8d5c19aef007c71e8c2efd0ab329` tree and registered
the workflow on `main`.

The shared **AGENTS release rule v2** defines that bootstrap policy. The shared
**checker v3** is a separate reviewed file, SHA-256
`6e666710a350d9c176a5f79a17a7a0ac16f5aeaa0e0801baf0a967485524fdea`;
neither shared file nor the workflow was changed during publication or this
freshness correction.

[Bootstrap run 41387837849](https://github.com/rustcommons/linux-abi-tools/actions/runs/41387837849)
completed **SUCCESS** on `6f1e5bd` before any Release entry was created. The seven
entries were then published in ascending version order from the committed
`docs/releases/vX.Y.Z.md` files using `--notes-file` and `--verify-tag`. Older
entries used `--latest=false`; only `v0.4.0` used `--latest=true`.

[Full published-verification run 41388248802](https://github.com/rustcommons/linux-abi-tools/actions/runs/41388248802)
completed **SUCCESS** on the same merge, including `--published`. The local full
published checker also passed. Verification confirmed exact committed note bodies,
unchanged existing tag commits, only `v0.4.0` latest, and **zero attached assets**
for every entry. All 75 distinct note-link targets returned HTTP 200 with browser
HTML headers; the initial machine-header 404 responses for registry UI routes
were recorded separately and resolved without changing the notes.

Published entries: [v0.1.0](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.1.0),
[v0.2.0](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.2.0),
[v0.3.0](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.3.0),
[v0.3.1](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.3.1),
[v0.3.2](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.3.2),
[v0.3.3](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.3.3), and
[v0.4.0](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.4.0).
Their GitHub posting dates are 2026-11-02; original registry dates and source
identities remain as recorded in the immutable notes. No crate was republished,
no tag was created or moved, and no binary assets were added. This completes
release-documentation backfill, not additional product/runtime qualification.
Routine merges now use the registered documentation CI gate above.

## Cutting a release

1. For an authorized new software version, synchronize the five manifests,
   internal dependency requirements and lock as described by the versioning
   policy. Move completed user-visible `Unreleased` changes into its dated
   changelog entry and commit `docs/releases/vX.Y.Z.md` before publication.
2. Run applicable validation from [AGENTS.md](../AGENTS.md), including the local
   release-documentation gate and registered CI. Kernel code/profile changes
   also need the local/container self-tests. Data, provenance/license and provider
   gates retain their scope. Documentation backfill does not rerun these suites.
3. For new crate publication, run `cargo package --workspace --locked`, inspect
   all five archives/allowlists/licenses and VCS identity, and validate on the
   exact approved source. Existing published versions are immutable; skip crate
   publishing entirely when backfilling documentation.
4. With required gates and explicit approval, publish new packages in dependency
   order: `linux-targets` → `kernel-abi-tools` → `linux-abi-scan` → `libc-abi-tools`
   → `linux-abi-tools`. Upstream glibc data/source crates must already be available
   at the required versions. Only the authorized publisher performs this step.
5. Create a new source tag only for an authorized new version, on its exact
   approved release commit. For existing-tag documentation, verify the existing
   tag/commit and preserve it; never recreate, move or force-push a tag.
6. Verify official registry archives, checksums, non-yanked state, clean VCS and
   all five versioned docs.rs builds/pages. Preserve source/package evidence
   separately from later documentation commits and consumer adoption.
7. After approval and a green documentation workflow, publish GitHub Release
   notes from the reviewed committed file, with no attachments. For historical
   backfill, work in ascending version order; older versions use `--latest=false`,
   and only the actual newest stable version uses `--latest`:

   ```sh
   gh release create vX.Y.Z --verify-tag --title "vX.Y.Z — concise outcome" \
     --notes-file docs/releases/vX.Y.Z.md --latest=false
   ```

   Use the final stable version's reviewed file with `--latest` instead of
   `--latest=false`. Do not generate substitute bodies or falsify posting dates.
8. Verify the live release tag, exact text and links, registry/source identity,
   absence of unauthorized assets, and latest marker. Run the published checker
   and its CI mode above before claiming release-documentation completion.

A published crate version cannot be replaced. `cargo yank` only stops new
lockfiles from selecting it; it is not a way to rewrite a release.

## Versioning

All five crates share one version while they are 0.x.

- **Patch:** data refreshes and preset corrections that do not loosen any
  simulation, documentation, and fixes.
- **Minor (0.x):** new presets, kernels, rules, API or CLI additions, and any
  breaking change. A simulation may become stricter (closer to the real
  kernel); release notes call this out.

## Offline and disconnected networks

Default builds embed ABI, syscall and preset data. Source-mode provider preparation
uses embedded inputs; container mode and container runners can need network access.
With bundled ABI data disabled, supply an external data directory.

- **Build from crates.io offline:** run `cargo vendor` on a connected machine
  for the consuming project (for example cargo-libc). Carry `vendor/` and the
  printed `.cargo/config.toml` snippet across, then build with `--offline`.
- **Install the binaries offline:** `cargo install kernel-abi-tools
  --locked --offline` against the vendored sources, or build once and copy
  `kernel-seccomp`.
- **Container runner:** `podman save`/`podman load` each image, or push it to
  an internal registry and pass `--image`. If the hosts' libseccomp is older
  than 2.6, carry the static crun from `scripts/install-crun.sh` and verify
  its pinned checksum.
- **Local runner:** needs nothing beyond the binary.

Checksums on crates.io protect integrity, not publisher identity. Mirror trust
is a separate decision (see the kernel roadmap).


## Planned 0.4.0 clock overlay release

> Historical preparation record: pending states below describe the successive
> candidates at that time. The [published 0.4.0 record](#published-040-archives-and-documentation)
> supersedes these release-status statements without rewriting their outcomes.

Preparation only; 0.3.3 remains the published workspace release. New public
`provider::link_profile` API requires a minor release under the lockstep policy:
all five versions/internal minima and the workspace lock are prepared at 0.4.0.
No new upstream glibc dependency requirements, data regeneration, kernel profiles,
runtime implementation or unwinder changes are included. Raw provider generation
and descriptor v1 remain faithful; the explicitly requested derived overlay has
its own content/policy identity. See the [API contract](USING-PROVIDERS.md#derived-clock-link-overlay-api).

The 2026-11-02 producer investigation covered 746 cross-link cases (742 successful
links, four expected static failures). That ordering investigation is diagnostic
evidence, not a production ordering fix: later mixed-version probes disproved
ordering alone for GNU ld. The committed overlay regression separately checks
recorded 2.17–2.44 stub inputs, plus 32 GNU ld 2.40 / LLD 23.1.1 links on 2.22:
all five ordinary/explicit-old clocks, four call/address combinations,
PIE/non-PIE and both as-needed states, retaining `timer_create` as an RT control.
These are link/ELF inspections, not runtime executions. Portable fixture tests
cover byte preservation, invalid inputs, cache identity/corruption and concurrent
writers. The explicit linker gate is ignored in ordinary portable test runs and
requires the recorded stub directory plus cross tools; it never substitutes host
libraries. The explicit `selected_real_dso_copy` gate also passed against Debian's real
x86_64 libc/librt 2.28-10+deb10u1: exactly five changed bytes, all metadata other
than the clock default flags preserved, inputs unchanged. Download:
[Debian archived libc6 package](https://archive.debian.org/debian/pool/main/g/glibc/libc6_2.28-10+deb10u1_amd64.deb),
SHA-256 `0ee0f0bfcd7b59ffda1dbc76ef58fa79512913204f2a336206467bfbf45bc69f`.
That byte-derivation check does not qualify compiler sysroot discovery or runtime
execution. Full-sysroot consumer orchestration and independent review remain
pending. No old 0.3.3 qualification run proves this new feature.

Before a release candidate: complete fmt, MSRV, workspace/default/no-default
clippy/tests/docs and five-package archive gates on the exact final source;
independent producer review and consumer/native qualification remain required.
Publish only after the coordinator's approval, in order `linux-targets` →
`kernel-abi-tools` → `linux-abi-scan` → `libc-abi-tools` → `linux-abi-tools`.
The coordinator alone publishes and confirms all five before any new tag.
Tier 3 remains deferred, not an ordinary-release blocker; authenticated prebuilt
delivery remains deferred pending legal review. Cache verification is local
content integrity, not producer authentication.


The 2026-11-02 independent review rejected producer source `1b3b51c8afad88e05b0dc238c15f5ee879f94184`:
a conflicting partial PT_LOAD could evade table validation, and directory rename
could replace a concurrently created empty cache entry. That source is blocked;
its earlier passing owner checks do not qualify a corrected source. The revised
implementation checks partial and 4 KiB load-page shadows, retaining consistent
aliases, and publishes complete private generations with a no-clobber hard-linked
manifest index. Tests cover racing empty destinations, winner persistence, loser
cleanup, invalid generation names and ignored unindexed partial generations.
Exact old-ref reproductions remain historical evidence; corrected probes account
for the new generation manifest field and intercept hard-link publication rather
than rename. Independent re-review and exact-source consumer/native qualification
remain required before any publication.


Corrected producer source `e857e43f113fd5fb02d0f2afff58b90fcfbd1ee6` passed the
producer-local gates on 2026-11-02 (aarch64 Linux host, x86_64 cross-link targets):
127 workspace/all-targets tests, six doctests, 57 no-default tests; fmt; Rust
1.90 workspace/all-targets check; workspace/default and root/no-default clippy;
warning-denied all-feature workspace docs; five-package workspace packaging and
archive audit with clean VCS identity. Focused evidence comprises 12 ELF/cache
integration tests plus two publication unit tests, the separately invoked
28-release/32-link gate and real Debian 2.28 DSO gate described above. Both
independent reviewer reproducers were rerun: the original blocked source still
reproduces its two failures; corrected-source assertions require partial-map
refusal and an unchanged racing empty index. Only the internal generation field,
hard-link hook and expected fixed outcomes changed in the probe harnesses.

This is bounded producer preparation evidence. Hosted checks, independent GO and
full consumer/native qualification remain pending; no registry adoption,
publication or runtime compatibility is established by these local checks.


### Manual source and Windows evidence (2026-11-02)

These checks use corrected producer source
`e857e43f113fd5fb02d0f2afff58b90fcfbd1ee6`; later evidence-only changes do not
alter its source, manifests or repository lock. They supplement the local gates
above and do not turn queued hosted checks into passes.

- Native x86_64 WSL, Rust/Cargo 1.90.0: `cargo test --test link_profile --lib
  --locked --offline` passed 22 library and 12 integration tests. The two
  external Linux linker/real-DSO gates were explicitly ignored in this run and
  were separately executed in the producer cross-link evidence above. Raw log
  `producer-e857-msrv-native.log`, SHA-256
  `240aa5591891bc681f2b88c371479d6fc2e8a1e51410b8f74e02760f9dff3de4`.
- Windows x86_64 MSVC/NTFS, Rust/Cargo 1.99.0, isolated target directory: the same
  focused command passed 22 library and 11 integration tests. The Unix-only
  symlink case is excluded by configuration, and the two Linux external gates
  were explicitly ignored. This directly covers standard-library hard-link
  publication, concurrent reuse and no-clobber behavior on NTFS; it does not
  qualify Windows-host provider preparation or target linking. Summary
  `producer-e857-windows-qualification.md`, SHA-256
  `3b7d71e168dfc89583c0cd26be6725051c8f5e955e26715852fd88bdc67cbe7e`.
- Source consumer `e3e31e44b2368ebea4a06974d931f254da4e1ac6` with that producer,
  native x86_64 WSL/Rust 1.99.0, recorded 01:32:08–01:35:48 UTC: source-driver
  gates passed, including 16 GNU ld 2.42 / LLD cases, host-sysroot selection,
  old-to-new same-target-directory relinking, shell/TOML exports, commands and
  C/CMake plus native-host-SDK/local-3.10-simulation smokes. The root-only source
  patch retained registry kernel/scan/targets 0.3.3, glibc-abilists 0.1.2 and
  glibc-provider-sources 0.1.1; this is not an all-five 0.4.0 registry graph or a
  change to this repository's locked 0.1.0 upstream inputs. Archive
  `native-e857-e3-proof.zip`, SHA-256
  `e6fd5de1920eb1e5c408c217a898265ce9bae131b861cd1a2c429f6b98af7743`.
  Inspection verified all 13 member hashes, the recorded source identities and
  graph, all-five libc defaults, mixed ordinary/explicit-old `clock_gettime`,
  and unchanged non-clock imports in all 16 comparisons. This inspection did
  not rerun the native script. No clock-setting fixture was executed.

Remaining producer release gates: independent approval of the corrected ELF/cache
changes and evidence-only delta, applicable hosted/manual gate acceptance, and
final corrected source-consumer qualification. The recorded consumer run is
preliminary while its named-library-script bypass remains open; the corrected
final native Rust 1.90 source pair is separate. Container-mode preparation and
profile integration are not covered by that ZIP. The unchanged kernel,
scanner/target and ordinary provider generation/container-builder code informs
the scope of additional checks; it is not fresh execution evidence. Real-registry
adoption and three-way consumer validation follow producer publication and remain
separate Cargo release gates. No producer merge or publication approval is
inferred here. Tier 3 and the authenticated-prebuilt legal deferral are unchanged.


## Published 0.4.0 archives and documentation

All five workspace crates were published on 2026-11-02 from clean merged release
commit [`7ce231df7128582a355c2dce47ae6436364d5e88`](https://github.com/rustcommons/linux-abi-tools/commit/7ce231df7128582a355c2dce47ae6436364d5e88),
now tagged [`v0.4.0`](https://github.com/rustcommons/linux-abi-tools/tree/v0.4.0).
The merge tree exactly matches independently approved `9af795eb23759bb37c0ed656418e71d7ce075239`;
its source/manifests/lock match tested `e857e43f113fd5fb02d0f2afff58b90fcfbd1ee6`,
with the expected evidence-only documentation delta. Publication followed targets
→ kernel → scan → libc tools → umbrella. The merged five-package verification
and archive audit preceded publication; no source changes or version/lock edits
were made for this final evidence record.

This release adds the explicitly requested [derived clock overlay API](USING-PROVIDERS.md#derived-clock-link-overlay-api).
Raw recorded ABI, ordinary provider/stub generation, descriptor v1, kernel/scanner
implementation and unwinder inputs remain unchanged. New API scope requires the
lockstep 0.4.0 minor version; published 0.3.3 remains immutable and lacks this API.
There is no new CLI clock knob or runtime implementation.

Final hosted checks passed on approved `9af795e` before the normal merge:

- [CI 41373834696](https://github.com/rustcommons/linux-abi-tools/actions/runs/41373834696): 39/39 jobs successful.
- [Kernel 41373834504](https://github.com/rustcommons/linux-abi-tools/actions/runs/41373834504): 4/4 jobs successful.
- [Provider integration 41373834529](https://github.com/rustcommons/linux-abi-tools/actions/runs/41373834529): 84/84 jobs successful.

These complement the exact-source local and [manual native/Windows evidence](#manual-source-and-windows-evidence-2026-11-02)
above. The blocked `1b3` source and ordering-only investigation remain historical;
they are not release qualification. Independent approval covers the corrected
ELF/cache code and narrow evidence delta. Cargo's named-script/final-pair,
registry-adoption and three-way validation are separate consumer gates, not a
producer API blocker. The earlier preliminary consumer ZIP is not promoted to
final Cargo qualification here.

Independent post-publication checks on 2026-11-02 downloaded every official
archive and compared SHA-256 with both the crates.io version API and sparse index,
as well as the prepared merged archives. All five are non-yanked. Each archive's
`.cargo_vcs_info.json` identifies the exact release commit with no dirty marker;
file inventories, normalized manifests, internal 0.4.0 minima, licenses and source
payloads match the audited preparation. All five docs.rs status endpoints report
`doc_status: true`, version `0.4.0`; the linked exact-version API pages return
HTTP 200.

| Package / official archive | SHA-256 | Files | docs.rs API / build status |
| --- | --- | --- | --- |
| [linux-targets 0.4.0](https://crates.io/crates/linux-targets/0.4.0) / [archive](https://static.crates.io/crates/linux-targets/linux-targets-0.4.0.crate) | `f080d0c9bff7e88e0ffdbbfb4f8b235b671e63fa6bf724fceff75c84d36990f0` | 8 | [API](https://docs.rs/linux-targets/0.4.0/linux_targets/) / [status](https://docs.rs/crate/linux-targets/0.4.0/status.json) |
| [kernel-abi-tools 0.4.0](https://crates.io/crates/kernel-abi-tools/0.4.0) / [archive](https://static.crates.io/crates/kernel-abi-tools/kernel-abi-tools-0.4.0.crate) | `5fa80059e2cb3200a851576a54e2dcdf942b338b53a53bc6ee3785d1acabf139` | 15 | [API](https://docs.rs/kernel-abi-tools/0.4.0/kernel_abi_tools/) / [status](https://docs.rs/crate/kernel-abi-tools/0.4.0/status.json) |
| [linux-abi-scan 0.4.0](https://crates.io/crates/linux-abi-scan/0.4.0) / [archive](https://static.crates.io/crates/linux-abi-scan/linux-abi-scan-0.4.0.crate) | `54fe03feacc99f77ed07d995434dc1abf5c59faaff07a3fb79e596b7d2a51986` | 11 | [API](https://docs.rs/linux-abi-scan/0.4.0/linux_abi_scan/) / [status](https://docs.rs/crate/linux-abi-scan/0.4.0/status.json) |
| [libc-abi-tools 0.4.0](https://crates.io/crates/libc-abi-tools/0.4.0) / [archive](https://static.crates.io/crates/libc-abi-tools/libc-abi-tools-0.4.0.crate) | `915611aa54c3fe77eefedb34bac233b077e59669130a116e2f20bc25297af009` | 83 | [API](https://docs.rs/libc-abi-tools/0.4.0/libc_abi_tools/) / [status](https://docs.rs/crate/libc-abi-tools/0.4.0/status.json) |
| [linux-abi-tools 0.4.0](https://crates.io/crates/linux-abi-tools/0.4.0) / [archive](https://static.crates.io/crates/linux-abi-tools/linux-abi-tools-0.4.0.crate) | `c85d44102d3f8f56570d98ae15d80c57184fd7e6c5c72b66c2b67ac301a848d6` | 7 | [API](https://docs.rs/linux-abi-tools/0.4.0/linux_abi_tools/) / [status](https://docs.rs/crate/linux-abi-tools/0.4.0/status.json) |

The profile's documented bounds remain: selected little-endian x86_64 ELF64 DSOs,
ordinary headers, 64 MiB inputs and 100,000 dynamic symbols; libc must supply all
five GLIBC_2.17 defaults. Invalid loader mappings or existing cache entries refuse
without fallback/repair. Cache roots are caller-controlled and require ordinary
same-filesystem hard links; content integrity is not publisher authentication.
Caller-owned selection/containment and same-position substitution remain separate
from the producer's validated byte derivation. Link/modern-host simulation results
do not prove universal old-system runtime compatibility. Tier 3 stays deferred,
not an ordinary-release blocker; authenticated prebuilt delivery remains deferred
pending legal review.

Cargo registry adoption is now authorized after all five producer versions became
available. This producer record proves availability/identity, not completion of
consumer installation, the final source pair or three-way registry qualification.
The historical [cargo-libc 0.2.2 adoption record](https://github.com/rustcommons/cargo-libc/blob/1ca27da9086ebf6cf19a6f2c0e858108b38cac46/docs/RELEASE-0.2.2.md)
retains its original scope. New Cargo adoption must be recorded by its own gates.
