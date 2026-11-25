#!/usr/bin/env python3
"""Offline release signing controls; only throwaway secret keys are generated."""
import hashlib
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('signing', Path(__file__).with_name('release-signing.py'))
signing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(signing)


class SigningControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        with signing.gnupg_home() as home:
            signing.gpg(home, '--pinentry-mode', 'loopback', '--passphrase-fd', '0',
                        '--quick-generate-key', 'Throwaway release test', 'ed25519', 'cert', '1d',
                        data=b'throwaway-test-passphrase\n')
            listing = signing.gpg(home, '--with-colons', '--fingerprint', '--list-keys')
            cls.fingerprint = next(l.split(':')[9] for l in listing.splitlines() if l.startswith('fpr:'))
            signing.gpg(home, '--pinentry-mode', 'loopback', '--passphrase-fd', '0',
                        '--quick-add-key', cls.fingerprint, 'ed25519', 'sign', '1d',
                        data=b'throwaway-test-passphrase\n')
            listing = signing.gpg(home, '--with-colons', '--fingerprint', '--list-keys')
            cls.subkey = [l.split(':')[9] for l in listing.splitlines() if l.startswith('fpr:')][-1]
            cls.public = signing.gpg(home, '--armor', '--export').encode()
            cls.private = signing.gpg(home, '--pinentry-mode', 'loopback', '--passphrase-fd', '0',
                                      '--armor', '--export-secret-keys', data=b'throwaway-test-passphrase\n').encode()

    def test_committed_public_key_pin(self):
        self.assertEqual(signing.PRIMARY_FINGERPRINT, 'B281C3010A76A11737B4D066C7396F6EA7647CC6')
        with signing.gnupg_home() as home:
            signing.import_public(home, signing.PUBLIC_KEY.read_bytes(), signing.PRIMARY_FINGERPRINT)

    def test_roundtrip_and_rejections(self):
        with tempfile.TemporaryDirectory() as tmp:
            dist = Path(tmp)
            payload = dist / 'a.crate'
            payload.write_bytes(b'packaged source')
            checksums = hashlib.sha256(payload.read_bytes()).hexdigest() + '  a.crate\n'
            (dist / 'SHA256SUMS').write_text(checksums)
            signing._sign(dist, self.private, b'throwaway-test-passphrase', self.subkey)
            signing._verify(dist, self.public, self.fingerprint)
            with self.assertRaises(ValueError):
                signing.verify(dist)  # Real entry point cannot accept the throwaway signer.
            with self.assertRaises(ValueError):
                signing._verify(dist, self.public, signing.PRIMARY_FINGERPRINT)
            payload.write_bytes(b'tampered')
            with self.assertRaises(subprocess.CalledProcessError):
                signing._verify(dist, self.public, self.fingerprint)
            (dist / 'SHA256SUMS').write_text(checksums + '\n')
            with self.assertRaises(ValueError):
                signing._verify(dist, self.public, self.fingerprint)
            (dist / 'SHA256SUMS.asc').unlink()
            with self.assertRaises(ValueError):
                signing._verify(dist, self.public, self.fingerprint)

    def test_production_signing_failure_removes_signature(self):
        with tempfile.TemporaryDirectory() as tmp:
            dist = Path(tmp)
            (dist / 'SHA256SUMS').write_text('not important')
            with patch.dict(os.environ, RELEASE_SIGNING_KEY=self.private.decode(),
                            RELEASE_SIGNING_PASSPHRASE='throwaway-test-passphrase'):
                with self.assertRaises(ValueError):
                    signing.sign(dist)  # Pinned subkey absent: must fail before publishing.
            self.assertFalse((dist / 'SHA256SUMS.asc').exists())

    def test_temporary_home_cleanup_on_failure(self):
        with self.assertRaises(ValueError):
            with signing.gnupg_home() as home:
                self.assertEqual(home.stat().st_mode & 0o777, 0o700)
                signing.gpg(home, '--list-keys')
                raise ValueError('failure')
        self.assertFalse(home.exists())

    def test_workflow_secret_scope_and_order(self):
        workflow = (Path(signing.__file__).resolve().parents[1] /
                    '.github/workflows/trusted-publishing.yml').read_text()
        publish = workflow.split('\n  publish:', 1)[1].split('\n  github-release:', 1)[0]
        self.assertLess(publish.index('release-signing.py sign'), publish.index('crates-io-auth-action@'))
        self.assertEqual(workflow.count('secrets.RELEASE_SIGNING_KEY'), 1)
        self.assertEqual(workflow.count('secrets.RELEASE_SIGNING_PASSPHRASE'), 1)
        self.assertIn('--no-verify', publish)
        signing_step = publish.split('- name: Sign and independently verify release checksums', 1)[1]
        signing_step = signing_step.split('      - uses:', 1)[0]
        self.assertNotIn('if:', signing_step)  # Also signs when PENDING is empty.

    def test_no_cli_test_bypass(self):
        result = subprocess.run(['python3', str(Path(signing.__file__)), 'verify',
                                 '--dist', '/tmp', '--fingerprint', self.fingerprint], capture_output=True)
        self.assertEqual(result.returncode, 2)


if __name__ == '__main__':
    unittest.main()
