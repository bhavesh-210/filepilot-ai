import copy
from pathlib import Path
import tempfile
import unittest
from filepilot.dataset import load_records, validate_records

DATA = Path(__file__).resolve().parents[1] / "ai/datasets/starter-v0.1.0/records.jsonl"


class DatasetTests(unittest.TestCase):
    def setUp(self):
        self.rows = load_records(DATA)

    def test_starter_inventory(self):
        result = validate_records(self.rows)
        self.assertEqual(result["record_count"], 102)
        self.assertEqual(result["family_count"], 54)
        self.assertFalse(result["training_eligible"])
        self.assertEqual(result["missing_category_split_coverage"], {})
        self.assertEqual(result["policies"]["leave_untouched"], 6)

    def test_family_leakage(self):
        self.rows[1]["split"] = "test"
        with self.assertRaisesRegex(ValueError, "family leakage"):
            validate_records(self.rows)

    def test_invalid_records(self):
        changes = [{"category": "Banking"}, {"filename": "../secret"}, {"filename": "/tmp/secret"},
                   {"filename": "a\\b"}, {"split": "evaluation"}, {"reviewed": "yes"},
                   {"expected_policy": "move"}, {"schema_version": "99"},
                   {"category": "Unknown", "expected_policy": "suggest_category"},
                   {"provenance": {"kind": "personal"}}]
        for change in changes:
            with self.subTest(change=change):
                rows = copy.deepcopy(self.rows)
                rows[0].update(change)
                with self.assertRaises(ValueError):
                    validate_records(rows)

    def test_duplicate_ids_and_names(self):
        for field in ("id", "filename"):
            with self.subTest(field=field):
                rows = copy.deepcopy(self.rows)
                rows[1][field] = rows[0][field]
                with self.assertRaises(ValueError):
                    validate_records(rows)
        self.rows[1]["filename"] = self.rows[0]["filename"].upper()
        with self.assertRaises(ValueError):
            validate_records(self.rows)

    def test_review_and_rights_gate(self):
        for row in self.rows:
            row["reviewed"] = True
        self.assertFalse(validate_records(self.rows)["training_eligible"])
        for row in self.rows:
            row["provenance"]["rights_status"] = "approved"
        self.assertTrue(validate_records(self.rows)["training_eligible"])

    def test_malformed_jsonl(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "bad.jsonl"
            for content in ("{broken}\n", "\n"):
                path.write_text(content)
                with self.assertRaises(ValueError):
                    load_records(path)
            path.write_text("")
            with self.assertRaises(ValueError):
                validate_records(load_records(path))

    def test_generator_reproduces_records(self):
        import runpy
        module = runpy.run_path(str(DATA.with_name("build_starter.py")))
        self.assertEqual(module["build"](), self.rows)


    def test_invalid_provenance_types(self):
        for value in ([], None, True, {}):
            rows = copy.deepcopy(self.rows)
            rows[0]["provenance"]["rights_status"] = value
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    validate_records(rows)

    def test_duplicate_json_keys(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.jsonl"
            path.write_text('{"category":"Finance","category":"Personal"}\n')
            with self.assertRaisesRegex(ValueError, "Duplicate JSON key"):
                load_records(path)

    def test_missing_category_cannot_be_training_eligible(self):
        rows = [copy.deepcopy(row) for row in self.rows if row["category"] != "Media"]
        for row in rows:
            row["reviewed"] = True
            row["provenance"]["rights_status"] = "approved"
        result = validate_records(rows)
        self.assertFalse(result["training_eligible"])
        self.assertIn("Media", result["missing_category_split_coverage"])
