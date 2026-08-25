use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tauri::{AppHandle, Manager, Emitter};
use xcap::Monitor;
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use tauri_plugin_store::StoreExt;
use serde::Serialize;

pub struct AppState {
    pub is_paused: Arc<AtomicBool>,
    pub capture_interval_secs: std::sync::atomic::AtomicU64,
    pub db_path: std::sync::Mutex<Option<String>>,
    pub toggle_menu_item: std::sync::Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
}

#[derive(Serialize)]
pub struct StorageHealth {
    pub ok: bool,
    pub save_path_exists: bool,
    pub save_path_writable: bool,
    pub db_parent_exists: bool,
    pub db_parent_writable: bool,
    pub db_file_exists: bool,
    pub db_file_writable: bool,
    pub issues: Vec<String>,
}

fn can_write_temp_file(dir: &Path) -> bool {
    let file_name = format!(
        ".chrono-snap-write-test-{}-{}",
        std::process::id(),
        chrono::Local::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let test_path = dir.join(file_name);
    match OpenOptions::new().write(true).create_new(true).open(&test_path) {
        Ok(_) => {
            let _ = fs::remove_file(test_path);
            true
        }
        Err(_) => false,
    }
}

#[tauri::command]
pub fn check_storage_health(save_path: String, db_path: String) -> StorageHealth {
    let save_dir = PathBuf::from(save_path);
    let db_file = PathBuf::from(db_path);
    let db_parent = db_file.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(""));

    let save_path_exists = save_dir.is_dir();
    let save_path_writable = save_path_exists && can_write_temp_file(&save_dir);
    let db_parent_exists = db_parent.is_dir();
    let db_parent_writable = db_parent_exists && can_write_temp_file(&db_parent);
    let db_file_exists = db_file.is_file();
    let db_file_writable = if db_file_exists {
        OpenOptions::new().read(true).write(true).open(&db_file).is_ok()
    } else {
        db_parent_writable
    };

    let mut issues = Vec::new();
    if !save_path_exists {
        issues.push("截图保存目录不存在，请重新选择一个可用目录".into());
    } else if !save_path_writable {
        issues.push("截图保存目录不可写，请检查目录权限或更换目录".into());
    }

    if !db_parent_exists {
        issues.push("数据库文件所在目录不存在，请重新选择或创建数据库文件".into());
    } else if !db_parent_writable {
        issues.push("数据库文件所在目录不可写，请检查目录权限或更换位置".into());
    } else if db_file_exists && !db_file_writable {
        issues.push("数据库文件不可写，请检查文件权限或更换数据库文件".into());
    }

    StorageHealth {
        ok: issues.is_empty(),
        save_path_exists,
        save_path_writable,
        db_parent_exists,
        db_parent_writable,
        db_file_exists,
        db_file_writable,
        issues,
    }
}

#[tauri::command]
pub fn update_db_path(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    crate::db::init_db(&path).map_err(|e| e.to_string())?;
    let mut db_path = state.db_path.lock().unwrap();
    *db_path = Some(path);
    Ok(())
}

