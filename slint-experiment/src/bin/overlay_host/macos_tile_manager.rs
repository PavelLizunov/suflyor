//! macOS multi-tile window manager for the suflyor overlay host.
//!
//! Spawns and tracks up to N independent `TileWindow` instances on macOS
//! so AI answer tiles can remain visible simultaneously.

use std::cell::RefCell;
use std::collections::VecDeque;

use slint::ComponentHandle;
use slint::SharedString;

use crate::ui;

const MAX_MACOS_TILES: usize = 4;

pub(super) struct MacTileManager {
    tiles: RefCell<VecDeque<ui::TileWindow>>,
}

fn md_blocks(source: &str) -> Vec<ui::MarkdownBlock> {
    slint_replay::markdown::parse(source)
        .into_iter()
        .map(|block| ui::MarkdownBlock {
            kind: block.kind,
            text: SharedString::from(block.text),
            display_text: SharedString::from(block.display_text),
            lang: SharedString::from(block.lang),
            marked: false,
        })
        .collect()
}

impl MacTileManager {
    pub(super) fn new() -> Self {
        Self {
            tiles: RefCell::new(VecDeque::new()),
        }
    }

    /// Present a new tile request as an independent floating `TileWindow`.
    pub(super) fn present_tile(
        &self,
        spec: overlay_backend::events::TileSpec,
        kind: overlay_backend::events::TileKind,
    ) {
        let win = match ui::TileWindow::new() {
            Ok(w) => w,
            Err(e) => {
                slint_replay::logging::line(&format!("[macos] TileWindow::new failed: {e}"));
                return;
            }
        };

        let title = match kind {
            overlay_backend::events::TileKind::Mic => "Микрофон",
            overlay_backend::events::TileKind::System => "Собеседник",
            overlay_backend::events::TileKind::Ai => "Ответ AI",
            overlay_backend::events::TileKind::Auto => "Авто-ответ",
            _ => "Ответ AI",
        };

        win.set_tile_title(SharedString::from(title));
        win.set_source_label(SharedString::from(match kind {
            overlay_backend::events::TileKind::Mic => "mic",
            overlay_backend::events::TileKind::System => "sys",
            _ => "ai",
        }));

        let text = format!("**{}**\n\n{}", spec.question, spec.answer);
        let blocks = md_blocks(&text);
        win.set_blocks(slint::ModelRc::new(slint::VecModel::from(blocks)));

        let _ = win.show();
        if let Err(e) = slint_replay::native::window::configure_floating(win.window()) {
            slint_replay::logging::line(&format!("[macos] tile configure_floating failed: {e}"));
        }
        let _ = slint_replay::native::window::raise_key_front(win.window());

        let mut queue = self.tiles.borrow_mut();
        if queue.len() >= MAX_MACOS_TILES {
            if let Some(oldest) = queue.pop_front() {
                let _ = oldest.hide();
            }
        }
        queue.push_back(win);
    }
}
