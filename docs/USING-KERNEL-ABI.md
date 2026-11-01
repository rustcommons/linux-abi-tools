# Using kernel ABI tools

Install `cargo install kernel-abi-tools --locked` with Rust/Cargo 1.90+.
The package installs `kernel-seccomp` and `syscall-probe`; the library APIs are
[documented on docs.rs](https://docs.rs/kernel-abi-tools).

```sh
kernel-seccomp distros
kernel-seccomp missing --distro sles-12-sp5
kernel-seccomp profile --kernel 4.12 --output linux-4.12.json
kernel-seccomp generate profiles
kernel-seccomp run --kernel 4.12 -- ./my-app
kernel-seccomp run --distro sles-12-sp5 -- ./my-app
```

`generate` creates reference OCI JSON profiles from the compiled-in data. Local
runners generate their BPF filter directly. Neither changes the host kernel.

### Choosing a runner

| | `--runner local` | `--runner container` |
| --- | --- | --- |
| Userland | The host's | The image's |
| Needs | Linux x86_64 | podman or docker (crun with libseccomp 2.6+ for every profile) |
| Root | No | No with rootless podman |
| Default when | No image is known (`--kernel` alone) | `--image` is given or `--distro` supplies one |

The local runner sets `no_new_privs` and installs a classic-BPF seccomp
filter built from the same data as the profiles, then executes the program.
The filter is inherited by every child and cannot be removed; setuid programs
inside it cannot gain privileges. It does not change the userland, so use the
container to run against the target's real glibc.

The container runner mounts the working directory, and the program file if it is outside it,
at their own absolute paths, and forwards `CARGO*`/`RUST*` variables by name
on both Docker and Podman. Stdin stays attached, including redirected input;
no TTY is allocated. A bare
program name (`sh`) runs from the image. Engine and runtime options come from
`CONTAINER_ENGINE` (default `podman`), `KERNEL_ABI_ENGINE_ARGS` (e.g.
`--runtime crun`) and `KERNEL_ABI_RUN_ARGS` (e.g. `--network none`).

Environment forwarding is a trust boundary: all existing `CARGO*`/`RUST*`
variables reach the selected engine and container, including credentials such
as `CARGO_REGISTRY_TOKEN` and named-registry tokens. Forwarding by name keeps
values out of the runner's arguments and logs; it does not hide them from the
container. Use trusted images/programs and unset credentials not needed by the
run. This preserves the existing Podman forwarding contract; no implicit token
exclusion or new allowlist is introduced.

Mounts use `:z` (and `:ro,z` for an external program file). On SELinux hosts
this can recursively relabel the mounted working tree; use a dedicated project
directory, not a broad directory such as your home. See the
[engine's bind-mount guidance](https://docs.docker.com/engine/storage/bind-mounts/).
The runner does not select a container user or repair file ownership. Rootful
engines with a root image user can create root-owned files in the mounted tree;
select an appropriate user through `KERNEL_ABI_RUN_ARGS` when needed. Mount and
user defaults are unchanged; SELinux/rootful ownership tests remain pending.

### As a Cargo runner

Cargo runs every `cargo run` / `cargo test` binary through the runner, so the
whole test suite executes in the simulated environment. Build scripts and
procedural macros are not affected:

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
# Kernel only, host userland:
runner = ["kernel-seccomp", "run", "--kernel", "4.12", "--"]
# Or the real SLES 12 SP5 userland:
# runner = ["kernel-seccomp", "run", "--distro", "sles-12-sp5", "--"]
```

For the container, the binary must be built for a glibc no newer than the
image's (use cargo-libc), and test data must live under the package directory,
which is the working directory Cargo uses for tests.

## How a profile works

A profile is an OCI seccomp profile (Podman, Docker, CRI-O). The local
filter (`filter::program`) follows the same data for native x86_64 calls, and its tests
check it against the data for every syscall number and advice value of every
generic kernel and preset. The local filter rejects i386 and x32 calls with
`ENOSYS`; the OCI profile selects x86_64, leaving foreign-ABI rejection to the
runtime. Matching i386/x32 errno/status behavior is not qualified:

- **Default action `ENOSYS`; allowlist of the syscalls the target kernel has.**
  It is an allowlist because runtimes resolve names with their own libseccomp
  and silently drop names it does not know. A blocklist would fail open for the
  newest syscalls, which is exactly what we observed during development (13
  syscalls from 6.8 onward reached the kernel). With the allowlist an unknown
  name fails closed and the probe reports it.
- **One `madvise` rule per advice value** (allowed or `EINVAL`), plus one for
  values above the highest known. No two rules overlap, so the result never
  depends on how a runtime orders conflicting rules.

Profiles are a testing aid, not a security boundary.

## Runtime requirements

| Runtime | Works for | Why |
| --- | --- | --- |
| **crun with libseccomp ≥ 2.6** (e.g. the static build from `crates/kernel-abi-tools/scripts/install-crun.sh`) | All profiles | Applies the filter immediately before `execve`, and names every syscall |
| crun or runc with libseccomp 2.5.x | Kernels 4.11 to 6.7 | libseccomp 2.5 cannot name 6.8+ syscalls; runc's init also calls `statx` (4.11) after installing the filter, so runc cannot start older profiles |

Select a runtime with `KERNEL_ABI_ENGINE_ARGS="--runtime crun"` or
`--runtime /path/to/crun`.

## Verification

- **Focused CI on matching source/manifest changes or manual dispatch:** every generic and preset profile tested with the
  local filter on the host, then the generic kernels and three presets in
  SLES 12 SP5 userland under static crun 1.28, both with a static musl probe
  on GitHub's Ubuntu 24.04 runner. Each profile runs 68 syscall checks and 107
  `madvise` checks.
- **Local, 2026-10-10:** host kernel 6.18 (WSL2). Container: Podman 5.8,
  crun 1.28, all profiles pass. Local filter, as an unprivileged user: all 47
  profiles pass. The only skip is `map_shadow_stack`, which that host kernel
  also lacks.
- **End to end:** a memmap2 reader built with a glibc 2.17 floor runs correctly
  in SLES 12 SP5 userland under the 4.12 profile. The same program built
  normally on Ubuntu 24.04 is rejected by the SLES loader (`GLIBC_2.28` not
  found).

## Releases

Published to crates.io only; kernel/preset data is compiled in and needs no
runtime download. Container engines can still need network access to pull images. See [RELEASING.md](RELEASING.md), which also covers
disconnected networks. The `profiles/` directory is generated for reference
and is not part of the crate (`kernel-seccomp generate DIR` recreates it).

## What this does not cover

- **Other flags and behavior:** only `madvise` advice is modelled. `mmap`
  silently ignores unknown flags, which seccomp cannot reproduce, and flags
  on `memfd_create`, `openat`, `statx` and `clone` are on the roadmap.
- **Vendor backports:** presets use the mainline base, so a profile can be
  stricter than the real vendor kernel, never looser.
- **Kernels before 3.3:** the data floor is v3.3. Rust std's documented
  minimum, 3.2, is treated as 3.3.
- **Real old-system claims:** qualification requires the actual target kernel
  and userland; simulation and containers on a modern host are not equivalent.
  That infrastructure/testing is currently unavailable and
  [deferred](ROADMAP.md#deferred-qualification). Ordinary tool releases can ship
  with passing applicable automated gates and the explicit tier 3 gap; see
  [release qualification scope](RELEASING.md#qualification-scope-for-releases).


## Probe and maintainer self-tests

Use `syscall-probe dump` without a filter to record a baseline, then
`syscall-probe check --kernel 4.12 --baseline FILE` under the selected filter.
The probe compares syscall
and madvise results with an unfiltered baseline; it is a testing tool, not an
application dependency. From a source checkout, build the probe and generator
and use `crates/kernel-abi-tools/scripts/self-test.sh PROBE local` for generic
kernel profiles. Pass a container image instead of `local` for container checks;
a static musl probe can run across older userlands. Pass kernel versions or
preset IDs after the runner/image to select profiles. CI lists the exact build
and runtime setup in [the kernel workflow](../.github/workflows/kernel-abi-tools.yml).

For upstream data regeneration, use [the maintainer guide](MAINTAINING.md#refresh-kernel-data).
