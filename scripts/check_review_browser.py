"""Exercise the offline review screen in Chromium using synthetic data only"""
import json
import os
from pathlib import Path
import tempfile
import unittest
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]


class BrowserTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runtime=sync_playwright().start()
        cls.browser=cls.runtime.chromium.launch(headless=True, executable_path=os.environ.get('FILEPILOT_TEST_CHROMIUM'), args=['--no-sandbox'])

    @classmethod
    def tearDownClass(cls):
        cls.browser.close();cls.runtime.stop()

    def setUp(self):
        self.context=self.browser.new_context(accept_downloads=True)
        self.addCleanup(self.context.close)
        self.page=self.context.new_page()
        self.errors=[];self.requests=[]
        self.page.on('pageerror',lambda error:self.errors.append(str(error)))
        self.page.on('request',lambda request:self.requests.append(request.url))
        self.page.goto((ROOT/'frontend/index.html').as_uri())

    def test_demo_review_mode_change_and_clear(self):
        self.page.get_by_role('button',name='Load synthetic demo').click()
        self.assertEqual(self.page.locator('#count').inner_text(),'3')
        self.assertEqual(self.page.locator('#blocked').inner_text(),'1')
        self.page.get_by_role('button',name='Mark reviewed lecture_notes.txt',exact=True).click()
        self.assertEqual(self.page.locator('#reviewed').inner_text(),'1')
        self.page.locator('#mode').select_option('smart')
        self.assertEqual(self.page.locator('#reviewed').inner_text(),'0')
        self.page.get_by_role('button',name='Clear session').click()
        self.assertEqual(self.page.locator('#count').inner_text(),'0')
        self.assertFalse(self.errors,self.errors)
        self.assertFalse([url for url in self.requests if url.startswith(('http:','https:'))])

    def test_export_has_no_execution_permission(self):
        self.page.get_by_role('button',name='Load synthetic demo').click()
        self.page.get_by_role('button',name='Mark reviewed receipt.pdf',exact=True).click()
        with self.page.expect_download() as event:
            self.page.get_by_role('button',name='Export review choices').click()
        result=json.loads(Path(event.value.path()).read_text())
        self.assertFalse(result['execution_authorized'])
        self.assertEqual(result['report_type'],'review_choices')
        self.assertEqual(result['reviewed_proposals'],['2'*24])

    def test_import_treats_markup_as_data(self):
        self.page.get_by_role('button',name='Load synthetic demo').click()
        value=self.page.evaluate('structuredClone(demo)')
        value['proposals'][0]['source']='<img src=x onerror=alert(1)>.txt'
        self.page.locator('#import').set_input_files(dict(name='synthetic.json',mimeType='application/json',buffer=json.dumps(value).encode()))
        self.page.get_by_text(value['proposals'][0]['source'],exact=True).wait_for()
        self.assertEqual(self.page.locator('img').count(),0)
        self.assertFalse(self.errors,self.errors)

    def test_invalid_import_preserves_current_plan(self):
        self.page.get_by_role('button',name='Load synthetic demo').click()
        self.page.locator('#import').set_input_files(dict(name='invalid.json',mimeType='application/json',buffer=b'{}'))
        self.page.get_by_text('Could not open this file as a valid bounded organisation plan').wait_for()
        self.assertEqual(self.page.locator('#count').inner_text(),'3')

    def test_narrow_layout(self):
        self.page.set_viewport_size(dict(width=390,height=844))
        self.page.get_by_role('button',name='Load synthetic demo').click()
        self.assertLessEqual(self.page.evaluate('document.documentElement.scrollWidth'),390)
        with tempfile.TemporaryDirectory() as folder:
            self.page.screenshot(path=str(Path(folder)/'review.png'),full_page=True)
        self.assertFalse(self.errors,self.errors)


if __name__=='__main__':
    unittest.main(verbosity=2)
