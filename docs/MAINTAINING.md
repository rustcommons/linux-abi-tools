# Maintaining linux-abi-tools

For installed tools and library consumers, start with [Workflows](WORKFLOWS.md).
This guide covers source-checkout validation and data/provider maintenance.
Read [AGENTS.md](../AGENTS.md) before changing the repository.

## Build and validate a checkout

Rust/Cargo 1.90+ is required. Root Cargo commands select `libc-abi-tools` alone;
use `--workspace` for all five crates. Fetch locked dependencies before offline checks:

```sh
cargo fetch --locked
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked --offline
cargo test --workspace --doc --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

For a local CLI installation, `cargo install --path . --locked` installs `libc-abi`.
Kernel/profile changes need the local and container self-tests described in
[AGENTS.md](../AGENTS.md) and the [kernel guide](USING-KERNEL-ABI.md).
Keep API docs warning-free and use the declared MSRV in addition to the CI toolchain.
Packaging and publication gates live in [Releasing](RELEASING.md) and the
[next release plan](RELEASE-PLAN.md); publishing requires explicit authorization.

## Producing the data and providers

### 9.1 glibc ABI lists

Repository [glibc-abilists](https://github.com/rustcommons/glibc-abilists); its
collector is MIT and not published.

1. Clone glibc's repository without file contents (`git clone --bare --filter=blob:none`).
2. For each release tag in `upstream.toml`'s range (2.17–2.44), require the tag object
   to agree between sourceware and the GitHub mirror.
3. Verify the tag's signature against the release managers' keys in `keys/`, merged
   from every source. 2.30's key is unpublished, which is recorded as an exception.
4. Select the x86_64 `*.abilist` files by the rule in `upstream.toml`, and record each
   file's path, git blob id, SHA-256, size, and the release's license evidence.
5. Write `data/` and `src/generated.rs`; `--check` regenerates and compares instead.
6. Publish the crate. `libc-abi-tools` compiles the lists in (feature
   `bundled-glibc-data`).

### 9.2 Kernel data

`crates/kernel-abi-tools/scripts`, run online only when refreshing:

1. `fetch-syscall-tables.sh`: for each mainline Linux tag, download the x86_64 syscall
   table and the generic `mman` header.
2. `build-syscall-data.sh` and `build-madvise-data.sh`: derive the first release of every
   64-bit syscall and `madvise` advice value into the committed TSV data. The 3.3
   floor is reported as the floor, not as the true origin.
3. Build: the data is compiled in, and the seccomp profiles (OCI JSON and classic BPF)
   are generated from it; unit tests check both against the data.
4. `self-test.sh` with `syscall-probe`: run every profile with the local runner and in
   a container (crun with libseccomp 2.6+).

### 9.3 Provider pipeline

One release at a time, in `examples/` (`glibc-two-paths/` holds the shared scripts
and 2.17's two routes; `glibc-X/` holds each later release's pinned inputs). Step
by step:

1. **Pin the inputs:** `examples/glibc-X/INPUTS.toml` records:
   - the GNU archive URL and SHA-256, and its signature check;
   - the digest-pinned builder image and the signed package archive;
   - the configure options and any source patch.

   Builders:
   - CentOS 7 for 2.17–2.25;
   - Debian 9 for 2.26–2.36;
   - Debian 12 for 2.37–2.44.
2. **Fetch the source:** `fetch-glibc.sh` downloads from ftp.gnu.org, falling back to
   the kernel.org GNU mirror, and requires the pinned SHA-256.
3. **Build glibc from source in the builder:** `run.sh` starts the pinned image and
   `build-inputs.sh` inside it:
   - installs the signature-checked packages and records them;
   - copies the builder's real `libgcc_s.so.1`, used only to run the tests;
   - configures (`--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2`),
     builds, and installs into an isolated `source/` tree;
   - copies the kernel UAPI headers.

   2.17 also has a `prebuilt` route made of CentOS 7 packages, and a one-line
   source adjustment for its freestanding header build.
4. **Audit the real libraries:** `elf-and-check.sh`:
   - reads each of the eight real DSOs with `libc-abi glibc elf` and compares the result
     with GNU `readelf`;
   - compares every public versioned definition with the pinned ABI list
     (`libc-abi glibc snapshot`), requiring zero differences;
   - writes the release's matrix and a manifest of input hashes.
5. **Generate and audit the stubs:** `prepare-and-check.sh`:
   - generates the eight link-only DSOs from the measured metadata;
   - checks the exact exported set, default and compatibility versions, types,
     binding, visibility, object sizes, alignment, shared storage, SONAME and
     absence of dependencies (`check-full.py --abi-only`);
   - generates the `libgcc_s` stub (`libc-abi libgcc stub`) and checks its exports and
     version chain against the recorded GCC 4.8.5 interface, and that the real library
     the tests run with provides all of it (`check-libgcc.py stub`).
6. **Exhaustive linking:** `check-link-matrix.py`, against each isolated stub library.
   - Every interface present in the release links.
   - Every absent interface fails.
   - Every measured default binds to its version.
   - Every compatibility-only name rejects a new unversioned reference.
   - A synthetic TLS fixture covers TLS relocations.
7. **Package for CI** (CI only): `ci-bundle.py pack` writes a checksummed inventory
   for the application jobs; `unpack` verifies it on each runner.
8. **Applications on the real runtime:** `application-and-check.sh` and
   `behavior-and-check.sh`.
   - **Rust program:** allocation, threads and TLS destructors, and unwinding.
   - **C programs:** both `memcpy` versions, copy-relocated historical data views,
     `fmemopen` versions and default, the pthread default-attribute boundary, and the
     `sigvec` default removal.
   - **Too-new link test:** a matrix-chosen symbol (`too-new-symbol.py`) must fail to
     link.
   - **Five library behavior suites:** libm, libdl, librt, libutil and libresolv.

   Everything runs under the release's real loader. Runtime dependency paths and
   loaded modules are audited (`check-full.py --application-only`,
   `check-behavior.py`).
9. **Cargo integration:** `provider-and-check.sh` exports a v1 descriptor
   (`provider_glibc` example) and uses cargo-libc for:
   - `build` with the versioned target and the tier 1 scan;
   - `run` under a Linux 3.10 local filter;
   - the host-glibc refusal (for providers newer than the host);
   - `check`;
   - conflicting `--glibc` and mismatched-release rejection;
   - workspace, feature, profile and cached-build fixtures;
   - the matrix-chosen too-new Cargo link failure.

   Link maps and runtime paths are audited (`check-provider.py`).

### 9.4 Evidence and reference

1. `scripts/import-glibc-elf-profile.py` adds the release's measured profile to
   `data/glibc-history/` after rechecking hashes, membership and differences, and
   refuses to replace an existing profile.
2. `scripts/build-glibc-reference.py` regenerates the matrices and the Markdown
   interface reference; `--check` verifies it.
3. The release's `docs/GLIBC-X.md` and the [provider index](PROVIDERS.md) record the
   inputs, results, measured changes from the previous release and hosted runs.

### 9.5 Hosted qualification

`.github/workflows/glibc-2.17.yml`, on every push:
- one **preparation** job per release runs steps 2–7, building through container mode
  (`libc-abi provider prepare --mode container --keep-work WORK`), then compares the
  source-mode provider with that build (`source-mode-and-check.sh`) and with the
  container-mode provider (`modes-and-check.sh`), and exports the container-mode
  provider;
- two **application** jobs per release, one per Clang (21 and 24), run steps 8–9 with
  Rust 1.99 and the pinned cargo-libc commit; they import the container-mode provider,
  prepare the source-mode one, and run the Cargo suite against both too
  (`ABI_PROVIDER_MODES`);
- diagnostics are kept as artifacts.

Main CI covers formatting, tests, clippy (Windows and Linux), the minimum Rust
version, the ABI lists and the reference check.

### 9.6 Release

[RELEASING.md](RELEASING.md):
1. Set one version across the five crates.
2. Run the full validation and `cargo package --workspace --locked`.
3. Publish in dependency order from a commit with green CI.
4. Tag `vX.Y.Z` and record the release.

Required glibc-abilists and glibc-provider-sources versions must already be published.
Unchanged upstream crates need no new release.


## Refresh kernel data

Run from `crates/kernel-abi-tools`. These maintenance scripts fetch upstream
inputs; preserve provenance and review generated diffs. Never edit generated data
or profiles by hand.

```sh
scripts/fetch-syscall-tables.sh /tmp/raw
scripts/build-syscall-data.sh /tmp/raw data/linux/syscalls-x86_64.tsv
scripts/build-madvise-data.sh /tmp/raw data/linux/madvise-x86_64.tsv
cargo test -p kernel-abi-tools --locked
cargo run -p kernel-abi-tools --release --locked --bin kernel-seccomp -- generate profiles
```

The final command builds the generator before using it; `cargo test` alone does
not produce a release binary. Follow with the required local/container probes and
review the profile/data diffs before committing.
