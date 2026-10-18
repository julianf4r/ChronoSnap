#[tauri::command]
pub fn desktop_platform() -> &'static str {
    std::env::consts::OS
}
