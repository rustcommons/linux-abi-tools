#!/usr/bin/env python3
"""Verify that crates.io serves exactly the attested release archives.

Used by the `verify` job of the standard Trusted publishing pipeline. For each
release crate: the attested .crate records the tagged commit as a clean VCS
checkout, the crates.io index checksum equals its SHA-256, the version is not
yanked, and the official download has the same SHA-256. Never publishes.

    verify-published-crates.py --dist DIR --version X.Y.Z --commit SHA --crates JSON
"""

import argparse
import hashlib
import json
from pathlib import Path
import tarfile
import time
import urllib.error
import urllib.request


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "rustcommons-release-check"})
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def registry_entry(name, version):
    # Index records are public and require no publishing credential.
    url = f"https://index.crates.io/{name[:2]}/{name[2:4]}/{name}"
    try:
        records = fetch(url).splitlines()
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise
    for line in records:
        entry = json.loads(line)
        if entry["vers"] == version:
            return entry
    return None


def archive_identity(path, name, version, head):
    with tarfile.open(path, "r:gz") as archive:
        info = json.load(archive.extractfile(f"{name}-{version}/.cargo_vcs_info.json"))
    if info["git"]["sha1"] != head or info["git"].get("dirty", False):
        raise ValueError(f"{name}: archive VCS identity differs from clean tag")
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def verify_published(name, version, expected):
    # Index/CDN propagation can lag a successful upload; the wait is bounded
    # and network errors are never treated as checksum success.
    for attempt in range(30):
        entry = registry_entry(name, version)
        if entry is not None:
            if entry["yanked"] or entry["cksum"] != expected:
                raise ValueError(f"{name}: registry checksum/yank mismatch")
            try:
                actual = hashlib.sha256(fetch(
                    f"https://static.crates.io/crates/{name}/{name}-{version}.crate"
                )).hexdigest()
            except urllib.error.HTTPError as error:
                if error.code != 404:
                    raise
            else:
                if actual != expected:
                    raise ValueError(f"{name}: official archive checksum mismatch")
                print(f"Verified {name} {version} SHA256 {expected}", flush=True)
                return
        if attempt < 29:
            time.sleep(2)
    raise ValueError(f"{name}: registry/archive propagation timed out")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dist", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--crates", required=True, help="JSON list of release crates")
    args = parser.parse_args()
    crates = json.loads(args.crates)
    if not crates:
        raise ValueError("no release crates to verify")
    for name in crates:
        path = args.dist / f"{name}-{args.version}.crate"
        verify_published(name, args.version, archive_identity(path, name, args.version, args.commit))


if __name__ == "__main__":
    main()
