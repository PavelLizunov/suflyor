"""Source fixtures for wave4_worker3_cicd C01, C02, and C03 (confirmed mechanisms).
No PowerShell script execution, no Cargo builds, no git hook triggers.
"""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def classify_tier_model(changed_files, full=False):
    crate_order = [
        "slint-experiment",
        "overlay-backend",
        "suflyor-wsola",
        "suflyor-tts",
        "suflyor-teratts",
    ]
    affected_crates = [
        crate
        for crate in crate_order
        if any(f.lower().startswith(f"{crate}/") for f in changed_files)
    ]
    docs_regex = re.compile(r"(^|/)[^/]+\.(md|html|txt)$", re.IGNORECASE)
    docs_only = all(docs_regex.search(f) for f in changed_files)

    tier = "targeted"
    if docs_only:
        tier = "docs"
    if full:
        tier = "full"
    return tier, affected_crates


class CicdNativeGateClassificationFixtures(unittest.TestCase):
    def test_knowledge_markdown_classified_as_docs_only_despite_compiled_in(self):
        # overlay-backend/knowledge/glossary.md is pulled in via include_str! in kb.rs
        changed = ["overlay-backend/knowledge/glossary.md"]
        tier, affected = classify_tier_model(changed)
        # Regex matches .md ending, classifying it as 'docs' and exiting without cargo test
        self.assertEqual(tier, "docs")
        self.assertEqual(affected, ["overlay-backend"])

        text = source("scripts/git-gate-native.ps1")
        self.assertIn("$_ -notmatch '(^|/)[^/]+\\.(md|html|txt)$'", text)
        self.assertIn('if ($tier -eq \'docs\') {', text)
        self.assertIn('Write-Host "[gate:$Stage] OK (docs-only; no Cargo work)"', text)

    def test_crate_order_omits_suflyor_mlx_and_standalone_modules(self):
        text = source("scripts/git-gate-native.ps1")
        body = text[text.index("$crateOrder = @("):text.index("function Invoke-Git")]
        # Only tracks the 5 crates, omitting swift/mlx sidecars and scripts
        self.assertIn("'slint-experiment'", body)
        self.assertIn("'overlay-backend'", body)
        self.assertIn("'suflyor-wsola'", body)
        self.assertIn("'suflyor-tts'", body)
        self.assertIn("'suflyor-teratts'", body)
        self.assertNotIn("suflyor-mlx", body)

    def test_diff_arguments_empty_when_not_in_commit_stage(self):
        text = source("scripts/git-gate-native.ps1")
        body = text[text.index("$diffArguments = @()"):text.index("if ($changed.Count -eq 0)")]
        # In manual/push stages without commit, diffArguments defaults to empty
        self.assertIn("$diffArguments = @()", body)

    def test_slint_ui_only_diff_skips_clippy_and_restricts_to_hardcoded_guards(self):
        text = source("scripts/git-gate-native.ps1")
        body = text[text.index("if ($slintUiOnly) {"):text.index("Write-Host \"[gate:$Stage] OK (targeted)\"")]
        # Runs check and fixed list of guard tests, completely skipping clippy
        self.assertIn("& $cargo check --locked --manifest-path $manifest --bin overlay-host", body)
        self.assertIn("$guards = @(", body)
        self.assertIn("'settings_reset_guard'", body)
        self.assertIn("'i18n_guard'", body)
        self.assertNotIn("clippy", body[:body.index("else {")])

    def test_kb_source_verifies_include_str_for_knowledge_md_files(self):
        text = source("overlay-backend/src/kb.rs")
        self.assertIn('include_str!("../knowledge/glossary.md")', text)
        self.assertIn('include_str!("../knowledge/commands.md")', text)
        self.assertIn('include_str!("../knowledge/patterns.md")', text)

    def test_original_c01_c02_c03_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker3_cicd-C01"], "confirmed")
        self.assertEqual(found["wave4_worker3_cicd-C02"], "confirmed")
        self.assertEqual(found["wave4_worker3_cicd-C03"], "confirmed")


if __name__ == "__main__":
    unittest.main()
