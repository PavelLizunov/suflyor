#!/usr/bin/env python3
"""Read-only receipt/Git artifact integrity, not rerunning tests or native acceptance."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

ARTIFACTS = {
    'schema_inventory': 'docs/agent-map/schema/inventory.json',
    'native_inventory': 'docs/agent-map/native/inventory.json',
    'name_edge_inventory': 'docs/agent-map/schema/config-ui-name-edges.json',
    'selected_SDK_inventory': 'docs/agent-map/native/windows-sdk-name-edges.json',
    'backend_SDK_inventory': 'docs/agent-map/native/backend-sdk-name-edges.json',
}


def git(root, *args):
    result = subprocess.run(['git', *args], cwd=root, capture_output=True, timeout=30)
    if result.returncode:
        raise ValueError('Git object unavailable: ' + ' '.join(args[:2]))
    return result.stdout


def verify(root, path):
    raw=path.read_bytes();v=json.loads(raw);sha=v.get('tested_commit');errors=[];checks=[]
    if not isinstance(sha,str) or len(sha)!=40 or any(c not in '0123456789abcdef' for c in sha):
        return {'receipt':str(path.relative_to(root)), 'errors':['invalid_exact_commit'], 'artifacts':[]}
    try:
        git(root,'cat-file','-e',sha+'^{commit}')
        snapshot=json.loads(git(root,'show',sha+':docs/agent-map/reconciliation/snapshot.json'))
        baseline=snapshot['source_commit']
    except (ValueError,KeyError):
        return {'receipt':str(path.relative_to(root)), 'tested_commit':sha, 'errors':['missing_tested_commit_or_snapshot'], 'artifacts':[]}
    absent_flags=[]
    for flag in ('native_application_acceptance','independent_acceptance','full_project_coverage'):
        if flag not in v:
            absent_flags.append(flag)  # Historical schema absence is not fabricated acceptance.
        elif v[flag] is not False:
            errors.append('invalid_acceptance_boundary:'+flag)

    def artifact(path, expected, size=None):
        try:
            data=git(root,'show',sha+':'+path);actual=hashlib.sha256(data).hexdigest()
        except ValueError:
            errors.append('missing_tested_artifact:'+path);return
        if actual!=expected or (size is not None and size!=len(data)):
            errors.append('tested_artifact_mismatch:'+path)
        checks.append({'path':path,'actual_sha256':actual,'expected_sha256':expected,'actual_bytes':len(data),'expected_bytes':size,'hash_and_size_match':actual==expected and (size is None or size==len(data))})
    for key,artifact_path in ARTIFACTS.items():
        if key in v:artifact(artifact_path,v[key]['sha256'],v[key].get('bytes'))
    for name,expected in v.get('artifact_sha256',{}).items():
        if name not in {'files.json','declarations.jsonl'}:errors.append('unexpected_syntax_artifact:'+name)
        else:artifact('docs/agent-map/syntax/'+name,expected)
    return {'receipt':str(path.relative_to(root)), 'receipt_sha256':hashlib.sha256(raw).hexdigest(),
            'tested_commit':sha,'source_baseline':baseline,'artifacts':checks,'errors':errors,'historical_absent_acceptance_flags':absent_flags,
            'tests_rerun_by_this_check':False,'native_independent_acceptance':False}


def build(root):
    files=sorted((root/'docs/agent-map/reconciliation').glob('portable-recovery*.json'))
    rows=[verify(root,p) for p in files]
    return {'schema_version':1,'method':'read-only Git object/hash/size/baseline and receipt-flag checks; not test rerun',
            'receipts':rows,'receipt_count':len(rows),'artifact_hash_checks':sum(len(r['artifacts']) for r in rows),
            'error_count':sum(len(r['errors']) for r in rows),'native_acceptance':False,'independent_acceptance':False,
            'limits':['receipt test counts are attestations; this tool does not re-execute them','no proof every claim/source semantics independently accepted','historic artifacts remain immutable; Git backing required for receipt integrity']}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3]);p.add_argument('--output',type=Path);a=p.parse_args();v=build(a.root.resolve())
    if a.output:a.output.write_text(json.dumps(v,indent=2)+'\n')
    print(json.dumps({'receipt_count':v['receipt_count'],'artifact_hash_checks':v['artifact_hash_checks'],'errors':v['error_count'],'tests_rerun':False}));raise SystemExit(bool(v['error_count']))


if __name__=='__main__':main()
