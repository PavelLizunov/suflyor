//! Throwaway offscreen renders of the windows changed on 2026-10-25 (scratch
//! branch only). Run: `cargo test --bin overlay-host ui_shots -- --exact
//! ui_shots::render` with `UI_SHOTS_DIR` and `UI_SHOTS_SET` (after|before).

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize, SharedString};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

thread_local! {
    static LAST: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

struct ShotPlatform;

impl Platform for ShotPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let w = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        LAST.with(|l| *l.borrow_mut() = Some(w.clone()));
        Ok(w)
    }
}

fn take_adapter() -> Rc<MinimalSoftwareWindow> {
    LAST.with(|l| l.borrow_mut().take()).expect("window adapter")
}

fn out_dir() -> PathBuf {
    PathBuf::from(std::env::var("UI_SHOTS_DIR").unwrap_or_else(|_| "ui-shots".into()))
}

/// Render, composite over a desktop grey, trim the uniform bottom, save PNG.
fn save(adapter: &MinimalSoftwareWindow, w: u32, h: u32, name: &str, trim: bool) {
    for _ in 0..3 {
        slint::platform::update_timers_and_animations();
    }
    let mut buf = vec![PremultipliedRgbaColor::default(); (w * h) as usize];
    adapter.request_redraw();
    adapter.draw_if_needed(|r| {
        r.render(&mut buf, w as usize);
    });
    let bg = [0x2b_u32, 0x2f, 0x37];
    let mut rgb: Vec<u8> = Vec::with_capacity((w * h * 3) as usize);
    for p in &buf {
        let a = u32::from(p.alpha);
        for (c, b) in [u32::from(p.red), u32::from(p.green), u32::from(p.blue)]
            .into_iter()
            .zip(bg)
        {
            rgb.push((c + b * (255 - a) / 255).min(255) as u8);
        }
    }
    let mut rows = h;
    if trim {
        let row = |y: u32| &rgb[(y * w * 3) as usize..((y + 1) * w * 3) as usize];
        let last = row(h - 1).to_vec();
        let x0 = (w / 3 * 3) as usize;
        let mut y = h - 1;
        while y > 0 && row(y)[x0..] == last[x0..] {
            y -= 1;
        }
        rows = (y + 28).clamp(400.min(h), h);
    }
    let path = out_dir().join(format!("{name}.png"));
    let file = std::fs::File::create(&path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, rows);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().expect("png header");
    wr.write_image_data(&rgb[..(w * rows * 3) as usize])
        .expect("png data");
    eprintln!("ui_shots: {} {w}x{rows}", path.display());
}

fn select(lang: &str) {
    if let Err(e) = slint::select_bundled_translation(lang) {
        assert_eq!(lang, "en", "select {lang}: {e:?}");
    }
}

fn settings(lang: &str, tab: i32, name: &str, setup: impl FnOnce(&crate::ui::SettingsWindow)) {
    let win = crate::ui::SettingsWindow::new().expect("Settings");
    let adapter = take_adapter();
    let (w, h) = (720, 3200);
    win.window().set_size(PhysicalSize::new(w, h));
    win.set_active_tab(tab);
    win.global::<crate::ui::Platform>().set_is_linux(true);
    setup(&win);
    win.show().expect("show");
    save(&adapter, w, h, &format!("{lang}-settings-{name}"), true);
    win.hide().expect("hide");
}

#[test]
fn render() {
    slint::platform::set_platform(Box::new(ShotPlatform)).expect("platform");
    std::fs::create_dir_all(out_dir()).expect("out dir");
    let after = std::env::var("UI_SHOTS_SET").map_or(true, |s| s != "before");
    // Slint registers the bundled translations with the first component.
    let _warm = crate::ui::SettingsWindow::new().expect("warm-up window");
    let _ = take_adapter();
    for lang in ["en", "ru"] {
        select(lang);
        let ru = lang == "ru";
        settings(lang, 6, "recording", |_| {});
        settings(lang, 10, "profiles", |_| {});
        settings(lang, 11, "ai-providers", |w| w.set_ai_provider_index(0));
        settings(lang, 14, "memory", |_| {});
        settings(lang, 17, "vision", |w| {
            w.set_vision_same_available(false);
            w.set_vision_provider_index(1);
        });

        let help = crate::ui::HelpWindow::new().expect("Help");
        let adapter = take_adapter();
        help.window().set_size(PhysicalSize::new(640, 2400));
        help.global::<crate::ui::Platform>().set_is_linux(true);
        help.show().expect("show");
        save(&adapter, 640, 2400, &format!("{lang}-help"), true);
        help.hide().expect("hide");

        if !after {
            continue;
        }
        settings(lang, 13, "diagnostics-mic", |w| {
            w.set_diag_mic_level(4);
            w.set_diag_mic_detail(SharedString::from(crate::diagnostics::mic_open_failed(
                ru,
                ("Check all", "Проверить всё"),
            )));
        });
        settings(lang, 4, "updates-error", |w| {
            let e = anyhow::anyhow!("GET releases/latest: connection reset");
            w.set_update_status(SharedString::from(
                crate::settings_updates::update_failure_copy(&e, ru),
            ));
        });

        let wiz = crate::ui::WizardWindow::new().expect("Wizard");
        let adapter = take_adapter();
        wiz.window().set_size(PhysicalSize::new(760, 560));
        wiz.set_step(3);
        wiz.set_mic_level(4);
        wiz.set_mic_detail(SharedString::from(crate::diagnostics::mic_open_failed(
            ru,
            ("Record test", "Записать тест"),
        )));
        wiz.show().expect("show");
        save(&adapter, 760, 560, &format!("{lang}-wizard-mic"), false);
        wiz.hide().expect("hide");

        let tile = crate::ui::TileWindow::new().expect("Tile");
        let adapter = take_adapter();
        tile.window().set_size(PhysicalSize::new(460, 360));
        let (title, _, trigger, _) =
            crate::vision_capture::vision_tile_copy(overlay_backend::vision::VisionMode::Ocr, ru);
        tile.set_tile_title(SharedString::from(title));
        tile.set_trigger_label(SharedString::from(trigger));
        tile.set_trigger_tone(crate::ui::TileTone::Cyan);
        let failure = overlay_backend::ocr::OcrFailure::MissingLanguageData(
            "tesseract-data-rus".into(),
        );
        let detail = crate::vision_capture::ocr_failure_text(&failure, ru);
        crate::fill_ocr_error_tile(tile.as_weak(), ru, Some(&detail));
        tile.show().expect("show");
        save(&adapter, 460, 360, &format!("{lang}-tile-ocr-error"), false);
        tile.hide().expect("hide");

        // Same format as archive::summary_tile_title (private to aux_windows).
        let tile = crate::ui::TileWindow::new().expect("Tile");
        let adapter = take_adapter();
        tile.window().set_size(PhysicalSize::new(460, 360));
        let (label, meeting, body) = if ru {
            ("Сводка", "Встреча с командой", "**Итоги**\n\n- Порт на Omarchy: CI зелёный на трёх ОС.\n- Осталась живая проверка на ВМ.")
        } else {
            ("Summary", "Team meeting", "**Outcome**\n\n- Omarchy port: CI green on three OSes.\n- A live check on the VM remains.")
        };
        tile.set_tile_title(SharedString::from(format!("{label} · {meeting}")));
        tile.set_blocks(slint::ModelRc::new(slint::VecModel::from(crate::to_md_blocks(body))));
        tile.show().expect("show");
        save(&adapter, 460, 360, &format!("{lang}-tile-summary"), false);
        tile.hide().expect("hide");
    }
}
