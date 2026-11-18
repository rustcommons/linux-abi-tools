#!/usr/bin/env bash
set -euo pipefail
cd "$SRC/linux-abi-tools"
export RUSTUP_TOOLCHAIN=nightly-2025-09-05 CARGO_BUILD_JOBS=2
rustc --version
cargo fuzz --version
python3 scripts/build-test-fixtures.py
python3 scripts/seed-fuzz-corpus.py
cargo fetch --locked --manifest-path fuzz/Cargo.toml
before=$(sha256sum fuzz/Cargo.lock)
# Use the builder's matching libFuzzer engine/C++ runtime, not a host archive.
export CUSTOM_LIBFUZZER_PATH="$LIB_FUZZING_ENGINE"
export CUSTOM_LIBFUZZER_STD_CXX=c++
CARGO_NET_OFFLINE=true cargo fuzz build -O --debug-assertions --target x86_64-unknown-linux-gnu
[ "$before" = "$(sha256sum fuzz/Cargo.lock)" ]
python3 - "$OUT" <<'PY'
from pathlib import Path
import shutil
import sys
import zipfile
out = Path(sys.argv[1])
for source in sorted(Path('fuzz/fuzz_targets').glob('*.rs')):
    name = source.stem
    shutil.copy2(Path('fuzz/target/x86_64-unknown-linux-gnu/release') / name, out / name)
    (out / (name + '.options')).write_text('[libfuzzer]\nmax_len=65536\nrss_limit_mb=512\ntimeout=5\n')
    with zipfile.ZipFile(out / (name + '_seed_corpus.zip'), 'w', zipfile.ZIP_DEFLATED) as archive:
        for seed in sorted((Path('fuzz/corpus') / name).iterdir()):
            if seed.is_file():
                archive.write(seed, seed.name)
PY
