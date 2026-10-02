"""UI/stream source and pure interleaving models, not Rust race/live UI tests."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[3]


def source(path):return (ROOT/path).read_text()


class OverlayTileStateSourceFixtures(unittest.TestCase):
    def test_slint_submit_busy_empty_and_block_model_resets(self):
        s=source('slint-experiment/ui/tile.slint')
        submit=s[s.index('function do-submit()'):s.index('\n    Rectangle {',s.index('function do-submit()'))]
        self.assertIn('if (root.followup-busy)',submit)
        self.assertIn('if (root.followup-text == "")',submit)
        self.assertLess(submit.index('root.followup-submitted'),submit.index('root.followup-text = ""'))
        self.assertIn('changed blocks =>',s)
        reset=s[s.index('changed blocks =>'):s.index('\n',s.index('changed blocks =>'))]
        for token in ('mark-anchor = -1','capture-pending = false','select-mode = false'):self.assertIn(token,reset)
        self.assertNotIn('followup-busy = false',reset)

    def test_generation_gate_check_before_emit_separate_slot_swap_and_ui_closure(self):
        s=source('slint-experiment/src/bin/overlay_host/tile_controller.rs')
        emit=s[s.index('impl RuntimeEvents for GenGatedEvents'):s.index('fn spawn_tile_full',s.index('impl RuntimeEvents for GenGatedEvents'))]
        self.assertLess(emit.index('.load(Ordering::SeqCst)'),emit.index('self.inner.emit'))
        install=s[s.index('pub(crate) fn install_streaming_tile'):s.index('struct GenGatedEvents')]
        self.assertLess(install.index('bridge.stream_gen.fetch_add'),install.index('*slot = Some(new_tile)'))
        delta=s[s.index('ai::AiEvent::Delta { text } => {',s.index('fn handle_ai_event')):s.index('ai::AiEvent::Done { reason } => {',s.index('fn handle_ai_event'))]
        self.assertIn('stream.accumulated.push_str(&text)',delta)
        closure=delta[delta.index('slint::invoke_from_event_loop'):]
        self.assertNotIn('stream_gen',closure)

    def test_model_gate_pass_then_slot_swap_allows_wrong_slot_mechanism_only(self):
        # Explicit pure model: same sequencing implied by separate source operations.
        generation=1; event_generation=1; current={'id':'A','text':''}
        allowed=(generation==event_generation)
        current={'id':'B','text':''}; generation+=1  # new ask after old check
        if allowed:current['text']+='old-A-delta'  # forwarded event reads active slot
        self.assertEqual(current,{'id':'B','text':'old-A-delta'})
        # This demonstrates a TOCTOU precondition, not actual Rust threading/native repro.
        self.assertNotEqual(generation,event_generation)

    def test_terminal_done_error_clear_busy_but_receiver_eof_has_no_terminal_synthesis(self):
        s=source('slint-experiment/src/bin/overlay_host/tile_controller.rs')
        handler=s[s.index('fn handle_ai_event'):s.index('fn inc_ai_in_flight')]
        self.assertEqual(handler.count('tile.set_followup_busy(false)'),2)
        runtime=source('overlay-backend/src/runtime.rs')
        body=runtime[runtime.index('pub async fn ask_stream_loop('):]
        self.assertIn('while let Some(ev) = ai_rx.recv().await',body)
        self.assertIn('events.emit("ai:event", payload)',body)
        eof=body[body.index('let input_tokens ='):]
        self.assertNotIn('"ai:event"',eof)
        self.assertNotIn('AiEvent::Done',eof);self.assertNotIn('AiEvent::Error',eof)

    def test_ptt_error_does_not_clear_busy_mlx_error_does_and_missing_history_returns(self):
        p=source('slint-experiment/src/bin/overlay_host/tile_ptt.rs')
        body=p[p.index('pub(crate) fn ptt_tile_error'):p.index('pub(crate) fn fire_ptt_ask')]
        self.assertNotIn('set_followup_busy(false)',body)
        self.assertIn('tile.set_followup_busy(true)',p)
        mlx=source('slint-experiment/src/bin/overlay_host/mlx_lifecycle.rs')
        self.assertIn('tile.set_followup_busy(false)',mlx[mlx.index('pub(crate) fn show_mlx_runtime_error'):])
        follow=source('slint-experiment/src/bin/overlay_host/tile_followup.rs')
        missing=follow[follow.index('None => {',follow.index('pub(crate) fn fire_followup_ask')):follow.index('// New request =')]
        self.assertIn('t.set_followup_busy(false)',missing)
        self.assertIn('return;',missing)

    def test_shared_abort_registry_close_and_ptt_independent_limit(self):
        w=source('slint-experiment/src/bin/overlay_host/tile_window.rs')
        self.assertIn('TILE_STREAMS',w);self.assertIn('m.borrow_mut().remove(&tile_id)',w)
        main=source('slint-experiment/src/bin/overlay_host_windows.rs')
        self.assertIn('MAX_LIVE_TILES: usize = 16',main)
        self.assertIn('abort_tile_stream(dropped.get_tile_id())',main)
        ptt=source('slint-experiment/src/bin/overlay_host/tile_ptt.rs')
        self.assertIn('never stored in ai_task',ptt)
        self.assertNotIn('register_tile_stream(',ptt)


    def test_original_c01_c03_statuses_remain_hypotheses(self):
        import json
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker3_tile-C01"], "hypothesis")
        self.assertEqual(found["wave3_worker3_tile-C03"], "hypothesis")


if __name__=='__main__':unittest.main()
