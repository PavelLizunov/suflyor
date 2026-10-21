//! StatusNotifierItem restore/control surface for the native Wayland app.

use ksni::blocking::TrayMethods;

pub struct Status {
    ru: bool,
}
impl ksni::Tray for Status {
    fn id(&self) -> String {
        "suflyor".into()
    }
    fn title(&self) -> String {
        "Suflyor".into()
    }
    fn icon_name(&self) -> String {
        "audio-input-microphone".into()
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = super::desktop::send_command("show");
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let labels = if self.ru {
            ["Показать", "Настройки", "Пауза", "Завершить"]
        } else {
            ["Show", "Settings", "Pause", "Quit"]
        };
        labels
            .into_iter()
            .zip(["show", "settings", "pause", "quit"])
            .map(|(label, command)| {
                ksni::menu::StandardItem {
                    label: label.into(),
                    activate: Box::new(move |_| {
                        let _ = super::desktop::send_command(command);
                    }),
                    ..Default::default()
                }
                .into()
            })
            .collect()
    }
}
pub struct Guard(ksni::blocking::Handle<Status>);
impl Drop for Guard {
    fn drop(&mut self) {
        let _shutdown = self.0.shutdown();
        // A stalled session bus must not strand application exit/relaunch.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
        while !self.0.is_closed() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
pub fn install(ru: bool) -> Result<Guard, String> {
    Status { ru }
        .spawn()
        .map(Guard)
        .map_err(|_| "native status watcher unavailable".into())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use ksni::Tray;
    #[test]
    fn restore_controls_are_present_in_both_languages() {
        for ru in [false, true] {
            let menu = Status { ru }.menu();
            assert_eq!(menu.len(), 4);
            let ksni::MenuItem::Standard(first) = &menu[0] else {
                panic!("missing restore action")
            };
            assert_eq!(first.label, if ru { "Показать" } else { "Show" });
        }
    }
}
