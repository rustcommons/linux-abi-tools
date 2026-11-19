#!/usr/bin/env python3
"""Build original x86_64 ELF test data from reviewed assembly; never execute it."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    sources = {}
    for directory in ['elf', 'clock-profile']:
        base = ROOT / 'tests/fixtures' / directory
        for line in (base / 'SHA256SUMS').read_text().splitlines():
            expected, name = line.split()
            source = base / name
            if Path(name).name != name or source.is_symlink() or source.suffix not in ('.S', '.map'):
                raise SystemExit('Unexpected fixture source entry')
            actual = hashlib.sha256(source.read_bytes()).hexdigest()
            if actual != expected:
                raise SystemExit('Fixture source checksum changed: ' + str(source))
            sources[source.relative_to(ROOT).as_posix()] = actual
    if args.verify_only:
        manifest = json.loads((ROOT / 'tests/fixtures/generated-manifest.json').read_text())
        if manifest['source_sha256'] != sources:
            raise SystemExit('Fixture artifact came from different assembly sources')
        expected_names = {'tests/fixtures/elf/symbols.so', 'tests/fixtures/clock-profile/c.so', 'tests/fixtures/clock-profile/rt.so'}
        if set(manifest['sha256']) != expected_names:
            raise SystemExit('Unexpected fixture artifact inventory')
        for name, expected in manifest['sha256'].items():
            actual = hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
            if actual != expected:
                raise SystemExit('Fixture artifact checksum mismatch: ' + name)
        return
    assembler = shutil.which('x86_64-linux-gnu-as') or shutil.which('as')
    linker = shutil.which('x86_64-linux-gnu-ld') or shutil.which('ld')
    if not assembler or not linker:
        raise SystemExit('GNU x86_64 assembler/linker required; see docs/SECURITY-BASELINE.md')
    records = {}
    with tempfile.TemporaryDirectory(prefix='linux-abi-fixtures-') as temp:
        for subdir, name, soname, extra in [
            ('elf', 'symbols', 'libfixture.so.1', []),
            ('clock-profile', 'c', 'libc.so.6', ['--hash-style=both']),
            ('clock-profile', 'rt', 'librt.so.1', ['--hash-style=both']),
        ]:
            directory = ROOT / 'tests/fixtures' / subdir
            obj = Path(temp) / (name + '.o')
            output = directory / (name + '.so')
            subprocess.run([assembler, '--64', str(directory / (name + '.S')), '-o', str(obj)], check=True)
            subprocess.run([linker, '-shared', '--build-id=none', '-soname', soname,
                            '--version-script=' + str(directory / (name + '.map')),
                            *extra, str(obj), '-o', str(Path(temp) / (name + '.so'))], check=True)
            (Path(temp) / (name + '.so')).replace(output)
            records[output.relative_to(ROOT).as_posix()] = hashlib.sha256(output.read_bytes()).hexdigest()
    manifest = ROOT / 'tests/fixtures/generated-manifest.json'
    manifest.write_text(json.dumps({'source_sha256': sources, 'tools': {p: subprocess.check_output([p, '--version'], text=True).splitlines()[0]
                                             for p in [assembler, linker]}, 'sha256': records}, indent=2) + '\n')
    print(manifest.read_text(), end='')


if __name__ == '__main__':
    main()
