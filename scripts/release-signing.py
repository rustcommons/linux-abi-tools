#!/usr/bin/env python3
"""Sign/verify release checksums with the pinned Rust Commons OpenPGP key.

No CLI/env fingerprint override or test mode. Tests exercise the private helpers
with throwaway keys; production entry points always enforce the committed pin.
"""
import argparse
from contextlib import contextmanager
import os
from pathlib import Path
import re
import subprocess
import tempfile

PRIMARY_FINGERPRINT = 'B281C3010A76A11737B4D066C7396F6EA7647CC6'
SIGNING_SUBKEY = '248DFD639B75D1EFF2A87289D7C7DE7BBCCF35EC'
PUBLIC_KEY = Path(__file__).resolve().parents[1] / '.github/release-signing-key.asc'


@contextmanager
def gnupg_home():
    with tempfile.TemporaryDirectory(prefix='release-gnupg-') as tmp:
        home = Path(tmp)
        home.chmod(0o700)
        try:
            yield home
        finally:
            subprocess.run(['gpgconf', '--homedir', str(home), '--kill', 'gpg-agent'],
                           check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def gpg(home, *args, data=None, allow_failure=False):
    # Never forward release secrets to child environments or surface stderr
    # from key import/signing (including exception diagnostics).
    env = {k: v for k, v in os.environ.items()
           if k not in ('RELEASE_SIGNING_KEY', 'RELEASE_SIGNING_PASSPHRASE')}
    result = subprocess.run(['gpg', '--no-options', '--homedir', str(home), '--batch', *args],
                            input=data, capture_output=True, env=env)
    if result.returncode and not allow_failure:
        raise ValueError('GnuPG operation failed')
    return result.stdout.decode()


def import_public(home, key, expected):
    gpg(home, '--import', data=key)
    listing = gpg(home, '--with-colons', '--fingerprint', '--list-keys')
    primary = []
    want_fingerprint = False
    for line in listing.splitlines():
        fields = line.split(':')
        if fields[0] == 'pub':
            want_fingerprint = True
        elif fields[0] == 'fpr' and want_fingerprint:
            primary.append(fields[9])
            want_fingerprint = False
    if primary != [expected]:
        raise ValueError('Public key primary fingerprint mismatch')


def check_signature_status(status, listing, expected):
    records = [line.split()[1:] for line in status.splitlines()
               if line.startswith('[GNUPG:] ')]
    forbidden = {'REVKEYSIG', 'KEYREVOKED', 'BADSIG', 'ERRSIG', 'EXPSIG', 'NO_PUBKEY'}
    if any(record[0] in forbidden for record in records):
        raise ValueError('Revoked, bad, expired-signature or unknown-key status')
    good = [r for r in records if r[0] in ('GOODSIG', 'EXPKEYSIG')]
    valid = [r for r in records if r[0] == 'VALIDSIG']
    # VALIDSIG's last field identifies the primary even for subkey signatures.
    if len(good) != 1 or len(valid) != 1 or len(valid[0]) != 11 or valid[0][-1] != expected:
        raise ValueError('Expected exactly one good signature from the pinned primary')
    # KEYEXPIRED can describe a sibling; only EXPKEYSIG concerns this signature.
    if any(r[0] == 'EXPKEYSIG' for r in records):
        # Look up expiry for the actual signing fingerprint, not the primary.
        expiry = None
        current_expiry = None
        for line in listing.splitlines():
            fields = line.split(':')
            if fields[0] in ('pub', 'sub'):
                current_expiry = fields[6]
            elif fields[0] == 'fpr' and fields[9] == valid[0][1]:
                expiry = current_expiry
                break
        timestamp = valid[0][3]  # GnuPG status field 5 (including [GNUPG:]).
        if not expiry or not expiry.isdecimal() or not timestamp.isdecimal():
            raise ValueError('Missing signing key expiry or signature timestamp')
        if int(expiry) <= 0 or int(timestamp) >= int(expiry):
            raise ValueError('Signature was made at or after signing key expiry')


def _verify_signature(dist, public_key, expected):
    with gnupg_home() as home:
        import_public(home, public_key, expected)
        # Like the org reference, decide from all status records, even if gpg
        # exits nonzero. Missing/invalid records fail closed below.
        status = gpg(home, '--status-fd', '1', '--verify',
                     str(dist / 'SHA256SUMS.asc'), str(dist / 'SHA256SUMS'),
                     allow_failure=True)
        listing = gpg(home, '--with-colons', '--fingerprint', '--list-keys')
        check_signature_status(status, listing, expected)


def _verify(dist, public_key, expected):
    _verify_signature(dist, public_key, expected)
    # Release manifests contain only flat asset names, never paths/options.
    lines = (dist / 'SHA256SUMS').read_text().splitlines()
    if not lines or any(not re.fullmatch(r'[0-9a-f]{64}  [A-Za-z0-9][A-Za-z0-9._+-]*', line)
                        for line in lines):
        raise ValueError('Invalid checksum manifest')
    subprocess.run(['sha256sum', '--strict', '--check', 'SHA256SUMS'], cwd=dist, check=True)


def verify(dist):
    _verify(dist, PUBLIC_KEY.read_bytes(), PRIMARY_FINGERPRINT)


def _sign(dist, private_key, passphrase, subkey):
    with gnupg_home() as home:
        gpg(home, '--import', data=private_key)
        gpg(home, '--pinentry-mode', 'loopback', '--passphrase-fd', '0',
            '--local-user', subkey + '!', '--armor', '--detach-sign',
            '--output', str(dist / 'SHA256SUMS.asc'), str(dist / 'SHA256SUMS'),
            data=passphrase + b'\n')


def sign(dist):
    key = os.environ.pop('RELEASE_SIGNING_KEY').encode()
    passphrase = os.environ.pop('RELEASE_SIGNING_PASSPHRASE').encode()
    try:
        _sign(dist, key, passphrase, SIGNING_SUBKEY)
        verify(dist)
    except BaseException:
        (dist / 'SHA256SUMS.asc').unlink(missing_ok=True)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['sign', 'verify'])
    parser.add_argument('--dist', required=True, type=Path)
    args = parser.parse_args()
    try:
        {'sign': sign, 'verify': verify}[args.operation](args.dist.resolve())
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError):
        parser.exit(1, 'Release signature/checksum validation failed\n')


if __name__ == '__main__':
    main()
