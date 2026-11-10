"""Production React tests with synthetic native IPC and no personal files"""
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os
import threading
import unittest
from playwright.sync_api import sync_playwright
ROOT = Path(__file__).resolve().parents[1]
MOCK = '''
window.testCalls=[];
window.testSettings={mode:'full',rename:true,monitoring:true,personalization:true,trash_proposals:true,revision:0};
window.testHistory=[];
window.testFolder={id:1,path:'/synthetic/demo',active:true,initial_reviewed:!window.testSetup};
window.inventory=()=>({root_id:1,files:1,folders:['College'],items:['lecture.txt'],protected:1,errors:[],proposals:window.testExpired||window.testHistory.some(o=>o.state==='applied')?[]:[{id:7,root_id:1,source:window.syntheticSource||'lecture.txt',destination:'College/lecture.txt',category:'Education',reason:'Reuse an existing category folder',blocked:false,rename:false,certain:true}]});
window.__TAURI_INTERNALS__={
 transformCallback:(handler)=>{window.testTauriEvent=handler;return 1;},unregisterCallback:()=>{},
 invoke:async(command,args)=>{
  window.testCalls.push(command);
  if(command==='plugin:event|listen')return 1;
  if(command==='plugin:event|unlisten')return null;
  if(command==='snapshot')return {settings:window.testSettings,preferences:{reuse_existing:true},roots:[window.testFolder],inventories:window.testSetup&&!window.testPrepared?[]:[window.inventory()],history:window.testHistory,notice:''};
  if(command==='choose_folders')return {ids:[1],errors:[],cancelled:false};
  if(command==='prepare_setup'){window.testPrepared=true;return {token:42,inventory:window.inventory(),create_folders:[]};}
  if(command==='finish_setup'){if(args.token!==42)throw 'Wrong token';window.testFolder.initial_reviewed=true;return null;}
  if(command==='update_settings'){window.testSettings=args.settings;window.testExpired=true;return null;}
  if(command==='scan'){window.testExpired=false;return window.inventory();}
  if(command==='semantic_suggestions')return {available:true,content_used:true,model:'Synthetic model',ranking:[{category:'Education',score:0.8}]};
  if(command==='review_destination')return {...window.inventory().proposals[0],certain:false,reason:'User selected destination still requires explicit move approval'};
  if(command==='collection_title')return 'Important study files';
  if(command==='plan_collection'){window.collectionPlan={token:99,root_id:1,folder:args.folder,create_folder:args.createFolder,proposals:[{id:9,source:'lecture.txt',destination:args.folder+'/lecture.txt'}]};return window.collectionPlan;}
  if(command==='cancel_collection'){window.collectionPlan=null;return null;}
  if(command==='apply_collection'&&window.testCollectionFailure)return {operations:[],errors:['Interrupted change needs reconciliation']};
  if(command==='apply_collection'){window.testHistory=[{id:2,root_id:1,source:'lecture.txt',destination:window.collectionPlan.folder+'/lecture.txt',state:'applied',kind:'move'}];return {operations:[2],errors:[]};}
  if(command==='trash_file')return null;
  if(command==='apply'){window.testHistory=[{id:1,root_id:1,source:'lecture.txt',destination:'College/lecture.txt',state:'applied',kind:'move'}];return 1;}
  if(command==='undo'){window.testHistory[0].state='undone';return null;}
  return null;
 }
};
'''
class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *args): pass
class DesktopBrowserTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server=ThreadingHTTPServer(('127.0.0.1',0),partial(QuietHandler,directory=str(ROOT/'desktop/dist')))
        threading.Thread(target=cls.server.serve_forever,daemon=True).start()
        cls.runtime=sync_playwright().start()
        options={'headless':True}
        if os.environ.get('FILEPILOT_CHROMIUM'):options['executable_path']=os.environ['FILEPILOT_CHROMIUM']
        cls.browser=cls.runtime.chromium.launch(**options)
    @classmethod
    def tearDownClass(cls):
        cls.browser.close();cls.runtime.stop();cls.server.shutdown();cls.server.server_close()
    def open(self,native=True,setup=False):
        context=self.browser.new_context(viewport={'width':1440,'height':1050})
        self.addCleanup(context.close)
        if native:context.add_init_script(('window.testSetup=true;' if setup else '')+MOCK)
        page=context.new_page();self.errors=[]
        page.on('pageerror',lambda e:self.errors.append(str(e)))
        page.goto(f'http://127.0.0.1:{self.server.server_port}')
        page.get_by_role('heading',name='Welcome to FilePilot' if setup or not native else 'A calmer place for your files',exact=True).wait_for()
        return page
    def nav(self,page,label):page.get_by_role('navigation').get_by_role('button',name=label).click()
    def review(self,page):
        self.nav(page,'Review inbox')
        page.get_by_role('button',name='Approve this move',exact=True).wait_for()
    def test_browser_cannot_access_native_operations(self):
        page=self.open(False)
        self.nav(page,'Your folders')
        self.assertTrue(page.get_by_role('button',name='Choose folders',exact=True).is_disabled())
        self.nav(page,'Preferences')
        self.assertTrue(page.get_by_role('button',name='Enable login item').is_disabled())
        self.assertFalse(self.errors,self.errors)
    def test_one_time_setup_previews_before_enabling_maintenance(self):
        page=self.open(setup=True)
        page.get_by_role('button',name='Continue to folder access').click()
        page.get_by_role('button',name='Choose folders',exact=True).click()
        page.get_by_role('button',name='Understand my folders').click()
        page.get_by_text('Your first cleanup preview',exact=True).wait_for()
        page.get_by_role('button',name='Start maintaining my folders').wait_for(state='visible')
        self.assertNotIn('finish_setup',page.evaluate('window.testCalls'))
        self.assertNotIn('apply',page.evaluate('window.testCalls'))
        self.assertNotIn('create_categories',page.evaluate('window.testCalls'))
        page.get_by_role('button',name='Start maintaining my folders').click()
        page.get_by_role('heading',name='A calmer place for your files').wait_for()
        self.assertIn('finish_setup',page.evaluate('window.testCalls'))
        self.assertFalse(self.errors,self.errors)
    def test_review_inbox_does_not_need_a_manual_scan(self):
        page=self.open();self.review(page)
        self.assertNotIn('scan',page.evaluate('window.testCalls'))
        self.assertEqual(page.get_by_text('College/lecture.txt',exact=True).count(),1)
    def test_apply_and_undo(self):
        page=self.open();self.review(page)
        page.get_by_role('button',name='Approve this move').click()
        page.get_by_role('heading',name='Nothing waiting here').wait_for()
        self.nav(page,'Activity')
        page.get_by_role('button',name='Undo',exact=True).click()
        page.get_by_text('undone',exact=True).wait_for()
        self.assertFalse(self.errors,self.errors)
    def test_mode_change_expires_pending_actions(self):
        page=self.open();self.review(page);self.nav(page,'Preferences')
        page.get_by_label('Autonomy',exact=True).select_option('smart')
        page.get_by_text('Preferences saved pending proposals will be refreshed',exact=True).wait_for()
        self.nav(page,'Review inbox')
        self.assertEqual(page.get_by_role('button',name='Approve this move').count(),0)
    def test_semantic_selection_is_separate_from_move_approval(self):
        page=self.open();self.review(page)
        page.get_by_role('button',name='Inspect locally').click()
        page.get_by_role('button',name='Education',exact=True).click()
        page.get_by_text('Destination selected explicitly approve the move separately',exact=True).wait_for()
        self.assertNotIn('apply',page.evaluate('window.testCalls'))
        self.assertTrue(page.get_by_role('button',name='Approve this move').is_enabled())
    def test_untrusted_filename_is_only_text(self):
        page=self.open();page.evaluate("window.syntheticSource='<img src=x onerror=alert(1)>.txt';window.testTauriEvent({payload:{applied:false}})")
        self.review(page)
        self.assertEqual(page.get_by_text('<img src=x onerror=alert(1)>.txt',exact=True).count(),2)
        self.assertEqual(page.locator('img').count(),0)
        self.assertFalse(self.errors,self.errors)
    def test_trash_cancel_does_not_approve_any_move(self):
        page=self.open();self.review(page)
        page.get_by_text('More options',exact=True).click()
        page.get_by_role('button',name='Review Trash action').click()
        page.get_by_text('Trash cancelled no file moved',exact=True).wait_for()
        self.assertNotIn('apply',page.evaluate('window.testCalls'))
        self.assertTrue(page.get_by_role('button',name='Approve this move').is_enabled())
    def test_learning_is_explicit_and_does_not_move_files(self):
        page=self.open();self.review(page);page.get_by_text('More options',exact=True).click()
        page.get_by_role('button',name='Remember this destination').click()
        page.get_by_text('Explicit file example saved future nearest matches will still require review',exact=True).wait_for()
        self.assertNotIn('apply',page.evaluate('window.testCalls'))
    def test_background_refresh_updates_inbox_without_scan_click(self):
        page=self.open();self.review(page)
        page.evaluate('window.testTauriEvent({payload:{applied:false}})')
        self.assertTrue(page.get_by_role('button',name='Approve this move').is_visible())
        page.evaluate('window.testExpired=true;window.testTauriEvent({payload:{applied:true,root_ids:[1]}})')
        page.get_by_role('heading',name='Nothing waiting here').wait_for()
        self.assertEqual(page.get_by_role('button',name='Approve this move').count(),0)
    def test_pause_is_available_from_overview(self):
        page=self.open();page.get_by_role('button',name='Pause maintenance').click()
        page.get_by_role('button',name='Resume maintenance').wait_for()
        self.assertFalse(page.evaluate('window.testSettings.monitoring'))
    def test_small_screen_navigation_does_not_overflow(self):
        page=self.open();page.set_viewport_size({'width':390,'height':844})
        self.review(page)
        self.assertLessEqual(page.evaluate('document.documentElement.scrollWidth'),390)
        self.assertFalse(self.errors,self.errors)
    def test_named_collection_is_previewed_before_moving(self):
        page=self.open();self.nav(page,'Collections')
        page.get_by_label('Select lecture.txt',exact=True).check()
        page.get_by_role('button',name='Preview collection',exact=True).click()
        page.get_by_role('button',name='Confirm collection',exact=True).wait_for()
        self.assertNotIn('apply_collection',page.evaluate('window.testCalls'))
        self.assertEqual(page.get_by_label('Collection folder title').input_value(),'Important study files')
        page.get_by_role('button',name='Confirm collection',exact=True).click()
        page.get_by_text('1 files added to Important study files use Activity to undo each move',exact=True).wait_for()
        self.nav(page,'Activity');self.assertTrue(page.get_by_role('button',name='Undo',exact=True).is_enabled())
        self.assertFalse(self.errors,self.errors)
    def test_existing_folder_collection_preserves_the_destination(self):
        page=self.open();self.nav(page,'Collections');page.get_by_label('Select lecture.txt',exact=True).check()
        page.get_by_label('Collection destination type').select_option('existing')
        page.get_by_label('Existing collection folder').select_option('College')
        page.get_by_role('button',name='Preview collection',exact=True).click()
        page.get_by_role('button',name='Confirm collection',exact=True).wait_for()
        self.assertFalse(page.evaluate('window.collectionPlan.create_folder'))
        self.assertEqual(page.evaluate('window.collectionPlan.folder'),'College')
        self.assertNotIn('apply_collection',page.evaluate('window.testCalls'))
    def test_editing_a_collection_requires_a_new_preview(self):
        page=self.open();self.nav(page,'Collections');page.get_by_label('Select lecture.txt',exact=True).check()
        page.get_by_label('Collection folder title').fill('My exam essentials')
        page.get_by_role('button',name='Preview collection',exact=True).click()
        page.get_by_role('button',name='Edit selection',exact=True).click()
        page.get_by_role('button',name='Preview collection',exact=True).wait_for(state='visible')
        self.assertEqual(page.get_by_role('button',name='Confirm collection').count(),0)
        self.assertIn('cancel_collection',page.evaluate('window.testCalls'))
        self.assertNotIn('apply_collection',page.evaluate('window.testCalls'))
    def test_selected_unwanted_file_still_needs_native_trash_confirmation(self):
        page=self.open();self.nav(page,'Collections');page.get_by_label('Select lecture.txt',exact=True).check()
        page.get_by_role('button',name='Move selected to Trash',exact=True).click()
        page.get_by_text('Trash cancelled no files moved',exact=True).wait_for()
        self.assertTrue(page.get_by_label('Select lecture.txt',exact=True).is_checked())
        self.assertNotIn('apply_collection',page.evaluate('window.testCalls'))
        self.assertEqual(page.evaluate('window.testHistory.length'),0)
    def test_collection_interruption_does_not_claim_no_files_changed(self):
        page=self.open();self.nav(page,'Collections');page.get_by_label('Select lecture.txt',exact=True).check()
        page.get_by_role('button',name='Preview collection',exact=True).click()
        page.get_by_role('button',name='Confirm collection',exact=True).wait_for()
        page.evaluate('window.testCollectionFailure=true')
        page.get_by_role('button',name='Confirm collection',exact=True).click()
        page.get_by_text('0 moves verified before work stopped Interrupted change needs reconciliation Check Activity before trying again',exact=True).wait_for()
        self.assertFalse(self.errors,self.errors)
    def test_visual_snapshots(self):
        directory=ROOT/'desktop/test-results';directory.mkdir(exist_ok=True)
        page=self.open();page.screenshot(path=str(directory/'overview.png'),full_page=True)
        self.review(page);page.screenshot(path=str(directory/'review.png'),full_page=True)
        self.nav(page,'Collections');page.screenshot(path=str(directory/'collections.png'),full_page=True)
        page=self.open(setup=True);page.screenshot(path=str(directory/'setup.png'),full_page=True)
        self.assertFalse(self.errors,self.errors)
if __name__=='__main__':unittest.main(verbosity=2)
