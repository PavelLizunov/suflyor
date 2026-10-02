"""Temporary Git fixtures for CI C04/C05. Not PowerShell gate execution."""
import subprocess
import tempfile
import unittest
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], check=True, capture_output=True, text=True).stdout


def classify(paths):
    changed = sorted({path.replace("\\", "/") for path in paths if path})
    docs = all(path.endswith((".md", ".markdown", ".html", ".txt", ".po")) for path in changed)
    return "docs" if changed and docs else "targeted"


class GateClassificationFixtures(unittest.TestCase):
    def test_missing_base_uses_previous_commit_only(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            git(repo, "init", "-q")
            git(repo, "config", "user.email", "fixture@example.com")
            git(repo, "config", "user.name", "Fixture")
            git(repo, "commit", "--allow-empty", "-qm", "initial")
            (repo / "code.rs").write_text("fn main() {}\n", encoding="utf-8")
            git(repo, "add", "code.rs")
            git(repo, "commit", "-qm", "feat")
            (repo / "README.md").write_text("docs\n", encoding="utf-8")
            git(repo, "add", "README.md")
            git(repo, "commit", "-qm", "docs")
            missing = subprocess.run(["git", "-C", repo, "rev-parse", "--verify", "origin/master"], capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            names = git(repo, "diff", "--name-only", "--diff-filter=ACMRD", "HEAD~1...HEAD").splitlines()
            self.assertEqual(names, ["README.md"])
            self.assertEqual(classify(names), "docs")
            first = git(repo, "rev-list", "--max-parents=0", "HEAD").strip()
            full = git(repo, "diff", "--name-only", "--diff-filter=ACMRD", first, "HEAD").splitlines()
            self.assertCountEqual(full, ["README.md", "code.rs"])
            self.assertEqual(classify(full), "targeted")

    def test_name_only_quotes_unusual_filename(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            git(repo, "init", "-q")
            git(repo, "config", "user.email", "fixture@example.com")
            git(repo, "config", "user.name", "Fixture")
            (repo / "README.md").write_text("base\n", encoding="utf-8")
            git(repo, "add", "README.md")
            git(repo, "commit", "-qm", "base")
            name = 'quote".md'
            (repo / name).write_text("docs\n", encoding="utf-8")
            git(repo, "add", "--", name)
            git(repo, "commit", "-qm", "quoted")
            shown = git(repo, "diff", "--name-only", "HEAD~1...HEAD").strip()
            self.assertEqual(shown, '"quote\\".md"')
            self.assertNotEqual(classify([shown]), "docs")

    def test_zero_terminated_name_preserves_quote(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            git(repo, "init", "-q")
            git(repo, "config", "user.email", "fixture@example.com")
            git(repo, "config", "user.name", "Fixture")
            (repo / "README.md").write_text("base\n", encoding="utf-8")
            git(repo, "add", "README.md")
            git(repo, "commit", "-qm", "base")
            name = 'quote".md'
            (repo / name).write_text("docs\n", encoding="utf-8")
            git(repo, "add", "--", name)
            git(repo, "commit", "-qm", "quoted")
            raw = subprocess.run(["git", "-C", repo, "diff", "-z", "--name-only", "HEAD~1...HEAD"], check=True, capture_output=True).stdout
            parsed = raw.split(b"\0")[0].decode()
            self.assertEqual(parsed, name)
            self.assertEqual(classify([parsed]), "docs")

    def test_classifier_source_has_fallback_and_no_zero_delimiter(self):
        text = (ROOT / "scripts/git-gate-native.ps1").read_text()
        body = text[text.index("elseif ($Stage -eq 'push')"):text.index("$changedInput =")]
        self.assertIn("$Base = 'HEAD~1'", body)
        diff = text[text.index("$changedInput ="):text.index("if ($includeUntracked)")]
        self.assertIn("'--name-only'", diff)
        self.assertNotIn("'-z'", diff)
        self.assertIn("$_.Replace('\\', '/')", text)

    def test_docs_pattern_matches_suffix_not_directory(self):
        self.assertEqual(classify(["docs/readme.md", "notes/readme.html"]), "docs")
        self.assertEqual(classify(["docs/readme.md", "src/main.rs"]), "targeted")
        self.assertEqual(classify(["docs.md/secret.rs"]), "targeted")

    def test_original_c04_c05_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker3_cicd-C04"], "hypothesis")
        self.assertEqual(found["wave4_worker3_cicd-C05"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
