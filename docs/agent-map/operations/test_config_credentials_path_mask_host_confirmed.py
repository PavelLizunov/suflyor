"""Source and pure string/path fixtures for wave1_worker3_config C01 and C02 (confirmed mechanisms).
No live credentials written, no filesystem mutation outside temporary directories.
"""
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def data_root_in_mock(base):
    brand = base / "suflyor"
    legacy = base / "overlay-mvp"
    if not brand.exists() and legacy.exists():
        return legacy
    else:
        return brand


def mask_host_mock(url):
    url = url.strip()
    if not url:
        return ""
    scheme = ""
    rest = url
    idx = url.find("://")
    if idx != -1:
        scheme = url[: idx + 3]
        rest = url[idx + 3 :]
    first_delim = None
    for delim in ("/", "?", "#"):
        pos = rest.find(delim)
        if pos != -1 and (first_delim is None or pos < first_delim):
            first_delim = pos
    if first_delim is not None:
        authority = rest[:first_delim]
        path = rest[first_delim:]
    else:
        authority = rest
        path = ""
    host_port = authority
    at_idx = authority.rfind("@")
    if at_idx != -1:
        host_port = authority[at_idx + 1 :]
    port = ""
    if host_port.startswith("["):
        close_idx = host_port.find("]")
        if close_idx != -1:
            after = host_port[close_idx + 1 :]
            if after.startswith(":") and len(after) > 1 and after[1:].isdigit():
                port = after
    else:
        colon_idx = host_port.rfind(":")
        if colon_idx != -1:
            after = host_port[colon_idx + 1 :]
            if after and after.isdigit():
                port = host_port[colon_idx:]
    return f"{scheme}***{port}{path}"


class ConfigPathsMaskHostFixtures(unittest.TestCase):
    def test_credentials_path_creates_suflyor_directory_via_ensure_dir_permissions(self):
        text = source("overlay-backend/src/credentials.rs")
        body = text[text.index("fn credentials_path() -> Result<PathBuf> {"):text.index("fn read_map()")]
        # Joins "suflyor" and calls ensure_dir_permissions(&dir)
        self.assertIn('.join("suflyor")', body)
        self.assertIn("ensure_dir_permissions(&dir)", body)
        dir_fn = text[text.index("fn ensure_dir_permissions(dir: &Path) -> Result<()>"):text.index("fn credentials_path")]
        self.assertIn("fs::create_dir_all(dir)", dir_fn)

    def test_paths_data_root_in_logic_orphans_legacy_when_brand_exists(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            base = Path(tmpdir)
            legacy = base / "overlay-mvp"
            legacy.mkdir()
            (legacy / "config.json").write_text('{"legacy": true}', encoding="utf-8")
            # When brand directory exists (e.g. created by credentials_path), data_root_in returns brand
            brand = base / "suflyor"
            brand.mkdir()
            picked = data_root_in_mock(base)
            self.assertEqual(picked, brand)
            self.assertNotEqual(picked, legacy)
            # Legacy files in overlay-mvp are not picked up
            self.assertFalse((picked / "config.json").exists())

    def test_mask_host_masks_authority_but_preserves_query_fragment_and_path(self):
        url = "https://192.168.1.100:8080/v1/chat/completions?api_key=secret123#session456"
        masked = mask_host_mock(url)
        # Authority host is masked, port is preserved, path/query/fragment remain verbatim
        self.assertEqual(masked, "https://***:8080/v1/chat/completions?api_key=secret123#session456")
        self.assertIn("api_key=secret123", masked)
        self.assertIn("#session456", masked)

    def test_mask_host_userinfo_stripped_and_portless_hosts(self):
        url = "http://admin:pass123@internal.corp.net/dashboard"
        masked = mask_host_mock(url)
        self.assertEqual(masked, "http://***/dashboard")
        self.assertNotIn("pass123", masked)
        self.assertNotIn("admin", masked)

    def test_source_mask_host_format_string_concatenation(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn mask_host(url: &str) -> String {"):text.index("pub fn preview_server_settings(")]
        self.assertIn('format!("{scheme}***{port}{path}")', body)
        self.assertIn("match rest.find(['/', '?', '#'])", body)

    def test_original_c01_c02_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker3_config-C01"], "confirmed")
        self.assertEqual(found["wave1_worker3_config-C02"], "confirmed")


if __name__ == "__main__":
    unittest.main()
