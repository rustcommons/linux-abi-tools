#!/usr/bin/env python3
"""Offline draft/resume and immutable-asset release controls."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('release', Path(__file__).with_name('github-release.py'))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseControls(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dist = Path(self.tmp.name)
        for name in ('a-1.2.3.crate', 'a-1.2.3.cdx.json', 'SHA256SUMS', 'SHA256SUMS.asc',
                     'linux-abi-tools-1.2.3.crates.intoto.jsonl'):
            (self.dist / name).write_text('contents')
        self.notes = self.dist / 'notes.md'
        self.notes.write_text('# Release 1.2.3\n\nNotes.\n')
        self.assets = [{'id': 1, 'name': p.name, 'size': p.stat().st_size}
                       for p in release.release_files(self.dist, '1.2.3')]
        self.state = {'tag_name': 'v1.2.3', 'name': 'Release 1.2.3',
                      'body': self.notes.read_text(), 'prerelease': False,
                      'draft': True, 'assets': []}
        self.commands = []
        retained = patch.object(release, 'retain_signature')
        self.retain = retained.start()
        self.addCleanup(retained.stop)

    def fake_gh(self, *args):
        self.commands.append(args)
        if args[:2] == ('release', 'upload'):
            path = Path(args[3])
            self.state['assets'].append({'name': path.name, 'size': path.stat().st_size})
        if args[:2] == ('release', 'edit'):
            self.state['draft'] = False
        return b''

    def run_release(self):
        release.publish('owner/repo', '1.2.3', self.dist, self.notes)

    def test_create_resume_and_published_rerun(self):
        with patch.object(release, 'snapshot', side_effect=lambda *_: copy.deepcopy(self.state)), \
             patch.object(release, 'gh', side_effect=self.fake_gh), \
             patch.object(release, 'check_asset') as check:
            self.state['assets'] = [self.assets[0]]
            self.run_release()
            self.assertFalse(self.state['draft'])
            self.assertEqual(len(self.state['assets']), 5)
            self.assertEqual(sum(c[:2] == ('release', 'upload') for c in self.commands), 4)
            self.assertEqual(self.commands[-1][:2], ('release', 'edit'))
            self.assertGreaterEqual(check.call_count, 11)
            self.commands.clear()
            self.run_release()
            self.assertEqual(self.commands, [])  # Never modify a published release.
            self.retain.assert_called_once()

    def test_new_release_is_draft_first(self):
        snapshots = [None, copy.deepcopy(self.state)]
        with patch.object(release, 'snapshot', side_effect=lambda *_: snapshots.pop(0) if snapshots else copy.deepcopy(self.state)), \
             patch.object(release, 'gh', side_effect=self.fake_gh), \
             patch.object(release, 'check_asset'):
            self.run_release()
        self.assertIn('--draft', self.commands[0])
        self.assertIn('--verify-tag', self.commands[0])
        self.assertEqual(self.commands[-1][:2], ('release', 'edit'))

    def test_metadata_or_published_missing_asset_refuses_mutation(self):
        for field, value in (('name', 'wrong'), ('body', 'wrong'), ('tag_name', 'wrong'),
                             ('prerelease', True), ('draft', False)):
            with self.subTest(field=field):
                state = copy.deepcopy(self.state)
                state[field] = value
                with patch.object(release, 'snapshot', return_value=state), \
                     patch.object(release, 'gh') as gh:
                    with self.assertRaises(ValueError):
                        self.run_release()
                    gh.assert_not_called()

    def test_bad_existing_asset_refuses_upload(self):
        self.state['assets'] = [self.assets[0]]
        with patch.object(release, 'snapshot', return_value=self.state), \
             patch.object(release, 'check_asset', side_effect=ValueError('mismatch')), \
             patch.object(release, 'gh') as gh:
            with self.assertRaises(ValueError):
                self.run_release()
            gh.assert_not_called()

    def test_asset_size_and_served_checksum(self):
        path = self.dist / self.assets[0]['name']
        asset = self.assets[0]
        with self.assertRaises(ValueError):
            release.check_asset('o/r', {**asset, 'size': 0}, path)
        def download(*args):
            return b'tampered'
        with patch.object(release, 'gh', side_effect=download):
            with self.assertRaises(ValueError):
                release.check_asset('o/r', asset, path)

    def test_existing_signature_must_verify_before_retention(self):
        # Invalid existing signatures block uploads and publication.
        original = self.retain.side_effect
        self.retain.side_effect = ValueError('invalid signature')
        self.state['assets'] = [a for a in self.assets if a['name'] == 'SHA256SUMS.asc']
        with patch.object(release, 'snapshot', return_value=self.state), patch.object(release, 'gh') as gh:
            with self.assertRaises(ValueError):
                self.run_release()
            gh.assert_not_called()
        self.retain.side_effect = original

    def test_retention_enforces_pinned_verification(self):
        # Use the real function, restored temporarily from the patch's wraps.
        spec = importlib.util.spec_from_file_location('fresh_release', Path(release.__file__))
        fresh = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(fresh)
        with patch.object(fresh, 'download_asset', return_value=b'existing signature'), \
             patch.object(fresh.signing, '_verify_signature') as verify:
            fresh.retain_signature('o/r', {'id': 123}, self.dist)
            self.assertEqual(verify.call_args.args[1], fresh.signing.PUBLIC_KEY.read_bytes())
            self.assertEqual(verify.call_args.args[2], fresh.signing.PRIMARY_FINGERPRINT)
            self.assertEqual((self.dist / 'SHA256SUMS.asc').read_bytes(), b'existing signature')

    def test_signature_required_in_new_layout(self):
        (self.dist / 'SHA256SUMS.asc').unlink()
        with self.assertRaises(ValueError):
            release.release_files(self.dist, '1.2.3')


if __name__ == '__main__':
    unittest.main()
