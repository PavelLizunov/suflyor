"""Bounded source assertions, not Rust/native/endpoint/UI/privacy acceptance."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
HOST = "slint-experiment/src/bin/overlay_host/"


def source(path):
    return (ROOT / path).read_text(encoding="utf-8")


class StartupHealthSourceTests(unittest.TestCase):
    def test_preflight_order_first_run_and_relaunch_wait(self):
        s = source(HOST + "app_bootstrap.rs")
        names = ["migrate_data_root()", "logging::init()", "acquire_singleton(wait_ms)",
                 "repair_unfinalized_in", "Builder::new_multi_thread()", "let first_run"]
        positions = [s.index(x) for x in names]
        self.assertEqual(positions, sorted(positions))
        self.assertIn('if is_relaunch { 8_000 } else { 0 }', s)
        first = s[s.index("let first_run"):s.index("Ok(Some(AppBootstrap")]
        self.assertIn('.map(|p| !p.exists())', first)
        self.assertNotIn("wizard_done", first)
        self.assertIn(".unwrap_or(false)", first)

    def test_first_run_wizard_is_windows_only_and_existing_config_recovery(self):
        s = source("slint-experiment/src/bin/overlay_host_windows.rs")
        block = s[s.index("    if first_run {") - 25:s.index('if !first_run && std::env::var("SLINT_OVERLAY_RECOVERY")')]
        self.assertIn("#[cfg(windows)]", block)
        self.assertIn("from_millis(2200)", block)
        self.assertIn("open_wizard(", block)
        self.assertIn('#[cfg(windows)]\n#[path = "overlay_host/wizard.rs"]', s)
        settings = source(HOST + "settings_controller.rs")
        self.assertIn("on_open_wizard_clicked", settings)

    def test_wizard_mode_saves_but_finish_only_clears_window_and_reopen_reuses(self):
        s = source(HOST + "wizard.rs")
        self.assertNotIn("wizard_done", s)
        mode = s[s.index("win.on_mode_selected"):s.index("// Step 2:", s.index("win.on_mode_selected"))]
        self.assertIn("if let Err(e) = overlay_backend::config::save(&c)", mode)
        opener = s[s.index("pub(crate) fn open_wizard"):]
        finished = opener[opener.index("win.on_finished"):opener.index("    present_window_stealth_aware(", opener.index("win.on_finished"))]
        self.assertIn("*slot.borrow_mut() = None", finished)
        self.assertNotIn("config::save", finished)
        self.assertIn("if let Some(existing) = slot.as_ref()", opener)
        self.assertNotIn("set_step(0)", opener)
        self.assertIn("present_window_stealth_aware", opener)

    def test_wizard_summary_avoids_readiness_detail_but_keeps_live_detail(self):
        s = source(HOST + "wizard.rs")
        summary = s[s.index("pub(crate) fn refill_wizard_summary"):s.index("/// Wire all")]
        self.assertIn("pick(w.get_ai_detail(), r.ai.configured)", summary)
        self.assertNotIn("r.ai.detail", summary)
        self.assertIn("if !live.is_empty()", summary)
        # A raw truncated test error is not proven redacted by a summary comment.
        check = s[s.index("win.on_ai_test_clicked"):s.index("// Step 3")]
        self.assertIn('let chain = format!("{e:#}")', check)
        self.assertIn('format!("[err] {chain}")', check)
        self.assertIn('.take(80)', check)
        self.assertNotIn("redact_urls(", check)

    def test_diagnostics_readiness_levels_and_raw_detail_copy(self):
        s = source(HOST + "diagnostics.rs")
        populate = s[s.index("pub(crate) fn populate_diagnostics"):s.index("pub(crate) fn redact_ipv4")]
        self.assertIn("set_diag_ai_level(if r.ai.configured { 0 } else { 2 })", populate)
        self.assertIn("set_diag_ai_detail(SharedString::from(r.ai.detail))", populate)
        self.assertIn("set_diag_mic_level(3)", populate)
        self.assertIn("set_diag_sys_level(3)", populate)
        self.assertNotIn("redact_urls(", populate)

    def test_diagnostics_live_mic_sys_and_synthetic_vision_are_not_config_only(self):
        s = source(HOST + "diagnostics.rs")
        body = s[s.index("win.on_diagnostics_check_all_clicked"):s.index('// F — "Собрать логи"')]
        for token in ("test_connection_endpoint(ai_endpoint)", "test_connection_backend(&stt_backend)",
                      "vision::test_connection_endpoint(ep)", "try_acquire_mic()",
                      "record_mic_blocking(3000, mic_device)", "play_tone_and_capture(sys_device)"):
            self.assertIn(token, body)
        self.assertLess(body.index("record_mic_blocking"), body.index("drop(mic_guard)"))
        vision = source("overlay-backend/src/vision.rs")
        self.assertIn('build_vision_request(SYNTHETIC_TEST_IMAGE_DATA_URL, "Reply with: ok")', vision)

    def test_health_thresholds_and_latest_ai_error_order(self):
        s = source("overlay-backend/src/health.rs")
        for token in ("ai_err_raw >= ai_ok_raw", "Self::classify(ai_age, 180_000, 600_000)",
                      "Self::classify(mic_age, 15_000, 60_000)",
                      "Self::classify(stt_age, 60_000, 180_000)", "Self::worst("):
            self.assertIn(token, s)
        host = source("slint-experiment/src/slint_session.rs")
        gate = host[host.index("while let Some(chunk) = src_rx.recv().await"):]
        self.assertLess(gate.index(".last_mic_frame_ms"), gate.index("if paused"))
        self.assertIn("MissedTickBehavior::Skip", host)
        self.assertIn('events_for_tick.emit("health:update", payload)', host)

    def test_report_and_log_scrub_paths_are_distinct_from_live_checks(self):
        s = source(HOST + "diagnostics.rs")
        report = s[s.index("pub(crate) fn build_diag_report"):s.index("pub(crate) fn wire_diagnostics")]
        logs = s[s.index("fn collect_redacted_log"):s.index("#[cfg(test)]")]
        for body in (report, logs):
            for fn in ("redact_user_home", "redact_urls", "redact_ipv4", "redact_secrets"):
                self.assertIn(fn, body)
        self.assertIn("std::fs::read_to_string", logs)
        self.assertIn("std::fs::write", logs)
        self.assertLess(logs.index("redact_ipv4"), logs.index("std::fs::write"))


if __name__ == "__main__":
    unittest.main()
