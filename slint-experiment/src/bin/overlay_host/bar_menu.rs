use super::*;

const PINNABLE_IDS: &[&str] = &[
    "capture", "aggressive", "restore", "archive", "summary", "kb", "settings", "help", "tray",
];
const MAX_PINS: usize = 8;

// Unknown ids remain in Config; only the visible model is filtered and capped.
fn visible_pins(ids: &[String]) -> Vec<String> {
    let mut pins = Vec::new();
    for id in ids {
        if PINNABLE_IDS.contains(&id.as_str()) && !pins.contains(id) && pins.len() < MAX_PINS {
            pins.push(id.clone());
        }
    }
    pins
}

fn pins_model(ids: &[String]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        visible_pins(ids)
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

fn refresh_menu(menu: &BarMenuWindow, bar: &OverlayBarWindow) {
    menu.set_pins(bar.get_pins());
    menu.set_aggressive_active(bar.get_aggressive_active());
    menu.set_can_restore_tile(bar.get_can_restore_tile());
    menu.set_tray_available(bar.get_tray_available());
    menu.set_settings_open(bar.get_settings_open());
    menu.set_archive_open(bar.get_archive_open());
    menu.set_help_open(bar.get_help_open());
    menu.set_summary_busy(bar.get_summary_busy());
}

fn close_menu(
    menu: &BarMenuWindow,
    bar: &slint::Weak<OverlayBarWindow>,
    focus: &RefCell<LockMenuFocusState>,
) {
    focus.borrow_mut().close();
    menu.set_confirmation(SharedString::default());
    if let Err(e) = menu.hide() {
        diag!("bar-menu hide failed: {e}");
    }
    if let Some(bar) = bar.upgrade() {
        bar.set_menu_open(false);
    }
}

// Menu and pinned chips share this route; no action handler is duplicated.
fn dispatch_bar_action(bar: &OverlayBarWindow, id: &str, kb: &dyn Fn()) {
    match id {
        "capture" => bar.invoke_capture_clicked(),
        "aggressive" => bar.invoke_aggressive_toggle_clicked(),
        "restore" if bar.get_can_restore_tile() => bar.invoke_restore_tile_clicked(),
        "archive" => bar.invoke_archive_clicked(),
        "summary" => bar.invoke_summary_clicked(),
        "kb" => kb(),
        "settings" => bar.invoke_open_settings_clicked(),
        "help" => bar.invoke_help_clicked(),
        "tray" if bar.get_tray_available() => bar.invoke_hide_to_tray_clicked(),
        // These existing handlers do not require the legacy bar armed flags.
        "restart" => bar.invoke_restart_confirm(),
        "quit" => bar.invoke_quit_confirm(),
        _ => {}
    }
}

pub(super) fn wire_bar_menu(
    overlay: &OverlayBarWindow,
    menu: &Rc<BarMenuWindow>,
    cfg: &config::SharedConfig,
    palette: &Rc<RefCell<Option<PaletteWindow>>>,
    tiles: &TileWindows,
    state: &slint_replay::app_state::SharedState,
) -> Timer {
    overlay.set_pins(pins_model(&cfg.read().bar_pins));
    refresh_menu(menu, overlay);
    let focus = Rc::new(RefCell::new(LockMenuFocusState::default()));
    let weak = overlay.as_weak();
    let palette = palette.clone();
    let tiles = tiles.clone();
    let state = state.clone();
    let weak_for_kb = weak.clone();
    let kb: Rc<dyn Fn()> = Rc::new(move || {
        // Same F4 toggle semantics and the same palette opener.
        let open = palette.borrow().is_some();
        if open {
            if let Some(p) = palette.borrow_mut().take() {
                let _ = p.hide();
            }
        } else {
            open_palette(&palette, &tiles, &state, &weak_for_kb);
        }
    });
    {
        let weak = weak.clone();
        let kb = kb.clone();
        let menu_weak = Rc::downgrade(menu);
        let focus = focus.clone();
        overlay.on_pin_activated(move |id| {
            if let Some(menu) = menu_weak.upgrade() {
                close_menu(&menu, &weak, &focus);
            }
            if let Some(bar) = weak.upgrade() {
                // Never accept destructive ids through the pin callback.
                if PINNABLE_IDS.contains(&id.as_str()) {
                    dispatch_bar_action(&bar, id.as_str(), kb.as_ref());
                }
            }
        });
    }
    {
        let weak = weak.clone();
        let menu_weak = Rc::downgrade(menu);
        let focus = focus.clone();
        menu.on_action(move |id| {
            if let Some(menu) = menu_weak.upgrade() {
                close_menu(&menu, &weak, &focus);
            }
            if let Some(bar) = weak.upgrade() {
                dispatch_bar_action(&bar, id.as_str(), kb.as_ref());
            }
        });
    }
    {
        let weak = weak.clone();
        let menu_weak = Rc::downgrade(menu);
        let focus = focus.clone();
        overlay.on_menu_dismissed(move || {
            if let Some(menu) = menu_weak.upgrade() {
                close_menu(&menu, &weak, &focus);
            }
        });
    }
    {
        let weak = weak.clone();
        let menu_weak = Rc::downgrade(menu);
        let focus = focus.clone();
        menu.on_dismissed(move || {
            if let Some(menu) = menu_weak.upgrade() {
                close_menu(&menu, &weak, &focus);
            }
        });
    }
    {
        let cfg = cfg.clone();
        let weak = weak.clone();
        let menu_weak = Rc::downgrade(menu);
        menu.on_pin_toggled(move |id| {
            if !PINNABLE_IDS.contains(&id.as_str()) {
                return;
            }
            let model = {
                let mut c = cfg.write();
                let mut candidate = c.bar_pins.clone();
                if candidate.iter().any(|pin| pin == id.as_str()) {
                    candidate.retain(|pin| pin != id.as_str());
                } else if visible_pins(&candidate).len() < MAX_PINS {
                    candidate.push(id.to_string());
                } else {
                    return;
                }
                // Preserve unknown ids on disk, including when editing other pins.
                let previous = std::mem::replace(&mut c.bar_pins, candidate);
                if let Err(e) = overlay_backend::config::save(&c) {
                    c.bar_pins = previous;
                    diag!("bar-menu pin save failed: {e}");
                    return;
                }
                pins_model(&c.bar_pins)
            };
            if let Some(bar) = weak.upgrade() {
                bar.set_pins(model);
                if let Some(menu) = menu_weak.upgrade() {
                    refresh_menu(&menu, &bar);
                }
            }
        });
    }
    {
        use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
        use winit::event::WindowEvent;
        let weak = weak.clone();
        let menu_weak = Rc::downgrade(menu);
        let focus = focus.clone();
        menu.window().on_winit_window_event(move |_, event| {
            let WindowEvent::Focused(focused) = event else {
                return EventResult::Propagate;
            };
            let transition = focus.borrow_mut().focus_changed(*focused);
            if transition == FocusTransition::DismissOutside {
                let owner_focused = weak
                    .upgrade()
                    .and_then(|bar| grab_hwnd(bar.window()).ok())
                    .is_some_and(is_foreground_window);
                if owner_focused {
                    focus.borrow_mut().suppress_next_open();
                    let clear = focus.clone();
                    Timer::single_shot(Duration::from_millis(150), move || {
                        clear.borrow_mut().clear_suppressed_open();
                    });
                }
                if let Some(menu) = menu_weak.upgrade() {
                    // close() preserves owner-chip suppression until the
                    // matching click drains, and invalidates pending reveals.
                    close_menu(&menu, &weak, &focus);
                }
            }
            EventResult::Propagate
        });
    }
    {
        let menu = menu.clone();
        let cfg = cfg.clone();
        let weak = weak.clone();
        let focus = focus.clone();
        overlay.on_menu_opened(move |anchor_x, anchor_y| {
            if focus.borrow_mut().consume_suppressed_open() {
                return;
            }
            if menu.window().is_visible() {
                close_menu(&menu, &weak, &focus);
                return;
            }
            let Some(bar) = weak.upgrade() else { return };
            refresh_menu(&menu, &bar);
            menu.set_confirmation(SharedString::default());
            menu.global::<ui::Theme>()
                .set_scheme(clamp_scheme(cfg.read().color_scheme));
            let Ok(bar_hwnd) = grab_hwnd(bar.window()) else { return };
            #[cfg(windows)]
            let (bar_left, bar_top) = {
                let pos = bar.window().position();
                if pos.x != -32000 && pos.x != 0 {
                    (pos.x, pos.y)
                } else {
                    get_window_rect(bar_hwnd)
                        .map(|(l, t, _, _)| (l, t))
                        .unwrap_or((pos.x, pos.y))
                }
            };
            #[cfg(target_os = "macos")]
            let (bar_left, bar_top) = match get_window_rect(bar_hwnd) {
                Ok((left, top, _, _)) => (left, top),
                Err(_) => return,
            };
            #[cfg(windows)]
            let scale = bar.window().scale_factor().max(0.1);
            #[cfg(target_os = "macos")]
            let scale = 1.0_f32;
            let logical_height = if bar.get_tray_available() {
                386.0
            } else {
                358.0
            };
            menu.window()
                .set_size(slint::LogicalSize::new(408.0, logical_height));
            let width = (408.0 * scale).round() as i32;
            let height = (logical_height * scale).round() as i32;
            let anchor_x = bar_left + (anchor_x * scale).round() as i32;
            let anchor_y = bar_top + (anchor_y * scale).round() as i32;
            let Some(work) = work_area_for_point(anchor_x, anchor_y) else { return };
            let gap = (4.0 * scale).round() as i32;
            let max_x = (work.right - width).max(work.left);
            let max_y = (work.bottom - height).max(work.top);
            let x = (anchor_x - width).clamp(work.left, max_x);
            // The callback anchors the top-row chip; placement clears the entire bar.
            let bar_bottom =
                bar_top + (bar.global::<ui::BarSpec>().get_height() * scale).round() as i32;
            let below = anchor_y.max(bar_bottom) + gap;
            let y = if below + height <= work.bottom {
                below
            } else {
                bar_top - height - gap
            }
            .clamp(work.top, max_y);
            set_platform_window_position(menu.window(), -32000, -32000);
            let generation = focus.borrow_mut().begin_open();
            if let Err(e) = menu.show() {
                close_menu(&menu, &weak, &focus);
                diag!("bar-menu show failed: {e}");
                return;
            }
            bar.set_menu_open(true);
            let focus_for_reveal = focus.clone();
            let reveal = Rc::new(move |window: &BarMenuWindow| {
                if !focus_for_reveal.borrow().is_current_open(generation) {
                    return true;
                }
                let Ok(hwnd) = grab_hwnd(window.window()) else { return false };
                set_window_owner(hwnd, bar_hwnd);
                if set_skip_taskbar(hwnd, true).is_err() || set_always_on_top(hwnd, true).is_err() {
                    return false;
                }
                if global_stealth() && set_stealth(hwnd, true).is_err() {
                    return false;
                }
                if move_window_pos_only(hwnd, x, y).is_err() {
                    return false;
                }
                set_platform_window_position(window.window(), x, y);
                if !focus_for_reveal.borrow_mut().mark_revealed(generation) {
                    return true;
                }
                focus_window(hwnd);
                let arm = focus_for_reveal.clone();
                Timer::single_shot(Duration::ZERO, move || {
                    arm.borrow_mut().arm(generation);
                });
                true
            });
            let focus_for_fallback = focus.clone();
            let weak_for_fallback = weak.clone();
            let fallback = Rc::new(move |window: &BarMenuWindow| {
                if focus_for_fallback.borrow().is_current_open(generation) {
                    close_menu(window, &weak_for_fallback, &focus_for_fallback);
                    diag!("bar-menu reveal failed: native window unavailable");
                }
            });
            realize_with_retries(menu.as_ref(), reveal, fallback);
        });
    }
    // Retained by main; reflect asynchronous window/busy/toggle state while open.
    let poll = Timer::default();
    let menu_weak = Rc::downgrade(menu);
    poll.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        if let (Some(menu), Some(bar)) = (menu_weak.upgrade(), weak.upgrade()) {
            if menu.window().is_visible() {
                refresh_menu(&menu, &bar);
            }
        }
    });
    poll
}
