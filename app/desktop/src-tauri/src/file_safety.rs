use std::fs::Metadata;
use std::path::Path;

// canonicalize() adds a verbatim prefix on Windows. That representation change
// does not mean the selected file moved. Keep comparing every actual component
// so a replaced parent/junction still fails closed.
pub(crate) fn unchanged_location(requested: &Path, canonical: &Path) -> bool {
    #[cfg(not(windows))]
    {
        requested == canonical
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let mut a = requested.components();
        let mut b = canonical.components();
        let prefixes_match = match (a.next(), b.next()) {
            (Some(Component::Prefix(a)), Some(Component::Prefix(b))) => {
                match (a.kind(), b.kind()) {
                    (
                        Prefix::Disk(a) | Prefix::VerbatimDisk(a),
                        Prefix::Disk(b) | Prefix::VerbatimDisk(b),
                    ) => a.eq_ignore_ascii_case(&b),
                    (
                        Prefix::UNC(a, x) | Prefix::VerbatimUNC(a, x),
                        Prefix::UNC(b, y) | Prefix::VerbatimUNC(b, y),
                    ) => a == b && x == y,
                    _ => false,
                }
            }
            _ => false,
        };
        prefixes_match && a.eq(b)
    }
}

pub(crate) fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Includes directory junctions and other reparse points, not only symlinks.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.len() > 32767 || path.contains('\0') {
        return Err("文件路径为空、过长或包含无效字符。".to_string());
    }
    #[cfg(windows)]
    validate_windows_path(path)?;
    Ok(())
}

#[cfg(any(windows, test))]
fn validate_windows_path(path: &str) -> Result<(), String> {
    let normalized = path.replace('/', "\\");
    let text = normalized.strip_prefix("\\\\?\\").unwrap_or(&normalized);
    if text.starts_with("\\\\.\\") || text.to_ascii_uppercase().starts_with("GLOBALROOT\\") {
        return Err("不接受 Windows 设备路径。".to_string());
    }
    for (index, component) in text.split('\\').enumerate() {
        if component.is_empty() {
            continue;
        }
        if index == 0
            && component.len() == 2
            && component.as_bytes()[0].is_ascii_alphabetic()
            && component.ends_with(':')
        {
            continue;
        }
        if component.ends_with([' ', '.'])
            || component
                .chars()
                .any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
        {
            return Err("Windows 路径包含设备名、数据流或不明确的文件名。".to_string());
        }
        let stem = component
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end_matches(' ')
            .to_uppercase();
        if matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        }) {
            return Err("不接受 Windows 保留设备名。".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_names_reject_devices_streams_and_ambiguous_aliases() {
        for path in [
            r"C:\data\NUL.md",
            r"C:\data\NUL .md",
            r"C:\data\COM1.txt",
            r"C:\data\COM¹.txt",
            r"C:\data\x:stream.md",
            r"C:\data\notes.md ",
            r"\\.\PhysicalDrive0",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1",
            r"C:\data\..\secret.txt",
        ] {
            assert!(validate_windows_path(path).is_err(), "{path}");
        }
    }
    #[test]
    fn windows_names_preserve_unicode_spaces_and_normal_extended_paths() {
        for path in [
            r"C:\Users\研发 用户\notes.md",
            r"D:\project\LPT10.md",
            r"\\?\C:\工程\review.docx",
            r"\\server\approved-share\report.pdf",
        ] {
            assert!(validate_windows_path(path).is_ok(), "{path}");
        }
    }
}
