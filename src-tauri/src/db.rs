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

#[derive(Serialize, Deserialize, Clone)]
pub struct PlanTask {
    pub id: i64,
    pub title: String,
    pub start_date: String,
    pub end_date: String,
    pub main_tag_id: Option<i64>,
    pub sub_tag_id: Option<i64>,
    pub notes: String,
    pub is_completed: bool,
    pub completed_at: Option<String>,
    pub sort_order: i64,
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
        "CREATE TABLE IF NOT EXISTS plan_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            start_date TEXT NOT NULL,
            end_date TEXT NOT NULL,
            main_tag_id INTEGER,
            sub_tag_id INTEGER,
            notes TEXT NOT NULL DEFAULT '',
            is_completed BOOLEAN NOT NULL DEFAULT 0,
            completed_at TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(main_tag_id) REFERENCES tags(id),
            FOREIGN KEY(sub_tag_id) REFERENCES tags(id)
        )",
        [],
    )?;

    let has_plan_task_sort_order = {
        let mut stmt = conn.prepare("PRAGMA table_info(plan_tasks)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut found = false;
        for column in columns {
            if column? == "sort_order" {
                found = true;
                break;
            }
        }
        found
    };

    if !has_plan_task_sort_order {
        conn.execute(
            "ALTER TABLE plan_tasks ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    conn.execute(
        "UPDATE plan_tasks SET sort_order = id WHERE sort_order = 0",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_reminders_date_minute ON reminders(date, minute)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_plan_tasks_dates ON plan_tasks(start_date, end_date)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_plan_tasks_sort_order ON plan_tasks(sort_order, id)",
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

    let task_used_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM plan_tasks
         WHERE main_tag_id = ?1
            OR sub_tag_id = ?1
            OR main_tag_id IN (SELECT id FROM tags WHERE parent_id = ?1)
            OR sub_tag_id IN (SELECT id FROM tags WHERE parent_id = ?1)",
        params![id],
        |row| row.get(0),
    )?;

    if task_used_count > 0 {
        anyhow::bail!("该标签或其子标签正在被任务使用，无法删除");
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
    let overlap_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM events
         WHERE date = ?1
           AND id != ?2
           AND start_minute < ?4
           AND end_minute > ?3",
        params![event.date, event.id, event.start_minute, event.end_minute],
        |row| row.get(0),
    )?;

    if overlap_count > 0 {
        anyhow::bail!("该时间段与已有事件重叠");
    }

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

pub fn get_plan_tasks(path: &str) -> anyhow::Result<Vec<PlanTask>> {
    let conn = open_connection(path)?;
    let mut stmt = conn.prepare(
        "SELECT id, title, start_date, end_date, main_tag_id, sub_tag_id, notes, is_completed, completed_at, sort_order
         FROM plan_tasks
         ORDER BY sort_order, id",
    )?;
    let iter = stmt.query_map([], |row| {
        Ok(PlanTask {
            id: row.get(0)?,
            title: row.get(1)?,
            start_date: row.get(2)?,
            end_date: row.get(3)?,
            main_tag_id: row.get(4)?,
            sub_tag_id: row.get(5)?,
            notes: row.get(6)?,
            is_completed: row.get(7)?,
            completed_at: row.get(8)?,
            sort_order: row.get(9)?,
        })
    })?;

    let mut tasks = Vec::new();
    for task in iter {
        tasks.push(task?);
    }
    Ok(tasks)
}

pub fn save_plan_task(path: &str, task: PlanTask) -> anyhow::Result<i64> {
    let conn = open_connection(path)?;
    if task.id > 0 {
        conn.execute(
            "UPDATE plan_tasks
             SET title=?1, start_date=?2, end_date=?3, main_tag_id=?4, sub_tag_id=?5,
                 notes=?6, is_completed=?7, completed_at=?8, updated_at=CURRENT_TIMESTAMP
             WHERE id=?9",
            params![
                task.title,
                task.start_date,
                task.end_date,
                task.main_tag_id,
                task.sub_tag_id,
                task.notes,
                task.is_completed,
                task.completed_at,
                task.id
            ],
        )?;
        Ok(task.id)
    } else {
        let next_sort_order: i64 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM plan_tasks",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO plan_tasks
             (title, start_date, end_date, main_tag_id, sub_tag_id, notes, is_completed, completed_at, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                task.title,
                task.start_date,
                task.end_date,
                task.main_tag_id,
                task.sub_tag_id,
                task.notes,
                task.is_completed,
                task.completed_at,
                next_sort_order
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn swap_plan_tasks(path: &str, first_id: i64, second_id: i64) -> anyhow::Result<()> {
    if first_id == second_id {
        return Ok(());
    }

    let mut conn = open_connection(path)?;
    let tx = conn.transaction()?;
    let first_order: i64 = tx.query_row(
        "SELECT sort_order FROM plan_tasks WHERE id = ?1",
        params![first_id],
        |row| row.get(0),
    )?;
    let second_order: i64 = tx.query_row(
        "SELECT sort_order FROM plan_tasks WHERE id = ?1",
        params![second_id],
        |row| row.get(0),
    )?;

    tx.execute(
        "UPDATE plan_tasks SET sort_order = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![second_order, first_id],
    )?;
    tx.execute(
        "UPDATE plan_tasks SET sort_order = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![first_order, second_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn delete_plan_task(path: &str, id: i64) -> anyhow::Result<()> {
    let conn = open_connection(path)?;
    conn.execute("DELETE FROM plan_tasks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn toggle_plan_task(path: &str, id: i64, is_completed: bool) -> anyhow::Result<()> {
    let conn = open_connection(path)?;
    conn.execute(
        "UPDATE plan_tasks
         SET is_completed=?1,
             completed_at=CASE WHEN ?1 THEN datetime('now', 'localtime') ELSE NULL END,
             updated_at=CURRENT_TIMESTAMP
         WHERE id=?2",
        params![is_completed, id],
    )?;
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
