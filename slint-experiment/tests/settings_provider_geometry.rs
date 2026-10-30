//! Provider groups must report their wrapped height to the Settings scroll area.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

mod ui {
    slint::include_modules!();
}

fn labelled(win: &ui::SettingsWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(win, label)
        .next()
        .unwrap_or_else(|| panic!("missing Settings element: {label}"))
}

fn scroll_to_end(win: &ui::SettingsWindow, width: f32) {
    win.window()
        .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
            position: LogicalPosition::new(width - 40.0, 500.0),
            delta_x: 0.0,
            delta_y: -100_000.0,
        });
    slint::platform::update_timers_and_animations();
}

fn card_for(win: &ui::SettingsWindow, title: &str) -> ElementHandle {
    let title = labelled(win, title);
    let pos = title.absolute_position();
    ElementHandle::find_by_element_type_name(win, "SettingsCard")
        .filter(|card| {
            let origin = card.absolute_position();
            let size = card.size();
            pos.x >= origin.x
                && pos.x < origin.x + size.width
                && (pos.y - origin.y - 12.0).abs() < 1.0
        })
        .min_by(|a, b| a.size().width.total_cmp(&b.size().width))
        .expect("card containing its title")
}

#[test]
fn provider_cards_contain_wrapped_content_in_both_languages() {
    i_slint_backend_testing::init_no_event_loop();
    for (language, test, models, capabilities, caption) in [
        (
            "",
            "Test OpenAI connection",
            "Choose a model",
            "What the model can do",
            "Fast answers can search the web and read public pages when enabled: at most two lookups with a 15-second lookup budget. The model decides when to look up; web queries go to DuckDuckGo. A server without tool support answers without web lookup.",
        ),
        (
            "ru",
            "Проверить подключение OpenAI",
            "Выберите модель",
            "Возможности модели",
            "Быстрые ответы тоже могут искать в интернете и читать публичные страницы, если поиск включён: до двух обращений с общим бюджетом поиска 15 секунд. Модель решает, когда искать; запросы отправляются в DuckDuckGo. Сервер без поддержки инструментов отвечает без веб-поиска.",
        ),
    ] {
        if !language.is_empty() {
            slint::select_bundled_translation(language).expect("select interface language");
        }
        let win = ui::SettingsWindow::new().expect("create Settings");
        win.set_active_tab(11);
        win.show().expect("show Settings");
        for width in [720.0, 600.0, 480.0] {
            win.window().set_size(LogicalSize::new(width, 600.0));
            win.set_ai_provider_index(2);
            scroll_to_end(&win, width);
            let connections = if language == "ru" { "Подключения" } else { "Connections" };
            labelled(&win, connections).invoke_accessible_default_action();
            scroll_to_end(&win, width);
            let button = labelled(&win, test);
            let card = card_for(&win, models);
            let card_bottom = card.absolute_position().y + card.size().height;
            assert!(
                button.absolute_position().y >= card_bottom + 8.0 - 0.5,
                "{language} {width}px: model catalogue overlaps the connection form"
            );
            labelled(&win, connections).invoke_accessible_default_action();

            win.set_ai_provider_index(1);
            scroll_to_end(&win, width);
            let card = card_for(&win, capabilities);
            let caption = labelled(&win, caption);
            let caption_bottom = caption.absolute_position().y + caption.size().height;
            let card_bottom = card.absolute_position().y + card.size().height;
            assert!(
                caption_bottom <= card_bottom - 12.0 + 0.5,
                "{language} {width}px: capability caption clipped: caption bottom={caption_bottom}, card bottom={card_bottom}"
            );
        }
        win.hide().expect("hide Settings");
    }
}
