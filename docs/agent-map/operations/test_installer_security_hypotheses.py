"""Source fixtures for wave4_worker2_installers C01/C02.
No real network calls, no actual tar bombs, no process execution.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class InstallerSecurityFixtures(unittest.TestCase):
    def test_extract_tar_bz2_invokes_bsdtar_with_no_exclusion_flags(self):
        text = source("overlay-backend/src/download.rs")
        body = text[text.index("pub(crate) fn extract_tar_bz2"):text.index("pub(crate) fn system_bsdtar")]
        # Arguments to tar: -xf, tarball, -C, dest_dir
        self.assertIn('.arg("-xf")', body)
        self.assertIn('.arg("-C")', body)
        # Note absence of zip-slip/symlink restriction flags like --no-same-owner or security exclusion filters
        self.assertNotIn("--exclude", body)
        self.assertNotIn("--no-same-permissions", body)
        self.assertNotIn("canonicalize", body)

    def test_verify_sha256_reads_path_not_locked_fd_or_in_memory_before_extract(self):
        # C02: verify_sha256 reads from Path, closes, then callers call extract_tar_bz2 on the same Path
        text = source("overlay-backend/src/download.rs")
        body = text[text.index("pub(crate) fn verify_sha256"):text.index("pub(crate) fn hex")]
        self.assertIn("std::fs::read(path)", body)
        # File is read in memory, validated, then returned. No file descriptor or handle is kept open.
        self.assertNotIn("File::open", body)
        self.assertNotIn("lock", body.lower())

    def test_ocr_install_extraction_flow_sequence(self):
        text = source("overlay-backend/src/ocr_install.rs")
        body = text[text.index("pub fn install("):text.index("fn dest_has_engine")]
        # Sequence: curl_download -> verify_sha256 -> extract_tar_bz2
        idx_dl = body.index("curl_download(BUNDLE_URL, &tarball)")
        idx_vf = body.index("verify_sha256(&tarball, BUNDLE_SHA256, \"OCR\")")
        idx_ex = body.index("extract_tar_bz2(&tarball, &root)")
        self.assertLess(idx_dl, idx_vf)
        self.assertLess(idx_vf, idx_ex)
        # Extraction target is &root (%APPDATA%\suflyor), trusting archive members to live under tesseract/
        self.assertIn("extract_tar_bz2(&tarball, &root)", body)

    def test_post_extract_verification_checks_expected_files(self):
        text = source("overlay-backend/src/ocr_install.rs")
        body = text[text.index("fn dest_has_engine"):text.index("#[cfg(test)]")]
        # Checks tesseract.exe and rus.traineddata exist
        self.assertIn('dest.join("tesseract.exe").is_file()', body)
        self.assertIn('dest.join("tessdata").join("rus.traineddata").is_file()', body)
        # If dest_has_engine fails, it cleans up dest
        install_body = text[text.index("if !dest_has_engine(&dest)"):text.index("on(OcrProgress::Installed)")]
        self.assertIn("std::fs::remove_dir_all(&dest)", install_body)

    def test_update_installer_writes_then_spawns_without_reverification(self):
        text = source("overlay-backend/src/update.rs")
        body_dl = text[text.index("pub async fn download_installer"):text.index("pub fn run_installer")]
        # in-memory check then write
        self.assertIn("hasher.update(&bytes)", body_dl)
        self.assertIn("std::fs::write(&path, &bytes)", body_dl)
        # run_installer spawns path directly
        body_run = text[text.index("pub fn run_installer"):text.index("#[cfg(test)]")]
        self.assertIn("std::process::Command::new(path)", body_run)
        self.assertIn(".spawn()", body_run)
        self.assertNotIn("verify", body_run.lower())

    def test_original_c01_c02_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker2_installers-C01"], "hypothesis")
        self.assertEqual(found["wave4_worker2_installers-C02"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
