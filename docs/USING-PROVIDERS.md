# Using glibc providers

A **provider** is everything needed to *link* an x86_64 Linux program for one glibc
release, 2.17 through 2.44, on a newer machine. It contains:

- that release's headers and startup objects (`crt1.o`, `crti.o`, `crtn.o`, the
  nonshared archives);
- link-only stubs of its eight libraries, which expose exactly the symbols and
  versions that release defines;
- a link-only `libgcc_s` stub with GCC 4.8.5's interface;
- your GCC's `crtbegin.o` and `crtend.o`.

A provider is described by a six-field v1 descriptor, `provider.txt`
([PROVIDER-CONTRACT](https://github.com/rustcommons/cargo-libc/blob/main/docs/PROVIDER-CONTRACT.md)).
It is not a runtime: the program runs on the target system's own glibc.

You rarely prepare one by hand. [cargo-libc](https://github.com/rustcommons/cargo-libc)
prepares the provider for the release you select on first use. The commands below
are the same in both tools: `libc-abi provider ...` here, `cargo libc provider ...`
there.

## Choosing a mode

| Mode | What it does | Needs | Time | Use it when |
| --- | --- | --- | --- | --- |
| `source` (default) | Compiles glibc's startup sources, shipped in the tool through the [glibc-provider-sources](https://github.com/rustcommons/glibc-provider) crate, with your GCC. Every object is compared with the qualified container build before the provider is used; a difference refuses it. | GCC, GNU binutils | seconds | Almost always: offline, fast, nothing downloaded. |
| `container` | Runs the pinned container build that CI qualifies: the release's signed source archive, built in a digest-pinned CentOS 7, Debian 9 or Debian 12 image. | podman or docker, network, GCC, binutils, curl | 5–30 minutes per release | Source mode refused your GCC, or you want the objects CI built byte for byte. |
| `prebuilt` | CI-built objects, downloaded. | — | — | Not available: it waits on a legal review. |

Both available modes generate the same stubs, `libgcc_s` stub and headers.
Startup/nonshared objects are built by different routes. Container mode also
retains real runtime libraries; source mode uses a link-only loader stub and
supplies no runtime implementation. CI qualifies both available modes for every
release ([PROVIDERS.md](PROVIDERS.md#delivery-modes-per-release)). One mode never
falls back to another: if `source` refuses, nothing is used until you choose
`container`.

**Source mode and your GCC.** GCC 11 through 16 are qualified: hosted CI rebuilds
every release with them and checks the result
([glibc-provider's replay](https://github.com/rustcommons/glibc-provider/blob/main/docs/REPLAY.md)).
Another GCC is still tried, and its objects are verified the same way before use.
Clang cannot be used: it compiles some of glibc's startup code differently
([why](PROVIDER-DELIVERY.md#why-source-mode-compiles-with-gcc-not-clang)). Choose a
GCC with `--cc GCC` (cargo-libc: `CARGO_LIBC_PROVIDER_GCC`).

## Commands

```sh
libc-abi provider prepare --glibc 2.28                     # source mode
libc-abi provider prepare --glibc 2.28 --mode container
libc-abi provider prepare --all                            # every release, source mode
libc-abi provider list
libc-abi provider path --glibc 2.28                        # the descriptor to use
libc-abi provider verify --glibc 2.28                      # recheck files and stubs
libc-abi provider export --glibc 2.28 --to DIR
libc-abi provider import DIR
libc-abi provider remove --glibc 2.28                      # or --all
```

`prepare` prints the descriptor's path on standard output. It reuses a cached
provider with the same inputs, unless you pass `--force`. Container-mode options:
- `--engine PATH` selects the container engine;
- `--archives DIR` uses glibc source archives you already downloaded;
- `--keep-work DIR` keeps the build tree for inspection. It requires
  `--mode container`, one `--glibc` release (not `--all`), and a directory that
  does not already exist.

## The cache

Providers are cached in `$CARGO_LIBC_PROVIDER_DIR`, otherwise in
`$CARGO_HOME/libc-providers`, otherwise in `~/.cargo/libc-providers`. Use
`--cache DIR` to choose another location. Each entry has this path:

```text
x86_64-unknown-linux-gnu/glibc-2.28/source-<hash>/
  provider.txt  manifest.json  libraries.tsv  verification.tsv
  sysroot/  stubs/  compiler-runtime/
```

The hash covers everything the provider depends on:
- the tool version and the release's data;
- for container mode, the image and archive;
- your GCC, binutils and `crtbegin.o`/`crtend.o`.

So an upgraded compiler yields a new entry rather than a stale one. `manifest.json`
records each file's SHA-256, and `verify` checks them all. Delete an entry with
`remove`, or delete the directory itself.

## Offline and air-gapped machines

**Source mode works offline.** The sources are compiled into the tool, so a
machine with the tool, GCC and binutils needs nothing else. Install the tool:
- with `cargo install libc-abi-tools --locked` on a connected machine;
- or offline, from source vendored with `cargo vendor`;
- or by copying a binary built for the destination host. For cargo-libc's helper
  installation, follow its [user guide](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md).

**Container mode needs the network** to pull the image and download the source
archive. On a machine that cannot reach them:

1. Prepare and export the provider on a connected machine:

   ```sh
   cargo libc provider prepare --glibc 2.17 --mode container
   cargo libc provider export --glibc 2.17 --mode container --to /media/usb/providers
   ```

2. Import it on the offline machine. Import checks every file against the
   manifest, and points the provider at that machine's own `crtbegin.o` and
   `crtend.o`:

   ```sh
   cargo libc provider import /media/usb/providers
   ```

3. Use it. If both machines have the same GCC and binutils, automatic selection
   finds it. Otherwise the cache hash differs, so pass the imported descriptor
   explicitly; `import` prints its path, and so does `list`:

   ```sh
   cargo libc build --provider ~/.cargo/libc-providers/x86_64-unknown-linux-gnu/glibc-2.17/container-<hash>/provider.txt \
     --target x86_64-unknown-linux-gnu.2.17 --artifact target/x86_64-unknown-linux-gnu/release/app -- --release
   ```

Without exporting, `--archives DIR` avoids the source download, but the image
still has to be pulled from its registry.

## What a provider does not cover

A provider proves only that your program links against what the selected release
defines. It does not cover:
- the program's kernel needs (see the
  [testing tiers](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md#testing-tiers));
- other shared libraries it loads;
- distribution changes to glibc ([DISTRIBUTION-DIFFERENCES.md](DISTRIBUTION-DIFFERENCES.md)).

Executables built with a 2.34 or later provider need glibc 2.34, because from 2.34
`crt1.o` references `__libc_start_main@GLIBC_2.34`.


## Derived clock link overlay API

The published 0.4.0 API `provider::link_profile` accepts actual selected
`libc.so.6` and `librt.so.1` ELF files. It is independent of provider preparation:
source/container providers, explicit providers and full sysroots can supply the
same inputs. Ordinary stub rendering, provider caches, raw recorded ABI tables
and the `libc-abi-provider-v1` descriptor keep their faithful contracts. The CLI
has no clock policy option. Published 0.3.3 does not expose this API.

Call `link_profile::prepare(libc, librt, cache_root,
LinkProfile::LibcClocksV1)` to validate the inputs and derive the overlay.
`LinkProfile::id()` is `"libc-clocks-v1"`. `Overlay` returns the two original
SHA-256s, the profile and an optional `search_dir`. A nonempty directory contains
regular `librt.so` and `librt.so.1` copies plus `manifest.json`. Its identity binds
both input contents and the policy. A complete private generation is published by
atomically hard-linking its manifest to a content-key index file, with no replace
semantics. The manifest includes a validated single-component generation name,
policy, both raw hashes and the overlay hash. Readers follow only that index and
verify the complete generation. Existing index files or directories are never
repaired or overwritten, including if they appear during publication. Losing
writers verify the published winner and clean up only their own unpublished
generation. Unindexed partial generations are ignored. The cache filesystem must support
ordinary same-filesystem hard links; unsupported publication returns an error. The local
cache root must be controlled by the caller; this is not authenticated delivery
or a promise of crash-durable filesystem transactions.

The helper requires bounded little-endian x86_64 ELF64 DSOs with ordinary
section/program headers, expected SONAMEs, and consistent loader/section tables.
It uses the [ELF metadata limits](ELF-METADATA.md), including 64 MiB per input and
100,000 dynamic symbols. Stripped section tables, linker scripts, unsupported
inputs and inconsistent cache contents return errors without fallback. libc must
provide all five default `GLIBC_2.17` clock definitions: `clock_gettime`,
`clock_getres`, `clock_settime`, `clock_nanosleep`, `clock_getcpuclockid`.

The derived copy sets only the GNU versym nondefault bit for these clocks'
`GLIBC_2.2.5` default definitions in librt. It retains their explicit old versions;
all remaining bytes, code, aliases, binding, symbol kind, sizes, library identity
and dependencies remain unchanged. Derived metadata describes the copy separately
from raw measurement. The recorded 2.17–2.29 stubs need this change; 2.30 has only
compatibility clock definitions and 2.31+ has none in librt. Already compatible
inputs return `search_dir: None`, with validated hashes and no cache creation.
The overlay supplies no runtime implementation and makes no unwinder changes.

The caller must resolve the exact selected ELF files and prove they belong to the
selected provider/sysroot. Compiler probes must not leak host libraries or accept
ambiguous scripts. Prefix the overlay search directory for ordinary `-lrt` and
`-l:librt.so`/`-l:librt.so.1`. Absolute paths or scripts require identity-checked
substitution at the same argument position, or refusal. No library arguments,
groups, archives, response-file contents or linker state may be reordered.
Host compilation and inspection remain outside this target-link policy. A
consumer must include the policy ID and both hashes/path in its configuration and
fingerprint; this helper does not orchestrate Cargo or compiler probing.

GNU ld can merge mixed ordinary and explicit old-version references when library
order alone is changed. The byte-derived compatibility-only overlay avoids that
observed failure without removing explicit versions. The [release evidence](RELEASING.md#published-040-archives-and-documentation)
distinguish producer byte/link evidence from pending consumer qualification.
