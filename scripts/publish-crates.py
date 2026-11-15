#!/usr/bin/env python3
"""Publish an approved immutable tag, then compare official archive checksums.

Invoked only by the environment-approved Trusted Publishing job. Never creates
or moves tags. An existing registry version refuses the batch before any upload;
partial publication needs explicit release-owner recovery through existing policy.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import time
import tomllib
import urllib.error
import urllib.request

ORDER = (
    ("linux-targets", "crates/linux-targets/Cargo.toml"),
    ("kernel-abi-tools", "crates/kernel-abi-tools/Cargo.toml"),
    ("linux-abi-scan", "crates/linux-abi-scan/Cargo.toml"),
    ("libc-abi-tools", "Cargo.toml"),
    ("linux-abi-tools", "crates/linux-abi-tools/Cargo.toml"),
)


def output(*args):
    return subprocess.check_output(args, text=True).strip()


def validate(tag):
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
        raise ValueError("release tag must be vX.Y.Z")
    version = tag[1:]
    for name, manifest in ORDER:
        package = tomllib.loads(Path(manifest).read_text())["package"]
        if (package["name"], package["version"]) != (name, version):
            raise ValueError(f"{manifest}: package/version differs from tag")
    head = output("git", "rev-parse", "HEAD")
    if output("git", "rev-parse", f"refs/tags/{tag}^{{commit}}") != head:
        raise ValueError("tag does not point to checked-out source")
    if output("git", "status", "--porcelain", "--untracked-files=normal"):
        raise ValueError("refusing a dirty release checkout")
    return version, head


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
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_published(name, version, expected):
    # Index/CDN propagation can lag a successful upload; a bounded wait never
    # retries cargo publish, nor treats network errors as checksum success.
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
    raise ValueError(f"{name}: registry/archive propagation timed out; do not blindly republish")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    version, head = validate(args.tag)
    print(f"Validated tag {args.tag}: {head}", flush=True)
    if args.check_only:
        return
    if not os.environ.get("CARGO_REGISTRY_TOKEN"):
        raise ValueError("missing short-lived publishing token")
    for name, _ in ORDER:
        if registry_entry(name, version) is not None:
            raise ValueError(f"{name} {version} already exists; release-owner recovery required")
    metadata = json.loads(output("cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"))
    packages = Path(metadata["target_directory"]) / "package"
    for name, _ in ORDER:
        subprocess.run(["cargo", "publish", "--locked", "-p", name], check=True)
        path = packages / f"{name}-{version}.crate"
        expected = archive_identity(path, name, version, head)
        verify_published(name, version, expected)


if __name__ == "__main__":
    main()
