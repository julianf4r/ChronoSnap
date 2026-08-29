mod db;
mod engine;
mod tray;

use tauri::Manager;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            engine::toggle_pause,
            engine::get_pause_state,
            engine::get_timeline,
            engine::get_image_base64,
            engine::check_storage_health,
            engine::update_interval,
            engine::update_db_path,
            engine::get_tags,
            engine::add_tag,
            engine::delete_tag,
            engine::get_events,
            engine::get_events_range,
            engine::save_event,
            engine::delete_event,
            engine::get_reminders,
            engine::save_reminder,
            engine::delete_reminder,
            engine::toggle_reminder,
            engine::get_overdue_reminders_count,
            engine::get_reminders_by_month,
            engine::get_plan_tasks,
            engine::save_plan_task,
            engine::delete_plan_task,
            engine::toggle_plan_task,
            engine::reorder_plan_tasks,
            engine::write_file
        ])
        .setup(|app| {
            app.manage(engine::AppState {
                is_paused: Arc::new(AtomicBool::new(false)),
                capture_interval_secs: std::sync::atomic::AtomicU64::new(60),
                db_path: std::sync::Mutex::new(None),
                toggle_menu_item: std::sync::Mutex::new(None),
            });
            tray::create_tray(app.handle())?;
            engine::start_engine(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
