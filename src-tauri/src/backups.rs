use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::{GameSettings, Workspace};
use crate::error::AppError;
use crate::fsutil::{copy_dir, created_ms, modified_ms, path_exists, walk_stats};
use crate::rollback;
use crate::sys;
use crate::{BackupPhase, ProgressEvent, ProgressFn};

pub const SNAPSHOT_PREFIX: &str = "_snapshot_";

const META_FILE: &str = "vault-meta.json";
const ENTRY_LIMIT: usize = 500;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub name: String,
    pub path: String,
    pub workspace_id: String,
    pub created_at: f64,
    pub file_count: u64,
    pub total_size: u64,
    pub has_registry: bool,
    pub has_data: bool,
    pub is_snapshot: bool,
    pub note: String,
    /// 数据目录与注册表齐全，否则视为损坏
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceScan {
    pub workspace_id: String,
    /// 工作区目录已丢失
    pub missing: bool,
    pub backups: Vec<BackupSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDetail {
    #[serde(flatten)]
    pub summary: BackupSummary,
    /// 数据目录的顶层条目
    pub entries: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResult {
    pub has_registry: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistryOutcome {
    Ok,
    Missing,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    /// 恢复前捕获的回档点，null 表示未捕获
    pub rollback: Option<rollback::CaptureResult>,
    pub registry: RegistryOutcome,
}

#[derive(Debug, Default, Deserialize)]
struct BackupMeta {
    #[serde(rename = "createdAt")]
    created_at: Option<f64>,
    #[serde(default)]
    note: Option<String>,
}

/// 备份名称校验，返回给用户看的中文错误。
pub fn validate_backup_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::from("备份名称不能为空"));
    }
    if name != name.trim() {
        return Err(AppError::from("名称首尾不能有空格"));
    }
    if name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(AppError::from("名称不能包含 \\ / : * ? \" < > | 字符"));
    }
    if name == "." || name == ".." {
        return Err(AppError::from("非法的备份名称"));
    }
    if name.chars().count() > 100 {
        return Err(AppError::from("备份名称过长"));
    }
    Ok(())
}

/// 解析备份目录绝对路径，并确保其位于工作区根目录下一层。
pub fn backup_path(ws: &Workspace, name: &str) -> Result<PathBuf, AppError> {
    validate_backup_name(name)?;
    let root = crate::fsutil::absolutize(&ws.path);
    let full = root.join(name);
    if full.parent() != Some(root.as_path()) {
        return Err(AppError::from("非法的备份名称"));
    }
    Ok(full)
}

