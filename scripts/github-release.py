#!/usr/bin/env python3
"""Draft-first release upload: check metadata and every asset; never replace."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile


spec = importlib.util.spec_from_file_location('signing', Path(__file__).with_name('release-signing.py'))
signing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(signing)


def retain_signature(repo, asset, dist):
    # A re-signature has a new timestamp. Keep the existing remote bytes only
    # after verifying that they authenticate this exact manifest with our pin.
    with tempfile.TemporaryDirectory(prefix='release-signature-') as tmp:
        tmp = Path(tmp)
        (tmp / 'SHA256SUMS.asc').write_bytes(download_asset(repo, asset))
        (tmp / 'SHA256SUMS').write_bytes((dist / 'SHA256SUMS').read_bytes())
        signing._verify_signature(tmp, signing.PUBLIC_KEY.read_bytes(), signing.PRIMARY_FINGERPRINT)
        (dist / 'SHA256SUMS.asc').write_bytes((tmp / 'SHA256SUMS.asc').read_bytes())


def gh(*args):
    return subprocess.check_output(['gh', *args])


def snapshot(repo, tag):
    # The releases list includes drafts; the tag endpoint may not find them.
    pages = json.loads(gh('api', '--paginate', '--slurp', f'repos/{repo}/releases?per_page=100'))
    matches = [r for page in pages for r in page if r['tag_name'] == tag]
    if len(matches) > 1:
        raise ValueError('Multiple releases for this tag')
    return matches[0] if matches else None


def check_metadata(release, tag, title, body):
    if (release['tag_name'] != tag or release['name'] != title or
            release['body'] != body or release['prerelease']):
        raise ValueError('Release metadata differs from committed notes')


def download_asset(repo, asset):
    # The asset ID works for authenticated draft downloads as well as releases.
    return gh('api', f'repos/{repo}/releases/assets/{asset["id"]}',
              '-H', 'Accept: application/octet-stream')


def check_asset(repo, asset, path):
    if asset['size'] != path.stat().st_size:
        raise ValueError(f'Existing asset size differs: {path.name}')
    # Download even when an API digest is available, proving the served bytes.
    actual = hashlib.sha256(download_asset(repo, asset)).digest()
    if actual != hashlib.sha256(path.read_bytes()).digest():
        raise ValueError(f'Existing asset checksum differs: {path.name}')


def release_files(dist, version):
    files = (sorted(dist.glob('*.crate')) + sorted(dist.glob('*.cdx.json')) +
             [dist / 'SHA256SUMS', dist / 'SHA256SUMS.asc',
              dist / f'linux-abi-tools-{version}.crates.intoto.jsonl'])
    if not list(dist.glob('*.crate')) or not list(dist.glob('*.cdx.json')):
        raise ValueError('Missing crate archives or SBOMs')
    for path in files:
        if not path.is_file():
            raise ValueError(f'Missing release file: {path.name}')
    return files


def publish(repo, version, dist, notes):
    tag = 'v' + version
    body = notes.read_text()
    title = next(line[2:] for line in body.splitlines() if line.startswith('# '))
    files = release_files(dist, version)
    expected = {p.name: p for p in files}
    current = snapshot(repo, tag)
    if current is None:
        gh('release', 'create', tag, '--repo', repo, '--verify-tag', '--draft',
           '--title', title, '--notes-file', str(notes))
        current = snapshot(repo, tag)
        if current is None or not current['draft']:
            raise ValueError('Draft creation could not be confirmed')
    check_metadata(current, tag, title, body)
    assets = current['assets']
    if len({a['name'] for a in assets}) != len(assets) or any(a['name'] not in expected for a in assets):
        raise ValueError('Unexpected/duplicate release assets')
    existing = {a['name']: a for a in assets}
    if 'SHA256SUMS.asc' in existing:
        retain_signature(repo, existing['SHA256SUMS.asc'], dist)
    # Validate ALL existing files before any upload or publication.
    for name, asset in existing.items():
        check_asset(repo, asset, expected[name])
    missing = [p for p in files if p.name not in existing]
    if not current['draft'] and missing:
        raise ValueError('Published release is incomplete; refusing to modify it')
    for path in missing:
        gh('release', 'upload', tag, str(path), '--repo', repo)
    ready = snapshot(repo, tag)
    check_metadata(ready, tag, title, body)
    if ready['draft'] != current['draft']:
        raise ValueError('Release state changed unexpectedly')
    if len(ready['assets']) != len(expected) or {a['name'] for a in ready['assets']} != set(expected):
        raise ValueError('Release asset layout mismatch')
    for asset in ready['assets']:
        check_asset(repo, asset, expected[asset['name']])
    if ready['draft']:
        gh('release', 'edit', tag, '--repo', repo, '--draft=false')
    final = snapshot(repo, tag)
    check_metadata(final, tag, title, body)
    if final['draft'] or len(final['assets']) != len(expected) or {a['name'] for a in final['assets']} != set(expected):
        raise ValueError('Published release state/layout mismatch')
    for asset in final['assets']:
        check_asset(repo, asset, expected[asset['name']])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--dist', required=True, type=Path)
    parser.add_argument('--notes', required=True, type=Path)
    args = parser.parse_args()
    publish(args.repo, args.version, args.dist, args.notes)
