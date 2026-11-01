#!/usr/bin/env bash
# Native x86_64 regression for both supported engines: names, stdin and status.
# Usage: container-io-test.sh KERNEL_SECCOMP CRUN
set -euo pipefail
runner=$(realpath "${1:?kernel-seccomp binary required}")
crun=$(realpath "${2:?crun binary required}")
image=registry.suse.com/suse/sles12sp5:latest
export CARGO_KERNEL_ABI_IO_TEST='cargo value with spaces'
export RUST_KERNEL_ABI_IO_TEST='rust value with spaces'
export UNRELATED_KERNEL_ABI_IO_TEST='must not be forwarded'
for engine in docker podman; do
  export CONTAINER_ENGINE=$engine
  export KERNEL_ABI_ENGINE_ARGS=
  if [ "$engine" = podman ]; then KERNEL_ABI_ENGINE_ARGS="--runtime $crun"; fi
  export KERNEL_ABI_RUN_ARGS='--network none'
  result=$(printf 'redirected input\n' | "$runner" run --kernel 4.12 --image "$image" -- sh -c '
    test "$CARGO_KERNEL_ABI_IO_TEST" = "cargo value with spaces" || exit 21
    test "$RUST_KERNEL_ABI_IO_TEST" = "rust value with spaces" || exit 22
    test -z "${UNRELATED_KERNEL_ABI_IO_TEST+x}" || exit 23
    test ! -t 0 || exit 24
    IFS= read -r line || exit 25
    test "$line" = "redirected input" || exit 26
    printf "stdin and environment passed\n"
  ')
  test "$result" = 'stdin and environment passed'
  status=0
  "$runner" run --kernel 4.12 --image "$image" -- sh -c 'exit 7' </dev/null || status=$?
  test "$status" -eq 7
  printf '%s: stdin, environment and exit status passed\n' "$engine"
done
