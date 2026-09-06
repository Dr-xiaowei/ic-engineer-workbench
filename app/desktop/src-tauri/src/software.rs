use crate::storage::{open_database, record_audit};
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareLauncher {
    id: String,
    name: String,
    target_name: String,
    target_kind: String,
    updated_at: i64,
}

fn validate_target(path: &str) -> Result<(PathBuf, String), String> {
    if path.is_empty() || path.len() > 4096 {
        return Err("软件路径无效。".to_string());
    }
    let requested = Path::new(path);
    let metadata = fs::symlink_metadata(requested).map_err(|_| "无法读取所选软件。".to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("软件快捷入口不能指向符号链接。".to_string());
    }
    let canonical = requested
        .canonicalize()
        .map_err(|_| "无法验证软件路径。".to_string())?;
    if metadata.is_dir()
        && canonical
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
    {
        return Ok((canonical, "application".to_string()));
    }
    if metadata.is_file() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 != 0 {
                return Ok((canonical, "executable".to_string()));
            }
        }
        #[cfg(windows)]
        if canonical
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            return Ok((canonical, "executable".to_string()));
        }
    }
    Err("只允许普通可执行文件或 macOS .app 应用包。".to_string())
}

fn launcher_id(path: &Path) -> String {
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    format!(
        "software-{}",
        digest[..12]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn valid_id(id: &str) -> bool {
    id.starts_with("software-")
        && id.len() <= 80
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn load(app: &tauri::AppHandle) -> Result<Vec<SoftwareLauncher>, String> {
    let connection = open_database(app)?;
    let mut statement = connection.prepare("SELECT id,name,target_path,target_kind,updated_at FROM software_launchers ORDER BY updated_at DESC").map_err(|_| "无法读取软件快捷入口。".to_string())?;
    let records = statement
        .query_map([], |row| {
            let path: String = row.get(2)?;
            Ok(SoftwareLauncher {
                id: row.get(0)?,
                name: row.get(1)?,
                target_name: Path::new(&path)
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("应用")
                    .to_string(),
                target_kind: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|_| "无法读取软件快捷入口。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "软件快捷入口记录无效。".to_string())?;
    Ok(records)
}

#[tauri::command]
pub fn add_software_launcher(
    app: tauri::AppHandle,
    path: String,
    name: String,
) -> Result<SoftwareLauncher, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 120 || name.contains('\0') {
        return Err("软件名称无效。".to_string());
    }
    let (target, kind) = validate_target(&path)?;
    let target_text = target
        .to_str()
        .ok_or_else(|| "软件路径编码不受支持。".to_string())?;
    let id = launcher_id(&target);
    open_database(&app)?.execute("INSERT INTO software_launchers(id,name,target_path,target_kind) VALUES(?1,?2,?3,?4) ON CONFLICT(target_path) DO UPDATE SET name=excluded.name,target_kind=excluded.target_kind,updated_at=unixepoch()", params![id,name,target_text,kind]).map_err(|_| "无法保存软件快捷入口。".to_string())?;
    record_audit(&app, "software.launcher-add", "success");
    load(&app)?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "无法读取已保存的软件快捷入口。".to_string())
}

#[tauri::command]
pub fn list_software_launchers(app: tauri::AppHandle) -> Result<Vec<SoftwareLauncher>, String> {
    load(&app)
}

#[tauri::command]
pub fn remove_software_launcher(app: tauri::AppHandle, launcher_id: String) -> Result<(), String> {
    if !valid_id(&launcher_id) {
        return Err("软件快捷入口标识无效。".to_string());
    }
    open_database(&app)?
        .execute(
            "DELETE FROM software_launchers WHERE id=?1",
            params![launcher_id],
        )
        .map_err(|_| "无法移除软件快捷入口。".to_string())?;
    record_audit(&app, "software.launcher-remove", "success");
    Ok(())
}

#[tauri::command]
pub fn launch_software(app: tauri::AppHandle, launcher_id: String) -> Result<(), String> {
    if !valid_id(&launcher_id) {
        return Err("软件快捷入口标识无效。".to_string());
    }
    let connection = open_database(&app)?;
    let (path, stored_kind): (String, String) = connection
        .query_row(
            "SELECT target_path,target_kind FROM software_launchers WHERE id=?1",
            params![launcher_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "软件快捷入口不存在。".to_string())?;
    let (target, kind) = validate_target(&path)?;
    if kind != stored_kind {
        return Err("软件类型已变化，已拒绝启动。".to_string());
    }
    let result = if kind == "application" {
        #[cfg(target_os = "macos")]
        {
            Command::new("/usr/bin/open").arg(&target).spawn()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Command::new(&target).spawn()
        }
    } else {
        Command::new(&target).spawn()
    };
    result.map_err(|_| "无法启动所选软件。".to_string())?;
    record_audit(&app, "software.launch", "success");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn launcher_ids_are_bounded() {
        assert!(valid_id("software-0123abcd"));
        assert!(!valid_id("software-../../bin/sh"));
    }

    #[test]
    fn only_application_bundles_or_executables_are_accepted() {
        let temporary = std::env::temp_dir().join(format!(
            "ic-workbench-software-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&temporary).expect("temporary directory");
        let app = temporary.join("Example.app");
        fs::create_dir(&app).expect("application bundle");
        assert_eq!(
            validate_target(app.to_str().unwrap()).unwrap().1,
            "application"
        );

        let text = temporary.join("notes.txt");
        fs::write(&text, "not executable").expect("text fixture");
        assert!(validate_target(text.to_str().unwrap()).is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let executable = temporary.join("local-tool");
            fs::write(&executable, "#!/bin/sh\n").expect("executable fixture");
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
                .expect("executable permissions");
            assert_eq!(
                validate_target(executable.to_str().unwrap()).unwrap().1,
                "executable"
            );
        }
        fs::remove_dir_all(&temporary).expect("remove temporary test directory");
    }
}
