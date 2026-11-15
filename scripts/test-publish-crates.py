#!/usr/bin/env python3
"""Offline release-script controls; no upload or external commands are run."""
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("publish", Path(__file__).with_name("publish-crates.py"))
publish = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publish)


class ReleaseControls(unittest.TestCase):
    def test_order(self):
        self.assertEqual([name for name, _ in publish.ORDER], [
            "linux-targets", "kernel-abi-tools", "linux-abi-scan", "libc-abi-tools", "linux-abi-tools",
        ])

    def test_invalid_tag_before_any_subprocess(self):
        for tag in ("v1.2", "1.2.3", "v1.2.3;echo bad", "v1.2.3-rc1", "v1.2.3\n"):
            with self.subTest(tag=tag), patch.object(publish, "output") as command:
                with self.assertRaises(ValueError):
                    publish.validate(tag)
                command.assert_not_called()

    def test_workspace_tag_and_clean_controls(self):
        for answers, succeeds in ((["head", "head", ""], True),
                                  (["head", "other"], False),
                                  (["head", "head", " M src/lib.rs"], False)):
            with patch.object(publish, "ORDER", [("libc-abi-tools", "Cargo.toml")]), \
                 patch.object(publish.Path, "read_text", return_value='[package]\nname="libc-abi-tools"\nversion="1.2.3"'), \
                 patch.object(publish, "output", side_effect=answers):
                if succeeds:
                    self.assertEqual(publish.validate("v1.2.3"), ("1.2.3", "head"))
                else:
                    with self.assertRaises(ValueError):
                        publish.validate("v1.2.3")

    def test_version_mismatch(self):
        with patch.object(publish.Path, "read_text", return_value='[package]\nname="wrong"\nversion="1.2.3"'):
            with self.assertRaises(ValueError):
                publish.validate("v1.2.3")

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

    def test_propagation_timeout_does_not_publish(self):
        with patch.object(publish, "registry_entry", return_value=None) as entry, \
             patch.object(publish.time, "sleep"), patch.object(publish.subprocess, "run") as command:
            with self.assertRaisesRegex(ValueError, "timed out"):
                publish.verify_published("a", "1.2.3", "digest")
            self.assertEqual(entry.call_count, 30)
            command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
