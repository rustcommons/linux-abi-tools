# Releasing

See the [0.3.3 release record](REVIEW-FIXES.md#published-033-archives-and-documentation)
for current published checksums, source identity and documentation status.
The [0.3.2 qualification record](RELEASE-CANDIDATE-0.3.2.md) remains historical. The
[2026-10-31 plan](RELEASE-PLAN.md) retains the dependency analysis and release gates.

The workspace publishes five crates to **crates.io only**, after
[glibc-abilists](https://github.com/rustcommons/glibc-abilists), which holds the
glibc ABI lists and is released from its own repository first. No standalone
binary bundles or GitHub release assets are published. Nothing is published
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

## Cutting a release

1. Set the same `version` in the five manifests and in the internal
   dependency requirements (`version = "X.Y.Z"` next to each `path`). Run
   `cargo check --workspace` to update `Cargo.lock`.
2. Run the full validation from [AGENTS.md](../AGENTS.md). Kernel changes also
   require `crates/kernel-abi-tools/scripts/self-test.sh` with both `local` and
   a container image.
3. Dry run: `cargo package --workspace --locked`. Cargo packages all five
   crates and builds each from its packaged sources against a temporary
   registry that holds the others. Inspect the file lists with
   `cargo package --list -p <crate>`.
4. With CI green on the commit, publish in dependency order:

   ```sh
   # first, in the glibc-abilists and glibc-provider repositories: cargo publish
   cargo publish -p linux-targets
   cargo publish -p kernel-abi-tools
   cargo publish -p linux-abi-scan
   cargo publish -p libc-abi-tools
   cargo publish -p linux-abi-tools
   ```

   (`cargo publish --workspace` does the same in one command on Cargo 1.90+.)
5. Tag the commit `vX.Y.Z` and push the tag.
6. Confirm that docs.rs built every crate
   (`https://docs.rs/crate/<name>/<version>/status.json` reports `doc_status: true`)
   and that the crate pages read correctly. Add the release to the record above.

A published version cannot be replaced. `cargo yank` only stops new lockfiles
from selecting it.

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
