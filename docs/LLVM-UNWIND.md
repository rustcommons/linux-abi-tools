# Experimental LLVM archive preparation

This is an **unreleased, preparation-only foundation**, independent of the open
application-linking/default decision. Current CLI behavior and published 0.4.0
remain unchanged. An archive is not proof of interoperability, a measured GLIBC
floor or permission to distribute a binary. There is no new CLI option.

## Source and construction contract

`libc_abi_tools::runtime::llvm_unwind::prepare(&PrepareOptions)` returns
`Result<PreparedArchive, String>`. Preparation compiles an ordinary x86_64 GNU/Linux
archive; it never executes target code or changes linker arguments, raw glibc
inputs, provider descriptors or user-target C++ support. The internal C++17 build
is limited to pinned libunwind sources, with no exceptions, RTTI or C++ library
headers. It does not enable compiling arbitrary user C++ targets.

The bundled subset has 31 **unmodified** LLVM 23.1.4 files (including the complete
license), 759,620 source/license bytes plus a 2,681-byte inventory. The source
identity is:

- Tag: `llvmorg-23.1.4`; peeled commit
  `7a941dfb2877a0b8cf30323fefa85ad019e0b0f5`.
- [Official source archive](https://github.com/llvm/llvm-project/releases/download/llvmorg-23.1.4/llvm-project-23.1.4.src.tar.xz),
  179,272,572 bytes; SHA256
  `819ecf7351f1a66f76b2d701d8942faaf5faeb324cb90c9d5b090de142434bb2`.
- Every selected file was byte-compared to that archive; per-file SHA256s are in
  [SOURCES.tsv](../src/runtime/llvm_sources/SOURCES.tsv), also embedded in the API.
  Archive SHA matched the official release asset digest; the detached signature
  was **not** verified.
- Preserve the full [upstream license](../src/runtime/llvm_sources/LICENSE.TXT):
  Apache-2.0 WITH LLVM-exception and the historical libunwind MIT/NCSA notices.
  See [NOTICE](../NOTICE.md). Source inclusion is not legal clearance for a prebuilt
  runtime/SDK/binary distribution; the existing prebuilt legal gate is unchanged.

The nine compilation units are `libunwind.cpp`, `Unwind-EHABI.cpp`,
`Unwind-seh.cpp`, `UnwindLevel1.c`, `UnwindLevel1-gcc-ext.c`, `Unwind-sjlj.c`,
`Unwind-wasm.c`, `UnwindRegistersRestore.S` and `UnwindRegistersSave.S`.
Unused target branches remain upstream source, not additional supported targets.
No compiler-rt personality or generic compiler helpers are bundled by this API.
In particular, libunwind alone is not a complete `libgcc_s` replacement:
`__gcc_personality_v0`, arithmetic helpers and explicit GCC versions require
separate qualification. `GetDataRelBase`/`GetTextRelBase` have aborting non-AIX
implementations; a symbol inventory is not semantic completeness.

`ArchiveProfile::PrivatePicV1` builds PIC with hidden definitions;
`ExportablePicV1` builds PIC with default public visibility. These are construction
profiles, not an executable export/interposition policy, output-kind selection or
interop qualification. Neither is the default for any application.

## Selected roots, tools and observed dependencies

`PrepareOptions` contains `target`, `profile`, `sysroot`, `libc`,
`provider_files`, `toolchain` and `cache_root`. `Target::X86_64LinuxGnu` is the only
preparation target. `PreparedArchive` returns `archive`, `manifest`, `identity`,
`archive_sha256`, `profile` and `source_commit`.

- `sysroot/usr/include` is the complete selected header tree; no host supplements
  are searched. It is snapshotted, content-hashed and used from private staging.
- `libc` is `SelectedFile { root, path }`: a bounded real x86_64 ELF64 DSO with
  SONAME `libc.so.6`. A linker script is refused; the caller must select its actual
  ELF target deliberately. Each additional `provider_files` item has its **own**
  `SelectedFile` root. Header, library and descriptor roots need not share a
  common parent; descriptor v1 does not require one. Files must lie inside their
  declared roots. Caller owns semantic selection and header/library consistency.
  Physical containment and selecting 2.17 headers do not measure a runtime floor.
- Roots must be existing normalized absolute directories, with no symlink
  components. Header/resource trees and selected/support files refuse symlinks.
  Explicit compiler/archiver aliases are the exception: they resolve once to
  regular canonical executables and are resolved/rechecked on replay/post-build.
  A legitimate linked header tree must be materialized deliberately first; there
  is no implicit traversal outside the selected root.
- `Toolchain` names absolute `clang`, `clangxx`, `archiver` and
  `dependency_files`. The two compilers must identify as Clang, accept
  `--no-default-config`, target x86_64 and report matching resource-header content.
  Tested construction uses Clang 16; Clang 14's unsupported configuration flag
  is an explicit refusal, not a fallback. Windows-host preparation is unqualified.
- Executable bytes, version output, compiler resources and caller-declared support
  files contribute to identity. Callers must declare actual relevant loaded
  compiler libraries/support files. This API does **not** discover or guarantee
  that closure: loaded libraries, subprocesses, plugins and unobserved config or
  tool behavior may escape declaration. Version strings are not provenance
  certification. Trusted tools/local roots are prerequisites, not a sandbox.
- Every command clears inherited environment, including `CCC_OVERRIDE_OPTIONS`,
  `CPATH`, `CPLUS_INCLUDE_PATH`, `LIBRARY_PATH`, `COMPILER_PATH` and
  `GCC_EXEC_PREFIX`; only locale, private temporary paths and Windows SystemRoot
  are set. There is no PATH lookup. `--no-default-config`, explicit resource
  directory, `-nostdinc`, `-nostdinc++` and integrated assembly constrain the build.
- Each compile emits `-MD` dependencies. Every observed path and its bytes must
  match the exact staged source/header/resource inventory. Undeclared observed
  headers, escapes, symlinks or mutations refuse. This does not observe all
  compiler dependencies and must never be described as a hermetic build or a
  cache that detects every possible toolchain change.

The fixed recipe uses `-O2 -g0 -fPIC`, unwind tables, native-only libunwind,
`NDEBUG`, no stack protector, function/data sections and deterministic `ar rcsD`.
Frame-header cache and optional frame compatibility APIs are not enabled. File
prefix mapping removes private generation paths from source-location strings.
Tool commands run sequentially; driver timeout/output are polled at 120 seconds/1 MiB;
descendant processes are not sandboxed as a process group. The caller must limit concurrent preparations. No arbitrary static archive,
host unwinder, personality or helper fallback is performed.

## Manifest and immutable cache

The internal index/manifest schema is `libc-abi-llvm-archive-v1` (not provider
descriptor v1). The hard-linked content-key index contains:

| Field | Meaning |
| --- | --- |
| `generation` | Validated single-component `.gen-PID-SERIAL` name |
| `identity` | SHA256 of canonical serialized `inputs` |
| `inputs` | Schema/policy, explicit contract/limitations, source/archive/inventory pin, preparation implementation identity, profile, target, independent selected roots/libc hash, header/resource identities, tool paths/bytes/versions and declared support/provider file identities |
| `archive_sha256` | Conventional SHA256 of the entire ordinary archive |
| `object_inventory` | Exact nine object names and conventional SHA256s |
| `unresolved_symbols` | Undefined symbols remaining after subtracting archive definitions; NOT a resolved or measured GLIBC/helper closure |
| `build_log_sha256` | Conventional SHA256 of recorded commands and observed dependency rules |

Large tool/support/provider-file `content_identity` values are **not** raw-file
SHA256: `runtime-file-chunks-v1` hashes a domain-separated sequence of fixed 1 MiB
chunk SHA256s/lengths and total byte count. Header tree identities hash sorted native path components (length-prefixed
component bytes) and per-file SHA256s; no Unix backslash-to-separator conversion occurs. Sources, archive and object hashes are
conventional SHA256. Bounds: 10,000 files/64 MiB per header tree, 8 MiB per tree
file/object, 1 GiB per tool identity file, 32 MiB archive, 64 KiB manifest and
16 MiB aggregate build log. Oversized inputs refuse explicitly.

A complete private generation contains only archive, manifest, build log,
objects, license and source inventory. Inputs are copied privately and checked
again after execution. Publication uses `fs::hard_link` to a create-new index,
never directory replacement. An existing file, symlink or empty directory cannot
be overwritten. Competing losers validate the winner and remove only their own
unpublished staging. Readers follow only a published index and recheck contents;
unindexed partial generations are ignored. Corrupt entries refuse without repair.
Interrupted/power-loss state may refuse on replay; crash recovery is not repair.

Archive validation checks ordinary GNU ar framing/padding/names, exact member
bytes/inventory and complete symbol-index targets; thin archives are refused.
Each member must be bounded little-endian x86_64 ELF64 ET_REL with valid section,
string, symbol and RELA tables/ranges/links. Allowed relocation forms are
`R_X86_64_64` (non-executable sections only), `PC32`, `PLT32`, `GOTPCREL`,
`GOTPCRELX` and `REX_GOTPCRELX`. Required unwind definitions must have the chosen
visibility. Unexpected C++/GCC runtime imports refuse; all remaining unresolved
symbols are recorded. These structural checks do not prove final linked PIC,
exception semantics, API completeness, GLIBC owners/floors or application closure.

## 2026-11-02 research checkpoint

These are bounded **failed compatibility experiments**, not released behavior:

- Hidden LLVM 23.1.4 passes contained Rust panic but baseline-valid cross-DSO
  U10 aborts/U11 segfaults. Minimal `_Unwind_*` exports pass U07/U10/U11 under
  native GNU bfd and LLD (18 dynamic definitions in those executables).
- C-only `-fexceptions` pthread exit/cancellation fails SIGSEGV under both linkers
  even with the pinned compiler-rt personality selected. GCC baseline passes.
  Loader evidence shows handle-local GCC forced unwinding alongside interposed
  LLVM accessors. No Rust frames are on those worker stacks.
- Same unchanged C fixture, Zig 0.17.0 `-target x86_64-linux-gnu.2.22 -pthread
  -fexceptions`, fails linking without explicit `-lunwind`. The explicit-unwind
  binary passes backtrace but also SIGSEGVs on exit/cancel on host glibc 2.39.
  This is not Aero Rust or old-OS qualification and grants no compatibility waiver.
- A context-dispatch shim was audited, **not implemented**: deep-bound/direct
  GCC accessor calls can bypass dispatch; nonlocal callback exits complicate TLS
  registration lifetime. No foreign private-field guessing or source hacks apply.

Transferred proof inventory (SHA256, filenames identify retained local artifacts):

| Artifact | SHA256 |
| --- | --- |
| `unwind040-first-native-coexistence-proof.tar.gz` | `841673515218814d49825314791e57c403ca331309a59bd43ad819e36f61d283` |
| `unwind040-native-minimal-followup-proof.tar.gz` | `b9c9434aae3fcdeace10a0869075b7d0560181996151c9771a07538bd6316031` |
| `unwind040-native-personality-controls-proof.tar.gz` | `d37ee43cf662249185972ba56b96324afcaf2b87219c374ad1ef4fcdeb492fba` |
| `unwind040-zig-native-controls-proof.tar.gz` | `daab0ea02a5c2c56d359f75e67034656d9f270c2eb7128719d6048a7bc0863bf` |

Ordinary LLVM building, ELF inspection and these modern-host runs do not prove
general mixed-runtime compatibility or old-system/tier 3 qualification. The
application/default decision remains open. Shared libraries/PyO3 retain existing
behavior. No new release or binary distribution is authorized by this foundation.

## Durable source/provider/runtime regression gate

Any source, bundle, GLIBC data/provider/header, compiler/resource/tool-support,
visibility, archive recipe or application binding-policy change must revisit:

1. Verify source pin/digest/subset/license and package inventory; compile with
   selected headers/resources, record tool provenance and actual dependencies;
   compare clean-cache builds and replay without downloading sources.
2. Audit all member architecture/relocation/definitions/unresolved symbols; inspect
   exact linked maps, dynamic symbols and version-needed **owners and floors**.
   Check ordinary/explicit legacy clocks and genuine RT APIs without reordering.
3. Test required personalities, arithmetic/C-cleanup helpers and explicit GCC
   versions. Refuse unsupported requirements; do not turn missing helpers into a
   silent host fallback or count symbol spelling as semantic support.
4. Native Rust panic/drop/resume/thread/backtrace and baseline-valid originating-
   runtime cross-DSO round trips, both directions, under GNU/LLD and supported
   Rust versions; nested backtrace/threading and actual binding graphs. No foreign
   Rust-runtime catch or cancellation through Rust frames is required.
5. C-only backtrace/pthread_exit/deferred cancellation with and without
   `-fexceptions`, plus shared/PyO3 behavior and host/target/output-kind isolation.
   Include transitive/runtime dlopen, direct/handle-local and deep-bound behavior;
   absence of direct DT_NEEDED is insufficient. Preserve baseline-valid behavior.
6. Cache mutation/corruption/symlink/concurrent-writer/no-clobber/partial-generation
   gates, same-targetdir relink/config identity and packaged/registry adoption.
   Final Cargo integration, distribution clearance and actual old systems are
   separate gates. Tier 3 remains deferred, not an ordinary-release blocker.

Current focused builder gate is explicit/ignored in ordinary runs:
`cargo test --release --locked --offline --test llvm_unwind -- --ignored`.
It requires `LLVM_UNWIND_TEST_SYSROOT`, `LLVM_UNWIND_TEST_LIBRARY_ROOT`,
`LLVM_UNWIND_TEST_PROVIDER` (actual libc), `LLVM_UNWIND_TEST_CLANG`,
`LLVM_UNWIND_TEST_CLANGXX`, `LLVM_UNWIND_TEST_AR`, fresh
`LLVM_UNWIND_TEST_CACHE`, and optionally path-list `LLVM_UNWIND_TEST_TOOL_FILES`.
The test compiles x86_64 objects but does not execute target binaries; it covers
concurrent publication, independent-cache reproducibility, profile separation,
corruption and an existing empty index. Default focused parser/cache tests are
`cargo test --locked --offline --lib runtime::`.

This checklist is a maintenance/qualification hook, not a claim that a configured
CI job has run it. Hosted/native target/runtime qualification remains pending for
this new foundation. A new public API would require a future minor lockstep
release under [release policy](RELEASING.md); no version is changed here.

### Foundation validation scope

2026-11-02 local source checks: 11 focused parser/cache/input/dependency tests;
Clang 16 x86_64 construction against the selected 2.17 source-provider headers;
two concurrent writers, clean-cache byte reproducibility, both visibility
profiles, cache-hit verification, corruption refusal and empty-directory index
refusal. The final gate injects compiler environment variables into the test
process and verifies preparation clears them. An initial attempt injected those
variables into Cargo's own host link step and failed finding GCC's plugin; the
corrected gate runs the already-built test executable. It is not a builder pass.
A declared support-file symlink was also correctly refused before supplying its
resolved regular path. Inputs/support closure is declared, not certified complete.

Fresh workspace checks passed 135 tests and 6 doctests; four explicit opt-in tests
were ignored (official Rust ELF fixtures, recorded clock-link matrix, real clock
DSOs and this manual archive gate). The archive gate ran separately. Final root
checks cover subsequent validation refinements; workspace Clippy/all-target MSRV
1.90, warnings-denied docs and no-default focused tests passed. Shared target
artifacts initially lacked the new module; fresh isolated target checks replaced
that inconclusive compile attempt. These are compilation/structural/cache results,
not fresh native runtime, Windows, full provider matrix or tier 3 evidence.

The authored-file whitespace check passes. Two inherited extra EOF blank lines
in upstream `include/mach-o/compact_unwind_encoding.h` and `src/libunwind.cpp`
remain byte-identical to LLVM; the all-file diff check reports those two notices.
They are not normalized or patched to conceal source changes.

### Containment correction after c7a3d2d review

Independent review marked `c7a3d2d` **NO-GO**: a legal Unix filename literally
`..\escaped.h` became `../escaped.h` in a string-key normalization and wrote
outside the header/resource snapshot before depfile checks. The exact transferred
repro kit SHA256 is
`d59bc6e0fbd1d90f141be314e5bf300b64486baf4889cf0efd9d233f2b41e88a`.
The unchanged original probe reproduced the failure inside a fresh bounded
artifact directory; its original source and failed containment result are retained.
Earlier package/test successes did not cover or qualify that filename handling.

The correction preserves native `PathBuf` components without separator replacement,
rejects non-relative/non-normalized keys and duplicate insertion, preflights the
complete destination inventory before directory creation, checks destination
containment/directories, and creates files with create-new semantics. Existing
files, symlinks and file/directory aliases are refused without overwrite. A literal
Unix backslash remains a literal filename character, not a path separator. Native
key encoding changes the input/cache identity; old producer entries are not repaired.

Focused regression coverage exercises both header and resource staging, Unix
backslash/leading-backslash/drive-looking names, actual nested paths, forged parent/
absolute/noncanonical components, file/directory collisions, preexisting files and
destination symlinks, with unchanged original source bytes and no escaping writes.
Windows prefix/root/UNC cases are included conditionally; native Windows execution
of this new preparation API remains unqualified. The corrected standalone probe
changes only the expected outcome, compiling the current module with Rust 1.90.
These checks address the reproduced defect, not general runtime interoperability.

The independent reviewer verified the c7 source/license inventory and transferred
logs. The c7 package checksum/file count remained owner evidence because its crate
bytes were not in that packet. A corrected exact-ref source archive is included
with the new handoff so independent package hashing can close that separate gap.
No receipt validation, lifecycle invalidation, full-sysroot staging, contained-link
support, application binding policy or default is implemented by this correction.

Correction validation, 2026-11-02: 15 focused Rust 1.90 runtime-module tests pass,
including six input/staging tests; root all-target Clippy and fmt pass. The selected
Clang 16 archive/cache gate passes after native-path changes (both construction
profiles, concurrent writers and replay); no full Rust/native/runtime matrix was
rerun. Narrow independent review remains required before preparation integration.
