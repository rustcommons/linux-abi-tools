#!/usr/bin/env python3
"""Bounded real source-provider writers/readers. Usage: LIBC_ABI GCC NEW_EVIDENCE."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import threading

tool = str(Path(sys.argv[1]).resolve())
gcc, out = sys.argv[2], Path(sys.argv[3]).resolve()
out.mkdir(parents=True, exist_ok=False)
cache = out / 'cache'
results = []


def command(label, args):
    p = subprocess.run([tool, 'provider', *args], capture_output=True, timeout=180)
    (out / (label + '.stdout')).write_bytes(p.stdout)
    (out / (label + '.stderr')).write_bytes(p.stderr)
    results.append(dict(case=label, status=p.returncode, args=args))
    (out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    assert p.returncode == 0, (label, p.stderr.decode(errors='replace'))
    return p.stdout.decode().strip()


def writers(label, force):
    args = ['prepare', '--glibc', '2.17', '--mode', 'source', '--cc', gcc, '--cache', str(cache)]
    if force:
        args += ['--force']
    # Keep all three processes live at once; compilation itself is sequential
    # per provider. Each process has its own log file, never a shared pipe.
    processes = []
    try:
        for i in range(3):
            logs = [(out / f'{label}-{i}.{suffix}').open('wb') for suffix in ['stdout','stderr']]
            processes.append((subprocess.Popen([tool, 'provider', *args], stdout=logs[0], stderr=logs[1]), logs))
        descriptors = []
        for i, (p, logs) in enumerate(processes):
            code = p.wait(timeout=180)
            for log in logs:
                log.close()
            results.append(dict(case=f'{label}-{i}', status=code, args=args))
            (out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
            assert code == 0, (label, i, (out / f'{label}-{i}.stderr').read_text())
            descriptors.append(Path((out / f'{label}-{i}.stdout').read_text().strip()))
        return descriptors
    finally:
        for p, logs in processes:
            if p.poll() is None:
                p.kill()
                p.wait()
            for log in logs:
                log.close()


first = writers('normal', False)
assert len(set(first)) == 1
old = first[0]
paths = [old, old.parent/'manifest.json', old.parent/'sysroot/usr/lib64/crt1.o',
         old.parent/'stubs/libc.so.6', old.parent/'sysroot/usr/include/stdio.h']
expected = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
stop = threading.Event()
errors = []
observations = []


def read_held_paths():
    while not stop.is_set():
        try:
            for p in paths:
                assert hashlib.sha256(p.read_bytes()).hexdigest() == expected[str(p)], str(p)
            observations.append(True)
        except Exception as e:
            errors.append(repr(e))
            return
        stop.wait(0.001)


reader = threading.Thread(target=read_held_paths)
reader.start()
try:
    forced = writers('force', True)
finally:
    stop.set()
    reader.join(timeout=5)
assert not reader.is_alive() and observations and not errors, errors
assert len(set(forced)) == 3 and old not in forced
for descriptor in [old, *forced]:
    assert descriptor.is_file() and (descriptor.parent/'sysroot/usr/lib64/crt1.o').is_file()
current = command('path', ['path','--glibc','2.17','--cc',gcc,'--cache',str(cache)])
assert Path(current) in forced
assert len(command('list', ['list','--cache',str(cache)]).splitlines()) == 2
command('verify', ['verify','--glibc','2.17','--cache',str(cache)])
command('export', ['export','--glibc','2.17','--cache',str(cache),'--to',str(out/'export')])
command('import', ['import',str(out/'export'),'--cc',gcc,'--cache',str(out/'import')])
command('verify-import', ['verify','--glibc','2.17','--cache',str(out/'import')])
command('remove', ['remove','--glibc','2.17','--cache',str(cache)])
assert not old.exists() and all(not d.exists() for d in forced)
assert len(command('empty-list', ['list','--cache',str(cache)]).splitlines()) == 1
(out/'summary.json').write_text(json.dumps(dict(status='PASS', held_path_observations=len(observations),
    normal_writers=3,forced_writers=3,old_descriptor=str(old),selected=current,
    held_file_hashes=expected,errors=errors),indent=2)+'\n')
print(f'PASS: three normal writers, three forced writers, {len(observations)} held-path observations, verify/export/import/remove')
