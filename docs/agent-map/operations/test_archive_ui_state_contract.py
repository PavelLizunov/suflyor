"""Archive source and pure index model; no user data/native UI/destructive action."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[3]


def source(path):return (ROOT/path).read_text()


class ArchiveUiSourceFixtures(unittest.TestCase):
    def test_actual_list300_search60_and_open_query_failure_not_unavailable(self):
        s=source('slint-experiment/src/bin/overlay_host/aux_windows/archive.rs')
        self.assertIn('ARCHIVE_LIST_CAP: usize = 300',s)
        self.assertIn('ARCHIVE_LIST_CAP * 60 + 200 < 32_767',s)
        self.assertIn('st.search(&fts, 60)',s)
        initial=s[s.index('let initial: Option'):s.index('// Search-as-you-type')]
        self.assertIn('st.list_sessions().unwrap_or_default()',initial)
        self.assertIn('None => p.set_unavailable(true)',initial)
        self.assertNotIn('p.set_confirm_delete_index(-1)',initial)

    def test_query_rebuild_resets_rename_not_confirmations_and_no_model_id_binding(self):
        s=source('slint-experiment/src/bin/overlay_host/aux_windows/archive.rs')
        query=s[s.index('win.on_query_changed'):s.index('// Activate a row')]
        self.assertIn('p.set_renaming_index(-1)',query)
        self.assertNotIn('set_confirm_delete_index',query)
        self.assertNotIn('set_confirm_resummary_index',query)
        confirm=s[s.index('win.on_delete_confirmed'):s.index('win.on_delete_cancelled')]
        self.assertIn('archive_row_at(&results, idx)',confirm)
        self.assertNotIn('confirm_delete_title',confirm)
        self.assertNotIn('confirmed_session_id',confirm)

    def test_model_index_confirmation_refresh_changes_target_without_native_repro(self):
        rows=['session-A','session-B'];confirm_index=0;display_title=rows[confirm_index]
        rows=['session-C','session-A','session-B'] # asynchronous initial/query update
        acted_id=rows[confirm_index]
        self.assertNotEqual(display_title,acted_id)
        self.assertEqual(acted_id,'session-C') # pure possible operation sequence only

    def test_retranscribe_process_guard_survives_close_new_window_ui_false(self):
        s=source('slint-experiment/src/bin/overlay_host/aux_windows/archive.rs')
        self.assertIn('RETRANSCRIBE_BUSY: AtomicBool',s)
        self.assertIn('try_acquire_busy(&RETRANSCRIBE_BUSY)',s)
        fresh=s[s.index('let win = match ArchiveWindow::new()'):s.index('let store: StoreSlot')]
        self.assertNotIn('set_retranscribe_busy',fresh)
        close=s[s.index('win.on_close_requested'):s.index('// v0.17.1 — drag')]
        self.assertIn('*slot.borrow_mut() = None',close)
        self.assertNotIn('abort(',close)
        self.assertNotIn('RETRANSCRIBE_BUSY',close)

    def test_slint_confirm_escape_priority_busy_visibility_and_nonbusy_status_not_rendered(self):
        s=source('slint-experiment/ui/archive.slint')
        esc=s[s.index('key-pressed(event)'):s.index('HorizontalLayout {',s.index('key-pressed(event)'))]
        self.assertLess(esc.index('confirm-resummary-index'),esc.index('confirm-delete-index'))
        self.assertLess(esc.index('confirm-delete-index'),esc.index('renaming-index'))
        self.assertIn('if root.retranscribe-busy : Text',s)
        self.assertIn('if !root.retranscribe-busy && row.has-data',s)
        self.assertEqual(s.count('text: root.retranscribe-status'),1)
        self.assertIn('root.delete-confirmed(root.confirm-delete-index)',s)

    def test_session_snapshots_manual_rename_best_effort_and_confirm_then_refetch(self):
        s=source('slint-experiment/src/bin/overlay_host/aux_windows/archive.rs')
        self.assertIn('let recordings = Rc::new(recording_ids_snapshot())',s)
        self.assertIn('let ru = cfg.read().ui_language == "ru"',s)
        self.assertIn('win.set_stt_is_cloud(!cfg.read().stt_is_local())',s)
        self.assertIn('let active_id =',s)
        query=s[s.index('win.on_query_changed'):s.index('// Activate a row')]
        self.assertIn('conspect::session_ids()',query)
        self.assertNotIn('recording_ids_snapshot()',query)
        rename=s[s.index('win.on_rename_confirmed'):s.index('// v0.22.0 — ↻ regen')]
        self.assertIn('session_names::set(',rename)
        self.assertIn('p.invoke_query_changed(q)',rename)
        self.assertNotIn('if let Err',rename)


if __name__=='__main__':unittest.main()
