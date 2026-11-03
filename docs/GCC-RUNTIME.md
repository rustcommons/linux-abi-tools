# GCC runtime versions and deployment

Current reference, checked 2026-11-03; the latest checked upstream release is
[GCC 16.2 (2026-08-07)](https://gcc.gnu.org/gcc-16/). A normal cargo-libc build uses the selected
provider's recorded interfaces at link time. When the executable starts, the OS
loader selects real glibc and `libgcc_s.so.1` libraries using that environment's
loader rules and search paths. Provider preparation does not install an OS
runtime. Build, inspect the output, then exercise the intended deployment
libraries; a selected glibc floor is only one part of that workflow.

## Three different version identities

| Identity | Meaning | What it does not establish |
| --- | --- | --- |
| `libgcc_s.so.1` | The shared library's SONAME, retained across many GCC releases | `.1` is not GCC 1, a package release, or a complete list of available interfaces. |
| GCC release / distro package version | The compiler source release or the distributor's runtime build, such as GCC 4.8.5 or SUSE `13.3.0+git8781-1.16.1` | Neither a glibc floor nor the SONAME identifies the installed package. |
| `GCC_3.0`, `GCC_4.2.0`, etc. | ELF symbol-version namespaces; references bind a symbol name to a required namespace | A maximum node is not the installed compiler version and is not sufficient to establish all required exports or behavior. |

The [GCC 16.2 binary-compatibility manual](https://gcc.gnu.org/onlinedocs/gcc-16.2.0/gcc/Compatibility.html)
distinguishes ABI conformance, interoperability and behavior. The pinned
[GCC 16.2 libgcc manual source](https://github.com/gcc-mirror/gcc/blob/releases/gcc-16.2.0/gcc/doc/libgcc.texi)
describes arithmetic, exception-handling and other helpers; libgcc is not only
an unwinder. This reference adds no support for user-target C++ builds.

## Upstream namespace history and x86 evidence

The table identifies namespaces in **exact upstream release-tag references**,
not a minimum OS/package matrix. Older nodes generally remain in later maps;
rows highlight nodes of interest rather than repeat every inherited node.
A node's spelling is not proof of the exact release that first shipped it.
“No new namespace” compares only the named tags' generic and x86 map node sets,
not all symbols, point releases or ABI behavior. The 3.0–3.2 row compares only
the generic map, as stated there.

The generic map is `gcc/libgcc-std.ver` in older tags and
`libgcc/libgcc-std.ver.in` from GCC 4.7. Target maps add, exclude and reassign
symbols. For example, the [16.2 x86 GNU/Linux build rules](https://github.com/gcc-mirror/gcc/blob/releases/gcc-16.2.0/libgcc/config/i386/t-linux)
select the architecture map; [config.host](https://github.com/gcc-mirror/gcc/blob/releases/gcc-16.2.0/libgcc/config.host)
selects target build fragments, and the [map generator](https://github.com/gcc-mirror/gcc/blob/releases/gcc-16.2.0/libgcc/mkmap-symver.awk)
filters against the symbols in the actual build. **Source-map presence is not
an observed export from a built x86_64 runtime.** No runtime was built for this
documentation audit. “Source only” below explicitly leaves that measurement open.

| GCC release-tag reference (generic map) | Upstream namespace/reference observation | x86 GNU/Linux source distinction / actual export evidence |
| --- | --- | --- |
| [3.0.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.0.0/gcc/libgcc-std.ver), [3.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.1.0/gcc/libgcc-std.ver), [3.2.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.2.0/gcc/libgcc-std.ver) | `GCC_3.0`; no additional generic node in these three tags | No x86 export measurement; 3.0 is generic history, not x86_64 support evidence. |
| [3.3.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.0/gcc/libgcc-std.ver) | `GCC_3.3` adds `_Unwind_Backtrace` and other unwind interfaces | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.0/gcc/config/i386/libgcc-x86_64-glibc.ver). |
| [3.3.1](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.1/gcc/libgcc-std.ver) | `GCC_3.3.1` includes `__gcc_personality_v0` | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.1/gcc/config/i386/libgcc-x86_64-glibc.ver). |
| [3.3.2](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.2/gcc/libgcc-std.ver) | `GCC_3.3.2` is an empty generic node | An empty map node is not evidence of a built export. |
| [3.3.4](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.4/gcc/libgcc-std.ver), [3.3.5](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.5/gcc/libgcc-std.ver), [3.3.6](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.3.6/gcc/libgcc-std.ver) | `GCC_3.3.4` absent in the 3.3.4 generic map, present in 3.3.5/3.3.6 | Source reference, not a claim that node names exactly date releases. |
| [3.4.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.4.0/gcc/libgcc-std.ver) | `GCC_3.4` | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.4.0/gcc/config/i386/libgcc-x86_64-glibc.ver). |
| [3.4.2](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.4.2/gcc/libgcc-std.ver) | `GCC_3.4.2`; also contains `GCC_3.3.4` | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.4.2/gcc/config/i386/libgcc-x86_64-glibc.ver). |
| [3.4.4](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.4.4/gcc/libgcc-std.ver) | `GCC_3.4.4` | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-3.4.4/gcc/config/i386/libgcc-x86_64-glibc.ver). |
| [4.0.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.0.0/gcc/libgcc-std.ver) | `GCC_4.0.0` | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.0.0/gcc/config/i386/libgcc-x86_64-glibc.ver). |
| [4.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.1.0/gcc/libgcc-std.ver) | `GCC_4.1.0` is an empty generic node | Not in the recorded x86_64 GCC 4.8.5 export set. |
| [4.2.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.2.0/gcc/libgcc-std.ver) | `GCC_4.2.0` includes `_Unwind_GetIPInfo` | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.2.0/gcc/config/i386/libgcc-x86_64-glibc.ver). |
| [4.3.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.3.0/gcc/libgcc-std.ver) | `GCC_4.3.0` | The [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.3.0/gcc/config/i386/libgcc-x86_64-glibc.ver) also assigns binary128 helpers to `GCC_4.3.0` on x86_64. |
| [4.4.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.4.0/gcc/libgcc-std.ver) | `GCC_4.4.0` | The [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.4.0/gcc/config/i386/libgcc-glibc.ver) distinguishes 32-bit `GCC_4.4.0` from x86_64 `GCC_4.3.0` binary128 helpers. |
| [4.5.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.5.0/gcc/libgcc-std.ver) | `GCC_4.5.0` | The [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.5.0/gcc/config/i386/libgcc-glibc.ver) assigns `__extendxftf2` to this node on the 32-bit branch; not an x86_64 export claim. |
| [4.6.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.6.0/gcc/libgcc-std.ver) | `GCC_4.6.0` | Source only; node absent from recorded x86_64 GCC 4.8.5 exports. |
| [4.7.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.7.0/libgcc/libgcc-std.ver.in) | `GCC_4.7.0` includes split-stack helpers | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.7.0/libgcc/config/i386/libgcc-glibc.ver). |
| [4.8.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.8.0/libgcc/libgcc-std.ver.in), [4.8.5](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.8.5/libgcc/libgcc-std.ver.in) | `GCC_4.8.0` is empty in the generic map | The [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.8.5/libgcc/config/i386/libgcc-glibc.ver) supplies CPU-detection exports. Actual recorded GCC 4.8.5 x86_64 interface: 138 definitions, 11 nodes through `GCC_4.8.0`; see below. |
| [4.9.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-4.9.0/libgcc/libgcc-std.ver.in), [5.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-5.1.0/libgcc/libgcc-std.ver.in), [6.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-6.1.0/libgcc/libgcc-std.ver.in) | No new namespace in the inspected generic and x86 map node sets versus 4.8.5 | Source only; no claim that symbols, implementations or every point release are unchanged. |
| [7.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-7.1.0/libgcc/libgcc-std.ver.in) | `GCC_7.0.0` includes division helpers | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-7.1.0/libgcc/config/i386/libgcc-glibc.ver). |
| [8.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-8.1.0/libgcc/libgcc-std.ver.in), [9.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-9.1.0/libgcc/libgcc-std.ver.in), [10.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-10.1.0/libgcc/libgcc-std.ver.in), [11.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-11.1.0/libgcc/libgcc-std.ver.in) | No new namespace in the inspected generic and x86 map node sets versus 7.1.0 | Source only; no complete-series ABI or runtime qualification. |
| [12.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-12.1.0/libgcc/libgcc-std.ver.in) | No new generic node; generic history alone misses `GCC_12.0.0` | The [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-12.1.0/libgcc/config/i386/libgcc-glibc.ver) adds half-precision helpers under `GCC_12.0.0`; source only. |
| [13.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-13.1.0/libgcc/libgcc-std.ver.in) | No new generic node; generic history alone misses `GCC_13.0.0` | The [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-13.1.0/libgcc/config/i386/libgcc-glibc.ver) adds bfloat16 helpers under `GCC_13.0.0`; source only. |
| [14.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-14.1.0/libgcc/libgcc-std.ver.in) | `GCC_14.0.0` | The [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-14.1.0/libgcc/config/i386/libgcc-glibc.ver) also assigns bit-precision helpers to `GCC_14.0.0`; source only. |
| [15.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-15.1.0/libgcc/libgcc-std.ver.in), [16.1.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-16.1.0/libgcc/libgcc-std.ver.in), [16.2.0](https://github.com/gcc-mirror/gcc/blob/releases/gcc-16.2.0/libgcc/libgcc-std.ver.in) | No new namespace in the inspected generic and x86 map node sets versus 14.1.0 | Source only; [x86 map](https://github.com/gcc-mirror/gcc/blob/releases/gcc-16.2.0/libgcc/config/i386/libgcc-glibc.ver) still contains `GCC_12.0.0`, `GCC_13.0.0`, `GCC_14.0.0`. No GCC 16 binary export measurement here. |

The only complete x86_64 runtime interface record used by this producer is the
[pinned CentOS 7 GCC 4.8.5 record](../src/libgcc_s-gcc-4.8.5.tsv), measured by
[record-libgcc-s.sh](../scripts/record-libgcc-s.sh). Its 138 definitions use
`GCC_3.0`, `GCC_3.3`, `GCC_3.3.1`, `GCC_3.4`, `GCC_3.4.2`, `GCC_3.4.4`,
`GCC_4.0.0`, `GCC_4.2.0`, `GCC_4.3.0`, `GCC_4.7.0`, `GCC_4.8.0`.
Generic nodes such as `GCC_3.3.2`, `GCC_4.1.0`, `GCC_4.4.0`, `GCC_4.5.0`
and `GCC_4.6.0` must not be inferred to be exports of that binary.
The producer renders that recorded link interface for all its glibc releases;
see the [maintainer reference](PROVIDER-DELIVERY.md#the-libgcc_s-stub).

## Which libgcc does a GCC 16.2 build host select?

The host compiler version does not by itself select the application's libgcc.
Provider source compilation and final application linking are separate steps:

| Build path | Link-time selection | What remains to check |
| --- | --- | --- |
| Automatically managed source/container provider | The producer generates the recorded GCC 4.8.5 link interface. Cargo's ordinary provider link places the descriptor's `compiler-runtime` search directory before forwarded library arguments. | A newer host GCC used to compile provider inputs does not replace this with its own shared runtime. Inspect actual application imports and any explicit library overrides. |
| Explicit provider descriptor | Cargo uses the descriptor's supplied compiler-runtime directory; v1 does not promise that arbitrary supplied files are the producer-generated interface. | Validate the supplied files and resulting requirements; do not assume the managed GCC 4.8.5 record. |
| Full sysroot with selected GCC installation | Cargo's Clang driver uses the selected `--sysroot`, `--gcc-install-dir` and `--rtlib=libgcc`; no managed libgcc stub is added by this path. | Inspect the real selected sysroot library and final ELF; its GCC version can differ from the build host's. |
| Native/system or explicitly named libraries | Ordinary toolchain search and explicit link arguments determine inputs; a full library path can select a different input from `-lgcc_s`. | Inspect actual link inputs and output rather than assume either a producer stub or the host GCC 16 library was used. |

This selection inspection is pinned to [cargo-libc toolchain source](https://github.com/rustcommons/cargo-libc/blob/bcd20d5634d201ccd18590c0b1e29cba305f69b2/src/toolchain.rs#L1014-L1048),
including its [driver selection](https://github.com/rustcommons/cargo-libc/blob/bcd20d5634d201ccd18590c0b1e29cba305f69b2/src/toolchain.rs#L163-L171)
and [provider/sysroot branch](https://github.com/rustcommons/cargo-libc/blob/bcd20d5634d201ccd18590c0b1e29cba305f69b2/src/toolchain.rs#L1086-L1101).
It is not a guarantee about arbitrary linker scripts, response files or overrides.
An object requiring a helper absent from the selected link interface needs a
compatible input; a newer compiler version does not make that symbol available
in the recorded interface.

**Observed tool compatibility, 2026-11-02:** [glibc-provider replay run
41454181443, GCC 16 job](https://github.com/rustcommons/glibc-provider/actions/runs/41454181443/job/125306745150)
at `545013fc837076395439eb665113c8ded2f8db92` records `gcc (GCC) 16.2.0`
and GNU ld 2.44. Its [replay script](https://github.com/rustcommons/glibc-provider/blob/545013fc837076395439eb665113c8ded2f8db92/qualification/replay-all.sh)
compares compiled provider objects with recorded interfaces for 2.17–2.44;
all rows have zero `differs` and zero `build-failed`, retaining the report's
`identical-bytes`, `equivalent` and `accepted` distinctions. This supports that
specific provider-input compiler/tool combination. It does not measure GCC
16.2's complete libgcc exports, link an arbitrary application with that runtime,
or qualify deployment on an old OS. This documentation audit has no local GCC
16.2 executable and adds no fresh GCC 16.2 application-link execution evidence.

## Ordinary inspection and deployment workflow

1. Build with the selected provider or explicit sysroot using the maintained
   [cargo-libc user guide](https://github.com/rustcommons/cargo-libc/blob/main/docs/USER-GUIDE.md).
2. Inspect the executable's dynamic dependencies, version requirements and
   symbol references, then the actual runtime selected in the deployment
   environment. The [readelf commands](WORKFLOWS.md#3-inspect-a-built-binary-tier-1)
   are static and do not execute the application. `libc-abi glibc elf --input FILE`
   reports defined dynamic symbols, not application GCC requirement validation.
3. Match every required symbol/version pair to a definition, with compatible
   architecture, loader and complete dependency closure. Check the runtime's own
   dependencies and libraries loaded transitively or with `dlopen`; these are not
   established by a direct `DT_NEEDED` list or a maximum version node.
4. Exercise relevant behavior against named target-environment libraries and
   record the tools, versions and limits. Metadata availability is necessary,
   not a proof that unwinding or the whole application behaves correctly.
   [Tier 3](ROADMAP.md#deferred-qualification) remains deferred, not an ordinary
   release blocker; new actual-old-system claims need actual target evidence.

`linux-abi-scan` checks `GLIBC_*` floors and its documented kernel findings.
It excludes `GCC_*`, `GLIBCXX_*` and other libraries' version namespaces. No
GCC-floor option exists. `--strict` makes warnings affect status; `--verbose`
shows informational findings. Neither adds compiler-runtime validation.
`libc-abi provider verify` verifies provider inputs, not installed OS runtimes.
LLVM replacement is a separate unresolved interoperability question; none of
these checks authorizes dependency removal or a new runtime default.

## Saved Aero imports: bounded evidence

The [saved three-way run 41378111334](https://github.com/rustcommons/cargo-libc/actions/runs/41378111334)
used Cargo source `29582bc7af35cba1f741f989b47bc520979e13ae` and Aero source
`9c248300f1e2ab20ce34dc4b227eb322e33e4c93`. Reinspection on 2026-11-03 of the
old cargo-libc 0.2.4 and new 0.3.0 executables found the same 12 undefined GCC
imports, all owned by `libgcc_s.so.1`:

| Required namespace | Required symbols |
| --- | --- |
| `GCC_3.0` | `_Unwind_DeleteException`, `_Unwind_GetDataRelBase`, `_Unwind_GetIP`, `_Unwind_GetLanguageSpecificData`, `_Unwind_GetRegionStart`, `_Unwind_GetTextRelBase`, `_Unwind_RaiseException`, `_Unwind_Resume`, `_Unwind_SetGR`, `_Unwind_SetIP` |
| `GCC_3.3` | `_Unwind_Backtrace` |
| `GCC_4.2.0` | `_Unwind_GetIPInfo` |

Binary SHA-256 identities:

```text
old 2884c480365ffe8f3afd99c522602e98821c7415b3fed1689670323134f4d544
new 9843ac75bad2e74d0f5417767256f6c16269e435cc414292ce677d28caca2c6c
```

Both require at most `GLIBC_2.18`. The existing local scanner returned 0 for
`--glibc 2.22 --verbose` and 1 for `--glibc 2.17 --verbose`, identifying the
2.18 requirement; kernel checks were explicitly skipped. GCC imports were not
validated by those scans. This is saved-file inspection, not fresh Aero runtime,
old-SLES, helper-completeness or unwind-boundary qualification. The saved Zig
comparison executable has no direct GCC imports; that alone does not prove the
absence of transitive/lazy GCC use or mixed-runtime compatibility.

## Dated package example, not a distro mapping

SUSE's Google Cloud **SLES 12 SP5 image `v20250103-x86-64`** records
`glibc 2.22-114.34.1` and `libgcc_s1 13.3.0+git8781-1.16.1` in its
[official CycloneDX inventory](https://publiccloudimagechangeinfo.suse.com/google/sles-12-sp5-v20250103-x86-64/cdx.json);
its [package changes](https://publiccloudimagechangeinfo.suse.com/google/sles-12-sp5-v20250103-x86-64/package_version_changes.html)
also list the runtime update. This illustrates independent package evolution,
not a claim about every SLES installation, a measured export set from that
image or universal compatibility with that package.

Trace: [LAT-REQ-009 and LAT-REQ-020](REQUIREMENTS.md#requirements). This maintained
reference clarifies current scope; historical qualification and immutable
published package/API documentation remain as released.
