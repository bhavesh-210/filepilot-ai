import copy
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from filepilot.check import check_expansion, check_review_overlap, collect_checks, main, run_test_suite
from filepilot.dataset import load_records

ROOT = Path(__file__).resolve().parents[1]


class CheckTests(unittest.TestCase):
    def test_core_checks_do_not_train_models(self):
        with patch("filepilot.check.evaluate_ml", side_effect=AssertionError("ML was requested")):
            report = collect_checks(ROOT, include_tests=False)
        self.assertTrue(report["passed"], report)
        self.assertFalse(report["test_split_scored"])
        review = next(c for c in report["checks"] if c["name"] == "development review pool")
        self.assertEqual(review["details"]["pending_count"], 40)
        self.assertEqual(report["checks"][-1]["status"], "not_requested")

    def test_failed_tests_fail_the_overall_check(self):
        with patch("filepilot.check.run_test_suite", return_value={"passed": False}):
            report = collect_checks(ROOT)
        self.assertFalse(report["passed"])

    def test_missing_project_inputs_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            report = collect_checks(Path(directory), include_tests=False)
        self.assertFalse(report["passed"])
        self.assertTrue(any(c["status"] == "failed" for c in report["checks"]))

    def test_modified_original_record_is_rejected(self):
        original = load_records(ROOT / "ai/datasets/starter-v0.1.0/records.jsonl")
        expanded = copy.deepcopy(original)
        expanded[0]["category"] = "Unknown"
        with self.assertRaises(ValueError):
            check_expansion(original, expanded)

    def test_review_overlap_is_rejected(self):
        rows = [{"filename": "invoice.pdf"}]
        with self.assertRaises(ValueError):
            check_review_overlap([{"filename": "INVOICE.pdf"}], {"train": rows})

    def test_cli_exit_status_and_json(self):
        for passed, expected in ((True, 0), (False, 1)):
            with patch("filepilot.check.collect_checks", return_value={"passed": passed}), \
                 patch("sys.stdout", new_callable=io.StringIO) as output:
                self.assertEqual(main([]), expected)
                self.assertEqual(json.loads(output.getvalue())["passed"], passed)

    def test_no_tests_is_not_a_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "tests").mkdir()
            with self.assertRaises(ValueError):
                run_test_suite(root, False)
