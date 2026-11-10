import copy
from pathlib import Path
import sqlite3
import tempfile
import unittest
from filepilot.scanner import scan
from filepilot.policy import Mode, PolicySettings, decide
from filepilot.ranking import rank_destinations, ranking_metrics, relative_path
from filepilot.preferences import Preferences
from filepilot.planner import build_plan


class PolicyTests(unittest.TestCase):
    def decision(self, mode=Mode.FULL, **changes):
        args=dict(authorized=True, safe=True, certain=True, proposal_revision=0)
        args.update(changes)
        return decide('move', PolicySettings(mode=mode), **args)

    def test_autonomy_modes(self):
        self.assertTrue(self.decision()['allowed'])
        self.assertFalse(self.decision(Mode.SMART)['allowed'])
        self.assertTrue(self.decision(Mode.SMART, approved=True)['allowed'])
        self.assertFalse(self.decision(Mode.LIMITED)['allowed'])
        self.assertTrue(self.decision(Mode.LIMITED, rule_allowed=True)['allowed'])

    def test_safety_and_uncertainty(self):
        for key in ('authorized', 'safe', 'certain'):
            self.assertFalse(self.decision(**{key: False})['allowed'])
        self.assertTrue(self.decision(certain=False, approved=True)['allowed'])

    def test_policy_changes_invalidate_pending_approval(self):
        self.assertFalse(self.decision(Mode.SMART, approved=True, proposal_revision=1)['allowed'])
        self.assertFalse(self.decision(proposal_revision=None)['allowed'])
        self.assertFalse(self.decision(proposal_revision=True)['allowed'])

    def test_destructive_confirmation_is_separate(self):
        for mode in Mode:
            for action in ('trash', 'delete'):
                for confirmation, expected in ((False, False), (True, True)):
                    result=decide(action,PolicySettings(mode),authorized=True,safe=True,certain=True,
                                  approved=True,separately_confirmed=confirmation,proposal_revision=0)
                    self.assertEqual(result['allowed'], expected)

    def test_rename_and_invalid_settings(self):
        self.assertFalse(decide('rename',PolicySettings(rename_enabled=False), authorized=True,
                               safe=True,certain=True,approved=True,proposal_revision=0)['allowed'])
        self.assertFalse(decide('shell',PolicySettings())['allowed'])
        with self.assertRaises(ValueError): PolicySettings(mode='full')
        with self.assertRaises(ValueError): self.decision(safe=1)


class RankingTests(unittest.TestCase):
    def test_category_and_lexical_ranking(self):
        folders=[dict(path='College/Lectures',category='Education'),dict(path='Accounts',category='Finance')]
        self.assertEqual(rank_destinations('lecture.pdf','Education',folders)[0]['path'],'College/Lectures')
        self.assertEqual(rank_destinations('receipt.pdf','Finance',folders)[0]['path'],'Accounts')
        self.assertEqual(rank_destinations('untitled.pdf','Unknown',folders),[])

    def test_explicit_preference_and_determinism(self):
        folders=[dict(path='A',category='Education'),dict(path='B',category='Education')]
        self.assertEqual(rank_destinations('lecture.txt','Education',folders)[0]['path'],'A')
        self.assertEqual(rank_destinations('lecture.txt','Education',folders,preferred='B')[0]['path'],'B')
        self.assertEqual(rank_destinations('lecture.txt','Education',folders,preferred='Missing')[0]['path'],'A')

    def test_bad_candidates_and_paths(self):
        for path in ('../a','/a','a//b','a/./b','a\\b','a\0b'):
            with self.assertRaises(ValueError): relative_path(path)
        with self.assertRaises(ValueError): rank_destinations('x','Unknown',[dict(path='a',category='bad')])
        with self.assertRaises(ValueError): rank_destinations('x','Unknown',[],text='x'*16001)

    def test_ranking_metrics(self):
        result=ranking_metrics([dict(expected='B',ranked=['A','B']),dict(expected='A',ranked=['A'])])
        self.assertEqual(result['recall_at_3'],1)
        self.assertEqual(result['destination_accuracy'],.5)
        self.assertEqual(result['mean_reciprocal_rank'],.75)
        self.assertFalse(result['independent_benchmark'])


