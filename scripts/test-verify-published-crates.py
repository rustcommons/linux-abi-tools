#!/usr/bin/env python3
"""Offline controls for verify-published-crates.py; no network access."""
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("publish", Path(__file__).with_name("verify-published-crates.py"))
publish = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publish)


class ReleaseControls(unittest.TestCase):
    def test_vcs_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "a.crate"
            for sha, dirty, accepted in (("head", False, True), ("other", False, False), ("head", True, False)):
                raw = json.dumps({"git": {"sha1": sha, "dirty": dirty}}).encode()
                with tarfile.open(path, "w:gz") as archive:
                    info = tarfile.TarInfo("a-1.2.3/.cargo_vcs_info.json")
                    info.size = len(raw)
                    archive.addfile(info, io.BytesIO(raw))
                if accepted:
                    self.assertEqual(publish.archive_identity(path, "a", "1.2.3", "head"),
                                     publish.hashlib.sha256(path.read_bytes()).hexdigest())
                else:
                    with self.assertRaises(ValueError):
                        publish.archive_identity(path, "a", "1.2.3", "head")

    def test_index_and_official_archive_must_agree(self):
        data = b"official archive"
        digest = publish.hashlib.sha256(data).hexdigest()
        with patch.object(publish, "registry_entry", return_value={"yanked": False, "cksum": digest}), \
             patch.object(publish, "fetch", return_value=data):
            publish.verify_published("a", "1.2.3", digest)
        for entry in ({"yanked": True, "cksum": digest}, {"yanked": False, "cksum": "wrong"}):
            with patch.object(publish, "registry_entry", return_value=entry), \
                 patch.object(publish, "fetch") as fetch:
                with self.assertRaises(ValueError):
                    publish.verify_published("a", "1.2.3", digest)
                fetch.assert_not_called()
        with patch.object(publish, "registry_entry", return_value={"yanked": False, "cksum": digest}), \
             patch.object(publish, "fetch", return_value=b"tampered"):
            with self.assertRaises(ValueError):
                publish.verify_published("a", "1.2.3", digest)

    def test_propagation_timeout(self):
        with patch.object(publish, "registry_entry", return_value=None) as entry, \
             patch.object(publish.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "timed out"):
                publish.verify_published("a", "1.2.3", "digest")
            self.assertEqual(entry.call_count, 30)


if __name__ == "__main__":
    unittest.main()
