use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backups::RegistryOutcome;
use crate::config::GameSettings;
use crate::error::AppError;
use crate::fsutil::{
    copy_dir, created_ms, modified_ms, move_to_trash, normalized_lower, path_exists, point_stamp,
    walk_stats,
};
use crate::sys;
use crate::{BackupPhase, ProgressEvent, ProgressFn};

/// 回档点：恢复备份前自动保存的当前存档。
/// 放在应用数据目录而非工作区，以便与来源目录同卷、用 rename 零拷贝捕获。
const META_FILE: &str = "rollback-meta.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollbackMode {
    /// 同卷 rename（零拷贝）
    Move,
    /// 跨卷复制
    Copy,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub id: String,
    pub mode: RollbackMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackPoint {
    pub id: String,
    pub path: String,
    pub created_at: f64,
    pub note: String,
    /// 触发该回档点的备份
    pub source_backup: Option<String>,
    pub mode: RollbackMode,
    pub file_count: u64,
    pub total_size: u64,
    pub has_data: bool,
    pub has_registry: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackList {
    /// 回档点存放目录
    pub store_path: String,
    /// 保留份数，0 表示已关闭
    pub keep: u32,
    pub points: Vec<RollbackPoint>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackRestoreResult {
    pub registry: RegistryOutcome,
    pub points: Vec<RollbackPoint>,
}

#[derive(Debug, Default, Deserialize)]
struct PointMeta {
    #[serde(rename = "createdAt", default)]
    created_at: Option<f64>,
    #[serde(default)]
    note: Option<String>,
    #[serde(rename = "sourceBackup", default)]
    source_backup: Option<String>,
    #[serde(default)]
    mode: Option<RollbackMode>,
}

/// 回档点根目录：优先 %LOCALAPPDATA%（与存档同在 C: 卷，可零拷贝捕获）。
fn store_base() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if !local.trim().is_empty() {
            return PathBuf::from(local).join("EtG Save Vault");
        }
    }
    std::env::temp_dir().join("EtG Save Vault")
}

/// 按存档来源目录归属：恢复改写的始终是来源目录，多个工作区可能指向同一存档。
pub fn store_path(game: &GameSettings) -> PathBuf {
    let raw = if game.source_path.trim().is_empty() {
        "(unset)".to_string()
    } else {
        game.source_path.clone()
    };
    let key = short_hash(&normalized_lower(&raw));
    store_base().join("rollback").join(key)
}

/// 来源路径 -> 12 位十六进制摘要（FNV-1a 64 位，两次不同偏移拼接）。
fn short_hash(text: &str) -> String {
    fn fnv(text: &str, offset: u64) -> u64 {
        let mut hash = offset;
        for byte in text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
    let a = fnv(text, 0xcbf2_9ce4_8422_2325);
    let b = fnv(text, 0x9e37_79b9_7f4a_7c15);
    format!("{a:016x}{b:016x}")[..12].to_string()
}

/// 回档点目录名：yyyyMMdd_HHmmss_SSS
fn valid_point_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 19 {
        return false;
    }
    for (i, b) in bytes.iter().enumerate() {
        let expect_sep = i == 8 || i == 15;
        if expect_sep {
            if *b != b'_' {
                return false;
            }
        } else if !b.is_ascii_digit() {
            return false;
        }
    }
    true
}

fn resolve_point(dir: &Path, id: &str) -> Result<PathBuf, AppError> {
    if !valid_point_id(id) {
        return Err(AppError::from("非法的回档点标识"));
    }
    Ok(dir.join(id))
}

fn read_meta(dir: &Path) -> PointMeta {
    fs::read_to_string(dir.join(META_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn list_points(game: &GameSettings) -> Vec<RollbackPoint> {
    let dir = store_path(game);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut points = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|k| k.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let meta = read_meta(&path);
        let fallback = entry
            .metadata()
            .map(|m| {
                let created = created_ms(&m);
                if created > 0.0 {
                    created
                } else {
                    modified_ms(&m)
                }
            })
            .unwrap_or_default();
        let has_data = path_exists(&path.join(&game.data_dir_name));
        let has_registry = path_exists(&path.join(&game.reg_file_name));
        let (file_count, total_size) = if has_data {
            walk_stats(&path.join(&game.data_dir_name))
        } else {
            (0, 0)
        };
        points.push(RollbackPoint {
            id: entry.file_name().to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            created_at: meta.created_at.unwrap_or(fallback),
            note: meta.note.unwrap_or_default(),
            source_backup: meta.source_backup,
            mode: meta.mode.unwrap_or(RollbackMode::Copy),
            file_count,
            total_size,
            has_data,
            has_registry,
        });
    }
    points.sort_by(|a, b| {
        b.created_at
            .partial_cmp(&a.created_at)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.id.cmp(&a.id))
    });
    points
}

pub fn list(game: &GameSettings) -> RollbackList {
    RollbackList {
        store_path: store_path(game).to_string_lossy().into_owned(),
        keep: game.rollback_keep,
        points: list_points(game),
    }
}

/// 只保留最新的 keep 份，其余按时间从旧到新删除。
///
/// 这里是保留策略触发的自动裁剪，仍走永久删除：
/// 若也放进回收站，每次恢复都会往回收站丢一份完整存档，长期会占满磁盘。
fn prune(dir: &Path, keep: u32) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<(String, f64)> = entries
        .flatten()
        .filter(|e| e.file_type().map(|k| k.is_dir()).unwrap_or(false))
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let created = read_meta(&e.path()).created_at.unwrap_or_default();
            (name, created)
        })
        .collect();
    items.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.0.cmp(&a.0))
    });
    for (name, _) in items.into_iter().skip(keep as usize) {
        let _ = fs::remove_dir_all(dir.join(name));
    }
}

