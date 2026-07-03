//! G1 (ТЗ follow-up 2026-07-03) — layout-independent Ctrl+C / V / X / A / Z / Y.
//!
//! Slint 1.16 exposes only the layout-PRODUCED character in a key event (no physical
//! key / scancode — verified in `i-slint-common` `builtin_structs.rs` + winit backend),
//! so its built-in editing shortcuts match Latin c/v/x/a and silently die under any
//! non-US layout: the RU physical V-key emits "м", French AZERTY differs again, etc. A
//! per-alphabet character shim (the old ф/с/м/ч approach) can't scale to every layout.
//!
//! Fix (fable-designed, B-prime): intercept winit key events BEFORE Slint via the
//! official per-window filter (`WinitWindowAccessor::on_winit_window_event`, feature
//! `unstable-winit-030`). For a Ctrl-only combo whose produced char is NON-ASCII,
//! translate the PHYSICAL key to the virtual key under the ACTIVE layout
//! (`MapVirtualKeyW` — the native Windows shortcut semantics, correct for AZERTY etc.);
//! if it resolves to a/c/v/x/z/y, re-dispatch a synthetic ASCII key so Slint's built-in
//! shortcut fires on the FOCUSED field. Latin layouts take an early `is_ascii()` out
//! (zero regression); AltGr (= Ctrl+Alt) and Ctrl+Shift are excluded.
//!
//! This covers every editable field (built-in TextInput shortcuts) + read-only
//! `SelectableText` (copy/select-all; paste stays correctly blocked) with no `.slint`
//! edits, and lets the three old Cyrillic shims be deleted.
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, MapVirtualKeyW, MAPVK_VSC_TO_VK_EX, VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_SHIFT,
};

/// Wire the layout-independent shortcut filter onto a Slint window. Idempotent — Slint
/// stores a single per-window filter, so re-installing replaces it (nothing else uses it).
pub(crate) fn install(win: &slint::Window) {
    use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
    use winit::event::{ElementState, WindowEvent};
    use winit::keyboard::Key;
    use winit::platform::scancode::PhysicalKeyExtScancode;

    win.on_winit_window_event(|slint_win, ev| {
        let WindowEvent::KeyboardInput {
            event,
            is_synthetic: false,
            ..
        } = ev
        else {
            return EventResult::Propagate;
        };
        // Latin layouts already match Slint's built-in shortcuts — leave them untouched
        // (structurally excludes any US/DE/FR regression from the new path).
        if matches!(&event.logical_key, Key::Character(c) if c.is_ascii()) {
            return EventResult::Propagate;
        }
        // Ctrl only: excluding Alt keeps AltGr (Ctrl+Alt) combos out; excluding Shift
        // avoids hijacking Ctrl+Shift+<letter>.
        if !(key_down(VK_CONTROL) && !key_down(VK_MENU) && !key_down(VK_SHIFT)) {
            return EventResult::Propagate;
        }
        let Some(letter) = event
            .physical_key
            .to_scancode()
            .and_then(|sc| vk_to_letter(unsafe { MapVirtualKeyW(sc, MAPVK_VSC_TO_VK_EX) }))
            .filter(|c| matches!(c, 'a' | 'c' | 'v' | 'x' | 'z' | 'y'))
        else {
            return EventResult::Propagate;
        };
        let text = slint::SharedString::from(letter);
        let synth = match event.state {
            ElementState::Pressed => slint::platform::WindowEvent::KeyPressed { text },
            ElementState::Released => slint::platform::WindowEvent::KeyReleased { text },
        };
        // Core recomputes modifiers from its own tracked state, so the physically-held
        // Ctrl still applies → the synthetic letter arrives as Ctrl+<letter> on the
        // focused item. Swallow the original so the "м" isn't also handled.
        let _ = slint_win.try_dispatch_event(synth);
        EventResult::PreventDefault
    });
}

/// True if the given virtual key is currently held (GetKeyState high bit).
fn key_down(vk: VIRTUAL_KEY) -> bool {
    (unsafe { GetKeyState(vk.0 as i32) } as u16 & 0x8000) != 0
}

/// Pure part of the scancode→letter mapping: a virtual-key code (as returned by
/// `MapVirtualKeyW`) → its ASCII letter, lowercased; None if it isn't A–Z. Split out so
/// the letter logic is unit-testable without a live keyboard layout.
fn vk_to_letter(vk: u32) -> Option<char> {
    u8::try_from(vk)
        .ok()
        .filter(u8::is_ascii_uppercase)
        .map(|b| b.to_ascii_lowercase() as char)
}

#[cfg(test)]
mod tests {
    use super::vk_to_letter;

    #[test]
    fn vk_to_letter_maps_letters_only() {
        assert_eq!(vk_to_letter(0x56), Some('v')); // VK_V
        assert_eq!(vk_to_letter(0x43), Some('c')); // VK_C
        assert_eq!(vk_to_letter(0x41), Some('a')); // VK_A
        assert_eq!(vk_to_letter(0x58), Some('x')); // VK_X
        assert_eq!(vk_to_letter(0x30), None); // '0' digit — not a letter
        assert_eq!(vk_to_letter(0x11), None); // VK_CONTROL
        assert_eq!(vk_to_letter(0), None); // unmapped
        assert_eq!(vk_to_letter(0x1_0000), None); // out of u8 range
    }
}
