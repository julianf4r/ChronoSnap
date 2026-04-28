use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Emitter,
};

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "打开主界面", true, None::<&str>)?;
    let toggle_i = MenuItem::with_id(app, "toggle", "暂停记录", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_i, &toggle_i, &quit_i])?;

    // Store the toggle item in state for future updates
    let state = app.state::<crate::engine::AppState>();
    *state.toggle_menu_item.lock().unwrap() = Some(toggle_i.clone());

    TrayIconBuilder::with_id("main_tray")
        .tooltip("瞬影")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "toggle" => {
                let state = app.state::<crate::engine::AppState>();
                let current = state.is_paused.load(std::sync::atomic::Ordering::SeqCst);
                let new_state = !current;
                state.is_paused.store(new_state, std::sync::atomic::Ordering::SeqCst);
                
                // Update menu text
                if let Some(item) = state.toggle_menu_item.lock().unwrap().as_ref() {
                    let text = if new_state { "恢复记录" } else { "暂停记录" };
                    let _ = item.set_text(text);
                }
                
                let _ = app.emit("pause-state-changed", new_state);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
