"""Actual temporary Git + source-model fixtures. NOT native PowerShell gate proof."""
import re
import json
import hashlib
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CLASSIFIER = ROOT / '.github/scripts/is-docs-only.sh'
NATIVE = ROOT / 'scripts/git-gate-native.ps1'


class CiHypothesisFixtures(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.git('init', '-q')
        self.git('config', 'user.name', 'Research fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('-c', 'core.hooksPath=/dev/null', 'commit', '-q', '--allow-empty', '-m', 'fixture base')
        self.base = self.git('rev-parse', 'HEAD').strip()

    def git(self, *args):
        result = subprocess.run(['git', *args], cwd=self.root, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def commit(self, path, body='fixture'):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(body)
        self.git('add', '--', path)
        self.git('-c', 'core.hooksPath=/dev/null', 'commit', '-q', '-m', 'fixture change')
        return self.git('rev-parse', 'HEAD').strip()

    def source_model(self, changed):
        """Extract actual pattern/prefixes; emulate only ASCII path decisions, not PS runtime."""
        source = NATIVE.read_text()
        pattern = re.search(r"\$_ -notmatch '([^']+)'", source).group(1)
        crates = re.findall(r"'([^']+)'", source[source.index('$crateOrder = @('):source.index('function Invoke-Git')])
        normalized = sorted({p.replace('\\', '/') for p in changed if p})
        docs = all(re.search(pattern, p, re.IGNORECASE) for p in normalized)
        affected = [crate for crate in crates if any(p.lower().startswith(crate.lower() + '/') for p in normalized)]
        return {'tier': 'docs' if docs else 'targeted', 'crates': affected, 'paths': normalized}

    def github_classifier(self, base, head):
        return subprocess.run(['bash', str(CLASSIFIER), base, head], cwd=self.root, text=True, capture_output=True)

    def test_missing_push_base_single_commit_diff_excludes_earlier_rust(self):
        self.commit('overlay-backend/src/fixture.rs')
        head = self.commit('README.md')
        missing = subprocess.run(['git', 'rev-parse', '--verify', 'origin/master'], cwd=self.root, capture_output=True)
        self.assertNotEqual(missing.returncode, 0)
        fallback = self.git('diff', 'HEAD~1...HEAD', '--name-only', '--diff-filter=ACMRD').splitlines()
        full = self.git('diff', self.base + '...HEAD', '--name-only', '--diff-filter=ACMRD').splitlines()
        self.assertEqual(self.source_model(fallback)['tier'], 'docs')
        self.assertEqual(self.source_model(full)['crates'], ['overlay-backend'])
        self.assertEqual(self.source_model(full)['tier'], 'targeted')
        source = NATIVE.read_text()
        self.assertIn("if ($LASTEXITCODE -ne 0) { $Base = 'HEAD~1' }", source)
        self.assertIn('$diffArguments = @("$Base...HEAD")', source)
        self.assertEqual(self.github_classifier(self.base, head).stdout.strip(), 'false')

    def test_space_filename_preserves_crate_membership(self):
        head = self.commit('overlay-backend/src/name with space.rs')
        lines = self.git('diff', self.base, head, '--name-only').splitlines()
        self.assertEqual(self.source_model(lines)['crates'], ['overlay-backend'])

    def test_quote_filename_git_quoting_breaks_source_model_prefix_not_github(self):
        head = self.commit('overlay-backend/src/quoted"name.rs')
        lines = self.git('diff', self.base, head, '--name-only').splitlines()
        self.assertTrue(lines[0].startswith('"'))
        model = self.source_model(lines)
        self.assertEqual(model['tier'], 'targeted')
        self.assertEqual(model['crates'], [])
        result = self.github_classifier(self.base, head)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), 'false')

    def test_unicode_quote_path_depends_on_git_config(self):
        head = self.commit('overlay-backend/src/пример.rs')
        self.git('config', 'core.quotePath', 'true')
        quoted = self.git('diff', self.base, head, '--name-only').splitlines()
        self.assertEqual(self.source_model(quoted)['crates'], [])
        self.git('config', 'core.quotePath', 'false')
        unquoted = self.git('diff', self.base, head, '--name-only').splitlines()
        self.assertEqual(self.source_model(unquoted)['crates'], ['overlay-backend'])
        self.assertEqual(self.github_classifier(self.base, head).stdout.strip(), 'false')

    def test_github_nul_safe_tabs_and_newlines(self):
        for path in ('docs/a\tb.md', 'docs/a\nb.md'):
            with self.subTest(path=repr(path)):
                head = self.commit(path)
                result = self.github_classifier(self.base, head)
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout.strip(), 'true')
        head = self.commit('overlay-backend/src/a\nb.rs')
        self.assertEqual(self.github_classifier(self.base, head).stdout.strip(), 'false')

    def test_github_conservative_rename_invalid_ref_and_compiled_knowledge(self):
        code = self.commit('src/old.rs')
        (self.root/'docs').mkdir()
        self.git('mv', 'src/old.rs', 'docs/new.md')
        self.git('-c', 'core.hooksPath=/dev/null', 'commit', '-q', '-m', 'fixture rename')
        head = self.git('rev-parse', 'HEAD').strip()
        self.assertEqual(self.github_classifier(code, head).stdout.strip(), 'false')
        self.assertNotEqual(self.github_classifier('missing-ref', head).returncode, 0)
        head = self.commit('overlay-backend/knowledge/glossary.md')
        self.assertEqual(self.github_classifier(self.base, head).stdout.strip(), 'false')

    def test_hypothesis_evidence_keeps_original_identity_status_and_hashes(self):
        evidence=json.loads((ROOT/'docs/agent-map/reconciliation/hypothesis-ci-portable.json').read_text())
        register_path=ROOT/'docs/agent-map/reconciliation/candidates.json'
        self.assertEqual(hashlib.sha256(register_path.read_bytes()).hexdigest(),evidence['original_register_sha256'])
        register={r['id']:r for r in json.loads(register_path.read_text())}
        for row in evidence['original_candidates']:
            self.assertEqual(register[row['id']]['status'],row['status_unchanged'])
            self.assertFalse(row['native_reproduction'])
            self.assertFalse(row['independent_acceptance'])
        for row in evidence['source_references']:
            data=(ROOT/row['path']).read_bytes()
            self.assertEqual(hashlib.sha256(data).hexdigest(),row['source_sha256'])
            self.assertLessEqual(row['end_line'],len(data.splitlines()))
        self.assertFalse(evidence['full_project_coverage'])

    def test_ci_gate_scope_and_separate_security_not_external_branch_policy(self):
        ci = (ROOT/'.github/workflows/ci.yml').read_text()
        gate = ci[ci.index('  gate:'):ci.index('  validate:')]
        self.assertIn('needs: [changes, rust]', gate)
        self.assertNotIn('needs.macos', gate)
        self.assertNotIn('needs.validate', gate)
        self.assertIn('test "$CHANGES_RESULT" = "success" || exit 1', gate)
        security = (ROOT/'.github/workflows/security.yml').read_text()
        self.assertIn('pull_request:', security)
        self.assertIn('gitleaks', security)
        self.assertIn('cargo deny', security)


if __name__ == '__main__':
    unittest.main()
