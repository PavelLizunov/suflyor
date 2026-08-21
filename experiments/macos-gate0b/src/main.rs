use std::error::Error;

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(not(target_os = "macos"))]
    return Err("Gate 0B is a macOS-only feasibility prototype".into());

    #[cfg(target_os = "macos")]
    run_macos()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::ui::GateCapture;
    use slint::ComponentHandle;
    use std::ffi::{c_char, CStr};
    use std::time::Duration;

    #[repr(C)]
    #[derive(Default)]
    struct NativeSnapshot {
        mic_permission: u32,
        mic_device_available: u32,
        mic_running: u32,
        system_running: u32,
        screen_allowed: u32,
        screenshot_width: u32,
        screenshot_height: u32,
        last_error: i32,
        mic_frames: u64,
        mic_peak_milli: u32,
        system_starting: u32,
        system_frames: u64,
    }

    extern "C" {
        fn suflyor_gate0b_initialize();
        fn suflyor_gate0b_refresh();
        fn suflyor_gate0b_request_microphone();
        fn suflyor_gate0b_start_microphone();
        fn suflyor_gate0b_stop_microphone();
        fn suflyor_gate0b_start_system_audio();
        fn suflyor_gate0b_stop_system_audio();
        fn suflyor_gate0b_capture_screen();
        fn suflyor_gate0b_open_privacy_settings(section: u32);
        fn suflyor_gate0b_snapshot(snapshot: *mut NativeSnapshot);
        fn suflyor_gate0b_copy_message(buffer: *mut c_char, capacity: usize);
        fn suflyor_gate0b_shutdown();
    }

    pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
        unsafe { suflyor_gate0b_initialize() };
        let arguments: Vec<String> = std::env::args().collect();
        let auto_system = arguments.iter().any(|argument| argument == "--auto-system");
        let auto_screen = arguments.iter().any(|argument| argument == "--auto-screen");
        let auto_microphone = arguments
            .iter()
            .any(|argument| argument == "--auto-microphone");
        let auto_microphone_input = arguments
            .iter()
            .any(|argument| argument == "--auto-microphone-input");
        let window = GateCapture::new()?;
        seed(&window);
        wire_actions(&window);
        refresh_window(&window);

        let timer = slint::Timer::default();
        let weak = window.as_weak();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(250),
            move || {
                if let Some(window) = weak.upgrade() {
                    refresh_window(&window);
                }
            },
        );

        if auto_system {
            slint::Timer::single_shot(Duration::from_secs(1), || unsafe {
                suflyor_gate0b_start_system_audio();
            });
        }
        if auto_screen {
            slint::Timer::single_shot(Duration::from_secs(1), || unsafe {
                suflyor_gate0b_capture_screen();
            });
        }
        if auto_microphone {
            slint::Timer::single_shot(Duration::from_secs(1), || unsafe {
                suflyor_gate0b_request_microphone();
            });
        }
        if auto_microphone_input {
            slint::Timer::single_shot(Duration::from_secs(1), || unsafe {
                suflyor_gate0b_start_microphone();
            });
        }

        let smoke_timer = slint::Timer::default();
        if auto_system || auto_microphone_input {
            smoke_timer.start(
                slint::TimerMode::Repeated,
                Duration::from_secs(5),
                move || {
                    let mut snapshot = NativeSnapshot::default();
                    unsafe { suflyor_gate0b_snapshot(&mut snapshot) };
                    if auto_system {
                        eprintln!(
                            "[gate0b] system smoke running={} frames={} code={}",
                            snapshot.system_running, snapshot.system_frames, snapshot.last_error
                        );
                    }
                    if auto_microphone_input {
                        eprintln!(
                            "[gate0b] microphone smoke running={} frames={} peak_milli={} code={}",
                            snapshot.mic_running,
                            snapshot.mic_frames,
                            snapshot.mic_peak_milli,
                            snapshot.last_error
                        );
                    }
                },
            );
        }

        let result = window.run();
        unsafe { suflyor_gate0b_shutdown() };
        result.map_err(Into::into)
    }

    fn seed(window: &GateCapture) {
        window.set_title_text("Suflyor macOS Gate 0B".into());
        window.set_subtitle_text(
            "Native capture feasibility only - no AI, STT, storage, or virtual audio driver."
                .into(),
        );
        window.set_mic_title_text("Microphone".into());
        window.set_mic_request_text("Request access".into());
        window.set_mic_start_text("Start input".into());
        window.set_mic_stop_text("Stop input".into());
        window.set_system_title_text("System audio - Core Audio Tap".into());
        window.set_system_start_text("Start system stream".into());
        window.set_system_stop_text("Stop system stream".into());
        window.set_screen_title_text("ScreenCaptureKit".into());
        window.set_screen_capture_text("Capture this window".into());
        window.set_settings_text("Privacy settings".into());
        window.set_refresh_text("Refresh states".into());
    }

    fn wire_actions(window: &GateCapture) {
        window.on_request_microphone(|| unsafe {
            suflyor_gate0b_request_microphone();
        });
        window.on_start_microphone(|| unsafe {
            suflyor_gate0b_start_microphone();
        });
        window.on_stop_microphone(|| unsafe {
            suflyor_gate0b_stop_microphone();
        });
        window.on_start_system(|| unsafe {
            suflyor_gate0b_start_system_audio();
        });
        window.on_stop_system(|| unsafe {
            suflyor_gate0b_stop_system_audio();
        });
        window.on_open_system_settings(|| unsafe {
            suflyor_gate0b_open_privacy_settings(2);
        });
        window.on_capture_screen(|| unsafe {
            suflyor_gate0b_capture_screen();
        });
        window.on_open_mic_settings(|| unsafe {
            suflyor_gate0b_open_privacy_settings(0);
        });
        window.on_open_screen_settings(|| unsafe {
            suflyor_gate0b_open_privacy_settings(1);
        });
        let weak = window.as_weak();
        window.on_refresh(move || {
            unsafe { suflyor_gate0b_refresh() };
            if let Some(window) = weak.upgrade() {
                refresh_window(&window);
            }
        });
    }

    fn refresh_window(window: &GateCapture) {
        let mut snapshot = NativeSnapshot::default();
        unsafe { suflyor_gate0b_snapshot(&mut snapshot) };

        let permission = match snapshot.mic_permission {
            0 => "Not requested",
            1 => "Restricted",
            2 => "Denied",
            3 => "Allowed",
            _ => "Unknown",
        };
        window.set_mic_status_text(
            format!(
                "Permission: {permission} | Default input: {}",
                yes_no(snapshot.mic_device_available)
            )
            .into(),
        );
        window.set_mic_detail_text(
            format!(
                "Running: {} | Frames: {} | Peak: {:.3}",
                yes_no(snapshot.mic_running),
                snapshot.mic_frames,
                f64::from(snapshot.mic_peak_milli) / 1000.0
            )
            .into(),
        );
        window.set_system_status_text(
            format!(
                "Starting: {} | Running: {} | Frames: {}",
                yes_no(snapshot.system_starting),
                yes_no(snapshot.system_running),
                snapshot.system_frames
            )
            .into(),
        );
        window.set_system_detail_text(
            "Uses a private HAL tap and private aggregate device; playback is not muted.".into(),
        );
        window.set_screen_status_text(
            format!(
                "Screen access: {} | Last image: {}x{}",
                if snapshot.screen_allowed != 0 {
                    "Allowed"
                } else {
                    "Not allowed or not requested"
                },
                snapshot.screenshot_width,
                snapshot.screenshot_height
            )
            .into(),
        );
        window.set_screen_detail_text(
            "Captures only this app window and retains dimensions, not image data.".into(),
        );
        window.set_event_text(native_message(snapshot.last_error).into());
    }

    fn native_message(last_error: i32) -> String {
        let mut buffer = [0_i8; 320];
        unsafe { suflyor_gate0b_copy_message(buffer.as_mut_ptr(), buffer.len()) };
        let message = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy();
        if last_error == 0 {
            message.into_owned()
        } else {
            format!("{} (OSStatus/code {last_error})", message)
        }
    }

    fn yes_no(value: u32) -> &'static str {
        if value == 0 {
            "no"
        } else {
            "yes"
        }
    }
}

#[cfg(target_os = "macos")]
fn run_macos() -> Result<(), Box<dyn Error>> {
    macos::run()
}
