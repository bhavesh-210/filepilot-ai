"""Optional browser checks run separately from the standard library suite"""
from pathlib import Path
import unittest


class ReviewUiContractTests(unittest.TestCase):
    def test_no_network_or_privileged_bridge(self):
        html=(Path(__file__).resolve().parents[1]/'frontend/index.html').read_text()
        self.assertIn("connect-src 'none'",html)
        self.assertIn("execution_authorized:false",html)
        self.assertNotIn('innerHTML',html)
        self.assertNotIn('localStorage',html)
        self.assertNotIn('fetch(',html)
