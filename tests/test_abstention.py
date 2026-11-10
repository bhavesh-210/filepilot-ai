import unittest
from filepilot.abstention import decide, summarize


class AbstentionTests(unittest.TestCase):
    def test_threshold_and_unknown(self):
        self.assertTrue(decide([.8, .2], ["Finance", "Unknown"], .8)["suggestion_eligible"])
        self.assertEqual(decide([.8, .2], ["Finance", "Unknown"], .81)["category"], "Unknown")
        self.assertFalse(decide([.1, .9], ["Finance", "Unknown"], 0)["suggestion_eligible"])
        self.assertFalse(decide([.5, .5], ["Finance", "Personal"], 0)["suggestion_eligible"])

    def test_invalid_scores(self):
        for scores, classes, threshold in [([1], ["Finance"], -1), ([1], ["Finance"], float("nan")),
                ([float("nan")], ["Finance"], 0), ([.8], ["Finance"], 0),
                ([1], ["Invalid"], 0), ([.5, .5], ["Finance", "Finance"], 0), ([], [], 0)]:
            with self.subTest(scores=scores, threshold=threshold):
                with self.assertRaises(ValueError):
                    decide(scores, classes, threshold)

    def test_selective_metrics(self):
        rows = [{"category": "Finance"}, {"category": "Unknown"}, {"category": "Personal"}]
        decisions = [decide([.8, .2], ["Finance", "Unknown"], .5),
                     decide([.8, .2], ["Finance", "Unknown"], .5),
                     decide([.4, .6], ["Personal", "Unknown"], .5)]
        result = summarize(rows, decisions)
        self.assertEqual(result["suggestions"], 2)
        self.assertEqual(result["suggestion_accuracy"], .5)
        self.assertEqual(result["wrong_suggestions"], 1)
        self.assertEqual(result["known_category_deferred"], 1)
        self.assertEqual(result["unknown_category_suggested"], 1)

    def test_all_review_is_not_perfect_accuracy(self):
        decision = decide([.8, .2], ["Finance", "Unknown"], 1)
        result = summarize([{"category": "Finance"}], [decision])
        self.assertEqual(result["coverage"], 0)
        self.assertIsNone(result["suggestion_accuracy"])
        self.assertTrue(decision["review_required"])
        with self.assertRaises(ValueError):
            summarize([], [])
