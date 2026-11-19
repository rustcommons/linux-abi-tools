# Producer 0.6.1 release record

## Scope and status

**Published 2026-11-19 UTC** from the merge of
[PR #25](https://github.com/rustcommons/linux-abi-tools/pull/25),
`0e1e20b4723939c53d40c24c523aac2366cb9d39`. The owner authorized this patch
on 2026-11-18. It ships the kernel-abi-tools table decoder fix from
[PR #24](https://github.com/rustcommons/linux-abi-tools/pull/24); the other four
crates change only version metadata under the lockstep policy. See the
[user notes](releases/v0.6.1.md).

This is the first release published only through
[Trusted Publishing](../.github/workflows/trusted-publishing.yml), after owner
approval of the `crates-io` environment. No local token was used.

## Version and dependency contract

- All five crates are 0.6.1, and all seven internal requirements are 0.6.1.
- `Cargo.lock` and `fuzz/Cargo.lock` change only the five workspace versions.
  External dependencies, MSRV 1.90 and native-tool floors are unchanged.
- The root version is a provider-cache input, so the next preparation uses a
  new key.

## Pre-merge gates

All 140 hosted checks passed on release head
`cf053e109988d682bf4ee2ab50e7684925c37f11`, including the full CI matrix,
release-documentation, zizmor, cargo-deny, CodeQL and ClusterFuzzLite PR
fuzzing. Locally, `check-release-docs.py`, `test-publish-crates.py` and
`cargo package --workspace --locked` (all five verified) passed.

## Publication

- Annotated tag `v0.6.1` (object `97ad04ba3195d50ef94d9e94f28f02269b28bfa2`)
  points to `0e1e20b4723939c53d40c24c523aac2366cb9d39`.
- [Publish run 44459489261](https://github.com/rustcommons/linux-abi-tools/actions/runs/44459489261)
  succeeded on that commit. It published in dependency order and checked each
  official checksum before the next upload.
- crates.io `trustpub_data` for all five versions is provider `github`,
  repository `rustcommons/linux-abi-tools`, run `44459489261`, sha
  `0e1e20b4723939c53d40c24c523aac2366cb9d39`.

## Archive verification

Packages were rebuilt with Rust 1.90 `cargo package --workspace --locked` from a
clean checkout of the merge. Each local SHA-256 equals the crates.io index
`cksum` and the downloaded official archive. All are non-yanked, and each
`.cargo_vcs_info.json` records the merge sha with `dirty` false.

| Crate | Published (UTC) | SHA-256 |
| --- | --- | --- |
| `linux-targets` | 04:51:36 | `56e731b038e838d441d01f141330c5569ca44c34e04c0780a90d6a54e1270792` |
| `kernel-abi-tools` | 04:51:39 | `c9b45684cc8e511bce6cc2faeeedbb2bc61ea83999af084996eeaaf46731410e` |
| `linux-abi-scan` | 04:51:42 | `24731c0b81bd30c6c78e58d51e5f7454dd5131fb42c6243df01a36f93b01e468` |
| `libc-abi-tools` | 04:51:47 | `3888bf197d59766f38d3f60b58ff568551655c1d1cf3a6e0c5562a2390c75447` |
| `linux-abi-tools` | 04:51:52 | `16f0a9bf450f8f075867963a36bdec0185a20871fe10a673d3e19f20309f494e` |

## Documentation and consumer checks

- docs.rs reports a successful 0.6.1 build for all five crates, and each crate
  page returns HTTP 200.
- A fresh consumer (empty `CARGO_HOME`, Rust 1.90) depending on
  `linux-abi-tools = "=0.6.1"` resolved all five crates from crates.io at
  0.6.1, then built and ran. It resolved a glibc 2.17/kernel 3.10 selection,
  built a scan target, read 385 syscalls and 27 madvise values and called the
  libc ELF parser.
- The [GitHub Release](https://github.com/rustcommons/linux-abi-tools/releases/tag/v0.6.1)
  is marked Latest. `check-release-docs.py --published` passed: the live body
  matches `docs/releases/v0.6.1.md`.

## Limits

This record proves archive identity, provenance and a basic registry build. It
adds no old-OS, kernel, tier 3 or prebuilt-provider qualification. Downstream
cargo-libc adoption of 0.6.1 is a separate, optional consumer decision.
