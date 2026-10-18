// These system APIs have been available since macOS 10.15. Keep both the
// framework link and calls out of Windows builds.
#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[tauri::command]
pub fn desktop_platform() -> &'static str {
    std::env::consts::OS
}

#[tauri::command]
pub fn screen_capture_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: This parameterless CoreGraphics API has no pointer or thread
        // preconditions and only checks the current process's authorization.
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[tauri::command]
pub fn request_screen_capture_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: CoreGraphics owns the permission dialog and takes no arguments.
        unsafe { CGRequestScreenCaptureAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}
