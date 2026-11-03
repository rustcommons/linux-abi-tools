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

## 2026-11-03 pinned Zig comparison and known mixed-context blocker

Scoped preparation source/API/package review approved **`558f9cf6ba017c9b926d6c1390494ad0c1c8e525`**.
That closes the preparation containment/package review at that source, not runtime,
full-sysroot/lazy integration, default, merge or release gates. The current system
runtime remains unchanged. The following comparison records mechanisms, not a
claim that a construction profile is a safe application policy.

The exact retained [three-way run 41378111334](https://github.com/rustcommons/cargo-libc/actions/runs/41378111334)
uses Cargo `29582bc7af35cba1f741f989b47bc520979e13ae` and Aero
`9c248300f1e2ab20ce34dc4b227eb322e33e4c93`, Rust 1.99.1,
Zig 0.17.0 and cargo-zigbuild 0.23.4. Its Zig executable SHA256 is
`d2e3767e7b1c94d173eb9788a096dcd2bb4414adbcd912b0e6564224b82ce4da`;
ELF `.comment` identifies Clang/LLD 22.1.8. It is byte-identical to the earlier
[Aero CI run 41340622271](https://github.com/joey-huckabee/aero1553/actions/runs/41340622271)
artifact, whose install log names the PyPI x86_64 0.17.0 wheel. The three-way
behavioral comparison matched 182/182 invocations, **but the unchanged strict
comparator failed**: Cargo retained direct libgcc_s and twelve versioned unwind
imports; Zig had neither and had additional GLIBC imports from its static runtime.
This is clock-era application evidence, not fresh unwind/old-system qualification.

Versioned source reference: Zig 0.17.0's peeled source commit is
`7647adab80dd088f4de3610fd245915a912eb6ad`. All 31 compared source/license files
plus its personality match that commit in the preserved wheel. The named x86
wheel's RECORD declares the same 32 payload hashes; only ZIP metadata/RECORD was
range-fetched, **not** the full x86 wheel. The actual runner did not record an
installed compiler executable digest. Do not substitute current `master` or infer
the vendored libunwind upstream revision from Clang's version number.

| Aspect | Pinned Zig 0.17.0 reference | Producer 558 preparation / retained experiments | Consequence and evidence limit |
| --- | --- | --- | --- |
| Source and patches | Versioned [builder](https://codeberg.org/ziglang/zig/raw/commit/7647adab80dd088f4de3610fd245915a912eb6ad/src/libs/libunwind.zig), vendored source at that Zig commit | Unmodified LLVM 23.1.4 source commit `7a941dfb2877a0b8cf30323fefa85ad019e0b0f5`, [inventory](../src/runtime/llvm_sources/SOURCES.tsv) | 13/31 compared files are byte-identical; 18 differ. `UnwindLevel1.c`/`UnwindLevel1-gcc-ext.c` differ only in comments; the labelled AddressSpace/DwarfParser Zig patches already have the same code in LLVM 23.1.4. Other architecture/code changes remain; no whole-library equivalence claim. |
| Object/personality/helper set | Nine common units **plus** vendored `gcc_personality_v0.c`; compiler_rt is separate | Nine units only; no personality/general helper completeness. Separate pinned compiler-rt personality experiment was outside this API | The [0.17 personality](https://codeberg.org/ziglang/zig/raw/commit/7647adab80dd088f4de3610fd245915a912eb6ad/lib/libunwind/src/gcc_personality_v0.c) removes compiler-rt header dependencies and changes its abort macro to `__builtin_unreachable()`. It is not the upstream helper tested separately. Under the same Clang 16/x86/2.17 headers, their active main-personality function tokens match; preprocessing only, not helper/binary equivalence. The exact Zig crash occurs at its first context getter, before that abort path. |
| gcc_s handling | [cargo-zigbuild 0.23.4](https://github.com/rust-cross/cargo-zigbuild/blob/4258db97f6ac4bd1fc6e5c714d3f8d01db5c35e8/src/zig/linker_args.rs#L63-L64) rewrites `-lgcc_s` to `-lunwind`. Independently, pinned [Zig classification](https://codeberg.org/ziglang/zig/raw/commit/7647adab80dd088f4de3610fd245915a912eb6ad/src/target.zig) classifies direct `gcc_s` as both compiler_rt and libunwind | Preparation selects no link arguments and leaves the recorded GCC interface/provider descriptor unchanged; native diagnostic wrapper retained `-lgcc` as a bounded reference, not a production fallback | These are different layers. Neither nine-unit libunwind nor a symbol-name replacement proves arithmetic/helper or explicit GCC-version support. |
| Visibility and exports | `_LIBUNWIND_HIDE_SYMBOLS`, hidden/inlines-hidden/global-new-delete visibility; C probe has local hidden personality/getters | PrivatePicV1 hides symbols; ExportablePicV1 changes construction visibility only. Separate minimal-export diagnostic had 18 unwind definitions | Hidden LLVM fails valid cross-DSO cases; minimal exports pass only U07/U10/U11 under actual bfd 2.42 and LLD 20.1.8. Initial first-packet GNU labels were wrong (actual LLD); filenames are not linker evidence. Exported names do not override glibc handle-local GCC selection or qualify arbitrary loaders. |
| Link order/extraction | Preserved C link: object, UBSan archive, selective libunwind, glibc DSOs in m/c/ld/resolv/pthread/dl/rt/util order, nonshared and compiler_rt; no whole-archive or broad export. Exact Aero log retains the incoming rustc argument stream; the final Zig internal archive/member map was not retained | Factory does not link. C/personality experiments used whole-archive; Rust probes use same-position archive substitution and preserve other operands/state | Whole-archive is not necessary for the observed C failure: the selective Zig reference also fails. Do not reorder caller arguments, adopt broad exports or equate a reference C link line with the Aero final link map. |
| EH/table/configuration flags | Native-only, PIC, C99 `-fexceptions`, C++ no exceptions/RTTI, asynchronous unwind tables, runtime optimization derived from Zig config | Native-only PIC, C99 `-fexceptions`, C++17 no exceptions/RTTI/C++ headers, explicit `-O2 -g0 -DNDEBUG -funwind-tables`; selected header/resource snapshots | Observed Clang 16 emits asynchronous table mode 2 for the producer x86 flags. Exact Aero uses release ThinLTO, one codegen unit, stripped PIE; the retained C Zig control is debug/non-PIE with a UBSan archive. Compiler 18/22, headers (selected 2.17 vs Zig target 2.22), optimization/debug/sanitizer and final executable flags differ. No flag-for-flag/native equivalence assertion. |
| GNU versions and closure | Exact Aero Zig has no direct GCC version imports; retained C control also has no direct libgcc_s but glibc loads the real library on first native use | Archived definitions are unversioned; existing ordinary Cargo comparison retains twelve GCC unwind imports. Explicit `@GCC_*` and non-unwind helpers are separate requirements | Record symbol **versions and DSO owners**, selected members and runtime loads. A clean DT_NEEDED or maximum-GLIBC comparison is insufficient. No blanket libgcc removal or comparator relaxation. |

### Exact bounded Zig crash and what went wrong

`native-zig-pthread-exit.gdb.log` (2026-11-03), SHA256
`7f271d320834f1a0f282f4c63bf1817e12dbfb962e1982597fc68fe176551a15`,
closes the exact-stack gap for the existing C fixture/binary
`14a27cb5d6d8cf26a51e2a808d73b7b69d6bddc714868169dc7fe5d9ca6fe519`:

```text
glibc 2.39 pthread_exit
  -> real libgcc_s _Unwind_ForcedUnwind
  -> Zig-local __gcc_personality_v0
  -> Zig-local LLVM _Unwind_GetLanguageSpecificData
  -> __unw_get_proc_info
  -> SIGSEGV at a stack address
```

The source casts the opaque context to an LLVM cursor and calls its virtual
`getInfo`; the initiating engine here is GCC. This ownership mismatch explains
this bounded fault. No decoding of GCC private cursor fields or precise corrupted
instruction layout is claimed. The experiment has **no Rust worker frames**;
it is not a panic across forbidden FFI, all-Zig failure or glibc 2.22/old-kernel
execution. The original command without `-lunwind` failed linking and remains
separate; the successful explicit-unwind build is the one that later crashed.

The earlier reasoning gap was treating API names, archive visibility or a passing
local Rust catch as proof of process interoperability. C cleanup may ask glibc to
load a GCC unwinder through a real-library handle; crossed-frame personalities
may use a different implementation's getters/resume. A complete personality name
set fixes a link requirement, not this context contract. The pinned compiler-rt
helper trial and this *different* Zig personality both fail their named controls;
that is evidence of a shared mechanism in these cases, not proof every coherent
LLVM strategy is impossible or permission to exclude baseline-supported calls.

### Recorded hidden Zig archive-swap control

The previously proposed U07/U10/U11 control was subsequently executed and
independently reviewed on 2026-11-03. It reuses the same Rust 1.90 host/bridge
fixtures, GCC 13 dynamic-GCC DSO and local-loader scope, substituting the preserved
hidden Zig 0.17.0 archive under actual forced GNU bfd. The archive SHA256 is
`d438a1ba257b6562f37715313798d3ca74402d49950cc7fddab0a1ff43d2838c`.

The GCC baseline passes U07/U10/U11. The hidden Zig host passes U07 but
**U10 and U11 both SIGSEGV (exit -11)**. The harness exits **1**, with those two
compatibility oracles still failed. Independent review verified 54 hashes,
archive/fixture identities and actual bfd selection. The retained packet is
`zig017-hidden-roundtrip-native-proof.tar.gz`, SHA256
`f07367880377bea3154b9fcdee9a4a766fc1a8fed37f38823fce0131d2bee5d8`.
Maps, symbols and loader records preserve private executable LLVM definitions
alongside DSO getters/resume bound to real GCC.

This records the predicted bounded failure class for this supplied archive-swap
fixture. It is **not** a full cargo-zigbuild driver, Aero, old-OS or all-Zig result.
The earlier LLVM 23.1.4 hidden U10 aborted; this Zig U10 segfaults, so identical
fault sites or private context-field/instruction decoding are not established.
The exact Zig C pthread_exit GDB stack above remains a separate named case.
No failed oracle is relabelled expected-green, and no context bridge, export
workaround, runtime/default policy or release approval follows.

## Producer corner cases and release gates

Use this producer inventory with the consumer-owned [canonical U01–U22 release
checklist](https://github.com/rustcommons/cargo-libc/blob/d716c9c0e1e19057a3533e5d11982fbf7926a0f1/docs/UNWINDER-RELEASE-CHECKLIST.md#permanent-acceptance-cases-and-current-automation).
Output selection, receipts, R/I/A/E, freshness and execution guarding belong there;
this table records producer mechanisms and the evidence needed at that boundary.
U01–U22 retain their existing verification meanings; the LAT IDs below are existing
trace entries, not new contracts. A planned gate is not performed evidence.

| Producer case / trace | Mechanism and trigger | Expected oracle | Actual bounded evidence | Remaining gate / disposition |
| --- | --- | --- | --- | --- |
| Opaque contexts/visibility — LAT-REQ-031; U07/U10–12 | Static hidden/exported runtime, a crossed cleanup-bearing frame, or an embedding executable changes personality/getter ownership | Baseline-valid originating-runtime round trips retain payload and exact once-only drops; inspect actual bindings | First hidden LLVM reference: U07 pass, U10 abort, U11 SIGSEGV. Corrected minimal exports: those three cases pass on actual bfd/LLD only. Independently reviewed hidden Zig archive swap: U07 pass, U10/U11 SIGSEGV, harness exit 1; same fixture/actual bfd, not full Zig driver | **Application blocker** remains: C forced-unwind failure and bypass paths. Construction visibility is not process compatibility. |
| Native C cleanup/forced unwind — LAT-REQ-009/031; U13/U14 | Same C-only worker with/without `-fexceptions`, libc/libgcc/runtime/helper update | GCC baseline, exactly one cleanup, correct exit/join/cancel value; deferred cancellation only; no Rust frames | Plain controls 9/9 pass. C EH and pinned-helper packets retain exit/cancel SIGSEGVs; exact Zig GDB path above closes that one stack gap on host glibc 2.39 | **Known blocker**, not an expected-success negative test. Requalify actual engine/personality/getter/resume owners and lazy loads; Rust catches or missing direct NEEDED cannot waive it. |
| DSO/Python/plugins — LAT-REQ-010/031; U07–12/U19 | Multiple DSOs, Python embedding, load order/local/global/deep binding or direct handle lookup | Existing internal catches/error translation and baseline-valid C-unwind boundaries remain usable; measure the same loader scope/order | One Rust DSO mechanism tested; glibc direct-GCC lookup observed. Full PyO3/multiple-DSO/deepbind preservation is unqualified | **Integration/application gate**. An unchanged extension can still run under altered host bindings. Unsupported user-target C++ does not eliminate foreign C++ DSOs with valid C APIs/internal catches; do not require a foreign Rust panic catch. |
| Panic/rethrow/backtrace/lifetime — LAT-REQ-031; U01–05/U10–12 | Rust/std, personality/source/config, nested cleanup, thread or first-use backtrace changes | Exact drop order, payload identity, continued execution; bounded uncaught/double-panic statuses and meaningful first/repeated/threaded traces | ARM/system fixture sanity and named native round trips only. Zig backtrace passes despite exit/cancel failure. Rust 1.90 source can free a caught exception without Delete | **Broader candidate gate open**. No leaking owner map or exception-class/foreign-private-field ownership guess is approved; trace first lazy loads rather than final maps alone. |
| Personalities/helpers/version owners — LAT-REQ-007/009/031; U14–16/U21 | Changed source/object set, future compiler helper, arithmetic/C cleanup or explicit GCC-version requirement | Required/provided symbols **and** actual behavior, version/DSO owners and selected members; refuse unsupported requirements rather than host fallback | Factory has nine units, no compiler-rt personality/general helper completeness. Pinned helper selected but C cases still fail; Zig patched helper is distinct. Exact Aero old/new have twelve GCC unwind imports; Zig has none | **Link/application gate open** for arithmetic and explicit `@GCC_*` semantics. GetDataRelBase/GetTextRelBase non-AIX abort paths and future helper additions require behavior checks; inventories are not closure certificates. |
| ELF architecture/PIC/SONAME — LAT-REQ-007/010/031; U17/U18 | Wrong/truncated/thin archive, foreign object, relocation/configuration or selected libc change | Bounded refusal before use; exact nine ET_REL x86_64 members, chosen visibility; selected libc ET_DYN/SONAME `libc.so.6`; final linked no forbidden text relocations | Four archive parser tests plus preparation gate inspect framing/symbol indices/ranges/allowed RELA forms; LLVM archive built as PIC; source/API/package GO at 558 | **Structural pass only**. Final GNU/LLD linked PIC, complete ABI semantics, runtime target and unsupported architectures are not established by flags/headers. |
| ABI/library migration/clocks — LAT-REQ-004/007/029/031; U15/U16 | New glibc tables/provider/LLVM imports, explicit old clock version or genuine RT API; script/group/as-needed state | Preserve five libc defaults plus explicit legacy versions and real RT needs, exact library identity/caller order | Existing published clock qualification is retained; no new clock matrix rerun for these docs. ELF versions/default bindings are separate from unwind experiments | **Affected-update regression gate**. Compare old 2.17/2.22 inputs and the added release; no reordering or silent loss of explicit versions. |
| Header/provider identity versus runtime floor — LAT-REQ-010/011/027/031; U17/U21 | Different selected headers/libc/provider/compiler resource or sysroot profile | Correct independent containment/selection and actual ELF version/owner closure, followed by named runtime evidence | 558 Clang 16 construction uses selected 2.17 headers on an ARM host, with no x86 execution. Stub/SONAME/content checks do not measure library/header consistency or real GLIBC floor | **Route/runtime gate open**. Link-only stubs must never execute; target 2.22 in a binary is not execution on 2.22. Tier 3 stays deferred, not an ordinary-release blocker. |
| Real glibc behavior outside recorded bundles — LAT-REQ-003/004/031; U13/U17/U21 | Installed libc/libgcc/vendor update, lazy-loading behavior or backport changes while collected symbol/header inputs remain unchanged | Recheck the affected real runtime's forced unwind, backtrace and bindings against a valid baseline | Host glibc 2.39 direct-handle GCC path is demonstrated; ABI lists do not encode that behavioral contract | **Runtime-update gate** independent of producer content keys. No target-kernel/userland or vendor-backport claim from a source/data hash; unchanged bundles cannot certify a changed runtime. |
| Tools/resources/support closure — LAT-REQ-002/012/031; U17/U18 | Compiler/resource/config/plugin/shared support or archiver changes, including same-path/mtime bytes | Recorded executable/resource/support identity and observed `-MD` files agree; no ambient host supplements, deterministic reproducible construction | Clang 16 gate, environment scrubbing and exact observed dependency checks pass. Closure is caller-declared; loaded libraries/subprocess/plugin/config effects are not completely discovered | **Explicit observation limit**, not hermeticity/authentication. Source/binary/compiler versions and unverified signatures remain distinct; offline sources still need available selected tools/headers. |
| Native paths/aliases/multiarch/full sysroots — LAT-REQ-010/031; U17/U18 | Literal Unix backslashes, traversal, collisions, link aliases, foreign path syntax or ordered include roots | No escaping/overwriting write before compilation; preserve native names and immutable source. Legitimate contained selection must not become host fallback | c7 deterministic escape reproduced; 558 correction has six independent staging tests/original-case regression and actual package verification | **Containment scope pass; route expansion pending**. Current materialized flat header contract refuses symlinks. Contained aliases and ordered multiarch/full-sysroot staging need separate review; system behavior is unchanged. |
| Cache/concurrency/input mutation — LAT-REQ-013/031; U18/U20 | Partial generation, changed bytes/mtime, corrupt index/manifest/archive or competing writers | Only complete validated generations publish; existing entries never overwritten/repaired; mutations fail without source changes | 558 preparation/cache gate covers concurrent writers, independent-cache reproducibility, replay, corruption and empty-index refusal; scoped source/API/package review passed | **Preparation scope only**. Read-only recorded-input validation and consumer freshness remain design, not implemented gates. Consumer invalidation may write only its own state, never repair/delete producer caches. |
| Output-kind/host isolation — LAT-REQ-031; U06/U09/U17/U19/U20 | Mixed executable/shared/Python/host/intermediate/system builds or stale configuration | Shared/Python/host/system/partial outputs retain existing runtime and **no LLVM preparation/tools prerequisites**; actual completed output determines any future executable policy | Producer factory is explicit and unwired; completed-stream classifier evidence is consumer-owned, not proof of runtime/default integration | **Consumer integration gate open**. No eager tool probes or global LLVM search flags; managed/plain-Cargo/native freshness must be demonstrated by the consumer, not inferred from an archive. |
| Static runtime updates/security/licensing — LAT-REQ-025/026/028/031; U17–22 | LLVM/helper security fix or any source/header/provider/tool/profile patch; new data release or runtime/image update | Review new pin/patch/license and affected closure, rebuild archive, **relink and redeploy affected executables**, trace source/archive/output identities; preserve immutable existing cache keys/releases | 31 source/license files and 558 source package verified. No new runtime/security release, binary redistribution clearance or automated lifecycle is claimed | **Future release/update obligation**. Updating a shared runtime alone does not replace embedded LLVM code. Maintain affected-consumer inventory and repeat applicable old/new floor, C/DSO/Python/helper/clock gates; never silently replace cache entries. Prebuilt legal gate remains; no current vulnerability claim is made. |

This inventory does not select an explicit-only mode or a transparent LLVM default,
narrow baseline-supported boundaries, or declare every possible coherent strategy
impossible. Application failures stay visible while source preparation, lifecycle
contracts, application compatibility and release authorization remain separate.


## Durable source/provider/runtime regression gate

Use the [producer corner-case inventory](#producer-corner-cases-and-release-gates)
for triggers, actual evidence, oracles and open gates; the consumer owns its ledger.
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

2026-11-03 resolution of that narrow preparation review: independent scoped
source/API and actual-package GO was received for `558f9cf`; see the
[current comparison and blocker record](#2026-11-03-pinned-zig-comparison-and-known-mixed-context-blocker).
The preceding 2026-11-02 validation paragraph retains its then-pending review
state. No merge, application/default, freshness/full-sysroot integration or
release GO follows from the scoped preparation approval.
