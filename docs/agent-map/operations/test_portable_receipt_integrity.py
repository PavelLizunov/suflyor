"""Read-only Git artifact receipt checks; no native/test-result acceptance."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock
import verify_portable_receipts as receipt


class PortableReceiptFixtures(unittest.TestCase):
    def fixture(self,root,value):
        p=root/'portable.json';p.write_text(json.dumps(value));return p

    def base(self):
        return {'tested_commit':'a'*40,'native_application_acceptance':False,'independent_acceptance':False,'full_project_coverage':False}

    def git_mock(self,raw=b'artifact'):
        def fake(root,*args):
            if args[0]=='cat-file':return b''
            if args[-1].endswith('snapshot.json'):return b'{"source_commit":"frozen"}'
            return raw
        return fake

    def test_exact_hash_size_match_and_mismatch(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td);v=self.base();v['schema_inventory']={'sha256':hashlib.sha256(b'artifact').hexdigest(),'bytes':8};p=self.fixture(root,v)
            with mock.patch.object(receipt,'git',side_effect=self.git_mock()):
                result=receipt.verify(root,p);self.assertEqual(result['errors'],[]);self.assertFalse(result['tests_rerun_by_this_check'])
                v['schema_inventory']['bytes']=7;self.fixture(root,v);bad=receipt.verify(root,p)
            self.assertIn('tested_artifact_mismatch:docs/agent-map/schema/inventory.json',bad['errors'])

    def test_legacy_absent_flags_retained_not_rewritten_or_accepted(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td);v=self.base();del v['full_project_coverage'];p=self.fixture(root,v)
            with mock.patch.object(receipt,'git',side_effect=self.git_mock()):result=receipt.verify(root,p)
            self.assertEqual(result['errors'],[]);self.assertEqual(result['historical_absent_acceptance_flags'],['full_project_coverage'])
            self.assertFalse(result['native_independent_acceptance'])

    def test_true_acceptance_and_invalid_sha_fail_closed(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td);v=self.base();v['independent_acceptance']=True;p=self.fixture(root,v)
            with mock.patch.object(receipt,'git',side_effect=self.git_mock()):result=receipt.verify(root,p)
            self.assertIn('invalid_acceptance_boundary:independent_acceptance',result['errors'])
            v['tested_commit']='HEAD';self.fixture(root,v);self.assertIn('invalid_exact_commit',receipt.verify(root,p)['errors'])

    def test_unavailable_git_object_explicit_not_rerun(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td);p=self.fixture(root,self.base())
            with mock.patch.object(receipt,'git',side_effect=ValueError('missing')):result=receipt.verify(root,p)
            self.assertIn('missing_tested_commit_or_snapshot',result['errors'])


if __name__=='__main__':unittest.main()
