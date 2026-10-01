#!/usr/bin/env node
// Hash-gated prebuilt WASM research parser. Never runs NSIS/preprocessor/install.
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

const HASHES = {
  'web-tree-sitter/tree-sitter.cjs': 'aceb870a1ad6d6ff23acf6bbb2e45d5bd01e4a7adf46bcfe94d9c1dcf4d5074f',
  'web-tree-sitter/tree-sitter.wasm': 'f38dcc4b43b818f9a0785bc1c6d5611a75ac4cdd428ff3f02757c34ca4e46d7f',
  'tree-sitter-nsis/tree-sitter-nsis.wasm': 'f7f589e299f08017ad19ec388ac1dae8467f0dff65c7405c53eb072f7810c9f5',
};
const KINDS = new Set(['function_definition', 'section_definition', 'section_group',
  'page_ex_definition', 'macro_definition', 'variable_declaration', 'label',
  'preproc_directive']);

async function parse(source, label) {
  if (process.versions.node !== '22.23.2') throw new Error('Node research runtime must match 22.23.2');
  const base = process.env.SUFLYOR_RESEARCH_NSIS_WASM;
  if (!base) throw new Error('SUFLYOR_RESEARCH_NSIS_WASM must name isolated pinned package directory');
  for (const [file, hash] of Object.entries(HASHES)) {
    const actual = crypto.createHash('sha256').update(fs.readFileSync(path.join(base, file))).digest('hex');
    if (actual !== hash) throw new Error('NSIS WASM hash mismatch: ' + file);
  }
  const { Parser, Language } = require(path.resolve(base, 'web-tree-sitter/tree-sitter.cjs'));
  await Parser.init({ locateFile: () => path.resolve(base, 'web-tree-sitter/tree-sitter.wasm') });
  const language = await Language.load(path.resolve(base, 'tree-sitter-nsis/tree-sitter-nsis.wasm'));
  if (language.abiVersion !== 15) throw new Error('Unexpected NSIS language ABI');
  const parser = new Parser();
  parser.setLanguage(language);
  const tree = parser.parse(source);
  const declarations = [], errors = [];
  // web-tree-sitter indexes UTF16 code units; committed source ranges must be UTF8 bytes.
  const byte = index => Buffer.byteLength(source.slice(0, index), 'utf8');
  const sourceHash = crypto.createHash('sha256').update(source, 'utf8').digest('hex');

  function walk(node, parents, scopes, conditional) {
    if (node.type === 'ERROR' || node.isMissing) {
      errors.push({ kind: node.isMissing ? 'MISSING' : 'ERROR', node_type: node.type,
        start_byte: byte(node.startIndex), end_byte: byte(node.endIndex),
        start_line: node.startPosition.row + 1, end_line: node.endPosition.row + 1 });
    }
    if (['comment', 'block_comment', 'string'].includes(node.type)) return;
    const nextConditional = node.type === 'preproc_conditional'
      ? [...conditional, source.slice(node.startIndex, node.endIndex).split('\n')[0]] : conditional;
    let childParents = parents, childScopes = scopes;
    if (KINDS.has(node.type)) {
      const nameNode = node.childForFieldName('name');
      let name = nameNode?.text;
      if (!name && node.type === 'preproc_directive') {
        const directive = node.childForFieldName('directive')?.text || '';
        const argument = node.childrenForFieldName('argument')[0]?.text || '';
        name = [directive, argument].filter(Boolean).join(' ');
      }
      if (!name) name = node.childrenForFieldName('parameter').find(n => ['string', 'identifier'].includes(n.type))?.text;
      name ||= '<anonymous:' + node.type + '>';
      const begin = byte(node.startIndex), end = byte(node.endIndex);
      const id = `${label}:${begin}:${end}:${node.type}`;
      const headerEnd = ['function_definition', 'section_definition', 'section_group', 'page_ex_definition', 'macro_definition'].includes(node.type)
        ? source.indexOf('\n', node.startIndex) : node.endIndex;
      const signatureEnd = headerEnd < 0 ? node.endIndex : Math.min(headerEnd, node.endIndex);
      declarations.push({ id, path: label, language: 'nsis', kind: node.type, name,
        parent_ids: parents, scope_names: scopes, start_byte: begin, end_byte: end,
        start_line: node.startPosition.row + 1,
        end_line: node.endPosition.row + 1,
        signature_source: source.slice(node.startIndex, signatureEnd).trimEnd(),
        attributes: [], cfg_attributes: nextConditional, test_conditional: false,
        macro_expanded: false, semantic_acceptance: false, source_sha256: sourceHash });
      childParents = [...parents, id]; childScopes = [...scopes, name];
    }
    for (const child of node.children) if (child.isNamed || child.isMissing) walk(child, childParents, childScopes, nextConditional);
  }
  try {
    walk(tree.rootNode, [], [], []);
    return { declarations, parse_errors: errors, has_parse_error: tree.rootNode.hasError,
      runtime: 'Node 22.23.2/web-tree-sitter 0.25.10', grammar: 'tree-sitter-nsis 0.4.1/ABI15' };
  } finally {
    tree.delete(); parser.delete();
  }
}

if (require.main === module) {
  const [filename, label] = process.argv.slice(2);
  parse(fs.readFileSync(filename, 'utf8'), label || filename)
    .then(result => process.stdout.write(JSON.stringify(result) + '\n'))
    .catch(error => { process.stderr.write(error.message + '\n'); process.exitCode = 1; });
}
module.exports = { parse };
