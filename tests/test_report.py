import copy
from pathlib import Path
import tempfile
import unittest
from filepilot.report import validate_report
from filepilot.scanner import scan


class ReportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve(strict=True)
        (self.root / "app").mkdir()
        (self.root / "app" / "Cargo.toml").write_text("# synthetic")
        (self.root / "lecture.txt").write_text("synthetic")
        self.report = scan(self.root)

    def test_valid_report(self):
        self.assertIsNone(validate_report(self.report))

    def test_project_folder_label(self):
        row = next(e for e in self.report["entries"] if e["path"] == "app")
        self.assertEqual(row["project_root"], "app")
        self.assertEqual(row["classification"]["category"], "Development")
        self.assertEqual(row["classification"]["recommendation"], "leave_untouched")

    def test_reject_invalid_reports(self):
        mutations = [
            lambda r: r.update(read_only=False),
            lambda r: r.update(schema_version="99.0"),
            lambda r: r.update(generated_at="yesterday"),
            lambda r: r.update(generated_at="2026-01-01T00:00:00"),
            lambda r: r["entries"][0].update(path="../escape"),
            lambda r: r["entries"][0].update(path="/absolute"),
            lambda r: r["entries"][0].update(path="app//child"),
            lambda r: r["entries"][0].update(size_bytes=True),
            lambda r: r["entries"][0].update(project_root=None),
            lambda r: r["summary"].update(entry_count=999),
            lambda r: r["summary"].update(categories={}),
            lambda r: r["entries"].append(copy.deepcopy(r["entries"][0])),
            lambda r: r["entries"][0]["classification"].update(recommendation="suggest_category", suggested_category="Development", review_required=True),
            lambda r: r["entries"][-1]["classification"].update(review_required=False),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                invalid = copy.deepcopy(self.report)
                mutate(invalid)
                with self.assertRaises(ValueError):
                    validate_report(invalid)

    def test_extended_synthetic_inventory(self):
        for name in ("exam.pdf", "resume.docx", "diary.txt", "photo.heic", "clip.mov",
                     "audio.flac", "installer.dmg", "code.rs", "archive.zip",
                     "college_invoice.pdf", "नोट्स.txt", "quote\"newline\n.txt"):
            (self.root / name).write_text("Synthetic placeholder; not actual format bytes.")
        result = scan(self.root)
        rows = {e["path"]: e for e in result["entries"]}
        expected = {"exam.pdf": "Education", "resume.docx": "Professional",
                    "diary.txt": "Personal", "photo.heic": "Media", "clip.mov": "Media",
                    "audio.flac": "Media", "installer.dmg": "Software", "code.rs": "Development",
                    "archive.zip": "Unknown", "college_invoice.pdf": "Unknown", "नोट्स.txt": "Unknown"}
        for name, category in expected.items():
            self.assertEqual(rows[name]["classification"]["category"], category)
        validate_report(result)

    def test_project_prefix_does_not_capture_sibling(self):
        (self.root / "app-other").mkdir()
        (self.root / "app-other" / "lecture.txt").write_text("synthetic")
        row = next(e for e in scan(self.root)["entries"] if e["path"] == "app-other/lecture.txt")
        self.assertIsNone(row["project_root"])
        self.assertEqual(row["classification"]["category"], "Education")

    def test_symlink_marker_does_not_create_project(self):
        folder = self.root / "lookalike"
        folder.mkdir()
        (folder / "package.json").symlink_to(self.root / "lecture.txt")
        self.assertNotIn("lookalike", {p["path"] for p in scan(self.root)["projects"]})
