"""Synthetic release-boundary checks; no original data or network required."""
from pathlib import Path
import json
import hashlib
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from check_source import GAMEPLAY_MEDIA, allowed, audit_data, size_limit
from package_source import package


class SourcePackageTests(unittest.TestCase):
    def fixture(self, root):
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        files = {
            ".gitignore": "/private/\n/target/\n*.pk3\n*.exe\n",
            "Cargo.toml": '[package]\nname="fixture"\nversion="0.1.0"\n',
            "Cargo.lock": "version = 4\n",
            "build.rs": "fn main() {}\n",
            "src/main.rs": 'fn main() { let _ = include_str!("route.json"); }\n',
            "src/route.json": '[{"walk":[1,2,3]}]\n',
            "Launch.cmd": "@echo off\n",
            "Setup.cmd": "@echo off\n",
            "tools/launchers/Launch-Test.cmd": "@echo off\n",
            ".github/ISSUE_TEMPLATE/bug_report.yml": "name: Bug report\n",
        }
        for name, text in files.items():
            p = root / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text, encoding="utf-8")
        subprocess.run(["git", "-C", str(root), "add", "."], check=True)
        subprocess.run(["git", "-C", str(root), "-c", "user.name=Fixture",
                        "-c", "user.email=fixture@example.invalid", "commit", "-qm", "fixture"], check=True)
        return files

    def test_required_inputs_untracked_changes_and_private_boundary(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            files = self.fixture(root)
            (root / "private").mkdir()
            (root / "private/secret.txt").write_text("excluded research")
            (root / "original.pk3").write_bytes(b"excluded original data")
            (root / "src/new.rs").write_text("// new source\n")
            (root / "src/main.rs").write_text("fn main() { /* current change */ }\n")
            output = root / "private/review.zip"
            manifest = package(root, output)
            self.assertEqual(set(manifest["files"]), set(files) | {"src/new.rs"})
            with zipfile.ZipFile(output) as z:
                self.assertIn(b"current change", z.read("looking-glass/src/main.rs"))
            self.assertEqual(json.loads(output.with_suffix(".manifest.json").read_text())["sha256"], manifest["sha256"])
            with self.assertRaisesRegex(ValueError, "overwrite"):
                package(root, output)

    def test_unexpected_file_fails_before_creating_archive(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixture(root)
            (root / "transcript.txt").write_text("unreviewed content")
            output = root / "private/review.zip"
            with self.assertRaisesRegex(ValueError, "approved source path"):
                package(root, output)
            self.assertFalse(output.exists())

    def test_missing_build_script_is_not_silently_omitted(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixture(root)
            (root / "build.rs").unlink()
            with self.assertRaisesRegex(ValueError, "Required source input missing"):
                package(root, root / "private/review.zip")

    def test_sensitive_and_invalid_content_is_rejected(self):
        self.assertTrue(audit_data("src/key.rs", b'// ' + b'ghp_' + b'x' * 40))
        self.assertTrue(audit_data("src/binary.rs", b"\0binary"))
        self.assertTrue(audit_data("src/route.json", b"invalid JSON"))
        for path in ["private/note.md", "src/data.pk3", "src/../private/a.rs", "/src/main.rs"]:
            self.assertFalse(allowed(path), path)

    def test_curated_media_requires_exact_path_and_content(self):
        name = "docs/media/gameplay/dry-landing.mp4"
        sample = b"synthetic\0recording"
        with patch.dict(GAMEPLAY_MEDIA, {name: hashlib.sha256(sample).hexdigest()}):
            self.assertEqual(audit_data(name, sample), [])
            self.assertTrue(audit_data(name, sample + b"changed"))
            self.assertTrue(audit_data(name, b"x" * (size_limit(name) + 1)))
        for path in ["docs/media/gameplay/extra.mp4", "docs/media/gameplay/extra.gif",
                     "docs/media/gameplay/original.pk3", "docs/other/dry-landing.mp4"]:
            self.assertFalse(allowed(path), path)

    def test_review_package_preserves_approved_media(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixture(root)
            name = "docs/media/gameplay/dry-landing.gif"
            sample = b"GIF89a\0synthetic fixture"
            p = root / name
            p.parent.mkdir(parents=True)
            p.write_bytes(sample)
            with patch.dict(GAMEPLAY_MEDIA, {name: hashlib.sha256(sample).hexdigest()}):
                output = root / "private/review.zip"
                manifest = package(root, output)
                self.assertIn(name, manifest["files"])
                with zipfile.ZipFile(output) as z:
                    self.assertEqual(z.read("looking-glass/" + name), sample)

    def test_website_allowance_does_not_admit_unreviewed_assets(self):
        for path in ["docs/site/index.html", "docs/site/styles.css",
                     "docs/site/favicon.svg", "docs/site/sitemap.xml"]:
            self.assertTrue(allowed(path), path)
            self.assertTrue(audit_data(path, b"unreviewed\0binary"), path)
        for path in ["docs/site/data.pk3", "docs/site/extra.html",
                     "docs/site/extra.svg", "docs/site/media/capture.mp4"]:
            self.assertFalse(allowed(path), path)


if __name__ == "__main__":
    unittest.main()
