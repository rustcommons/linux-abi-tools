# linux-abi-tools

## Working conventions

- Read this file before changing the repository. Keep repository-specific constraints and validation instructions current here.
- Preserve changes made by the user or other agents. Keep work scoped to the requested task.
- Keep documentation aligned with implemented behavior. Record unconfirmed work as planned; do not invent capability or compatibility claims.
- Run checks appropriate to the change. For documentation-only changes, verify links, commands, and the diff; do not run unrelated builds.
- Publishing packages, creating releases, or changing production data requires explicit user authorization.

## Project overview

`linux-abi-tools` (formerly `libc-abi-tools`, renamed 2026-10-10) is a Cargo
workspace of rustcommons' ABI tools for targeting older Linux systems from Rust:

| Crate | Location | Scope |
| --- | --- | --- |
| `libc-abi-tools` | repository root | glibc ABI metadata, stub generation, and providers, inspired by (and partly adapted from) [Zig's libc ABI tools](https://codeberg.org/ziglang/libc-abi-tools); ABI data from glibc via [glibc-abilists](https://github.com/rustcommons/glibc-abilists); other libc implementations are future work |
| `kernel-abi-tools` | `crates/kernel-abi-tools` | Linux kernel syscall and `madvise` data and seccomp profiles that simulate older kernels |
| `linux-targets` | `crates/linux-targets` | Distribution presets (glibc and kernel floors, image) and target selection rules |
| `linux-abi-scan` | `crates/linux-abi-scan` | Static inspection of built executables against the floors: tier 1 of the testing tiers |
| `linux-abi-tools` | `crates/linux-abi-tools` | Re-exports the four crates above; no code of its own |

The companion Cargo integration is `rustcommons/cargo-libc`, which depends on
the published crates. `cargo` in the root acts on `libc-abi-tools` alone unless
given `--workspace` or `-p`. [docs/WORKFLOWS.md](docs/WORKFLOWS.md) describes
every workflow and its cargo-libc mapping; keep it in step with changes.

## Scope and implementation: libc-abi-tools

- Support glibc target releases starting at 2.17. Retain earlier GLIBC symbol
  versions: later releases still export and require them.
- glibc ABI data comes only from glibc's own release tags, through the
  `glibc-abilists` crate (default `bundled-glibc-data` feature, opt-out with
  `default-features = false` and `--data DIR`). Never vendor ABI lists here or
  take them from Zig or another redistributor.
- Pin upstream references to a commit, document derivations, and preserve the
  Zig contributors' copyright and MIT license with code adapted from Zig.
- Keep ABI collection/serialization separate from linker stub generation and
  real startup objects, headers, and runtime implementations.
- Preserve symbol versions, library identity, symbol kind, and object/TLS sizes.
  Raw recorded evidence and ordinary provider/stub rendering remain faithful.
  The explicitly requested `provider::link_profile::LinkProfile::LibcClocksV1` policy derives
  a separate immutable librt copy: only the five clock default-version bits may
  change after validating selected libc GLIBC_2.17 defaults. Never rewrite raw
  tables, descriptors or user inputs, reorder library arguments, or silently
  fall back when profile validation fails.
- Providers link the compiler runtime as a generated `libgcc_s` stub with GCC
  4.8.5's interface (`src/libgcc_s-gcc-4.8.5.tsv`). Regenerate that table with
  `scripts/record-libgcc-s.sh` from the pinned CentOS 7 image; never edit it by hand.
- `src/provider/` prepares providers (`libc-abi provider`). `source` mode compiles
  glibc-provider-sources' recorded sources with the host's GCC and must verify every
  object against the recorded interfaces before use; a difference refuses, never
  falls back. `container` mode runs the pinned `examples/glibc-*/build-inputs.sh`.
  Every mode generates the same stubs, from the release's recorded library table.
  `src/provider/build.rs` follows glibc-provider's `glibc-provider-replay`; keep the
  two in step.
- Use deterministic outputs and validate malformed inputs without panics.
- Provider preparation must not delete or mutate published generations. Stage
  with unique, exclusively created names; publish complete generations through
  an atomic selection file and retain previous reader paths. Cleanup may remove
  only an unpublished generation owned by that attempt. Explicit removal needs
  quiescent readers/writers; test forced/normal concurrency and publication failure.
- Keep implementation dependencies minimal; start with Rust's standard library.
- Ask about unresolved product decisions; do not claim Zig parity prematurely.

## Scope and implementation: kernel-abi-tools

- Support target kernels from 3.10; the syscall data floor is v3.3. Rust std's
  documented minimum (3.2) is treated as the floor and must be reported as such.
- Derive kernel data from upstream kernel tables and headers at release tags,
  record provenance, and regenerate with the scripts rather than editing
  generated files by hand.
- Profiles are fail-closed allowlists: default `ENOSYS`, allow only what the
  target kernel provides. Never switch to a blocklist; runtimes drop unknown
  names and a blocklist fails open. Argument rules must not overlap.
- Profiles simulate syscall availability and `madvise` advice only. Other
  flags, behavior, and vendor backports must be documented as planned until
  implemented and validated.
- Two runners apply the simulation: the local runner (in-process classic-BPF
  filter from `filter::program`, host userland, no root) and the container
  runner (OCI profile, image userland). The BPF filter and the JSON profile
  must make the same decisions for native x86_64 syscalls; `filter` tests check
  them against the data. Foreign-ABI rejection is runtime-specific for OCI:
  do not claim i386/x32 errno/status parity without runtime evidence.

## Scope and implementation: linux-targets

- Presets are aliases for a glibc floor, kernel floor, and image. Only mark
  values as checked when they were observed; backports must be sourced, never
  guessed.
- `--distro` excludes `--glibc`, `--kernel`, and a versioned `--target`. A
  missing kernel floor means the host kernel and must be reported as "not
  simulated".

## Scope and implementation: linux-abi-scan

- Tier 1 of three (inspect, simulate, qualify); always document it as the
  least reliable tier and never present a clean scan as compatibility.
- Every finding states its confidence: `Exact` only for ELF metadata the
  loader uses; anything recovered from machine code is `Heuristic`.
- Take kernel facts only from `kernel-abi-tools` and floors only from
  `linux-targets`, so the scan and the seccomp simulation cannot disagree.
- Parse untrusted ELF input with bounds checks and no panics.

## Shared rules

- Every crate stays std-only apart from rustcommons crates: `kernel-abi-tools`
  may depend on `linux-targets`, `linux-abi-scan` on both of those,
  `libc-abi-tools` on `glibc-abilists` and `glibc-provider-sources` (default
  `provider-sources` feature; published on crates.io), and
  `linux-abi-tools` on the other four.
- Distribution packages go to crates.io only. GitHub Releases carry the reviewed
  notes plus only the owner-authorized release files listed under
  [Release pipeline](#release-pipeline); no executables or other attachments
  (see [docs/RELEASING.md](docs/RELEASING.md)).
  Keep each manifest's `include` list minimal. The glibc ABI lists
  (LGPL-2.1-or-later) are published only in `glibc-abilists`, which must be
  released before `libc-abi-tools`.
- Ask about unresolved product decisions; do not invent compatibility claims.

## Validation

Before all-target tests/checks, run `python3 scripts/build-test-fixtures.py` (or verify the matching source-built CI artifact with `--verify-only`); see [fixture prerequisites](docs/SECURITY-BASELINE.md#retired-committed-binary-inventory).

Run `cargo fmt --all -- --check`, `cargo test --locked --offline`, and
`cargo clippy --all-targets --locked --offline -- -D warnings` for Rust changes;
add `--workspace` (or `-p kernel-abi-tools -p linux-targets`) when the other
crates change.
Use pinned upstream fixtures to validate the glibc port. Document which
architectures/releases and which stages have actually been tested.
Kernel profile or filter changes also require
`crates/kernel-abi-tools/scripts/self-test.sh` on Linux, both with `local`
(no root needed) and with a container runtime (crun with libseccomp 2.6+, e.g.
from `scripts/install-crun.sh`, covers every profile). Packaging changes
require `cargo package --workspace --locked`; never publish to crates.io
without the user's go-ahead. Distinguish host simulation from actual
oldest-target runtime qualification on a real kernel.

## Documentation and requirements traceability

- Follow the [Rust Commons documentation conventions](https://github.com/rustcommons/.github/blob/main/docs/DOCUMENTATION.md).
- Maintain [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) as the repository's index of confirmed requirements, their sources, implementation or documentation, verification evidence, and gaps. Preserve stable repository-scoped IDs and existing identifiers; never renumber or reuse them.
- Distinguish verified behavior, partial verification, planned work, deferred work, and scope exclusions. A source reference or passing test supports only the scope it actually checks; record missing evidence explicitly.
- Keep the README focused on purpose, prerequisites, setup, and guide links. Keep detailed workflows in maintained guides, future work in the roadmap, and dated release or validation evidence identifiable as historical records.
- Update affected requirements, documentation, and trace links together when behavior, scope, interfaces, prerequisites, or validation changes. Use exact versions or revisions for release evidence, and do not rewrite historical outcomes or treat repository edits as changes to an immutable published package.
- Validate documentation changes with source and command checks, links, rendering, and a clean diff. Run additional builds or tests only when changed examples, package contents, or behavior require them.

## Release documentation

- **Updating `CHANGELOG.md` is critical.** It is the source of the release notes,
  and the release gate refuses to publish a version without its dated
  `## [X.Y.Z] - YYYY-MM-DD` entry. Add each user-visible change to `Unreleased`
  in the same PR as the change; never leave it for release time.
- For a repository that ships versioned software, maintain a root `CHANGELOG.md` with an `Unreleased` section and dated, user-facing entries for shipped versions. Record noteworthy changes and their user impact as work is completed; do not substitute a roadmap, Git tag, CI log or validation report for a changelog.
- Before publishing a version, commit its release notes under `docs/releases/vX.Y.Z.md` and link them, the changelog, and the maintained release policy. Explain changes, upgrade or dependency impact, validation and known limits; distinguish software changes from documentation-only releases.
- A software release is not complete until its immutable source tag, applicable registry archives, GitHub Release notes, changelog, documentation, and required verification agree. GitHub Releases document the release; do not attach standalone binaries or other new distribution artifacts without authorization.
- Run the repository's release-documentation CI check before routine merges and release completion. For its first installation, review local checker/syntax evidence, register the workflow on the default branch, and require a green run before completing bootstrap or publishing notes. Verify the live GitHub Release and its tag, text and links after publication; a successful crate upload or tag alone is insufficient.
- When backfilling history, reconstruct facts from existing tags, registry records and source/evidence. Preserve original source identities, distinguish original release dates from documentation backfill dates, and state unavailable historical validation. Never fabricate checks, move tags, republish versions, or imply that repository edits change immutable published archives.
- For an organization profile, placeholder or project with no shipped version, record release verification as not applicable until its first authorized release. Do not invent versions, features, changelog history or GitHub Releases to satisfy this convention.

## Security automation

- Never use `[skip ci]`, `[ci skip]` or equivalent CI-skip directives in commit or merge messages. Keep CodeQL default setup enabled as managed by the coordinator; preserve historical commits without rewriting their messages.
- Follow [docs/SECURITY-BASELINE.md](docs/SECURITY-BASELINE.md): pin install inputs (Action SHAs, exact locked Cargo tool versions, hashed pip requirements, verified download hashes and container digests). Maintain source-built test fixtures and the unpublished fuzz-only workspace separately from shipped crate dependencies. Run parser fuzzing on relevant human PRs and weekly; treat crashes as bugs requiring regression coverage or an owner decision.

- All changes to `main` require a PR and the `lf-only` check; never push directly to `main`, force-push protected branches, or update/delete existing `v*` tags. Use normal reviewed merges.
- Pin every external GitHub Action to its full commit SHA with its exact `# vX.Y.Z` tag in a trailing comment, on a Node 24 (or Docker/composite) runtime. Use explicit runner labels (`ubuntu-24.04`, `windows-2025`), never `-latest`; moving to Ubuntu 26.04 is a deliberate change. Pin `dtolnay/rust-toolchain` to the SHA of the intended toolchain branch, without a `toolchain:` input. Keep workflow permissions read-only; grant necessary write permissions only to the job that uses them. Disable checkout credential persistence.
- Require the PR security jobs: zizmor (regular persona, including online audits), cargo-deny (RustSec advisories, licenses, bans, crates.io-only sources), and offline release-script controls. Fix findings or explain narrowly scoped suppressions inline. Keep Dependabot version updates monthly with a limit of two open PRs per ecosystem, retaining minor/patch groups and the seven-day cooldown; security updates remain immediate and independent of this schedule. Keep weekly Scorecard reporting active.
- Dependabot PRs run core workspace fmt/clippy/tests, cargo-deny, zizmor, release-script unit tests, LF and GitHub-managed CodeQL. Gate provider/ABI/container/kernel qualification and non-core jobs by both actor and PR author, so a human rerun does not accidentally expand a bot PR. Human PRs, main pushes, `v*` tags and owner manual dispatch retain full qualification. Restrict push triggers to main/release tags to avoid feature-branch duplicates; every repository-defined PR workflow cancels superseded runs for the same PR, without canceling non-PR qualification or crate publication.
- The crates.io publishing workflow must be named `.github/workflows/trusted-publishing.yml` (owner standard, 2026-11-15). Crate publication through it requires an immutable reviewed `v*` tag, the owner-approved `crates-io` environment, and per-crate Trusted Publisher registration. OIDC permission belongs only to the release `attest` (Sigstore) and `publish` (crates.io) jobs and Scorecard reporting, for their respective services. Future crate uploads, including recovery of a partial release (a re-run skips versions already on crates.io), use only this workflow with owner environment approval; the owner-machine crates.io token was revoked on 2026-11-16. Preserve all package/release qualification gates; configuring automation authorizes no new release.

## Release pipeline

Releases use the Rust Commons standard pipeline
([spec](https://github.com/rustcommons/.github/blob/main/docs/TRUSTED-PUBLISHING.md),
owner-approved 2026-11-21) in `.github/workflows/trusted-publishing.yml`; never
rename the file or the `crates-io` environment. Jobs: `gate` (Release gate),
`build` (Build), `attest` (Checksums and attestations), `publish` (Publish to
crates.io, the owner's approval), `github-release` (GitHub release) and `verify`
(Verify publication). A `v*` tag runs all six; pull requests that touch release
inputs and manual runs are dry runs of `gate` and `build` only. The gate
([scripts/release-gate.py](scripts/release-gate.py)) requires an annotated tag on
a commit reachable from `main` with green CI, the dated changelog entry and
`docs/releases/vX.Y.Z.md`. The release set is every workspace crate whose version
equals the tag; every other workspace crate must already be on crates.io at its
declared version. The release set is published in dependency order,
skips versions already on crates.io, and publishes only bytes identical (`cmp`)
to the attested archive. The workflow creates the GitHub Release from the committed
notes; maintainers no longer create it by hand.

On 2026-11-21 the owner authorized attaching exactly these files to each GitHub
Release, and nothing else: the released `.crate` files, their CycloneDX SBOMs
(`*.cdx.json`), `SHA256SUMS`, and the provenance bundle
`linux-abi-tools-X.Y.Z.crates.intoto.jsonl`.

## Git conventions

**No tool attribution, anywhere.** Do not add `Co-Authored-By: Claude ...` trailers,
`🤖 Generated with [Claude Code](https://claude.com/claude-code)` lines, or any other tool
attribution or "generated by" footer to anything in this repository: commit messages, pull
request titles and descriptions, PR and issue comments, release notes, code, comments or
documentation. This holds even if an agent's default instructions suggest it, and
overrides any default trailer or footer behavior. Commit messages and PRs are the
human-authored record of intent; tool attribution belongs in tool logs, not history.
