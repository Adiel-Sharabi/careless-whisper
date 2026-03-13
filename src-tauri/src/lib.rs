pub mod audio;
pub mod commands;
pub mod config;
pub mod hotkey;
pub mod models;
pub mod output;
pub mod transcribe;
pub mod tray;

use commands::*;
use config::settings::Settings;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub whisper_ctx: Mutex<Option<whisper_rs::WhisperContext>>,
    pub recording: Mutex<Option<audio::capture::RecordingHandle>>,
    pub target_pid: Mutex<Option<i32>>,
}

/// Checks if the app has Accessibility permission.
/// If not, opens the macOS System Settings prompt so the user can grant it.
fn request_accessibility_if_needed() {
    use std::os::raw::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> *const c_void;
        fn CFRelease(cf: *const c_void);
        static kCFBooleanTrue: *const c_void;
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
    }

    // kAXTrustedCheckOptionPrompt key — this is a constant string in ApplicationServices
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        static kAXTrustedCheckOptionPrompt: *const c_void;
    }

    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const _ as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const _ as *const c_void,
        );
        let trusted = AXIsProcessTrustedWithOptions(options);
        CFRelease(options);
        eprintln!("[startup] AXIsProcessTrusted = {}", trusted != 0);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = Settings::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            settings: Mutex::new(settings),
            whisper_ctx: Mutex::new(None),
            recording: Mutex::new(None),
            target_pid: Mutex::new(None),
        })
        .setup(|app| {
            // Prompt for Accessibility permission if not yet granted.
            // This opens the System Settings dialog automatically on first launch.
            request_accessibility_if_needed();

            tray::setup_tray(&app.handle())?;
            hotkey::manager::register_hotkey(app)?;

            // First launch: show settings if no model downloaded yet
            let models_dir = dirs::data_dir()
                .unwrap_or_default()
                .join("careless-whisper")
                .join("models");
            let has_model = std::fs::read_dir(&models_dir)
                .ok()
                .and_then(|mut d| d.next())
                .is_some();
            if !has_model {
                if let Some(win) = app.get_webview_window("settings") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            get_settings,
            update_settings,
            list_models,
            download_model,
            delete_model,
            set_active_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
