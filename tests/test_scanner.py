import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from filepilot.scanner import scan, classify
from filepilot.cli import main


class ScannerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        # macOS /var is a system symlink; use the physical fixture path.
        self.root = Path(self.temp.name).resolve(strict=True)

    def write(self, name, content="synthetic"):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def test_synthetic_inventory(self):
        self.write("lecture.pdf")
        self.write("invoice.csv")
        self.write("photo.JPG")
        self.write("mystery.bin")
        result = scan(self.root)
        rows = {e["path"]: e for e in result["entries"]}
        self.assertEqual(rows["lecture.pdf"]["classification"]["category"], "Education")
        self.assertEqual(rows["invoice.csv"]["classification"]["category"], "Finance")
        self.assertEqual(rows["photo.JPG"]["classification"]["category"], "Media")
        self.assertEqual(rows["mystery.bin"]["classification"]["recommendation"], "review")
        self.assertEqual(json.loads(json.dumps(result))["schema_version"], "0.1.0")

    def test_project_preservation(self):
        self.write("app/package.json", "{}")
        self.write("app/src/lecture.py")
        self.write("app/node_modules/hidden.js")
        result = scan(self.root)
        self.assertEqual(result["projects"][0]["path"], "app")
        row = next(e for e in result["entries"] if e["path"].endswith("lecture.py"))
        self.assertEqual(row["classification"]["recommendation"], "leave_untouched")
        self.assertFalse(any(e["path"].endswith("hidden.js") for e in result["entries"]))

    def test_symlinks_and_root_links(self):
        outside = self.root / "outside"
        outside.mkdir()
        self.write("outside/secret.txt")
        os.symlink(outside, self.root / "link")
        os.symlink(self.root / "missing", self.root / "broken")
        rows = {e["path"]: e for e in scan(self.root)["entries"]}
        self.assertEqual(rows["link"]["skip_reason"], "symlink_not_followed")
        self.assertIn("broken", rows)
        self.assertNotIn("link/secret.txt", rows)
        with self.assertRaises(OSError):
            scan(self.root / "link")

    def test_symlinked_temp_parent(self):
        # Reproduce /var -> /private/var on any supported host.
        physical = self.root / "physical"
        physical.mkdir()
        alias = self.root / "alias"
        alias.symlink_to(physical, target_is_directory=True)
        with tempfile.TemporaryDirectory(dir=alias) as raw:
            with self.assertRaises(OSError):
                scan(raw)
            result = scan(Path(raw).resolve(strict=True))
            self.assertEqual(result["errors"], [])
            self.assertEqual(result["summary"]["entry_count"], 0)

    def test_sensitive_and_bundles(self):
        self.write(".ssh/id_rsa")
        self.write(".env")
        self.write("Demo.app/Contents/executable")
        result = scan(self.root)
        self.assertEqual(len(result["entries"]), 3)
        self.assertTrue(all(e["status"] == "skipped" for e in result["entries"]))

    def test_limits(self):
        self.write("a/b/c.txt")
        self.write("other.txt")
        self.assertTrue(scan(self.root, max_entries=1)["truncated"])
        result = scan(self.root, max_depth=0)
        self.assertTrue(result["truncated"])
        self.assertFalse(any(e["path"] == "a/b" for e in result["entries"]))
        with self.assertRaises(ValueError):
            scan(self.root, max_entries=0)

    def test_inaccessible_directory(self):
        (self.root / "blocked").mkdir()
        original = os.open
        def denied(path, *args, **kwargs):
            if path == "blocked":
                raise PermissionError(13, "synthetic denial")
            return original(path, *args, **kwargs)
        with patch("filepilot.scanner.os.open", side_effect=denied):
            result = scan(self.root)
        self.assertEqual(result["errors"][0]["code"], "PermissionError")

    def test_special_file(self):
        os.mkfifo(self.root / "pipe")
        self.assertEqual(scan(self.root)["entries"][0]["skip_reason"], "special_file")

    def test_no_contents_read_or_mutations(self):
        path = self.write("payload.py", "raise RuntimeError('must never execute')")
        before = (path.read_bytes(), path.stat().st_mtime_ns)
        with patch("builtins.open", side_effect=AssertionError("content open")), \
             patch("os.rename", side_effect=AssertionError("rename")), \
             patch("os.unlink", side_effect=AssertionError("unlink")):
            scan(self.root)
        self.assertEqual(before, (path.read_bytes(), path.stat().st_mtime_ns))

    def test_conflicting_rules(self):
        self.assertEqual(classify("college_invoice.pdf", "document")["category"], "Unknown")

    def test_cli(self):
        self.write("resume.pdf")
        with patch("sys.argv", ["filepilot", str(self.root)]), contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(main(), 0)
        self.assertTrue(json.loads(out.getvalue())["read_only"])
        with patch("sys.argv", ["filepilot", str(self.root / "missing")]), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(main(), 2)

    def test_root_project(self):
        self.write("pyproject.toml")
        self.write("assignment.txt")
        self.assertTrue(all(e["project_root"] == "." for e in scan(self.root)["entries"]))


if __name__ == "__main__":
    unittest.main()