class PreferenceTests(unittest.TestCase):
    def test_explicit_corrections_disabled_and_cleared(self):
        db=sqlite3.connect(':memory:');self.addCleanup(db.close)
        preferences=Preferences(db)
        with self.assertRaises(ValueError): preferences.correct('demo','Education','College')
        preferences.correct('demo','Education','College',explicit=True)
        self.assertEqual(preferences.destination('demo','Education'),'College')
        self.assertIsNone(preferences.destination('other','Education'))
        preferences.set_enabled(False)
        self.assertIsNone(preferences.destination('demo','Education'))
        self.assertFalse(preferences.correct('demo','Education','New',explicit=True))
        preferences.set_enabled(True)
        self.assertEqual(preferences.destination('demo','Education'),'College')
        preferences.clear()
        self.assertIsNone(preferences.destination('demo','Education'))

    def test_persistence_and_parameterized_values(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'app.sqlite'
            with sqlite3.connect(path) as db:
                Preferences(db).correct('demo','Personal',"College/John's notes",explicit=True)
            with sqlite3.connect(path) as db:
                self.assertEqual(Preferences(db).destination('demo','Personal'),"College/John's notes")


class PlannerTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name).resolve()
        (self.root/'College').mkdir();(self.root/'Accounts').mkdir()
        (self.root/'lecture.txt').write_text('synthetic')
        (self.root/'receipt.txt').write_text('synthetic')
        (self.root/'unknown.txt').write_text('synthetic')
        (self.root/'project').mkdir();(self.root/'project/package.json').write_text('{}')
        (self.root/'project/invoice.py').write_text('synthetic')
        self.folders=[dict(path='College',category='Education'),dict(path='Accounts',category='Finance')]

    def test_read_only_preview_and_project_protection(self):
        report=scan(self.root); before={str(p.relative_to(self.root)):p.stat().st_mtime_ns for p in self.root.rglob('*')}
        plan=build_plan(report,self.folders)
        self.assertEqual(plan['summary']['proposals'],3)
        self.assertTrue(all(not p['executable'] and p['review_required'] and not p['policy']['allowed'] for p in plan['proposals']))
        self.assertIn('project/invoice.py',[p['path'] for p in plan['untouched']])
        self.assertEqual(next(p for p in plan['proposals'] if p['source']=='lecture.txt')['destination'],'College/lecture.txt')
        self.assertEqual(before,{str(p.relative_to(self.root)):p.stat().st_mtime_ns for p in self.root.rglob('*')})

    def test_collision_casefold_and_unicode(self):
        (self.root/'College/LECTURE.txt').write_text('existing')
        plan=build_plan(scan(self.root),self.folders)
        self.assertEqual(next(p for p in plan['proposals'] if p['source']=='lecture.txt')['reason'],'destination_collision')

    def test_incomplete_inventory_blocks_proposals(self):
        report=scan(self.root,max_entries=1)
        # Use an unrestricted report to keep folder candidates available
        report=scan(self.root); report['truncated']=True
        self.assertTrue(all(p['blocked'] for p in build_plan(report,self.folders)['proposals']))

    def test_project_and_missing_destinations_rejected(self):
        for path in ('project','missing','lecture.txt'):
            with self.assertRaises(ValueError): build_plan(scan(self.root),[dict(path=path,category='Education')])

    def test_same_folder_and_scoped_preferences(self):
        (self.root/'College/lecture_old.txt').write_text('synthetic')
        report=scan(self.root)
        plan=build_plan(report,self.folders)
        self.assertIn(dict(path='College/lecture_old.txt',reason='already_in_destination'),plan['untouched'])
        with sqlite3.connect(':memory:') as db:
            preferences=Preferences(db);preferences.correct('demo','Education','Accounts',explicit=True)
            plan=build_plan(report,self.folders,preferences=preferences,scope='demo')
            self.assertEqual(next(p for p in plan['proposals'] if p['source']=='lecture.txt')['destination'],'Accounts/lecture.txt')

    def test_malformed_report_rejected(self):
        report=scan(self.root); report['read_only']=False
        with self.assertRaises(ValueError): build_plan(report,self.folders)
