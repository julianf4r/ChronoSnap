use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

fn open_connection(path: &str) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: i64,
    pub date: String,
    pub start_minute: i32,
    pub end_minute: i32,
    pub main_tag_id: i64,
    pub sub_tag_id: Option<i64>,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Reminder {
    pub id: i64,
    pub date: String,
    pub minute: i32,
    pub content: String,
    pub is_completed: bool,
}

pub fn init_db(path: &str) -> anyhow::Result<()> {
    let conn = open_connection(path)?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            parent_id INTEGER,
            color TEXT NOT NULL,
            FOREIGN KEY(parent_id) REFERENCES tags(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            start_minute INTEGER NOT NULL,
            end_minute INTEGER NOT NULL,
            main_tag_id INTEGER NOT NULL,
            sub_tag_id INTEGER,
            content TEXT,
            FOREIGN KEY(main_tag_id) REFERENCES tags(id),
            FOREIGN KEY(sub_tag_id) REFERENCES tags(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS reminders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            minute INTEGER NOT NULL,
            content TEXT NOT NULL,
            is_completed BOOLEAN NOT NULL DEFAULT 0
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_date_start ON events(date, start_minute)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_reminders_date_minute ON reminders(date, minute)",
        [],
    )?;

    Ok(())
}

pub fn get_tags(path: &str) -> anyhow::Result<Vec<Tag>> {
    let conn = open_connection(path)?;
    let mut stmt = conn.prepare("SELECT id, name, parent_id, color FROM tags")?;
    let tag_iter = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            parent_id: row.get(2)?,
            color: row.get(3)?,
        })
    })?;

    let mut results = Vec::new();
    for tag in tag_iter {
        results.push(tag?);
    }
    Ok(results)
}