/// 捕获当前存档为回档点：同卷 rename（零拷贝），跨卷退化为复制。
/// move 会消耗源目录，copy 保留源目录由调用方删除；返回 None 表示未捕获。
pub fn capture(
    game: &GameSettings,
    source_backup: &str,
    progress: &ProgressFn,
) -> Result<Option<CaptureResult>, AppError> {
    let keep = game.rollback_keep;
    if keep == 0 {
        return Ok(None);
    }
    let source = crate::fsutil::absolutize(&game.source_path);
    if !path_exists(&source) {
        return Ok(None);
    }

    let dir = store_path(game);
    fs::create_dir_all(&dir)?;
    let id = point_stamp();
    let target = dir.join(&id);
    let data_dest = target.join(&game.data_dir_name);
    fs::create_dir_all(&target)?;

    let mut mode = RollbackMode::Move;
    if fs::rename(&source, &data_dest).is_err() {
        // 跨卷或目录被占用：退化为复制
        mode = RollbackMode::Copy;
        progress(&ProgressEvent {
            phase: BackupPhase::Snapshot,
            done: 0,
            total: 0,
            label: "复制回档点".into(),
        });
        copy_dir(&source, &data_dest, &mut || {})?;
    }

    // 注册表不能随目录搬迁，单独导出到回档点内
    sys::export_registry(&game.registry_key, &target.join(&game.reg_file_name));

    let meta = serde_json::json!({
        "createdAt": now_ms(),
        "note": format!("恢复「{source_backup}」前的自动回档点"),
        "sourceBackup": source_backup,
        "mode": mode,
        "sourcePath": game.source_path,
    });
    fs::write(target.join(META_FILE), serde_json::to_string_pretty(&meta)?)?;

    prune(&dir, keep);
    Ok(Some(CaptureResult { id, mode }))
}

/// 用回档点覆盖当前存档（「撤销本次恢复」的底层实现）。
pub fn restore_point(
    game: &GameSettings,
    id: &str,
    progress: &ProgressFn,
) -> Result<RegistryOutcome, AppError> {
    let dir = store_path(game);
    let src = resolve_point(&dir, id)?;
    let data_src = src.join(&game.data_dir_name);
    if !path_exists(&data_src) {
        return Err(AppError::from("该回档点缺少存档数据目录，无法还原"));
    }

    let source = crate::fsutil::absolutize(&game.source_path);
    let (count, _) = walk_stats(&data_src);
    let mut done = 0u64;
    progress(&ProgressEvent {
        phase: BackupPhase::Copy,
        done,
        total: count,
        label: "还原回档点".into(),
    });
    if path_exists(&source) {
        fs::remove_dir_all(&source)?;
    }
    copy_dir(&data_src, &source, &mut || {
        done += 1;
        if done.is_multiple_of(5) || done == count {
            progress(&ProgressEvent {
                phase: BackupPhase::Copy,
                done,
                total: count,
                label: "还原回档点".into(),
            });
        }
    })?;

    let reg_file = src.join(&game.reg_file_name);
    let outcome = if !path_exists(&reg_file) {
        RegistryOutcome::Missing
    } else if sys::import_registry(&reg_file) {
        RegistryOutcome::Ok
    } else {
        RegistryOutcome::Failed
    };
    progress(&ProgressEvent {
        phase: BackupPhase::Done,
        done: count,
        total: count,
        label: "完成".into(),
    });
    Ok(outcome)
}

/// 删除单份回档点：移入系统回收站，用户误删后仍可还原。
pub fn remove_point(game: &GameSettings, id: &str) -> Result<(), AppError> {
    let dir = store_path(game);
    let path = resolve_point(&dir, id)?;
    if path_exists(&path) {
        move_to_trash(&path)?;
    }
    Ok(())
}

/// 清空回档点：整份回档点目录移入回收站，避免「清空」变成永久删除。
pub fn clear_points(game: &GameSettings) -> Result<(), AppError> {
    let dir = store_path(game);
    if path_exists(&dir) {
        move_to_trash(&dir)?;
    }
    Ok(())
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_short() {
        let a = short_hash(&normalized_lower(r"C:\Users\Me\AppData\LocalLow\Dodge Roll\Enter the Gungeon"));
        assert_eq!(a.len(), 12);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        let b = short_hash(&normalized_lower(r"c:/users/me/appdata/locallow/dodge roll/enter the gungeon/"));
        assert_eq!(a, b);
    }

    #[test]
    fn point_id_validation() {
        assert!(valid_point_id("20260101_120000_123"));
        assert!(!valid_point_id("20260101_120000"));
        assert!(!valid_point_id("../escape"));
        assert!(!valid_point_id("20260101_120000_12x"));
    }

    #[test]
    fn timestamps_are_sortable() {
        let stamp = point_stamp();
        assert!(valid_point_id(&stamp), "{stamp}");
    }
}