#[tauri::command]
pub fn get_tags(state: tauri::State<'_, AppState>) -> Result<Vec<crate::db::Tag>, String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::get_tags(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_tag(state: tauri::State<'_, AppState>, name: String, parent_id: Option<i64>, color: String) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("标签名称不能为空".into());
    }

    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::add_tag(path, name, parent_id, &color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_tag(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::delete_tag(path, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_events(state: tauri::State<'_, AppState>, date: String) -> Result<Vec<crate::db::Event>, String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::get_events(path, &date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_events_range(state: tauri::State<'_, AppState>, start_date: String, end_date: String) -> Result<Vec<crate::db::Event>, String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::get_events_range(path, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_event(state: tauri::State<'_, AppState>, event: crate::db::Event) -> Result<i64, String> {
    if chrono::NaiveDate::parse_from_str(&event.date, "%Y-%m-%d").is_err() {
        return Err("事件日期格式无效".into());
    }
    if event.start_minute < 0 || event.start_minute >= 1440 || event.end_minute <= 0 || event.end_minute > 1440 {
        return Err("事件时间超出范围".into());
    }
    if event.end_minute <= event.start_minute {
        return Err("结束时间必须晚于开始时间".into());
    }
    if event.main_tag_id <= 0 {
        return Err("请选择主标签".into());
    }

    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::save_event(path, event).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_event(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::delete_event(path, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_reminders(state: tauri::State<'_, AppState>, date: String) -> Result<Vec<crate::db::Reminder>, String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::get_reminders(path, &date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_reminder(state: tauri::State<'_, AppState>, reminder: crate::db::Reminder) -> Result<i64, String> {
    if chrono::NaiveDate::parse_from_str(&reminder.date, "%Y-%m-%d").is_err() {
        return Err("提醒日期格式无效".into());
    }
    if reminder.minute < 0 || reminder.minute >= 1440 {
        return Err("提醒时间超出范围".into());
    }
    if reminder.content.trim().is_empty() {
        return Err("提醒内容不能为空".into());
    }

    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::save_reminder(path, reminder).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_reminder(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::delete_reminder(path, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_reminder(state: tauri::State<'_, AppState>, id: i64, is_completed: bool) -> Result<(), String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::toggle_reminder(path, id, is_completed).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_overdue_reminders_count(state: tauri::State<'_, AppState>, date: String, minute: i32) -> Result<i32, String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::get_overdue_reminders_count(path, &date, minute).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_reminders_by_month(state: tauri::State<'_, AppState>, year_month: String, today: String, now_minute: i32) -> Result<Vec<crate::db::DayStatus>, String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::get_reminders_by_month(path, &year_month, &today, now_minute).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_plan_tasks(state: tauri::State<'_, AppState>) -> Result<Vec<crate::db::PlanTask>, String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::get_plan_tasks(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_plan_task(state: tauri::State<'_, AppState>, mut task: crate::db::PlanTask) -> Result<i64, String> {
    task.title = task.title.trim().to_string();
    task.notes = task.notes.trim().to_string();
    if task.title.is_empty() {
        return Err("任务名称不能为空".into());
    }

    let start_date = chrono::NaiveDate::parse_from_str(&task.start_date, "%Y-%m-%d")
        .map_err(|_| "开始日期格式无效")?;
    let end_date = chrono::NaiveDate::parse_from_str(&task.end_date, "%Y-%m-%d")
        .map_err(|_| "结束日期格式无效")?;
    if end_date < start_date {
        return Err("结束日期不能早于开始日期".into());
    }
    if task.main_tag_id.is_none() {
        task.sub_tag_id = None;
    }

    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::save_plan_task(path, task).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_plan_task(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::delete_plan_task(path, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_plan_task(state: tauri::State<'_, AppState>, id: i64, is_completed: bool) -> Result<(), String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::toggle_plan_task(path, id, is_completed).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn swap_plan_tasks(state: tauri::State<'_, AppState>, first_id: i64, second_id: i64) -> Result<(), String> {
    let path = state.db_path.lock().unwrap();
    let path = path.as_ref().ok_or("Database path not set")?;
    crate::db::swap_plan_tasks(path, first_id, second_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_interval(state: tauri::State<'_, AppState>, seconds: u64) {
    state.capture_interval_secs.store(seconds.clamp(60, 600), Ordering::SeqCst);
}

#[tauri::command]
pub fn toggle_pause(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    let current = state.is_paused.load(Ordering::SeqCst);
    let new_state = !current;
    state.is_paused.store(new_state, Ordering::SeqCst);
    
    // Sync with Tray
    if let Some(item) = state.toggle_menu_item.lock().unwrap().as_ref() {
        let text = if new_state { "恢复记录" } else { "暂停记录" };
        let _ = item.set_text(text);
    }
    
    // Notify Frontend
    let _ = app.emit("pause-state-changed", new_state);
    
    new_state
}

#[tauri::command]
pub fn get_pause_state(state: tauri::State<'_, AppState>) -> bool {
    state.is_paused.load(Ordering::SeqCst)
}

use base64::{engine::general_purpose, Engine as _};

#[tauri::command]
pub fn get_image_base64(path: String) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
pub fn get_timeline(date: String, base_dir: String) -> Vec<serde_json::Value> {
    let mut results = Vec::new();
    let base_path = PathBuf::from(base_dir);
    
    // Parse current date
    let current_date = match chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return results,
    };
    let next_date = current_date + chrono::Duration::days(1);
    let next_date_str = next_date.format("%Y-%m-%d").to_string();

    // Helper to process a directory with a time filter
    let mut process_dir = |dir_date: &str, is_next_day: bool| {
        let dir_path = base_path.join(dir_date);
        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jpg") {
                    if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                        // file_name format: "14-30-00"
                        let parts: Vec<&str> = file_name.split('-').collect();
                        if parts.len() >= 1 {
                            if let Ok(hour) = parts[0].parse::<u32>() {
                                // Logic: If current day, must be >= 3. If next day, must be < 3.
                                let keep = if !is_next_day { hour >= 3 } else { hour < 3 };
                                if keep {
                                    let time_str = file_name.replace("-", ":").replace("_", " ");
                                    results.push(serde_json::json!({
                                        "time": time_str,
                                        "path": path.to_string_lossy().to_string(),
                                        "isNextDay": is_next_day
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    process_dir(&date, false);
    process_dir(&next_date_str, true);
    
    // Sort results by isNextDay (false first) then by time
    results.sort_by(|a, b| {
        let a_next = a["isNextDay"].as_bool().unwrap_or(false);
        let b_next = b["isNextDay"].as_bool().unwrap_or(false);
        if a_next != b_next {
            a_next.cmp(&b_next)
        } else {
            a["time"].as_str().unwrap_or("").cmp(b["time"].as_str().unwrap_or(""))
        }
    });

    results
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn start_engine(app: AppHandle) {
    // Start Cleanup routine
    let app_cleanup = app.clone();
    tauri::async_runtime::spawn(async move {
        // Run once on startup, then every 24 hours
        let mut ticker = interval(Duration::from_secs(60 * 60 * 24));
        loop {
            ticker.tick().await;
            if let Err(e) = run_cleanup(&app_cleanup).await {
                eprintln!("Cleanup error: {}", e);
            }
        }
    });

    let app_capture = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app_capture.state::<AppState>();
        let is_paused = state.is_paused.clone();
        
        // Check frequently to catch the aligned second
        let mut ticker = interval(Duration::from_millis(500));
        let mut last_capture_time = 0;
        
        loop {
            ticker.tick().await;

            if is_paused.load(Ordering::SeqCst) {
                continue;
            }

            let interval_secs = state.capture_interval_secs.load(Ordering::SeqCst);
            if interval_secs == 0 { continue; }

            let now = Local::now();
            let timestamp = now.timestamp() as u64;

            // Trigger if aligned to interval and we haven't captured this exact second yet
            if timestamp % interval_secs == 0 && timestamp != last_capture_time {
                last_capture_time = timestamp;
                println!("Tick: capturing screen at {}...", now.format("%H:%M:%S"));
                if let Err(e) = capture_screens(&app_capture).await {
                    eprintln!("Failed to capture screens: {}", e);
                } else {
                    let _ = app_capture.emit("refresh-timeline", ());
                }
            }
        }
    });
}

async fn run_cleanup(app: &AppHandle) -> anyhow::Result<()> {
    let store = match app.store("config.json") {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let save_path_value = store.get("savePath");
    let save_path_str = match save_path_value {
        Some(serde_json::Value::String(s)) => s,
        _ => return Ok(()), 
    };

    let base_dir = PathBuf::from(save_path_str);
    if !base_dir.exists() {
        return Ok(());
    }

    let retain_days = store
        .get("retainDays")
        .and_then(|v| v.as_u64())
        .unwrap_or(30) as i64;
    
    let now = chrono::Local::now().naive_local().date();

    if let Ok(entries) = fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(folder_date) = chrono::NaiveDate::parse_from_str(folder_name, "%Y-%m-%d") {
                        let duration = now.signed_duration_since(folder_date);
                        if duration.num_days() > retain_days {
                            let _ = fs::remove_dir_all(&path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn capture_screens(app: &AppHandle) -> anyhow::Result<()> {
    let store = match app.store("config.json") {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let save_path_value = store.get("savePath");
    let save_path_str = match save_path_value {
        Some(serde_json::Value::String(s)) => s,
        _ => return Ok(()), 
    };

    let base_dir = PathBuf::from(save_path_str);
    if !base_dir.is_dir() {
        return Ok(());
    }

    let merge_screens = true;

    let monitors = Monitor::all()?;
    let now = Local::now();
    let timestamp = now.format("%H-%M-%S").to_string();
    let date_folder = now.format("%Y-%m-%d").to_string();
    let save_dir = base_dir.join(date_folder);
    
    if !save_dir.exists() {
        fs::create_dir_all(&save_dir)?;
    }

    if merge_screens {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for m in &monitors {
            let x = m.x()?;
            let y = m.y()?;
            let width = m.width()? as i32;
            let height = m.height()? as i32;
            
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + width);
            max_y = max_y.max(y + height);
        }

        let total_width = (max_x - min_x) as u32;
        let total_height = (max_y - min_y) as u32;

        let mut combined_image = image::RgbaImage::new(total_width, total_height);

        for m in &monitors {
            let capture = m.capture_image()?;
            let offset_x = (m.x()? - min_x) as u32;
            let offset_y = (m.y()? - min_y) as u32;
            image::imageops::overlay(&mut combined_image, &capture, offset_x as i64, offset_y as i64);
        }

        let file_path = save_dir.join(format!("{}.jpg", timestamp));
        let dynamic_image = image::DynamicImage::ImageRgba8(combined_image);
        let rgb_image = dynamic_image.into_rgb8();
        rgb_image.save_with_format(file_path, image::ImageFormat::Jpeg)?;
    } else {
        for (i, m) in monitors.iter().enumerate() {
            let capture = m.capture_image()?;
            let file_path = save_dir.join(format!("{}_{}.jpg", timestamp, i));
            let dynamic_image = image::DynamicImage::ImageRgba8(capture);
            let rgb_image = dynamic_image.into_rgb8();
            rgb_image.save_with_format(file_path, image::ImageFormat::Jpeg)?;
        }
    }

    Ok(())
}
