"""Credential/process source boundaries only, no secret storage/child/native action."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[3]


def source(path):return (ROOT/path).read_text()


class CredentialProcessSourceFixtures(unittest.TestCase):
    def test_windows_credential_blob_zeroing_and_credfree_both_utf8_paths(self):
        s=source('overlay-backend/src/credentials.rs')
        write=s[s.index('pub fn write(slot:'):s.index('pub fn read(slot:')]
        self.assertLess(write.index('CredWriteW'),write.index('blob.fill(0)'))
        read=s[s.index('pub fn read(slot:'):s.index('pub fn delete(slot:')]
        self.assertIn('String::from_utf8(bytes.to_vec())',read)
        self.assertEqual(read.count('CredFree('),1)
        self.assertLess(read.index('let parsed ='),read.index('CredFree(raw.cast())'))
        self.assertLess(read.index('CredFree(raw.cast())'),read.index('parsed.map(Some)'))
        self.assertNotIn('log::',read)

    def test_unix_plaintext_secure_mode_temp_cleanup_and_atomic_rename(self):
        s=source('overlay-backend/src/credentials.rs')
        unix=s[s.index('mod posix_credentials'):]
        self.assertIn('serde_json::to_vec_pretty',unix)
        self.assertIn('perms.set_mode(0o700)',unix);self.assertIn('.mode(0o600)',unix)
        self.assertIn('file.flush()',unix);self.assertNotIn('file.sync_all()',unix);self.assertIn('fs::remove_file(&tmp)',unix)
        self.assertIn('fs::rename(&tmp, path)',unix)
        self.assertNotIn('Keychain',unix)

    def test_direct_keys_not_config_fields_and_endpoint_secret_error_suppression(self):
        config=source('overlay-backend/src/config.rs')
        declaration=config[config.index('pub struct Config'):config.index('pub type SharedConfig')]
        self.assertNotIn('pub openai_key:',declaration);self.assertNotIn('pub anthropic_key:',declaration)
        secret=config[config.index('fn protected_provider_secret'):config.index('fn default_',config.index('fn protected_provider_secret'))]
        self.assertIn('credentials::read(slot)',secret)
        self.assertIn('Err(_) => String::new()',secret)
        settings=source('slint-experiment/src/bin/overlay_host/settings_ai.rs')
        block=settings[settings.index('win.on_openai_key_save'):settings.index('win.on_groq_api_key_save')]
        self.assertIn('credentials::write',block);self.assertNotIn('config::save',block)

    def test_jobobject_best_effort_handle_leak_and_assign_result_not_returned(self):
        s=source('overlay-backend/src/local_ai.rs')
        body=s[s.index('pub(crate) fn assign_to_lifetime_job'):s.index('fn launch_hidden_wait(')]
        self.assertIn('JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE',body)
        self.assertIn('OnceLock<usize>',body)
        self.assertIn('AssignProcessToJobObject(job, proc)',body)
        self.assertIn('log::warn!',body)
        self.assertNotIn('CloseHandle',body)
        self.assertIn('fn assign_to_lifetime_job(child: &Child) {',body)

    def test_windows_tts_attach_exists_but_broken_pipe_drops_unwaited_child_handle(self):
        s=source('overlay-backend/src/tts.rs')
        spawn=s[s.index('fn spawn_engine_sidecar'):s.index('fn tts_root')]
        self.assertIn('download::no_window(&mut cmd).spawn()',spawn)
        self.assertIn('assign_to_lifetime_job(&proc)',spawn)
        write=s[s.index('fn write_raw('):s.index('\n    }',s.index('fn write_raw('))+6]
        self.assertIn('self.proc = None',write)
        self.assertNotIn('.wait()',write);self.assertNotIn('.kill()',write)
        self.assertNotIn('impl Drop for Sidecar',s)

    def test_nemotron_guard_kill_wait_and_managed_stop_tree_are_distinct(self):
        n=source('overlay-backend/src/nemotron_diar.rs')
        self.assertIn('impl Drop for ChildGuard',n)
        self.assertIn('self.0.kill()',n);self.assertIn('self.0.wait()',n)
        self.assertIn('assign_to_lifetime_job(&child)',n)
        local=source('overlay-backend/src/local_ai.rs')
        stop=local[local.index('fn terminate_child_tree'):local.index('fn exe_path_for_pid(')]
        self.assertIn('kill_pid_tree(&pid.to_string())',stop)
        self.assertIn('"/T", "/F"',local)
        self.assertIn('child.kill()',stop);self.assertIn('child.wait()',stop)


if __name__=='__main__':unittest.main()
