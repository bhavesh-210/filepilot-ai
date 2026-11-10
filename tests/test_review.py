import copy
from pathlib import Path
import unittest
from filepilot.dataset import load_records, validate_records
from filepilot.review import validate_review_pool

ROOT = Path(__file__).resolve().parents[1]


class ReviewTests(unittest.TestCase):
    def setUp(self):
        self.rows = load_records(ROOT / "ai/datasets/review-pool-v0.1.0/records.jsonl")

    def test_unlabeled_pool_is_not_ground_truth(self):
        result = validate_review_pool(self.rows)
        self.assertEqual(result["record_count"], 40)
        self.assertEqual(result["pending_count"], 40)
        self.assertEqual(result["human_reviewed_count"], 0)
        self.assertFalse(result["ready_for_development_scoring"])
        self.assertFalse(result["independent_final_test"])

    def test_review_requires_explicit_origin_and_evidence(self):
        row = copy.deepcopy(self.rows[0])
        row.update(review_label="Finance", reviewer="test reviewer", review_kind="human",
                   evidence_sufficient=True, review_note="synthetic test rationale")
        result = validate_review_pool([row])
        self.assertTrue(result["ready_for_development_scoring"])
        self.assertTrue(result["all_labels_human_reviewed"])
        self.assertFalse(result["independent_final_test"])
        row["review_kind"] = "ai"
        self.assertFalse(validate_review_pool([row])["all_labels_human_reviewed"])

    def test_invalid_reviews(self):
        changes = [{"review_label": "Finance"}, {"review_label": []}, {"reviewer": "invented reviewer"},
                   {"filename": "../escape"}, {"file_type": "invalid"}, {"purpose": "final_test"},
                   {"model_prediction": "Finance"}, {"review_note": []}]
        for change in changes:
            row = copy.deepcopy(self.rows[0])
            row.update(change)
            with self.subTest(change=change):
                with self.assertRaises(ValueError):
                    validate_review_pool([row])
        with self.assertRaises(ValueError):
            validate_review_pool([self.rows[0], self.rows[0]])

    def test_review_pool_cannot_enter_training_by_accident(self):
        with self.assertRaises(ValueError):
            validate_records(self.rows)

    def test_no_exact_overlap_with_training_snapshots(self):
        known = set()
        for name in ("starter-v0.1.0", "expanded-v0.1.0"):
            known.update(r["filename"].casefold() for r in load_records(ROOT / "ai/datasets" / name / "records.jsonl"))
        self.assertFalse(known & {r["filename"].casefold() for r in self.rows})
