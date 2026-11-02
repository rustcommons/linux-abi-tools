# Changelog

User-facing changes for the five linux-abi-tools workspace crates. Package/CLI
identities and setup are in the [README](https://github.com/rustcommons/linux-abi-tools/blob/main/README.md);
all five packages share a version under the [release policy](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/RELEASING.md).
The root package remains `libc-abi-tools`; `linux-abi-tools` is the umbrella library.

History was reconstructed on 2026-11-02 from existing tags, source differences,
registry records and dated evidence. Dates below are original registry publication
dates in UTC, not GitHub Release backfill dates. These changelog/notes files did
not exist at the old tags. Missing historical validation is stated in each note;
no tag was moved and no crate was republished.

## [Unreleased]

### Documentation

- Add this changelog and versioned user-facing notes for all seven existing tags;
  distinguish old test evidence, current artifact verification and remaining gaps.
- Add release navigation, the shared AGENTS release rule v2 and the stdlib-only
  checker v3/workflow. The seven existing-tag GitHub Releases are now backfilled
  after independent review and green bootstrap CI; exact bodies/tags, only
  v0.4.0 latest, zero attached assets and full published-verification CI passed.
  This documentation work introduces no new software version or binary.

## [0.4.0] - 2026-11-02

Consumers can explicitly request a one-library overlay that selects libc clock defaults while retaining librt compatibility versions and every other byte.

- `libc_abi_tools::provider::link_profile` adds `prepare`, `LinkProfile::LibcClocksV1` and `Overlay` for actual selected libc/librt ELF files. The five clocks use libc GLIBC_2.17 defaults; applicable librt GLIBC_2.2.5 definitions become compatibility-only in a copy. Already compatible inputs return no overlay.
- ELF validation rejects conflicting partial/load-page/zero-fill mappings. A content/policy-bound manifest index uses standard-library hard-link publication without replacing existing files/directories; concurrent losers verify the winner and clean only their unpublished generation.
- Ordinary rendering/providers, raw tables, descriptor v1 and linker argument order remain unchanged. There is no new CLI clock knob or unwinder/runtime implementation. All five packages share 0.4.0 under the lockstep API-addition policy.

[Changes, upgrade guidance, validation and limits](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/releases/v0.4.0.md).

## [0.3.3] - 2026-11-01

Malformed ELF processing is bounded without rejecting valid large fixed tables, and Docker/Podman runners retain explicitly requested environment and redirected stdin.

- ELF parsing uses checked ranges and bounded version-chain work; valid large fixed symbol/relocation tables, including the recorded Rust Cargo and rustc-driver files, remain accepted.
- Equivalent executable aliases coalesce while distinct address evidence remains represented. Excessive site expansion returns an explicit resource error rather than truncating evidence.
- Container execution forwards explicit environment selections and redirected stdin. Credentials are forwarded only when explicitly selected; the whole host environment is not implicitly copied.
- All five packages share 0.3.3. Upstream glibc inputs and kernel profile data are unchanged.

[Changes, upgrade guidance, validation and limits](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/releases/v0.3.3.md).

## [0.3.2] - 2026-10-31

Closing a CLI output pipe no longer causes a tool-owned stdout panic, while findings and later operation failures still retain their error status.

- `libc-abi`, the shared provider CLI, `linux-abi-scan` and kernel CLI output paths handle `BrokenPipe` at write/flush boundaries.
- Provider operations continue after output closes and still report later failures. Scanner findings remain exit 1; usage/read errors remain exit 2. Genuine output errors still fail, and ordinary output bytes stay unchanged.
- Packaged READMEs and user/maintainer guides clarify installation, package/CLI identities and feature choices. The five versions move together without ABI-data, API, upstream-input or MSRV changes.

[Changes, upgrade guidance, validation and limits](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/releases/v0.3.2.md).

## [0.3.1] - 2026-10-13

All five crates gain public API documentation and runnable crate examples, making the existing library interfaces easier to use.

- Public items are documented, crate overviews include examples, manifests describe docs.rs builds, and CI checks generated documentation.
- This is a documentation-only patch release: it does not add a provider mode, change ABI data, or change runtime/tool behavior.

[Changes, upgrade guidance, validation and limits](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/releases/v0.3.1.md).

## [0.3.0] - 2026-10-13

Providers can now be prepared and cached by the library or `libc-abi provider`, instead of assembling every descriptor and input directory manually.

- `libc-abi-tools` adds source/container preparation plus `prepare`, `path`, `list`, `verify`, `export`, `import` and `remove` provider commands.
- Default source mode compiles the recorded `glibc-provider-sources` inputs with host GCC and verifies object interfaces; an unreviewed difference refuses rather than falling back. Container mode runs the pinned glibc recipes.
- Prepared providers include a generated GCC 4.8.5 `libgcc_s` interface stub. The host-newer-symbol probe selects only versions above the host glibc floor.
- The source-input dependency is available from crates.io. The other four crates receive the synchronized version without a new implementation in their source bodies.

[Changes, upgrade guidance, validation and limits](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/releases/v0.3.0.md).

## [0.2.0] - 2026-10-11

Explicit provider selection now accepts every recorded glibc release from 2.17 through 2.44, rather than only 2.17, 2.18 and 2.22.

- `libc-abi-tools` extends `provider::RELEASES` to all 28 releases, keeping exact selection and refusal of a mismatched descriptor.
- `libpthread_nonshared.a` is required only before glibc 2.28, matching the recorded inputs.
- Per-release source-input recipes and measured ELF/conformance evidence cover the added releases. The other four packages share the workspace version without a new product API or implementation in their source bodies.

[Changes, upgrade guidance, validation and limits](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/releases/v0.2.0.md).

## [0.1.0] - 2026-10-11

The first workspace release supplies glibc ABI queries, explicit link-provider descriptors, target presets, kernel simulation and static executable inspection.

- `libc-abi-tools` supplies the `libc-abi` CLI, exact glibc 2.17–2.44 snapshots, ELF symbol evidence and link-only stubs. A target floor retains older symbol versions such as `GLIBC_2.2.5`.
- Explicit `libc-abi-provider-v1` descriptors support qualified glibc 2.17, 2.18 and 2.22 input profiles. Provider preparation is an external workflow in this version; automatic preparation arrives in 0.3.0.
- `linux-targets` resolves distribution/target floors; `kernel-abi-tools` supplies `kernel-seccomp` and `syscall-probe`; `linux-abi-scan` inspects ELF requirements. The `linux-abi-tools` library re-exports all four crates.

[Changes, upgrade guidance, validation and limits](https://github.com/rustcommons/linux-abi-tools/blob/main/docs/releases/v0.1.0.md).
