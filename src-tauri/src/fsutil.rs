use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::error::AppError;

/// 访问出错时视为不存在。
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

/// 移入系统回收站（Windows 回收站），文件与目录都支持。
///
/// 删除备份 / 回档点等存档数据统一走这里，让用户可以从回收站还原；
/// 失败时返回可直接展示的中文错误，而不是退回永久删除。
pub fn move_to_trash(path: &Path) -> Result<(), AppError> {
    trash::delete(path).map_err(|err| {
        let reason = match &err {
            trash::Error::TargetedRoot => "不能删除磁盘根目录".to_string(),
            trash::Error::CouldNotAccess { .. } => {
                "无法访问该目录，可能已被占用或权限不足".to_string()
            }
            trash::Error::CanonicalizePath { .. } => "路径无法解析".to_string(),
            trash::Error::Unknown { description } if description.contains("aborted") => {
                "操作已被取消".to_string()
            }
            _ => err.to_string(),
        };
        AppError(format!("移入回收站失败（{}）：{reason}", path.display()))
    })
}

/// 递归统计文件数与总字节数，无法读取的条目跳过。
pub fn walk_stats(dir: &Path) -> (u64, u64) {
    let mut count = 0u64;
    let mut size = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let entries = match fs::read_dir(&cur) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                count += 1;
                if let Ok(meta) = entry.metadata() {
                    size += meta.len();
                }
            }
        }
    }
    (count, size)
}

/// 目录（含子目录）中最近一次文件修改时间；无文件时退回目录自身修改时间。
///
/// 用于给用户提示「存档最后写入时间」，因此以文件 mtime 为准：
/// 仅查看顶层目录的 mtime 在 Windows 上并不可靠。
pub fn newest_file_ms(dir: &Path) -> f64 {
    let mut newest = 0.0f64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let Ok(entries) = fs::read_dir(&cur) else { continue };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                if let Ok(meta) = entry.metadata() {
                    newest = newest.max(modified_ms(&meta));
                }
            }
        }
    }
    if newest > 0.0 {
        return newest;
    }
    fs::metadata(dir).map(|m| modified_ms(&m)).unwrap_or(0.0)
}

/// 递归复制目录，每复制完一个文件回调一次（用于进度上报）。
pub fn copy_dir(src: &Path, dest: &Path, on_file: &mut dyn FnMut()) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir(&from, &to, on_file)?;
        } else if kind.is_file() {
            fs::copy(&from, &to)?;
            on_file();
        }
    }
    Ok(())
}

fn local_now() -> time::OffsetDateTime {
    time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc())
}

fn stamp_of(now: time::OffsetDateTime) -> String {
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

/// 备份名称时间戳：yyyyMMdd_HHmmss
pub fn timestamp() -> String {
    stamp_of(local_now())
}

/// 回档点 ID：yyyyMMdd_HHmmss_SSS，毫秒保证唯一且可按名排序。
pub fn point_stamp() -> String {
    let now = local_now();
    format!("{}_{:03}", stamp_of(now), now.millisecond())
}

/// 目录创建时间（毫秒）；平台不支持创建时间时退回修改时间。
pub fn created_ms(meta: &fs::Metadata) -> f64 {
    if let Ok(t) = meta.created() {
        if let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) {
            return d.as_secs_f64() * 1000.0;
        }
    }
    modified_ms(meta)
}

pub fn modified_ms(meta: &fs::Metadata) -> f64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

/// 解析为绝对路径，失败时退回原字符串。
pub fn absolutize(path: &str) -> PathBuf {
    let raw = Path::new(path);
    if raw.is_absolute() {
        return raw.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(raw))
        .unwrap_or_else(|_| raw.to_path_buf())
}

/// path 是否位于 root 之内（含 root 自身）。
///
/// 先做词法归一化（消掉 `.` 与 `..`），再按目录边界比较：
/// 既不让 `C:\backup` 命中 `C:\backup-old`，也不让 `C:\backup\..\Windows` 蒙混过关。
pub fn is_within(root: &str, path: &str) -> bool {
    if root.trim().is_empty() || path.trim().is_empty() {
        return false;
    }
    let root = comparable(&collapse(root));
    if root.is_empty() {
        return false;
    }
    let path = comparable(&collapse(path));
    path == root
        || path
            .strip_prefix(&root)
            .is_some_and(|rest| rest.starts_with('\\') || rest.starts_with('/'))
}

/// 词法归一化：绝对化后消掉 `.` 与 `..`，不解析符号链接。
///
/// 到根/前缀时 `PathBuf::pop` 返回 false，所以 `C:\..` 仍是 `C:\`。
fn collapse(path: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in absolutize(path).components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// 路径比较用的字符串形式：统一分隔符、去掉末尾分隔符、转小写。
fn comparable(path: &Path) -> String {
    let mut text = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        text = text.replace('/', "\\");
    }
    while text.ends_with('\\') || text.ends_with('/') {
        text.pop();
    }
    text.to_lowercase()
}

/// 小写归一化后的路径字符串，用于比较与哈希。
///
/// Windows 上 `/` 与 `\` 等价，统一成 `\`，否则同一目录会因为写法不同被当成两个工作区。
pub fn normalized_lower(path: &str) -> String {
    comparable(&absolutize(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_within_respects_directory_boundary() {
        let (root, inside, sibling) = if cfg!(windows) {
            (r"C:\vault", r"C:\vault\20260101_120000", r"C:\vault-old")
        } else {
            ("/vault", "/vault/20260101_120000", "/vault-old")
        };

        assert!(is_within(root, root));
        assert!(is_within(root, inside));
        assert!(!is_within(root, sibling));
        // 空路径不能当成「处处允许」
        assert!(!is_within("", inside));
        assert!(!is_within(root, ""));
    }

    #[cfg(windows)]
    #[test]
    fn is_within_ignores_case_and_separator_style() {
        assert!(is_within(r"C:\Vault\", r"c:/vault/backup"));
        assert!(!is_within(r"C:\Vault", r"C:\Vault2"));
    }

    /// 需要真实回收站：`cargo test --lib -- --ignored move_to_trash` 手动验证。
    #[cfg(windows)]
    #[test]
    #[ignore = "会真实写入系统回收站，仅手动运行"]
    fn move_to_trash_moves_directory_to_recycle_bin() {
        let dir = std::env::temp_dir().join(format!(
            "etg-vault-trash-test-{}-{}",
            std::process::id(),
            timestamp()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SlotA.save"), b"probe").unwrap();
        move_to_trash(&dir).unwrap();
        assert!(!dir.exists(), "移入回收站后原路径不应再存在");
    }

    #[test]
    fn is_within_collapses_dots_and_blocks_traversal() {
        let (root, inside, outside) = if cfg!(windows) {
            (
                r"C:\vault",
                r"C:\vault\sub\..\inner",
                r"C:\vault\..\Windows",
            )
        } else {
            ("/vault", "/vault/sub/../inner", "/vault/../etc")
        };

        // `..` 在范围内绕一圈仍然算范围内
        assert!(is_within(root, inside));
        // 但绝不能借 `..` 走出去
        assert!(!is_within(root, outside));
    }
}
