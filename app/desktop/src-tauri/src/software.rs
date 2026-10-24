use crate::storage::{open_database, record_audit};
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

fn native_executable(path: &Path, metadata: &fs::Metadata) -> bool {
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    #[cfg(windows)]
    {
        valid_windows_executable(path)
    }
    #[cfg(not(windows))]
    {
        let mut magic = [0_u8; 4];
        if fs::File::open(path)
            .and_then(|mut f| f.read_exact(&mut magic))
            .is_err()
        {
            return false;
        }
        matches!(
            magic,
            [0x7f, b'E', b'L', b'F']
                | [0xcf, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xce, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xce]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
                | [0xca, 0xfe, 0xba, 0xbf]
                | [0xbf, 0xba, 0xfe, 0xca]
        )
    }
}

// Check the native image format; the OS still enforces executable trust and loading.
#[cfg(any(windows, test))]
fn valid_windows_executable(path: &Path) -> bool {
    use std::io::{Seek, SeekFrom};
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
    {
        return false;
    }
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let Ok(metadata) = file.metadata() else {
        return false;
    };
    let mut dos = [0u8; 64];
    if file.read_exact(&mut dos).is_err() || dos[..2] != *b"MZ" {
        return false;
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if offset < 64
        || offset.saturating_add(26) > metadata.len()
        || file.seek(SeekFrom::Start(offset)).is_err()
    {
        return false;
    }
    let mut pe = [0u8; 26];
    if file.read_exact(&mut pe).is_err() || pe[..4] != *b"PE\0\0" {
        return false;
    }
    let machine = u16::from_le_bytes([pe[4], pe[5]]);
    let optional_size = u16::from_le_bytes([pe[20], pe[21]]) as u64;
    let flags = u16::from_le_bytes([pe[22], pe[23]]);
    let magic = u16::from_le_bytes([pe[24], pe[25]]);
    let sections = u16::from_le_bytes([pe[6], pe[7]]) as u64;
    matches!(
        (machine, magic),
        (0x14c, 0x10b) | (0x8664, 0x20b) | (0xaa64, 0x20b)
    ) && flags & 0x0002 != 0
        && flags & 0x2000 == 0
        && optional_size >= if magic == 0x20b { 112 } else { 96 }
        && sections > 0
        && sections <= 96
        && offset + 24 + optional_size + sections * 40 <= metadata.len()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareLauncher {
    id: String,
    name: String,
    target_name: String,
    target_kind: String,
    updated_at: i64,
}

pub(crate) fn validate_target(path: &str) -> Result<(PathBuf, String), String> {
    crate::file_safety::validate_path(path)?;
    if path.is_empty() || path.len() > 4096 {
        return Err("软件路径无效。".to_string());
    }
    let requested = Path::new(path);
    let metadata = fs::symlink_metadata(requested).map_err(|_| "无法读取所选软件。".to_string())?;
    if crate::file_safety::is_link(&metadata) {
        return Err("软件快捷入口不能指向符号链接。".to_string());
    }
    let canonical = requested
        .canonicalize()
        .map_err(|_| "无法验证软件路径。".to_string())?;
    if cfg!(target_os = "macos")
        && metadata.is_dir()
        && canonical
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
    {
        let contents = canonical.join("Contents");
        let valid_bundle = contents.join("Info.plist").is_file()
            && fs::read_dir(contents.join("MacOS")).is_ok_and(|entries| {
                entries.flatten().any(|entry| {
                    fs::symlink_metadata(entry.path()).is_ok_and(|meta| {
                        !crate::file_safety::is_link(&meta)
                            && native_executable(&entry.path(), &meta)
                    })
                })
            });
        if valid_bundle {
            return Ok((canonical, "application".to_string()));
        }
        return Err("所选目录不是完整的 macOS 应用包。".to_string());
    }
    if metadata.is_file() {
        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "sh" | "bash"
                | "zsh"
                | "fish"
                | "dash"
                | "csh"
                | "tcsh"
                | "ksh"
                | "cmd.exe"
                | "powershell.exe"
                | "pwsh"
                | "pwsh.exe"
        ) {
            return Err("快捷入口不接受 Shell 解释器。".to_string());
        }
        if native_executable(&canonical, &metadata) {
            return Ok((canonical, "executable".to_string()));
        }
    }
    Err(if cfg!(windows) {
        "只允许 Windows 原生 .exe 程序，不接受脚本、快捷方式或目录。"
    } else {
        "只允许普通可执行文件或 macOS .app 应用包。"
    }
    .to_string())
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
    if kind != stored_kind || target != Path::new(&path) {
        return Err("软件路径或类型已变化，已拒绝启动。".to_string());
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

    #[test]
    fn windows_executables_reject_truncated_dll_script_and_invalid_pe_headers() {
        let directory = std::env::temp_dir().join(format!(
            "icwb-pe-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("合成 工具.exe");
        let mut image = vec![0u8; 512];
        image[..2].copy_from_slice(b"MZ");
        image[60..64].copy_from_slice(&64u32.to_le_bytes());
        image[64..68].copy_from_slice(b"PE\0\0");
        image[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
        image[70..72].copy_from_slice(&1u16.to_le_bytes());
        image[84..86].copy_from_slice(&112u16.to_le_bytes());
        image[86..88].copy_from_slice(&2u16.to_le_bytes());
        image[88..90].copy_from_slice(&0x20bu16.to_le_bytes());
        fs::write(&path, &image).unwrap();
        assert!(valid_windows_executable(&path)); // Format fixture only; never executed.
        for (offset, replacement) in [(64, b'X'), (87, 0x20), (88, 0), (70, 0), (61, 0xff)] {
            let mut invalid = image.clone();
            invalid[offset] = replacement;
            fs::write(&path, invalid).unwrap();
            assert!(!valid_windows_executable(&path));
        }
        for data in [&b"MZ"[..], &b"#!/bin/sh\n"[..], &image[..90]] {
            fs::write(&path, data).unwrap();
            assert!(!valid_windows_executable(&path));
        }
        let script = directory.join("tool.cmd");
        fs::write(&script, &image).unwrap();
        assert!(!valid_windows_executable(&script));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_native_launcher_validates_real_program_and_rejects_shell() {
        let executable = std::env::current_exe().unwrap();
        assert_eq!(
            validate_target(executable.to_str().unwrap()).unwrap().1,
            "executable"
        );
        let shell = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/cmd.exe");
        assert!(validate_target(shell.to_str().unwrap()).is_err());
    }
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
        assert!(validate_target(app.to_str().unwrap()).is_err());

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
            assert!(validate_target(executable.to_str().unwrap()).is_err());
            assert!(validate_target("/bin/sh").is_err());
            let native = temporary.join("native-tool");
            fs::copy("/usr/bin/true", &native).unwrap();
            assert_eq!(
                validate_target(native.to_str().unwrap()).unwrap().1,
                "executable"
            );
            std::os::unix::fs::symlink(&native, temporary.join("linked-tool")).unwrap();
            assert!(validate_target(temporary.join("linked-tool").to_str().unwrap()).is_err());
            fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
            fs::write(
                app.join("Contents/Info.plist"),
                "<?xml version=\"1.0\"?><plist><dict/></plist>",
            )
            .unwrap();
            fs::copy(&native, app.join("Contents/MacOS/native-tool")).unwrap();
            if cfg!(target_os = "macos") {
                assert_eq!(
                    validate_target(app.to_str().unwrap()).unwrap().1,
                    "application"
                );
            } else {
                assert!(validate_target(app.to_str().unwrap()).is_err());
            }
        }
        fs::remove_dir_all(&temporary).expect("remove temporary test directory");
    }
}
