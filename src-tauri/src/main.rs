#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod claude;
mod persistence;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, LogicalSize, Manager, Window,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const FULL_WIDTH: f64 = 300.0;
const FULL_HEIGHT: f64 = 520.0;
const MINI_WIDTH: f64 = 160.0;
const MINI_HEIGHT: f64 = 160.0;

fn toggle_window_visibility(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn set_mini_mode(window: Window, mini: bool) -> Result<(), String> {
    let size = if mini {
        LogicalSize::new(MINI_WIDTH, MINI_HEIGHT)
    } else {
        LogicalSize::new(FULL_WIDTH, FULL_HEIGHT)
    };

    window.set_size(size).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn start_window_drag(window: Window) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
fn get_system_idle_ms() -> Result<u64, String> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut last_input = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };

    let success = unsafe { GetLastInputInfo(&mut last_input) };

    if success == 0 {
        return Err("GetLastInputInfo failed".to_string());
    }

    let now = unsafe { GetTickCount() };
    Ok(now.wrapping_sub(last_input.dwTime) as u64)
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
fn get_system_idle_ms() -> Result<u64, String> {
    Err("system idle is only supported on Windows".to_string())
}

fn main() {
    let toggle_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyF);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &toggle_shortcut && event.state() == ShortcutState::Pressed {
                        toggle_window_visibility(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let handle = app.handle().clone();
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let connection = persistence::open_database(app_data_dir.join("focusflow.sqlite3"))?;
            app.manage(persistence::PersistenceState::new(connection));

            let show_hide =
                MenuItem::with_id(&handle, "show_hide", "显示/隐藏", true, None::<&str>)?;
            let quit = MenuItem::with_id(&handle, "quit", "退出 FocusFlow", true, None::<&str>)?;
            let menu = Menu::with_items(&handle, &[&show_hide, &quit])?;

            let tray_result = TrayIconBuilder::with_id("focusflow")
                .tooltip("FocusFlow")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show_hide" => toggle_window_visibility(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app);

            if let Err(error) = tray_result {
                eprintln!("failed to create tray icon: {error}");
            }

            if let Err(error) = app.global_shortcut().register(toggle_shortcut) {
                eprintln!("failed to register Alt+F: {error}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            claude::parse_task_with_ai,
            claude::load_ai_config_status,
            claude::save_personal_ai_config,
            claude::save_invite_ai_config,
            claude::clear_ai_config,
            claude::get_app_local_data_dir,
            persistence::load_focus_flow_state,
            persistence::save_focus_flow_state,
            get_system_idle_ms,
            set_mini_mode,
            start_window_drag,
        ])
        .run(tauri::generate_context!())
        .expect("error while running focusflow");
}
