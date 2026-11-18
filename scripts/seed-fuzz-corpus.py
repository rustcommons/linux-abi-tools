#!/usr/bin/env python3
"""Stage reviewed text seeds and source-built ELF fixtures into a mutable corpus."""
import argparse
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parent.parent
TARGETS = ('abi_snapshot', 'metadata_tsv', 'provider_manifest', 'kernel_data', 'elf_scan', 'target_selection', 'clock_profile', 'recorded_libgcc')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'fuzz/corpus')
    args = parser.parse_args()
    for name in TARGETS:
        dest = args.output / name
        dest.mkdir(parents=True, exist_ok=True)
        seeds = ROOT / 'fuzz/seeds' / name
        if seeds.exists():
            for seed in sorted(seeds.iterdir()):
                if seed.is_file():
                    shutil.copyfile(seed, dest / seed.name)
    (args.output / 'elf_scan/header-only').write_bytes(bytes([0x7f]) + b'ELF')
    for relative in ('elf/symbols.so', 'clock-profile/c.so', 'clock-profile/rt.so'):
        source = ROOT / 'tests/fixtures' / relative
        if not source.is_file():
            raise SystemExit('First run python3 scripts/build-test-fixtures.py')
        shutil.copyfile(source, args.output / 'elf_scan' / (source.parent.name + '-' + source.name))
    for selector, name in [(0, 'c'), (1, 'rt')]:
        data = (ROOT / 'tests/fixtures/clock-profile' / (name + '.so')).read_bytes()
        (args.output / 'clock_profile' / (name + '-raw')).write_bytes(bytes([selector]) + data)


if __name__ == '__main__':
    main()
