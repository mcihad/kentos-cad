"""Real Git objects and disposable files: restore, corruption and scope checks."""
import copy
import contextlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import gui_evidence as evidence


class LocalEvidence(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.run_git("init", "-q")
        (self.root / ".gitattributes").write_text("*.png -filter -text\n")
        hooks = self.root / ".empty-hooks"
        hooks.mkdir()
        self.name = "docs/baseline/example/native-window.png"
        self.path = self.root / self.name
        self.original = b"\x89PNG\r\n\x1a\nopaque screenshot fixture\0\xff"
        for name in [self.name, "apps/desktop/assets/logo.png", "fixtures/raster/tile.png"]:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(self.original)
        self.run_git("add", ".")
        self.run_git("-c", "user.name=Evidence test", "-c", "user.email=test@example.invalid",
                     "-c", "commit.gpgsign=false", "-c", f"core.hooksPath={hooks}",
                     "commit", "-qm", "original files")
        self.manifest = evidence.record(self.root, "HEAD")
        self.assertEqual(self.manifest["imageCount"], 1)

    def run_git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], stderr=subprocess.PIPE)

    def test_restore_uses_git_history_after_file_and_index_removal(self):
        self.run_git("rm", "--cached", "--", self.name)
        self.path.unlink()
        self.assertFalse(evidence.inspect(self.root, self.manifest, verify=True)["allImageBytesVerified"])
        result = evidence.restore(self.root, self.manifest, [])
        self.assertEqual(result["restoredImages"], 1)
        self.assertEqual(self.path.read_bytes(), self.original)
        self.assertTrue(evidence.inspect(self.root, self.manifest, verify=True)["allImageBytesVerified"])
        self.assertEqual(evidence.restore(self.root, self.manifest, [self.name])["alreadyPresent"], 1)
        self.assertEqual(evidence.tracked(self.root)["trackedBaselineImages"], 0)
        self.assertTrue((self.root / "apps/desktop/assets/logo.png").is_file())
        self.assertTrue((self.root / "fixtures/raster/tile.png").is_file())

    def test_existing_changed_image_is_preserved_and_reported(self):
        changed = b"manually changed local evidence"
        self.path.write_bytes(changed)
        result = evidence.restore(self.root, self.manifest, [])
        self.assertEqual(result["conflicts"], [self.name])
        self.assertEqual(result["overwrittenImages"], 0)
        self.assertEqual(self.path.read_bytes(), changed)
        self.assertEqual(evidence.inspect(self.root, self.manifest, verify=True)["changedImages"], 1)

    def test_wrong_manifest_hash_cannot_publish_a_historical_blob(self):
        self.path.unlink()
        bad = copy.deepcopy(self.manifest)
        bad["images"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "bytes disagree"):
            evidence.restore(self.root, bad, [])
        self.assertFalse(self.path.exists())
        self.assertEqual(list(self.path.parent.glob(".gui-restore-*")), [])

    def test_metadata_reports_missing_without_claiming_byte_verification(self):
        self.path.unlink()
        result = evidence.inspect(self.root, self.manifest, verify=False)
        self.assertEqual(result["mode"], "metadata-only")
        self.assertEqual(result["missingImages"], 1)
        self.assertEqual(result["verifiedImages"], 0)
        self.assertFalse(result["allImageBytesVerified"])
        manifest = self.root / "manifest.json"
        manifest.write_text(json.dumps(self.manifest))
        for command, expected_exit in [("metadata", 0), ("verify", 1)]:
            with self.subTest(command=command):
                output = io.StringIO()
                with contextlib.redirect_stdout(output):
                    exit_code = evidence.main(["--root", str(self.root), "--manifest", str(manifest), command])
                self.assertEqual(exit_code, expected_exit)
                report = json.loads(output.getvalue())
                self.assertEqual(report["missingImages"], 1)
                self.assertFalse(report["allImageBytesVerified"])

    def test_manifest_cannot_restore_outside_the_baseline_or_through_a_symlink(self):
        path = self.root / "manifest.json"
        for name in ["docs/baseline/../../elsewhere.png", "fixtures/raster/tile.png", "/tmp/out.png"]:
            with self.subTest(name=name):
                bad = copy.deepcopy(self.manifest)
                bad["images"][0]["path"] = name
                path.write_text(json.dumps(bad))
                with self.assertRaisesRegex(ValueError, "Unsafe"):
                    evidence.load(self.root, path)
        elsewhere = self.root / "elsewhere"
        elsewhere.mkdir()
        self.path.unlink()
        self.path.symlink_to(elsewhere / "image.png")
        with self.assertRaisesRegex(ValueError, "symlink"):
            evidence.restore(self.root, self.manifest, [])
        self.assertFalse((elsewhere / "image.png").exists())


if __name__ == "__main__":
    unittest.main()
