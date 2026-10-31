# Using libc-abi

The `libc-abi` CLI and `libc_abi_tools` library inspect glibc ABI data and prepare
link providers for x86_64 Linux. For Cargo builds, start with the
[cargo-libc user guide](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md).
This guide covers direct use of the metadata tool; [Using providers](USING-PROVIDERS.md)
covers preparation and cache management.

## What works today

- Parse historical grouped ABI lists and modern per-line versioned lists.
- Load exact release snapshots for eight libraries: `m`, `c`, `ld`, `resolv`,
  `pthread`, `dl`, `rt`, and `util`.
- Export deterministic TSV or query one symbol across those libraries.
- Preserve historical symbol versions and object sizes; recognize explicit TLS
  entries when present. Absolute version-node markers and `GCC_*` entries are skipped.
- Collect the selected ABI files from a clean glibc source checkout at an exact
  `glibc-MAJOR.MINOR` tag, with source commit and license files.
- Collect defined dynamic-symbol evidence directly from ELF64 x86_64 shared
  libraries: SONAME, versions/defaults, binding, visibility, IFUNC/TLS kinds,
  64-bit sizes, and observed shared ranges. See [ELF metadata](ELF-METADATA.md).

The ABI lists for glibc **2.17 through 2.44** are compiled in from the
[glibc-abilists](https://github.com/rustcommons/glibc-abilists) crate (default
`bundled-glibc-data` feature; LGPL-2.1-or-later). Each comes unmodified from
glibc's release tag, with the tag's signature check and the file's blob id and
SHA-256 recorded there. A release label is not a claim of tested runtime
compatibility. Every release is covered by parser tests. To leave the LGPL
data out, build with `default-features = false` and pass `--data DIR`.
The default `provider-sources` feature adds
[glibc-provider-sources](https://github.com/rustcommons/glibc-provider) (glibc's
sources and headers, LGPL-2.1-or-later, and Linux UAPI headers) for `source` mode.
Build and tests require no Zig; the only dependencies are those two crates.

## Query and export ABI data

Install `libc-abi` as described in the [README](../README.md). Collection also
requires Git. Query one release:

```sh
libc-abi glibc snapshot --release 2.22 --symbol memcpy
```

Output:

```text
release target                   library symbol version       kind size
2.22    x86_64-unknown-linux-gnu   c       memcpy GLIBC_2.2.5   F    -
2.22    x86_64-unknown-linux-gnu   c       memcpy GLIBC_2.14    F    -
```

The real output uses tabs. Omit `--symbol` to export the whole release.
No matching symbol produces a header-only result. Unknown/missing releases fail;
the tool never substitutes a newer release.

```sh
libc-abi glibc snapshot --release 2.17 > glibc-2.17.tsv
libc-abi glibc export-data --output /path/to/glibc-data       # files + SOURCE.tsv + COPYING.LIB
libc-abi glibc snapshot --data /path/to/glibc-data --release 2.17
libc-abi glibc collect --source /path/to/glibc-checkout --output /path/to/new-data
libc-abi glibc elf --input /path/to/target/libc.so.6 > libc-elf.tsv
libc-abi libgcc stub --output /path/to/new-dir   # link-only libgcc_s.so.1 sources, GCC 4.8.5's interface
libc-abi glibc libraries --release 2.28 --input /path/to/lib64 > libraries.tsv
libc-abi provider prepare --glibc 2.28          # a link provider, built locally; prints its descriptor
libc-abi provider list
```

`libc-abi provider` prepares glibc link providers (stubs, startup objects,
nonshared archives, headers) for cargo-libc's `--provider` in a cache. The default
`source` mode compiles the recorded glibc sources of
[glibc-provider-sources](https://github.com/rustcommons/glibc-provider) with the
host's GCC, offline, and verifies every object against the qualified container
build; `container` mode runs that build locally. **[Using glibc providers](USING-PROVIDERS.md)**
explains when to choose each mode, the cache, and offline and air-gapped use; the
design is in [PROVIDER-DELIVERY.md](PROVIDER-DELIVERY.md#libc-abi-provider).

The installed executable carries the ABI lists; `--data` reads a directory with
the same layout instead (required when built without `bundled-glibc-data`).
Collection refuses an existing output release and validates the
inputs before writing. An I/O failure during writing can leave a partial new
directory; inspect it before retrying. Source tags identify a checkout, not its
authenticity: use a trusted source repository.

## Why exact release snapshots?

Symbols can move between libraries. The newer file may retain an old symbol
version while moving its library membership; inferring all history from the
latest file would put it in the wrong library for older targets. Tests cover
`pthread_sigmask` moving to libc in 2.32 and startup changes in 2.34.

glibc 2.17 is a **target release floor**, not a symbol-version filter.
`GLIBC_2.2.5` still belongs in a 2.17 or 2.22 snapshot.


## Compare releases and check membership

```sh
libc-abi glibc matrix --symbol fmemopen
libc-abi glibc diff --from 2.17 --to 2.22
libc-abi glibc changes
libc-abi glibc expect --release 2.22 --library c --symbol memcpy --version 2.14
```

`expect` reports accept/reject/unknown membership; missing evidence is not a
compatibility verdict. The first observed version is not necessarily the origin.
See [history and reference generation](GLIBC-HISTORY.md) for schemas and evidence,
and the [interface reference](reference/GLIBC-SYMBOLS.md) for browsable results.

## ELF evidence and link-only stubs

`glibc elf` reads definitions from an ELF64 x86_64 shared library. `glibc libraries`
checks the eight real libraries against one release's ABI lists and emits the
library table used by provider generation. See [ELF metadata](ELF-METADATA.md).

`libgcc stub` writes assembly and a version script into a new directory; it does
not build or supply a runtime library. `libc-abi --help` shows the assembler/linker
commands. Generated functions trap if executed: deploy real runtime libraries,
never the stubs. The [stub-generation experiments](STUB-GENERATION.md) and
[full-library validation](FULL-LIBRARIES.md) record what was checked.

## Library use

```toml
[dependencies]
libc-abi-tools = "0.3"
```

Use `glibc::Snapshot`, `Matrix`, `elf::Metadata`, and the `provider` module via
[the API documentation](https://docs.rs/libc-abi-tools). The default features
`bundled-glibc-data` and `provider-sources` are independent. Disabling defaults
removes both: supply external ABI data where required, and source-mode provider
preparation is unavailable unless `provider-sources` is enabled again.

For metadata-only code with bundled ABI lists but no provider source pack:

```toml
[dependencies]
libc-abi-tools = { version = "0.3", default-features = false, features = ["bundled-glibc-data"] }
```

## Maintenance

Collection from an upstream checkout, provider qualification, generated references,
and release procedures are covered in the [maintainer guide](MAINTAINING.md).
