#!/usr/bin/env python3
"""Rust name-matched member/call navigation, never resolved Config/Slint graph."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import syntax_index


def span(node):
    return {"start_byte": node.start_byte, "end_byte": node.end_byte,
            "start_line": node.start_point.row + 1, "end_line": node.end_point.row + 1}


def extract(data, path, fields, ui_names, parser=None):
    root = (parser or syntax_index.rust_parser()).parse(data).root_node
    if root.has_error:
        raise ValueError("Rust parse error: " + path)
    edges = []

    def text(n):
        return data[n.start_byte:n.end_byte].decode() if n else None

    def walk(node, container=None, in_closure=False):
        if node.type in {'macro_definition', 'macro_invocation', 'string_literal', 'raw_string_literal', 'line_comment', 'block_comment'}:
            return
        if node.type == 'function_item':
            container = {"name": text(node.child_by_field_name('name')), **span(node)}
        in_closure = in_closure or node.type == 'closure_expression'
        if node.type == 'field_expression':
            field = node.child_by_field_name('field')
            receiver = node.child_by_field_name('value')
            name = text(field)
            parent = node.parent
            method_call = parent is not None and parent.type == 'call_expression' and parent.child_by_field_name('function') == node
            kind = None
            matches = []
            if name in fields and not method_call:
                kind = 'Config_field_name_candidate'
            elif method_call and name in ui_names:
                kind = 'Slint_generated_method_name_candidate'
                matches = ui_names[name]
            if kind:
                lhs = parent is not None and parent.type in {'assignment_expression', 'compound_assignment_expr'} and parent.child_by_field_name('left') == node
                row = {"path": path, "kind": kind, "name": name,
                       "receiver_node_type": receiver.type,
                       "receiver_identifier": text(receiver) if receiver.type in {'identifier','self'} else None,
                       "containing_function": container, "inside_closure": in_closure,
                       "direct_assignment_lhs": lhs, "immediate_parent_kind": parent.type if parent else None,
                       "ui_declaration_candidates": matches, "resolved_type": False,
                       "semantic_acceptance": False, **span(node)}
                edges.append(row)
        for child in node.named_children:
            walk(child, container, in_closure)
    walk(root)
    return edges


def build(root):
    snapshot = json.loads((root/'docs/agent-map/reconciliation/snapshot.json').read_text())
    schema_path = root/'docs/agent-map/schema/inventory.json'
    schema = json.loads(schema_path.read_text())
    if schema['source_commit'] != snapshot['source_commit']:
        raise ValueError('schema baseline mismatch')
    fields = {r['name'] for r in schema['config_fields']}
    ui_names = {}
    for row in schema['ui_nodes']:
        name = row.get('name')
        if not name or row['kind'] not in {'property','callback'}:
            continue
        stem = name.replace('-','_')
        prefixes = ('set_','get_') if row['kind']=='property' else ('on_','invoke_')
        for prefix in prefixes:
            api = prefix+stem
            ui_names.setdefault(api, []).append({"path":row['path'],"kind":row['kind'],"name":name,"scope_names":row['scope_names'],"start_line":row['start_line'],"end_line":row['end_line']})
    inputs, edges = {}, []
    parser=syntax_index.rust_parser()
    for item in snapshot['source_files']:
        path=item['path']
        if not path.endswith('.rs') or not path.startswith(('overlay-backend/src/','slint-experiment/src/','suflyor-tts/src/','suflyor-teratts/src/','suflyor-wsola/src/')):
            continue
        data=syntax_index.safe_source(root,path).read_bytes();digest=hashlib.sha256(data).hexdigest()
        if digest!=item['sha256']:raise ValueError('source drift: '+path)
        inputs[path]={"sha256":digest,"bytes":len(data)}
        edges.extend(extract(data,path,fields,ui_names,parser))
    return {"schema_version":1,"source_commit":snapshot['source_commit'],"schema_inventory_sha256":hashlib.sha256(schema_path.read_bytes()).hexdigest(),"inputs":inputs,"name_edges":edges,
            "Config_field_name_candidate_counts":{name:sum(r['kind']=='Config_field_name_candidate' and r['name']==name for r in edges) for name in sorted(fields)},
            "semantic_acceptance":False,"complete_project_coverage":False,
            "limits":["same field name can belong to non-Config types; receiver identity is syntax only","same Slint API name can match multiple windows or non-Slint methods","macros strings aliases includes cfg/type/borrow/closure reachability not resolved","cfg(test) nodes included syntactically; no Rust/native execution or compiler callgraph"]}


def validate(root,report):
    errors=[]
    if report['semantic_acceptance'] is not False or report['complete_project_coverage'] is not False:errors.append('acceptance_boundary')
    snapshot=json.loads((root/'docs/agent-map/reconciliation/snapshot.json').read_text());frozen={r['path']:r['sha256'] for r in snapshot['source_files']}
    if report['source_commit']!=snapshot['source_commit']:errors.append('baseline')
    if hashlib.sha256((root/'docs/agent-map/schema/inventory.json').read_bytes()).hexdigest()!=report['schema_inventory_sha256']:errors.append('schema_hash')
    data={}
    for path,row in report['inputs'].items():
        data[path]=syntax_index.safe_source(root,path).read_bytes()
        if hashlib.sha256(data[path]).hexdigest()!=row['sha256'] or frozen.get(path)!=row['sha256']:errors.append('source_hash:'+path)
    seen=set()
    for row in report['name_edges']:
        ident=(row['path'],row['start_byte'],row['end_byte'],row['kind']);raw=data[row['path']]
        if ident in seen:errors.append('duplicate_edge:'+row['path'])
        seen.add(ident)
        if row['resolved_type'] is not False or row['semantic_acceptance'] is not False:errors.append('false_resolution')
        a,b=row['start_byte'],row['end_byte']
        if not 0<=a<b<=len(raw):errors.append('range:'+row['path'])
        elif row['name'] not in raw[a:b].decode():errors.append('name_range:'+row['path'])
        if not 1<=row['start_line']<=row['end_line']<=len(raw.splitlines())+1:errors.append('line_range:'+row['path'])
    for name,count in report['Config_field_name_candidate_counts'].items():
        if count!=sum(r['kind']=='Config_field_name_candidate' and r['name']==name for r in report['name_edges']):errors.append('count:'+name)
    return errors


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[3]);p.add_argument('--output',type=Path,required=True);p.add_argument('--worker',action='store_true');p.add_argument('--validate',action='store_true');args=p.parse_args()
    if args.validate:
        errors=validate(args.root.resolve(),json.loads(args.output.read_text()));print(json.dumps({'errors':errors,'resolved_type':False}));raise SystemExit(bool(errors))
    if not args.worker:
        r=subprocess.run([sys.executable,'-B',str(Path(__file__).resolve()),'--worker','--root',str(args.root.resolve()),'--output',str(args.output.resolve())],capture_output=True,text=True,timeout=60)
        if r.returncode:raise RuntimeError('isolated member census failed: '+str(r.returncode)+' '+r.stderr[-2000:])
        print(r.stdout,end='');return
    report=build(args.root.resolve());args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({'inputs':len(report['inputs']),'name_edges':len(report['name_edges']),'Config_field_names':len(report['Config_field_name_candidate_counts']),'resolved_type':False}))


if __name__=='__main__':main()