fn read_meta(dir: &Path) -> BackupMeta {
    fs::read_to_string(dir.join(META_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_meta(dir: &Path, created_at: f64, note: &str) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(&serde_json::json!({
        "createdAt": created_at,
        "note": note,
    }))?;
    fs::write(dir.join(META_FILE), text)?;
    Ok(())
}

/// 扫描工作区内的所有备份目录。
///
/// 这是启动路径上最重的一步：每个备份都要递归统计文件数与总字节数（目录元数据遍历）。
/// 备份之间互不依赖，因此按备份目录并行；单核机器上 rayon 会退化为串行。
pub fn list_backups(ws: &Workspace, game: &GameSettings) -> Result<WorkspaceScan, AppError> {
    let root = crate::fsutil::absolutize(&ws.path);
    if !path_exists(&root) {
        return Ok(WorkspaceScan {
            workspace_id: ws.id.clone(),
            missing: true,
            backups: Vec::new(),
        });
    }

    let dirs: Vec<(String, PathBuf)> = fs::read_dir(&root)?
        .flatten()
        .filter(|entry| entry.file_type().map(|k| k.is_dir()).unwrap_or(false))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            )
        })
        .collect();

    let mut backups: Vec<BackupSummary> = dirs
        .into_par_iter()
        .map(|(name, base)| {
            let data_dir = base.join(&game.data_dir_name);
            let has_data = path_exists(&data_dir);
            let has_registry = path_exists(&base.join(&game.reg_file_name));
            let meta = read_meta(&base);
            let created_at = meta.created_at.or_else(|| {
                fs::metadata(&base)
                    .ok()
                    .map(|m| created_ms(&m))
                    .or_else(|| fs::metadata(&base).ok().map(|m| modified_ms(&m)))
            });
            let (file_count, total_size) = if has_data {
                walk_stats(&data_dir)
            } else {
                (0, 0)
            };
            BackupSummary {
                is_snapshot: name.starts_with(SNAPSHOT_PREFIX),
                valid: has_data && has_registry,
                name,
                path: base.to_string_lossy().into_owned(),
                workspace_id: ws.id.clone(),
                created_at: created_at.unwrap_or_default(),
                file_count,
                total_size,
                has_registry,
                has_data,
                note: meta.note.unwrap_or_default(),
            }
        })
        .collect();

    backups.sort_by(|a, b| {
        b.created_at
            .partial_cmp(&a.created_at)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(WorkspaceScan {
        workspace_id: ws.id.clone(),
        missing: false,
        backups,
    })
}

/// 备份详情：概要信息 + 数据目录顶层条目。
pub fn backup_detail(
    ws: &Workspace,
    game: &GameSettings,
    name: &str,
) -> Result<Option<BackupDetail>, AppError> {
    let scan = list_backups(ws, game)?;
    let Some(summary) = scan.backups.into_iter().find(|b| b.name == name) else {
        return Ok(None);
    };

    let mut entries: Vec<FileEntry> = Vec::new();
    let data_dir = Path::new(&summary.path).join(&game.data_dir_name);
    if let Ok(list) = fs::read_dir(&data_dir) {
        for entry in list.flatten().take(ENTRY_LIMIT) {
            let Ok(kind) = entry.file_type() else { continue };
            let Ok(meta) = entry.metadata() else { continue };
            entries.push(FileEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir: kind.is_dir(),
                size: if kind.is_dir() { 0 } else { meta.len() },
                mtime: modified_ms(&meta),
            });
        }
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(Some(BackupDetail { summary, entries }))
}

/// 新建备份：复制存档来源目录 + 导出注册表 + 写 meta。
pub fn create_backup(
    ws: &Workspace,
    game: &GameSettings,
    name: &str,
    note: &str,
    overwrite: bool,
    progress: &ProgressFn,
) -> Result<CreateResult, AppError> {
    let target = backup_path(ws, name)?;
    if path_exists(&target) {
        if !overwrite {
            // 前端据此文案触发「覆盖确认」
            return Err(AppError(format!("备份「{name}」已存在")));
        }
        fs::remove_dir_all(&target)?;
    }
    let source = crate::fsutil::absolutize(&game.source_path);
    if !path_exists(&source) {
        return Err(AppError(format!(
            "找不到存档来源目录：{}",
            game.source_path
        )));
    }

    let (count, _) = walk_stats(&source);
    if count == 0 {
        return Err(AppError(format!(
            "来源目录中没有可备份的存档文件：{}",
            game.source_path
        )));
    }
    let mut done = 0u64;
    progress(&ProgressEvent {
        phase: BackupPhase::Copy,
        done,
        total: count,
        label: "复制存档文件".into(),
    });
    copy_dir(&source, &target.join(&game.data_dir_name), &mut || {
        done += 1;
        if done % 5 == 0 || done == count {
            progress(&ProgressEvent {
                phase: BackupPhase::Copy,
                done,
                total: count,
                label: "复制存档文件".into(),
            });
        }
    })?;

    progress(&ProgressEvent {
        phase: BackupPhase::Registry,
        done,
        total: count,
        label: "导出注册表".into(),
    });
    let has_registry = sys::export_registry(
        &game.registry_key,
        &target.join(&game.reg_file_name),
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or_default();
    write_meta(&target, now, note)?;
    progress(&ProgressEvent {
        phase: BackupPhase::Done,
        done: count,
        total: count,
        label: "完成".into(),
    });
    Ok(CreateResult { has_registry })
}

/// 恢复备份：先捕获回档点，再覆盖存档来源目录，最后导入注册表。
pub fn restore_backup(
    ws: &Workspace,
    game: &GameSettings,
    name: &str,
    progress: &ProgressFn,
) -> Result<RestoreResult, AppError> {
    let base = backup_path(ws, name)?;
    let data_src = base.join(&game.data_dir_name);
    if !path_exists(&data_src) {
        return Err(AppError::from("该备份缺少存档数据目录，无法恢复"));
    }
    if !path_exists(&base.join(&game.reg_file_name)) {
        return Err(AppError::from("该备份缺少注册表文件，已损坏，无法恢复"));
    }

    progress(&ProgressEvent {
        phase: BackupPhase::Snapshot,
        done: 0,
        total: 0,
        label: "保存回档点".into(),
    });
    let captured = rollback::capture(game, name, progress)?;

    let source = crate::fsutil::absolutize(&game.source_path);
    let (count, _) = walk_stats(&data_src);
    let mut done = 0u64;
    progress(&ProgressEvent {
        phase: BackupPhase::Copy,
        done,
        total: count,
        label: "恢复存档文件".into(),
    });

    // 先移走旧目录再复制：失败时旧数据仍在回档点里
    if path_exists(&source) {
        fs::remove_dir_all(&source)?;
    }
    let copy_result = copy_dir(&data_src, &source, &mut || {
        done += 1;
        if done % 5 == 0 || done == count {
            progress(&ProgressEvent {
                phase: BackupPhase::Copy,
                done,
                total: count,
                label: "恢复存档文件".into(),
            });
        }
    });
    if let Err(err) = copy_result {
        let hint = if captured.is_some() {
            "，原存档已保存在回档点，可在「回档点」中还原"
        } else {
            ""
        };
        return Err(AppError(format!("恢复存档文件失败：{err}{hint}")));
    }

    let registry = if sys::import_registry(&base.join(&game.reg_file_name)) {
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
    Ok(RestoreResult {
        rollback: captured,
        registry,
    })
}

pub fn delete_backup(ws: &Workspace, name: &str) -> Result<(), AppError> {
    let path = backup_path(ws, name)?;
    if path_exists(&path) {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn rename_backup(ws: &Workspace, name: &str, new_name: &str) -> Result<(), AppError> {
    let from = backup_path(ws, name)?;
    let to = backup_path(ws, new_name)?;
    if from == to {
        return Ok(());
    }
    if path_exists(&to) {
        return Err(AppError(format!("备份「{new_name}」已存在")));
    }
    fs::rename(from, to)?;
    Ok(())
}

pub fn set_note(ws: &Workspace, name: &str, note: &str) -> Result<(), AppError> {
    let dir = backup_path(ws, name)?;
    let meta = read_meta(&dir);
    let created_at = meta.created_at.unwrap_or_else(|| {
        fs::metadata(&dir)
            .map(|m| created_ms(&m))
            .unwrap_or_default()
    });
    write_meta(&dir, created_at, note)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Workspace;
    use std::time::Instant;

    fn workspace(path: &std::path::Path) -> Workspace {
        Workspace {
            id: "ws-test".into(),
            name: "ws-test".into(),
            path: path.to_string_lossy().into_owned(),
        }
    }

    #[test]
    fn name_validation_matches_original_rules() {
        assert!(validate_backup_name("20260101_120000").is_ok());
        assert!(validate_backup_name("").is_err());
        assert!(validate_backup_name("  pad  ").is_err());
        assert!(validate_backup_name("a/b").is_err());
        assert!(validate_backup_name("..").is_err());
        assert!(validate_backup_name(&"x".repeat(101)).is_err());
    }

    #[test]
    fn backup_path_stays_one_level_below_root() {
        let ws = workspace(std::path::Path::new(r"C:\vault"));
        assert_eq!(
            backup_path(&ws, "slot").unwrap(),
            std::path::PathBuf::from(r"C:\vault\slot")
        );
        // 名称校验先拦住穿越写法，拿不到上一层路径
        assert!(backup_path(&ws, "..").is_err());
        assert!(backup_path(&ws, r"..\other").is_err());
    }

    /// 需要真实工作区：BENCH_WS=<dir> cargo test --release -- --ignored --nocapture
    #[test]
    #[ignore = "需要 BENCH_WS 环境变量指向一个真实工作区"]
    fn bench_scan_serial_vs_parallel() {
        let Ok(root) = std::env::var("BENCH_WS") else {
            panic!("set BENCH_WS");
        };
        let root = std::path::PathBuf::from(root);
        let game = GameSettings::default();
        let dirs: Vec<std::path::PathBuf> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .filter(|e| e.file_type().map(|k| k.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        println!("备份目录数 = {}", dirs.len());

        let start = Instant::now();
        let serial: Vec<(u64, u64)> = dirs
            .iter()
            .map(|base| walk_stats(&base.join(&game.data_dir_name)))
            .collect();
        let serial_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        let parallel: Vec<(u64, u64)> = dirs
            .par_iter()
            .map(|base| walk_stats(&base.join(&game.data_dir_name)))
            .collect();
        let parallel_ms = start.elapsed().as_secs_f64() * 1000.0;

        assert_eq!(
            serial.iter().map(|s| s.0).sum::<u64>(),
            parallel.iter().map(|s| s.0).sum::<u64>()
        );
        println!(
            "串行 {serial_ms:.1} ms / 并行 {parallel_ms:.1} ms / 加速 {:.2}x (rayon 线程 {})",
            serial_ms / parallel_ms,
            rayon::current_num_threads()
        );
    }
}
