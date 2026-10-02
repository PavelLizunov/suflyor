#!/usr/bin/env python3
"""Source-linked interval navigation gaps, NOT reviewed lines/semantic coverage."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path


def merge_ranges(ranges, lines):
    """Inclusive source intervals, reject invalid; never count a directory pointer."""
    accepted=[]
    for start,end in ranges:
        if isinstance(start,bool) or isinstance(end,bool) or not isinstance(start,int) or not isinstance(end,int) or not 1<=start<=end<=lines:
            raise ValueError('invalid navigation range')
        accepted.append((start,end))
    merged=[]
    for start,end in sorted(accepted):
        if merged and start<=merged[-1][1]+1:merged[-1]=(merged[-1][0],max(end,merged[-1][1]))
        else:merged.append((start,end))
    return merged


def gaps(merged,lines):
    out=[];next_line=1
    for start,end in merged:
        if next_line<start:out.append((next_line,start-1))
        next_line=end+1
    if next_line<=lines:out.append((next_line,lines))
    return out


def build(root):
    maproot=root/'docs/agent-map';paths={'syntax':'syntax/files.json','features':'features/contracts.json','candidates':'reconciliation/candidates.json','snapshot':'reconciliation/snapshot.json'}
    inputs={key:{'path':'docs/agent-map/'+relative,'sha256':hashlib.sha256((maproot/relative).read_bytes()).hexdigest()} for key,relative in paths.items()}
    files=json.loads((maproot/paths['syntax']).read_text());contracts=json.loads((maproot/paths['features']).read_text());candidates=json.loads((maproot/paths['candidates']).read_text());snap=json.loads((maproot/paths['snapshot']).read_text())
    if files['source_commit']!=snap['source_commit']:raise ValueError('baseline mismatch')
    refs={};nonranges=[]
    def collect(rows,owner,kind):
        for row in rows:
            if 'start_line' not in row:
                nonranges.append({'owner':owner,'kind':kind,'path':row['path'],'reason':'directory_or_unranged_pointer_no_line_credit'});continue
            refs.setdefault(row['path'],[]).append({'owner':owner,'kind':kind,'start_line':row['start_line'],'end_line':row['end_line']})
    for f in contracts['features']:collect(f['source_references'],f['id'],'feature_contract')
    for c in candidates:collect(c['source_references'],c['id'],'original_candidate_source_triage')
    rows=[]
    for file in files['files']:
        if file['status']!='syntax_parsed':continue
        path=file['path'];data=(root/path).read_bytes();lines=len(data.splitlines())
        if hashlib.sha256(data).hexdigest()!=file['actual_sha256']:raise ValueError('source drift: '+path)
        here=refs.get(path,[]);feature=[(r['start_line'],r['end_line']) for r in here if r['kind']=='feature_contract'];candidate=[(r['start_line'],r['end_line']) for r in here if r['kind']=='original_candidate_source_triage']
        union=merge_ranges(feature+candidate,lines);fu=merge_ranges(feature,lines);cu=merge_ranges(candidate,lines)
        count=lambda rr:sum(b-a+1 for a,b in rr)
        rows.append({'path':path,'language_extension':file['extension'],'source_sha256':file['actual_sha256'],'source_lines':lines,'syntax_nodes':file['declaration_count'],
                     'feature_reference_line_union':count(fu),'candidate_reference_line_union':count(cu),'all_reference_line_union':count(union),'reference_intervals':union,'unreferenced_intervals':gaps(union,lines),'reference_owners':sorted({r['owner'] for r in here}),'reference_state':'some_precise_navigation' if union else 'no_precise_navigation',
                     'semantically_reviewed_lines':'not_established','semantic_acceptance':False})
    selected={r['path'] for r in rows}
    return {'schema_version':1,'baseline':snap['source_commit'],'inputs':inputs,'selected_syntax_file_count':len(rows),'files':rows,'nonrange_pointers':nonranges,
            'source_references_outside_selected_syntax_files':sorted(p for p in refs if p not in selected),
            'summary':{'files_with_some_precise_navigation':sum(bool(r['reference_intervals']) for r in rows),'files_without_precise_navigation':sum(not r['reference_intervals'] for r in rows),'selected_source_lines':sum(r['source_lines'] for r in rows),'precise_reference_line_union':sum(r['all_reference_line_union'] for r in rows),'semantically_reviewed_line_count':'not_established'},
            'limits':['references are navigation intervals, not reviewed semantics or accepted lines','directory/unranged pointers earn no line credit','macro cfg caller native parser/semantic tests not inferred from interval coverage','only selected syntax files; protected/vendor/nonselected documents/assets explicit outside census'],
            'full_project_coverage':False,'independent_acceptance':False}


def validate(root,v):
    errors=[]
    if v['full_project_coverage'] is not False or v['independent_acceptance'] is not False:errors.append('acceptance_boundary')
    for item in v['inputs'].values():
        if hashlib.sha256((root/item['path']).read_bytes()).hexdigest()!=item['sha256']:errors.append('input_hash:'+item['path'])
    for row in v['files']:
        data=(root/row['path']).read_bytes();lines=len(data.splitlines())
        if hashlib.sha256(data).hexdigest()!=row['source_sha256'] or lines!=row['source_lines']:errors.append('source_drift:'+row['path'])
        intervals=merge_ranges(row['reference_intervals'],lines)
        if row['all_reference_line_union']!=sum(b-a+1 for a,b in intervals) or [list(r) for r in gaps(intervals,lines)]!=row['unreferenced_intervals']:errors.append('union_gap:'+row['path'])
        if row['semantically_reviewed_lines']!='not_established' or row['semantic_acceptance'] is not False:errors.append('false_review_credit:'+row['path'])
    if v['selected_syntax_file_count']!=len(v['files']):errors.append('file_count')
    return errors


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3]);p.add_argument('--output',type=Path,required=True);p.add_argument('--validate',action='store_true');a=p.parse_args()
    if a.validate:
        errors=validate(a.root.resolve(),json.loads(a.output.read_text()));print(json.dumps({'errors':errors,'semantic_acceptance':False}));raise SystemExit(bool(errors))
    v=build(a.root.resolve());a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n');print(json.dumps(v['summary']))


if __name__=='__main__':main()
