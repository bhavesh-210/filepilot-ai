import copy
import importlib.util
from pathlib import Path
import unittest
from filepilot.dataset import load_records
from filepilot.research_baseline import features, run, score_predictions


@unittest.skipUnless(importlib.util.find_spec("sklearn"), "Optional ML dependency")
class ResearchBaselineTests(unittest.TestCase):
    def setUp(self):
        self.rows = load_records(Path(__file__).resolve().parents[1] / "ai/datasets/starter-v0.1.0/records.jsonl")

    def test_provisional_gate(self):
        with self.assertRaises(ValueError):
            run(self.rows)

    def test_feature_allowlist(self):
        row = copy.deepcopy(self.rows[0])
        expected = features(row)
        row.update(category="Unknown", label_reason="secret", family_id="secret", split="test")
        self.assertEqual(features(row), expected)

    def test_split_isolation(self):
        before, _ = run(self.rows, True)
        for row in self.rows:
            if row["split"] in {"test", "validation"}:
                row["filename"] = "heldoutsentinel_" + row["filename"]
            if row["split"] == "test":
                row["category"] = "Personal"
        after, model = run(self.rows, True)
        self.assertEqual(before["train"], after["train"])
        self.assertFalse(any("heldoutsentinel" in word for word in model.named_steps["tfidf"].vocabulary_))
        self.assertEqual(after["train"]["examples"], 64)
        self.assertEqual(after["validation"]["examples"], 16)


    def test_test_labels_cannot_change_metrics(self):
        before, _ = run(self.rows, True)
        for row in self.rows:
            if row["split"] == "test":
                row["filename"] = "heldout_" + row["filename"]
                row["category"] = "Personal"
        after, _ = run(self.rows, True)
        self.assertEqual(before, after)

    def test_metrics_on_known_errors(self):
        rows = [dict(filename="a", category="Finance", family_id="one"),
                dict(filename="b", category="Education", family_id="two")]
        result = score_predictions(rows, ["Finance", "Finance"])
        self.assertEqual(result["accuracy"], .5)
        self.assertEqual(result["macro_f1"], (2 / 3) / 8)
        self.assertEqual(result["per_category"]["Finance"]["precision"], .5)
        self.assertEqual(result["family_count"], 2)
        self.assertEqual(result["family_mean_accuracy"], .5)
        self.assertEqual(sum(map(sum, result["confusion_matrix"])), 2)
        with self.assertRaises(ValueError):
            score_predictions([], [])
