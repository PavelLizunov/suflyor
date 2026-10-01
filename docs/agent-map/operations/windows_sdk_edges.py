#!/usr/bin/env python3
"""Selected Windows SDK import/call name candidates; not resolved compiler graph."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
import syntax_index

FILES = ('slint-experiment/src/native/windows/screen.rs',
         'slint-experiment/src/native/windows/lifecycle.rs',
         'slint-experiment/src/win32.rs', 'slint-experiment/src/tray.rs')


def span(n):
    return {'start_byte':n.start_byte,'end_byte':n.end_byte,'start_line':n.start_point.row+1,'end_line':n.end_point.row+1}


def extract(data,path,parser=None):
    root=(parser or syntax_index.rust_parser()).parse(data).root_node
    if root.has_error:raise ValueError('Rust parse error: '+path)
    def text(n):return data[n.start_byte:n.end_byte].decode() if n else ''
    imports=[]
    def use(n,prefix=''):
        if n.type=='scoped_use_list':
            part=text(n.child_by_field_name('path'));joined='::'.join(x for x in (prefix,part) if x)
            for c in n.child_by_field_name('list').named_children:use(c,joined)
        elif n.type=='use_list':
            for c in n.named_children:use(c,prefix)
        else:
            source=n.child_by_field_name('path') if n.type=='use_as_clause' else n
            full='::'.join(x for x in (prefix,text(source)) if x)
            alias=text(n.child_by_field_name('alias')) if n.type=='use_as_clause' else full.rsplit('::',1)[-1]
            if full.startswith('windows::'):
                imports.append({'path':path,'import_path':full,'local_name':alias,'kind':'SDK_import_name','resolved_symbol':False,**span(n)})
    def collect(n):
        if n.type in {'macro_definition','macro_invocation','string_literal','raw_string_literal','line_comment','block_comment'}:return
        if n.type=='use_declaration':use(n.child_by_field_name('argument'))
        for c in n.named_children:collect(c)
    collect(root)
    names={r['local_name'] for r in imports};calls=[]
    def walk(n,container=None):
        if n.type in {'macro_definition','macro_invocation','string_literal','raw_string_literal','line_comment','block_comment'}:return
        if n.type=='function_item':container={'name':text(n.child_by_field_name('name')),**span(n)}
        if n.type=='call_expression':
            f=n.child_by_field_name('function');name=text(f)
            if f.type=='identifier' and name in names:
                candidates=[r['import_path'] for r in imports if r['local_name']==name]
                calls.append({'path':path,'callee_name':name,'import_path_candidates':sorted(set(candidates)),'containing_function':container,'kind':'SDK_direct_call_name_candidate','resolved_symbol':False,'semantic_acceptance':False,**span(n)})
            elif f.type=='scoped_identifier' and name.startswith('windows::'):
                calls.append({'path':path,'callee_name':name,'import_path_candidates':[name],'containing_function':container,'kind':'SDK_qualified_call_syntax','resolved_symbol':False,'semantic_acceptance':False,**span(n)})
        for c in n.named_children:walk(c,container)
    walk(root)
    return imports,calls


def build(root):
    snap=json.loads((root/'docs/agent-map/reconciliation/snapshot.json').read_text());frozen={r['path']:r['sha256'] for r in snap['source_files']}
    parser=syntax_index.rust_parser();inputs={};imports=[];calls=[]
    for path in FILES:
        data=syntax_index.safe_source(root,path).read_bytes();h=hashlib.sha256(data).hexdigest()
        if h!=frozen[path]:raise ValueError('source drift: '+path)
        inputs[path]={'sha256':h,'bytes':len(data)};a,b=extract(data,path,parser);imports.extend(a);calls.extend(b)
    return {'schema_version':1,'source_commit':snap['source_commit'],'inputs':inputs,'imports':imports,'calls':calls,'semantic_acceptance':False,'complete_project_coverage':False,'limits':['selected four adapter files only, not all Windows or backend SDK graph','file-level name candidates, import lexical shadowing/cfg/module scope not resolved','type/constant constructors count as call syntax too','macro token trees not expanded, callbacks/function pointers/aliases/cfg(test)/POSIX stubs not evaluated']}


def validate(root,v):
    errors=[];snap=json.loads((root/'docs/agent-map/reconciliation/snapshot.json').read_text());frozen={r['path']:r['sha256'] for r in snap['source_files']};data={}
    if v['semantic_acceptance'] is not False or v['complete_project_coverage'] is not False:errors.append('acceptance_boundary')
    if v['source_commit']!=snap['source_commit']:errors.append('baseline')
    for path,row in v['inputs'].items():
        data[path]=syntax_index.safe_source(root,path).read_bytes()
        if hashlib.sha256(data[path]).hexdigest()!=row['sha256'] or frozen.get(path)!=row['sha256']:errors.append('hash:'+path)
    for row in v['imports']+v['calls']:
        if row['resolved_symbol'] is not False:errors.append('false_resolution')
        a,b=row['start_byte'],row['end_byte'];raw=data[row['path']]
        if not 0<=a<b<=len(raw):errors.append('range:'+row['path'])
        if not 1<=row['start_line']<=row['end_line']<=len(raw.splitlines())+1:errors.append('lines:'+row['path'])
    return errors


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3]);p.add_argument('--output',type=Path,required=True);p.add_argument('--worker',action='store_true');p.add_argument('--validate',action='store_true');args=p.parse_args()
    if args.validate:
        errors=validate(args.root.resolve(),json.loads(args.output.read_text()));print(json.dumps({'errors':errors,'resolved_symbol':False}));raise SystemExit(bool(errors))
    if not args.worker:
        r=subprocess.run([sys.executable,'-B',str(Path(__file__).resolve()),'--worker','--root',str(args.root.resolve()),'--output',str(args.output.resolve())],text=True,capture_output=True,timeout=60)
        if r.returncode:raise RuntimeError('SDK candidate worker failed: '+str(r.returncode)+' '+r.stderr[-2000:])
        print(r.stdout,end='');return
    v=build(args.root.resolve());args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'files':len(v['inputs']),'imports':len(v['imports']),'calls':len(v['calls']),'resolved_symbol':False}))


if __name__=='__main__':main()
