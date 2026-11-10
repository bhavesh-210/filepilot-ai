import unittest
from filepilot.scanner import classify


class SourcePriorityTests(unittest.TestCase):
    def test_source_domain_keywords(self):
        for name in ("invoice_parser.java", "college_timer.py", "resume_builder.tsx"):
            with self.subTest(name=name):
                result = classify(name, "source")
                self.assertEqual(result["category"], "Development")
                self.assertTrue(result["review_required"])

    def test_project_still_preserved(self):
        result = classify("invoice_parser.java", "source", "demo-project")
        self.assertEqual(result["recommendation"], "leave_untouched")

    def test_document_rules_unchanged(self):
        self.assertEqual(classify("invoice.pdf", "document")["category"], "Finance")
        self.assertEqual(classify("lecture.pdf", "document")["category"], "Education")
