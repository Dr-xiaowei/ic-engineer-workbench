use crate::storage::{open_database, record_audit};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_FIELD: usize = 8_000;
const MAX_TASK_TITLE: usize = 240;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProgressInput {
    project_id: String,
    name: String,
    start_date: String,
    end_date: String,
    current_phase: String,
    milestone: String,
    weekly_focus: String,
    current_focus: String,
    risks: String,
    progress_percent: u8,
    starred: bool,
    note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProgress {
    project_id: String,
    name: String,
    root_path: String,
    start_date: String,
    end_date: String,
    current_phase: String,
    milestone: String,
    weekly_focus: String,
    current_focus: String,
    risks: String,
    progress_percent: u8,
    starred: bool,
    note: String,
    updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProgressHistory {
    id: String,
    project_id: String,
    snapshot: ProjectProgress,
    created_at: i64,
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() <= 100
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn valid_date(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let bytes = value.as_bytes();
    if !(bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit()))
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u16>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn bounded(value: &str, limit: usize) -> bool {
    value.len() <= limit && !value.contains('\0')
}

fn load_progress(connection: &Connection) -> Result<Vec<ProjectProgress>, String> {
    let mut statement = connection
        .prepare(
            "SELECT p.id,p.name,p.root_path,
                    COALESCE(g.start_date,''),COALESCE(g.end_date,''),COALESCE(g.current_phase,''),
                    COALESCE(g.milestone,''),COALESCE(g.weekly_focus,''),COALESCE(g.current_focus,''),
                    COALESCE(g.risks,''),COALESCE(g.progress_percent,0),COALESCE(a.starred,0),
                    COALESCE(a.note,''),COALESCE(g.updated_at,p.updated_at)
             FROM projects p LEFT JOIN project_progress g ON g.project_id=p.id
             LEFT JOIN project_annotations a ON a.project_id=p.id
             ORDER BY COALESCE(g.updated_at,p.updated_at) DESC",
        )
        .map_err(|_| "无法读取项目进度。".to_string())?;
    let records = statement
        .query_map([], |row| {
            Ok(ProjectProgress {
                project_id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                start_date: row.get(3)?,
                end_date: row.get(4)?,
                current_phase: row.get(5)?,
                milestone: row.get(6)?,
                weekly_focus: row.get(7)?,
                current_focus: row.get(8)?,
                risks: row.get(9)?,
                progress_percent: row.get::<_, i64>(10)? as u8,
                starred: row.get::<_, i64>(11)? != 0,
                note: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })
        .map_err(|_| "无法读取项目进度。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "项目进度记录无效。".to_string())?;
    Ok(records)
}

#[tauri::command]
pub fn list_project_progress(app: tauri::AppHandle) -> Result<Vec<ProjectProgress>, String> {
    load_progress(&open_database(&app)?)
}

#[tauri::command]
pub fn save_project_progress(
    app: tauri::AppHandle,
    input: ProjectProgressInput,
) -> Result<ProjectProgress, String> {
    if !valid_id(&input.project_id, "project-")
        || input.name.trim().is_empty()
        || !bounded(&input.name, 160)
        || !valid_date(&input.start_date)
        || !valid_date(&input.end_date)
        || (!input.start_date.is_empty()
            && !input.end_date.is_empty()
            && input.start_date > input.end_date)
        || input.progress_percent > 100
        || [
            &input.current_phase,
            &input.milestone,
            &input.weekly_focus,
            &input.current_focus,
            &input.risks,
            &input.note,
        ]
        .iter()
        .any(|value| !bounded(value, MAX_FIELD))
    {
        return Err("项目进度字段、日期或长度无效。".to_string());
    }
    let mut connection = open_database(&app)?;
    let transaction = connection
        .transaction()
        .map_err(|_| "无法开始项目进度更新。".to_string())?;
    let updated = transaction
        .execute(
            "UPDATE projects SET name=?2,updated_at=unixepoch() WHERE id=?1",
            params![input.project_id, input.name.trim()],
        )
        .map_err(|_| "无法更新项目名称。".to_string())?;
    if updated != 1 {
        return Err("项目不存在；请先关联本地项目目录。".to_string());
    }
    transaction
        .execute(
            "INSERT INTO project_progress(project_id,start_date,end_date,current_phase,milestone,weekly_focus,current_focus,risks,progress_percent)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(project_id) DO UPDATE SET start_date=excluded.start_date,end_date=excluded.end_date,
             current_phase=excluded.current_phase,milestone=excluded.milestone,weekly_focus=excluded.weekly_focus,
             current_focus=excluded.current_focus,risks=excluded.risks,progress_percent=excluded.progress_percent,updated_at=unixepoch()",
            params![input.project_id,input.start_date,input.end_date,input.current_phase,input.milestone,input.weekly_focus,input.current_focus,input.risks,input.progress_percent],
        )
        .map_err(|_| "无法保存项目进度。".to_string())?;
    transaction
        .execute(
            "INSERT INTO project_annotations(project_id,starred,note) VALUES(?1,?2,?3)
             ON CONFLICT(project_id) DO UPDATE SET starred=excluded.starred,note=excluded.note,updated_at=unixepoch()",
            params![input.project_id, input.starred as i64, input.note],
        )
        .map_err(|_| "无法保存项目标记与备注。".to_string())?;
    let history_id = format!(
        "history-{}",
        Sha256::digest(
            format!(
                "{}|{}|{:?}",
                input.project_id,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|value| value.as_nanos())
                    .unwrap_or_default(),
                input.progress_percent
            )
            .as_bytes()
        )[..12]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let snapshot_json = serde_json::to_string(&serde_json::json!({
        "projectId": input.project_id,
        "name": input.name.trim(),
        "rootPath": "",
        "startDate": input.start_date,
        "endDate": input.end_date,
        "currentPhase": input.current_phase,
        "milestone": input.milestone,
        "weeklyFocus": input.weekly_focus,
        "currentFocus": input.current_focus,
        "risks": input.risks,
        "progressPercent": input.progress_percent,
        "starred": input.starred,
        "note": input.note,
        "updatedAt": 0
    }))
    .map_err(|_| "无法创建项目进度快照。".to_string())?;
    transaction
        .execute(
            "INSERT INTO project_progress_history(id,project_id,snapshot_json) VALUES(?1,?2,?3)",
            params![history_id, input.project_id, snapshot_json],
        )
        .map_err(|_| "无法保存项目进度历史。".to_string())?;
    transaction
        .commit()
        .map_err(|_| "无法提交项目进度。".to_string())?;
    record_audit(&app, "project.progress-save", "success");
    load_progress(&open_database(&app)?)?
        .into_iter()
        .find(|record| record.project_id == input.project_id)
        .ok_or_else(|| "无法读取已保存的项目进度。".to_string())
}

#[tauri::command]
pub fn list_project_progress_history(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<Vec<ProjectProgressHistory>, String> {
    if !valid_id(&project_id, "project-") {
        return Err("项目标识无效。".to_string());
    }
    let connection = open_database(&app)?;
    let root_path = connection
        .query_row(
            "SELECT root_path FROM projects WHERE id=?1",
            params![project_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "项目不存在。".to_string())?;
    let mut statement = connection
        .prepare("SELECT id,snapshot_json,created_at FROM project_progress_history WHERE project_id=?1 ORDER BY created_at DESC,id DESC LIMIT 200")
        .map_err(|_| "无法读取项目进度历史。".to_string())?;
    let records = statement
        .query_map(params![project_id], |row| {
            let id: String = row.get(0)?;
            let json: String = row.get(1)?;
            let created_at: i64 = row.get(2)?;
            Ok((id, json, created_at))
        })
        .map_err(|_| "无法读取项目进度历史。".to_string())?
        .filter_map(Result::ok)
        .filter_map(|(id, json, created_at)| {
            serde_json::from_str::<ProjectProgress>(&json)
                .ok()
                .map(|mut snapshot| {
                    snapshot.root_path = root_path.clone();
                    snapshot.updated_at = created_at;
                    ProjectProgressHistory {
                        id,
                        project_id: project_id.clone(),
                        snapshot,
                        created_at,
                    }
                })
        })
        .collect();
    Ok(records)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarTaskInput {
    id: Option<String>,
    title: String,
    start_at: i64,
    end_at: Option<i64>,
    status: String,
    priority: String,
    remind_at: Option<i64>,
    project_id: Option<String>,
    notes: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarTask {
    id: String,
    title: String,
    start_at: i64,
    end_at: Option<i64>,
    status: String,
    priority: String,
    remind_at: Option<i64>,
    project_id: Option<String>,
    project_name: Option<String>,
    notes: String,
    updated_at: i64,
}

fn task_id(input: &CalendarTaskInput) -> String {
    let digest = Sha256::digest(
        format!(
            "{}|{}|{}",
            input.title,
            input.start_at,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or_default()
        )
        .as_bytes(),
    );
    format!(
        "task-{}",
        digest[..12]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn validate_task(input: &CalendarTaskInput) -> Result<(), String> {
    if input.title.trim().is_empty()
        || !bounded(&input.title, MAX_TASK_TITLE)
        || !bounded(&input.notes, MAX_FIELD)
        || input.start_at <= 0
        || input.end_at.is_some_and(|end| end < input.start_at)
        || !matches!(input.status.as_str(), "todo" | "doing" | "done")
        || !matches!(input.priority.as_str(), "normal" | "important")
        || input.id.as_ref().is_some_and(|id| !valid_id(id, "task-"))
        || input
            .project_id
            .as_ref()
            .is_some_and(|id| !valid_id(id, "project-"))
    {
        return Err("待办字段、时间、状态或长度无效。".to_string());
    }
    Ok(())
}

fn query_tasks(
    connection: &Connection,
    range_start: i64,
    range_end: i64,
) -> Result<Vec<CalendarTask>, String> {
    if range_start <= 0 || range_end <= range_start || range_end - range_start > 370 * 86_400 {
        return Err("日历查询范围无效或超过 370 天。".to_string());
    }
    let mut statement = connection.prepare(
        "SELECT t.id,t.title,t.start_at,t.end_at,t.status,t.priority,t.remind_at,t.project_id,p.name,t.notes,t.updated_at
         FROM calendar_tasks t LEFT JOIN projects p ON p.id=t.project_id
         WHERE t.start_at>=?1 AND t.start_at<?2 ORDER BY t.start_at,t.priority DESC",
    ).map_err(|_| "无法读取日历待办。".to_string())?;
    let records = statement
        .query_map(params![range_start, range_end], |row| {
            Ok(CalendarTask {
                id: row.get(0)?,
                title: row.get(1)?,
                start_at: row.get(2)?,
                end_at: row.get(3)?,
                status: row.get(4)?,
                priority: row.get(5)?,
                remind_at: row.get(6)?,
                project_id: row.get(7)?,
                project_name: row.get(8)?,
                notes: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|_| "无法读取日历待办。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "日历待办记录无效。".to_string())?;
    Ok(records)
}

#[tauri::command]
pub fn list_calendar_tasks(
    app: tauri::AppHandle,
    range_start: i64,
    range_end: i64,
) -> Result<Vec<CalendarTask>, String> {
    query_tasks(&open_database(&app)?, range_start, range_end)
}

#[tauri::command]
pub fn save_calendar_task(
    app: tauri::AppHandle,
    input: CalendarTaskInput,
) -> Result<CalendarTask, String> {
    validate_task(&input)?;
    let id = input.id.clone().unwrap_or_else(|| task_id(&input));
    let connection = open_database(&app)?;
    if let Some(project_id) = &input.project_id {
        let exists = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
                params![project_id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);
        if !exists {
            return Err("关联项目不存在。".to_string());
        }
    }
    connection.execute(
        "INSERT INTO calendar_tasks(id,title,start_at,end_at,status,priority,remind_at,project_id,notes)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(id) DO UPDATE SET title=excluded.title,start_at=excluded.start_at,end_at=excluded.end_at,
         status=excluded.status,priority=excluded.priority,remind_at=excluded.remind_at,project_id=excluded.project_id,
         notes=excluded.notes,updated_at=unixepoch()",
        params![id,input.title.trim(),input.start_at,input.end_at,input.status,input.priority,input.remind_at,input.project_id,input.notes],
    ).map_err(|_| "无法保存日历待办。".to_string())?;
    record_audit(&app, "calendar.task-save", "success");
    query_tasks(
        &connection,
        input.start_at.saturating_sub(1),
        input.start_at.saturating_add(2),
    )?
    .into_iter()
    .find(|task| task.id == id)
    .ok_or_else(|| "无法读取已保存待办。".to_string())
}

#[tauri::command]
pub fn delete_calendar_task(app: tauri::AppHandle, task_id: String) -> Result<(), String> {
    if !valid_id(&task_id, "task-") {
        return Err("待办标识无效。".to_string());
    }
    open_database(&app)?
        .execute("DELETE FROM calendar_tasks WHERE id=?1", params![task_id])
        .map_err(|_| "无法删除待办。".to_string())?;
    record_audit(&app, "calendar.task-delete", "success");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_dates_and_tasks_are_bounded() {
        assert!(valid_date("2026-09-06"));
        assert!(valid_date("2028-02-29"));
        assert!(!valid_date("2026-99"));
        assert!(!valid_date("2026-02-29"));
        assert!(!valid_date("2026-09-31"));
        let task = CalendarTaskInput {
            id: None,
            title: "完成评审".into(),
            start_at: 1_800_000_000,
            end_at: None,
            status: "doing".into(),
            priority: "important".into(),
            remind_at: None,
            project_id: None,
            notes: String::new(),
        };
        assert!(validate_task(&task).is_ok());
    }
}