pub fn add_tag(path: &str, name: &str, parent_id: Option<i64>, color: &str) -> anyhow::Result<i64> {
    let conn = open_connection(path)?;
    conn.execute(
        "INSERT INTO tags (name, parent_id, color) VALUES (?1, ?2, ?3)",
        params![name, parent_id, color],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_tag(path: &str, id: i64) -> anyhow::Result<()> {
    let mut conn = open_connection(path)?;
    let used_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM events
         WHERE main_tag_id = ?1
            OR sub_tag_id = ?1
            OR main_tag_id IN (SELECT id FROM tags WHERE parent_id = ?1)
            OR sub_tag_id IN (SELECT id FROM tags WHERE parent_id = ?1)",
        params![id],
        |row| row.get(0),
    )?;

    if used_count > 0 {
        anyhow::bail!("该标签或其子标签正在被事件使用，无法删除");
    }

    let tx = conn.transaction()?;
    tx.execute("DELETE FROM tags WHERE parent_id = ?1", params![id])?;
    tx.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
    tx.commit()?;
    Ok(())
}

pub fn get_events(path: &str, date: &str) -> anyhow::Result<Vec<Event>> {
    let conn = open_connection(path)?;
    let mut stmt = conn.prepare("SELECT id, date, start_minute, end_minute, main_tag_id, sub_tag_id, content FROM events WHERE date = ?1")?;
    let event_iter = stmt.query_map(params![date], |row| {
        Ok(Event {
            id: row.get(0)?,
            date: row.get(1)?,
            start_minute: row.get(2)?,
            end_minute: row.get(3)?,
            main_tag_id: row.get(4)?,
            sub_tag_id: row.get(5)?,
            content: row.get(6)?,
        })
    })?;
    let mut events = Vec::new();
    for event in event_iter {
        events.push(event?);
    }
    Ok(events)
}

pub fn get_events_range(path: &str, start_date: &str, end_date: &str) -> anyhow::Result<Vec<Event>> {
    let conn = open_connection(path)?;
    let mut stmt = conn.prepare("SELECT id, date, start_minute, end_minute, main_tag_id, sub_tag_id, content FROM events WHERE date >= ?1 AND date <= ?2 ORDER BY date, start_minute")?;
    let event_iter = stmt.query_map(params![start_date, end_date], |row| {
        Ok(Event {
            id: row.get(0)?,
            date: row.get(1)?,
            start_minute: row.get(2)?,
            end_minute: row.get(3)?,
            main_tag_id: row.get(4)?,
            sub_tag_id: row.get(5)?,
            content: row.get(6)?,
        })
    })?;
    let mut events = Vec::new();
    for event in event_iter {
        events.push(event?);
    }
    Ok(events)
}

pub fn save_event(path: &str, event: Event) -> anyhow::Result<i64> {
    let conn = open_connection(path)?;
    if event.id > 0 {
        conn.execute(
            "UPDATE events SET date=?1, start_minute=?2, end_minute=?3, main_tag_id=?4, sub_tag_id=?5, content=?6 WHERE id=?7",
            params![event.date, event.start_minute, event.end_minute, event.main_tag_id, event.sub_tag_id, event.content, event.id],
        )?;
        Ok(event.id)
    } else {
        conn.execute(
            "INSERT INTO events (date, start_minute, end_minute, main_tag_id, sub_tag_id, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![event.date, event.start_minute, event.end_minute, event.main_tag_id, event.sub_tag_id, event.content],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_event(path: &str, id: i64) -> anyhow::Result<()> {
    let conn = open_connection(path)?;
    conn.execute("DELETE FROM events WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_reminders(path: &str, date: &str) -> anyhow::Result<Vec<Reminder>> {
    let conn = open_connection(path)?;
    let mut stmt = conn.prepare("SELECT id, date, minute, content, is_completed FROM reminders WHERE date = ?1 ORDER BY minute")?;
    let iter = stmt.query_map(params![date], |row| {
        Ok(Reminder {
            id: row.get(0)?,
            date: row.get(1)?,
            minute: row.get(2)?,
            content: row.get(3)?,
            is_completed: row.get(4)?,
        })
    })?;
    let mut reminders = Vec::new();
    for r in iter {
        reminders.push(r?);
    }
    Ok(reminders)
}

pub fn save_reminder(path: &str, reminder: Reminder) -> anyhow::Result<i64> {
    let conn = open_connection(path)?;
    if reminder.id > 0 {
        conn.execute(
            "UPDATE reminders SET date=?1, minute=?2, content=?3, is_completed=?4 WHERE id=?5",
            params![reminder.date, reminder.minute, reminder.content, reminder.is_completed, reminder.id],
        )?;
        Ok(reminder.id)
    } else {
        conn.execute(
            "INSERT INTO reminders (date, minute, content, is_completed) VALUES (?1, ?2, ?3, ?4)",
            params![reminder.date, reminder.minute, reminder.content, reminder.is_completed],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn delete_reminder(path: &str, id: i64) -> anyhow::Result<()> {
    let conn = open_connection(path)?;
    conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn toggle_reminder(path: &str, id: i64, is_completed: bool) -> anyhow::Result<()> {
    let conn = open_connection(path)?;
    conn.execute("UPDATE reminders SET is_completed=?1 WHERE id=?2", params![is_completed, id])?;
    Ok(())
}

pub fn get_overdue_reminders_count(path: &str, date: &str, minute: i32) -> anyhow::Result<i32> {
    let conn = open_connection(path)?;
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM reminders WHERE is_completed = 0 AND (date < ?1 OR (date = ?1 AND minute < ?2))")?;
    let count: i32 = stmt.query_row(params![date, minute], |row| row.get(0))?;
    Ok(count)
}

#[derive(Serialize, Deserialize)]
pub struct DayStatus {
    pub date: String,
    pub has_overdue: bool,
    pub has_upcoming: bool,
}

pub fn get_reminders_by_month(path: &str, year_month: &str, today: &str, now_minute: i32) -> anyhow::Result<Vec<DayStatus>> {
    let conn = open_connection(path)?;
    // 查找该月份内有提醒的所有日期
    let mut stmt = conn.prepare("
        SELECT date, 
               MAX(CASE WHEN is_completed = 0 AND (date < ?2 OR (date = ?2 AND minute < ?3)) THEN 1 ELSE 0 END) as has_overdue,
               MAX(CASE WHEN is_completed = 0 AND (date > ?2 OR (date = ?2 AND minute >= ?3)) THEN 1 ELSE 0 END) as has_upcoming
        FROM reminders 
        WHERE date LIKE ?1
        GROUP BY date
    ")?;
    
    let rows = stmt.query_map(params![format!("{}%", year_month), today, now_minute], |row| {
        Ok(DayStatus {
            date: row.get(0)?,
            has_overdue: row.get::<_, i32>(1)? == 1,
            has_upcoming: row.get::<_, i32>(2)? == 1,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}
