use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 访问出错时视为不存在。
pub fn path_exists(path: &Path) -> bool {
    path.exists()
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

/// 小写归一化后的路径字符串，用于比较与哈希。
///
/// Windows 上 `/` 与 `\` 等价，统一成 `\`，否则同一目录会因为写法不同被当成两个工作区。
pub fn normalized_lower(path: &str) -> String {
    let mut text = absolutize(path).to_string_lossy().into_owned();
    if cfg!(windows) {
        text = text.replace('/', "\\");
    }
    while text.ends_with('\\') || text.ends_with('/') {
        text.pop();
    }
    text.to_lowercase()
}
