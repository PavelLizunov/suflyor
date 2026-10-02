"""Temporary credential-file models for C05/C07. No secrets or Rust execution."""
import json
import os
import stat
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def read_map(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}


def write_map(path, payload):
    tmp = path.with_suffix(".tmp")
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    fd = os.open(tmp, flags, 0o600)
    try:
        os.write(fd, json.dumps(payload).encode())
    finally:
        os.close(fd)
    os.chmod(tmp, 0o600)
    os.replace(tmp, path)


class CredentialFileFixtures(unittest.TestCase):
    def test_symlink_tmp_is_followed_by_regular_open(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            outside = root / "outside"
            outside.write_text("keep", encoding="utf-8")
            target = root / "credentials.json"
            tmp = target.with_suffix(".tmp")
            tmp.symlink_to(outside)
            write_map(target, {"slot": "dummy"})
            self.assertEqual(outside.read_text(encoding="utf-8"), '{"slot": "dummy"}')
            self.assertEqual(stat.S_IMODE(outside.stat().st_mode), 0o600)

    def test_invalid_json_resets_map_and_drops_other_slot(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "credentials.json"
            path.write_text("{bad", encoding="utf-8")
            payload = read_map(path)
            payload["openai"] = "dummy"
            write_map(path, payload)
            self.assertEqual(json.loads(path.read_text(encoding="utf-8")), {"openai": "dummy"})

    def test_directory_permission_check_is_metadata_then_chmod(self):
        text = source("overlay-backend/src/credentials.rs")
        body = text[text.index("fn ensure_dir_permissions"):text.index("fn credentials_path")]
        self.assertLess(body.index("metadata(dir)"), body.index("set_permissions"))
        self.assertNotIn("O_NOFOLLOW", body)
        self.assertNotIn("fchmod", body)

    def test_write_uses_extension_tmp_without_exclusive_create(self):
        text = source("overlay-backend/src/credentials.rs")
        body = text[text.index("fn write_to_path"):text.index("fn write_map")]
        self.assertIn('path.with_extension("tmp")', body)
        self.assertIn(".create(true)", body)
        self.assertNotIn(".create_new(true)", body)
        self.assertNotIn("O_NOFOLLOW", body)

    def test_read_errors_become_empty_map(self):
        text = source("overlay-backend/src/credentials.rs")
        body = text[text.index("fn read_map"):text.index("pub(super) fn write_to_path")]
        self.assertEqual(body.count("HashMap::new()"), 2)
        self.assertIn("unwrap_or_default()", body)
        self.assertNotIn("lock", body.lower())

    def test_original_c05_c07_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker3_config-C05"], "hypothesis")
        self.assertEqual(found["wave1_worker3_config-C07"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
