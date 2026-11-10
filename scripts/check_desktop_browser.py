"""Browser tests for the production React bundle using a synthetic IPC mock"""
from functools import partial
from http.server import SimpleHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
import threading
import unittest
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[1]
MOCK='''
window.__TAURI_INTERNALS__={
 transformCallback:()=>1, unregisterCallback:()=>{},
 invoke:async(command,args)=>{
  if(command==='plugin:event|listen')return 1;
  if(command==='plugin:event|unlisten')return null;
  if(command==='snapshot')return {settings:{mode:'full',rename:true,monitoring:true,personalization:true,trash_proposals:true,revision:0},roots:[{id:1,path:'/synthetic/demo',active:true,initial_reviewed:false}],history:window.testHistory||[],notice:''};
  if(command==='scan')return {root_id:1,files:1,protected:1,errors:[],proposals:[{id:7,source:window.syntheticSource||'lecture.txt',destination:'Education/lecture.txt',category:'Education',reason:'Category destination rule',blocked:false,rename:false}]};
  if(command==='semantic_suggestions')return {available:true,content_used:true,model:'Synthetic test model',ranking:[{category:'Education',score:0.8}],scores_are_probabilities:false};
  if(command==='review_destination')return {id:7,source:'lecture.txt',destination:'Education/lecture.txt',category:'Education',reason:'User selected destination still requires explicit move approval',blocked:false,rename:false};
  if(command==='trash_file')return null;
  if(command==='apply'){window.testHistory=[{id:1,root_id:1,source:'lecture.txt',destination:'Education/lecture.txt',state:'applied'}];return 1;}
  if(command==='undo'){window.testHistory[0].state='undone';return null;}
  return null;
 }
};
'''

class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self,*args):pass

class DesktopBrowserTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server=ThreadingHTTPServer(('127.0.0.1',0),partial(QuietHandler,directory=str(ROOT/'desktop/dist')))
        cls.thread=threading.Thread(target=cls.server.serve_forever,daemon=True);cls.thread.start()
        cls.runtime=sync_playwright().start();cls.browser=cls.runtime.chromium.launch(headless=True)
    @classmethod
    def tearDownClass(cls):
        cls.browser.close();cls.runtime.stop();cls.server.shutdown();cls.server.server_close()
    def open(self,native=False):
        context=self.browser.new_context();self.addCleanup(context.close)
        if native:context.add_init_script(MOCK)
        page=context.new_page();self.errors=[]
        page.on('pageerror',lambda e:self.errors.append(str(e)))
        page.goto(f'http://127.0.0.1:{self.server.server_port}')
        page.get_by_text('Your files under control').wait_for()
        return page
    def test_browser_preview_cannot_access_files(self):
        page=self.open()
        self.assertTrue(page.get_by_role('button',name='Choose folder').is_disabled())
        self.assertTrue(page.get_by_role('button',name='Enable login item').is_disabled())
        self.assertFalse(self.errors,self.errors)
    def test_synthetic_scan_apply_and_undo(self):
        page=self.open(True)
        page.get_by_role('button',name='demo',exact=True).click()
        page.get_by_role('button',name='Scan and preview').click()
        page.get_by_text('Education/lecture.txt',exact=True).wait_for()
        page.get_by_role('button',name='Approve this move').click()
        page.get_by_role('button',name='Undo',exact=True).wait_for()
        page.get_by_role('button',name='Undo',exact=True).click()
        page.get_by_text('undone',exact=True).wait_for()
        self.assertFalse(self.errors,self.errors)
    def test_mode_change_invalidates_preview(self):
        page=self.open(True)
        page.get_by_role('button',name='demo',exact=True).click()
        page.get_by_role('button',name='Scan and preview').click()
        page.get_by_role('button',name='Approve this move').wait_for()
        page.get_by_label('Autonomy',exact=True).select_option('smart')
        page.get_by_text('Settings saved pending proposals expired',exact=True).wait_for()
        self.assertEqual(page.get_by_role('button',name='Approve this move').count(),0)
        self.assertFalse(self.errors,self.errors)

    def test_semantic_choice_still_needs_explicit_move(self):
        page=self.open(True)
        page.get_by_role('button',name='demo',exact=True).click()
        page.get_by_role('button',name='Scan and preview').click()
        page.get_by_role('button',name='Inspect locally').click()
        page.get_by_role('button',name='Education',exact=True).click()
        page.get_by_text('Destination selected explicitly approve the move separately',exact=True).wait_for()
        self.assertEqual(page.get_by_text('No file operations yet',exact=True).count(),1)
        self.assertTrue(page.get_by_role('button',name='Approve this move').is_enabled())
        self.assertFalse(self.errors,self.errors)
    def test_untrusted_filename_is_text(self):
        page=self.open(True)
        page.evaluate("window.syntheticSource='<img src=x onerror=alert(1)>.txt'")
        page.get_by_role('button',name='demo',exact=True).click()
        page.get_by_role('button',name='Scan and preview').click()
        page.get_by_text('<img src=x onerror=alert(1)>.txt',exact=True).wait_for()
        self.assertEqual(page.locator('img').count(),0)
        self.assertFalse(self.errors,self.errors)

    def test_trash_cancellation_is_not_move_approval(self):
        page=self.open(True)
        page.get_by_role('button',name='demo',exact=True).click()
        page.get_by_role('button',name='Scan and preview').click()
        page.get_by_role('button',name='Review Trash action').click()
        page.get_by_text('Trash cancelled no file moved',exact=True).wait_for()
        self.assertEqual(page.get_by_text('No file operations yet',exact=True).count(),1)
        self.assertTrue(page.get_by_role('button',name='Approve this move').is_enabled())
        self.assertFalse(self.errors,self.errors)

    def test_remembering_a_destination_does_not_approve_a_move(self):
        page=self.open(True)
        page.get_by_role('button',name='demo',exact=True).click()
        page.get_by_role('button',name='Scan and preview').click()
        page.get_by_role('button',name='Remember this destination').click()
        page.get_by_text('Explicit file example saved future nearest matches will still require review',exact=True).wait_for()
        self.assertEqual(page.get_by_text('No file operations yet',exact=True).count(),1)
        self.assertFalse(self.errors,self.errors)

if __name__=='__main__':unittest.main(verbosity=2)
