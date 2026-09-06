use crate::{
    is_demo_mode,
    storage::{open_database, record_audit},
};
use rusqlite::params;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

#[tauri::command]
pub fn seed_demo_workspace(app: tauri::AppHandle) -> Result<(), String> {
    if !is_demo_mode() {
        return Err("合成工作区只允许在独立 Demo 中初始化。".to_string());
    }
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位 Demo 数据目录。".to_string())?;
    let project_dir = data_dir.join("synthetic-demo-project");
    fs::create_dir_all(project_dir.join("notes"))
        .map_err(|_| "无法创建合成项目目录。".to_string())?;
    let readme = project_dir.join("README.md");
    if !readme.exists() {
        fs::write(&readme,"# SYNTH-AMP-01 合成项目\n\n仅用于展示工作台，不含真实芯片数据。\n\n- 当前阶段：验证准备\n- 目标偏置电流：10 uA（合成值）\n").map_err(|_|"无法写入合成项目。".to_string())?;
    }
    let root = project_dir
        .canonicalize()
        .map_err(|_| "无法验证合成项目目录。".to_string())?;
    let root_text = root
        .to_str()
        .ok_or_else(|| "Demo 路径编码不受支持。".to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs() as i64)
        .unwrap_or(1);
    let connection = open_database(&app)?;
    let (start_date, end_date): (String, String) = connection.query_row("SELECT date(?1,'unixepoch','localtime','-14 days'),date(?1,'unixepoch','localtime','+45 days')", params![now], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|_| "无法生成合成项目日期。".to_string())?;
    connection.execute("INSERT INTO projects(id,name,root_path) VALUES('project-demo-analog','合成模拟前端',?1) ON CONFLICT(id) DO UPDATE SET name=excluded.name,root_path=excluded.root_path,updated_at=unixepoch()",params![root_text]).map_err(|_|"无法初始化合成项目。".to_string())?;
    connection.execute("INSERT OR IGNORE INTO project_progress(project_id,start_date,end_date,current_phase,milestone,weekly_focus,current_focus,risks,progress_percent) VALUES('project-demo-analog',?1,?2,'验证准备','完成合成回归计划','复核 PVT 覆盖；整理评审材料','关闭两个合成检查项','模型角与温度组合尚待复核',68)",params![start_date,end_date]).map_err(|_|"无法初始化合成进度。".to_string())?;
    connection.execute("INSERT OR IGNORE INTO project_annotations(project_id,starred,note) VALUES('project-demo-analog',1,'重点演示项目；所有参数均为合成数据。')",[]).map_err(|_|"无法初始化合成标记。".to_string())?;
    for (id, percent, phase, offset) in [
        ("history-demo-1", 25, "规格整理", 14_i64),
        ("history-demo-2", 48, "原理图复核", 7),
        ("history-demo-3", 68, "验证准备", 0),
    ] {
        let snapshot=serde_json::json!({"projectId":"project-demo-analog","name":"合成模拟前端","rootPath":"","startDate":start_date,"endDate":end_date,"currentPhase":phase,"milestone":"完成合成回归计划","weeklyFocus":"复核 PVT 覆盖；整理评审材料","currentFocus":"关闭两个合成检查项","risks":"模型角与温度组合尚待复核","progressPercent":percent,"starred":true,"note":"重点演示项目；所有参数均为合成数据。","updatedAt":0}).to_string();
        connection.execute("INSERT OR IGNORE INTO project_progress_history(id,project_id,snapshot_json,created_at) VALUES(?1,'project-demo-analog',?2,?3)",params![id,snapshot,now-offset*86400]).map_err(|_|"无法初始化合成进度历史。".to_string())?;
    }
    connection.execute("INSERT OR IGNORE INTO calendar_tasks(id,title,start_at,status,priority,remind_at,project_id,notes) VALUES('task-demo-review','紧急：合成设计评审',?1,'doing','important',?2,'project-demo-analog','演示重点日程的亮色与加粗状态')",params![now+3600,now-60]).map_err(|_|"无法初始化合成日程。".to_string())?;
    connection.execute("INSERT OR IGNORE INTO calendar_tasks(id,title,start_at,status,priority,project_id,notes) VALUES('task-demo-report','整理验证报告',?1,'todo','normal','project-demo-analog','演示普通待办')",params![now+86400]).map_err(|_|"无法初始化合成日程。".to_string())?;
    connection.execute("INSERT OR IGNORE INTO notes(id,title,body) VALUES('note-demo-datasheet','Datasheet 阅读检查单','## 阅读目标\n\n- [x] 核对供电范围\n- [ ] 复核绝对最大额定值\n\n| 参数 | 合成值 | 状态 |\n|---|---:|---|\n| VDD | 1.8 V | 已摘录 |\n| Ibias | 10 uA | 待复核 |')",[]).map_err(|_|"无法初始化合成笔记。".to_string())?;
    connection.execute("INSERT OR IGNORE INTO notes(id,title,body) VALUES('note-demo-review','本周评审记录','## 已完成\n\n完成规格表与测试条件整理。\n\n## 风险\n\n> 以下仅为合成演示内容。\n\n温度角覆盖仍需人工确认。')",[]).map_err(|_|"无法初始化合成笔记。".to_string())?;
    connection.execute("INSERT OR IGNORE INTO skills(id,name,description,instructions,source_name,enabled,version,source_type,permissions_json) VALUES('skill-demo-review','合成评审检查','按来源、单位、条件和未确认项复核工程摘要。','# 合成评审检查\n\n1. 保留来源位置。\n2. 保留单位和测试条件。\n3. 不确定内容标记待复核。','内置合成示例',1,'1.0.0','demo','[\"prompt\"]')",[]).map_err(|_|"无法初始化合成 Skill。".to_string())?;
    let calculator = "/System/Applications/Calculator.app";
    if fs::metadata(calculator)
        .map(|m| m.is_dir())
        .unwrap_or(false)
    {
        let _=connection.execute("INSERT OR IGNORE INTO software_launchers(id,name,target_path,target_kind) VALUES('software-demo-calculator','系统计算器（演示）',?1,'application')",params![calculator]);
    }
    record_audit(&app, "demo.workspace-seed", "success");
    Ok(())
}
