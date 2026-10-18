#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backups;
mod config;
mod error;
mod fsutil;
mod rollback;
mod sys;

use std::fs;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use backups::{BackupDetail, CreateResult, RestoreResult, WorkspaceScan};
use config::{AppConfig, GamePatch, GameSettings, Workspace, MAX_ROLLBACK_KEEP};
use error::AppError;
use fsutil::timestamp;
use rollback::{RollbackList, RollbackRestoreResult};

/// 进度事件名，与前端订阅名一致
const PROGRESS_EVENT: &str = "vault:progress";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupPhase {
    Copy,
    Registry,
    Snapshot,
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub phase: BackupPhase,
    pub done: u64,
    pub total: u64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    workspace_id: String,
    phase: BackupPhase,
    done: u64,
    total: u64,
    label: String,
}

impl ProgressPayload {
    fn new(workspace_id: &str, event: &ProgressEvent) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            phase: event.phase,
            done: event.done,
            total: event.total,
            label: event.label.clone(),
        }
    }
}

pub type ProgressFn = dyn Fn(&ProgressEvent);

/// 配置只有几十字节，读多写少；锁只在同步代码里短暂持有，不需要 async Mutex。
pub struct AppState {
    config: Mutex<AppConfig>,
}

impl AppState {
    fn new(config: AppConfig) -> Self {
        Self {
            config: Mutex::new(config),
        }
    }

    fn get(&self) -> AppConfig {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn game(&self) -> GameSettings {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .game
            .clone()
    }

    fn mutate(&self, edit: impl FnOnce(&mut AppConfig)) -> AppConfig {
        let mut guard = self.config.lock().unwrap_or_else(|e| e.into_inner());
        edit(&mut guard);
        guard.clone()
    }
}

// ---------------------------------------------------------------- 配置

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.get()
}

/// 更新游戏设置；rollbackKeep 超范围时钳制，避免写到非法份数。
#[tauri::command]
fn update_game(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: GamePatch,
) -> Result<AppConfig, AppError> {
    let next = state.mutate(|cfg| {
        let game = &mut cfg.game;
        if let Some(v) = patch.source_path {
            game.source_path = v.trim().to_string();
        }
        if let Some(v) = patch.registry_key {
            game.registry_key = v.trim().to_string();
        }
        if let Some(v) = patch.process_name {
            game.process_name = v.trim().to_string();
        }
        if let Some(v) = patch.data_dir_name {
            let v = v.trim().to_string();
            if !v.is_empty() {
                game.data_dir_name = v;
            }
        }
        if let Some(v) = patch.reg_file_name {
            let v = v.trim().to_string();
            if !v.is_empty() {
                game.reg_file_name = v;
            }
        }
        if let Some(v) = patch.rollback_keep {
            game.rollback_keep = v.min(MAX_ROLLBACK_KEEP);
        }
    });
    config::save(&app, &next).map_err(AppError::from)?;
    Ok(next)
}

/// 添加工作区的结果：区分「用户取消」「目录已存在」「新增成功」，
/// 避免前端把取消和重复添加都当成静默失败。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AddWorkspaceResult {
    config: AppConfig,
    added: Option<Workspace>,
    duplicate: bool,
}

/// 弹出目录选择框；取消时返回原配置并标记 added = None。
#[tauri::command]
async fn add_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AddWorkspaceResult, AppError> {
    let picked = app.dialog().file().blocking_pick_folder();
    let Some(folder) = picked else {
        return Ok(AddWorkspaceResult {
            config: state.get(),
            added: None,
            duplicate: false,
        });
    };
    let folder = folder
        .into_path()
        .map_err(|e| AppError(format!("无法解析所选目录：{e}")))?;
    let path = folder.to_string_lossy().into_owned();
    let mut duplicate = false;
    let mut added = None;
    let next = state.mutate(|cfg| {
        let same = cfg
            .workspaces
            .iter()
            .position(|w| fsutil::normalized_lower(&w.path) == fsutil::normalized_lower(&path));
        if same.is_some() {
            duplicate = true;
            return;
        }
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| path.clone());
        let ws = Workspace {
            // 时间戳在同一次点击里不会重复，够用且省掉 uuid 依赖
            id: format!("ws-{}-{}", timestamp(), cfg.workspaces.len()),
            name,
            path,
        };
        cfg.workspaces.push(ws.clone());
        added = Some(ws);
    });
    config::save(&app, &next).map_err(AppError::from)?;
    Ok(AddWorkspaceResult {
        config: next,
        added,
        duplicate,
    })
}

#[tauri::command]
fn remove_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<AppConfig, AppError> {
    let next = state.mutate(|cfg| cfg.workspaces.retain(|w| w.id != id));
    config::save(&app, &next).map_err(AppError::from)?;
    Ok(next)
}

// ---------------------------------------------------------------- 备份

fn workspace_of(state: &AppState, id: &str) -> Result<Workspace, AppError> {
    state
        .get()
        .workspaces
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| AppError::from("工作区不存在或已被移除"))
}

#[tauri::command]
fn list_backups(state: State<'_, AppState>, workspace_id: String) -> Result<WorkspaceScan, AppError> {
    let ws = workspace_of(&state, &workspace_id)?;
    backups::list_backups(&ws, &state.game())
}

#[tauri::command]
fn backup_detail(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
) -> Result<Option<BackupDetail>, AppError> {
    let ws = workspace_of(&state, &workspace_id)?;
    backups::backup_detail(&ws, &state.game(), &name)
}

#[tauri::command]
fn create_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
    note: String,
    overwrite: bool,
) -> Result<CreateResult, AppError> {
    let ws = workspace_of(&state, &workspace_id)?;
    let progress = progress_fn(&app, &workspace_id);
    backups::create_backup(&ws, &state.game(), &name, &note, overwrite, &progress)
}

#[tauri::command]
fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
    include_registry: bool,
) -> Result<RestoreResult, AppError> {
    let ws = workspace_of(&state, &workspace_id)?;
    let progress = progress_fn(&app, &workspace_id);
    backups::restore_backup(&ws, &state.game(), &name, include_registry, &progress)
}

#[tauri::command]
fn delete_backup(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
) -> Result<(), AppError> {
    let ws = workspace_of(&state, &workspace_id)?;
    backups::delete_backup(&ws, &name)
}

#[tauri::command]
fn rename_backup(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
    new_name: String,
) -> Result<(), AppError> {
    let ws = workspace_of(&state, &workspace_id)?;
    backups::rename_backup(&ws, &name, &new_name)
}

#[tauri::command]
fn set_note(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
    note: String,
) -> Result<(), AppError> {
    let ws = workspace_of(&state, &workspace_id)?;
    backups::set_note(&ws, &name, &note)
}

// ---------------------------------------------------------------- 回档点

#[tauri::command]
fn list_rollbacks(state: State<'_, AppState>) -> RollbackList {
    rollback::list(&state.game())
}

#[tauri::command]
fn restore_rollback(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<RollbackRestoreResult, AppError> {
    let game = state.game();
    let progress = progress_fn(&app, "");
    let registry = rollback::restore_point(&game, &id, &progress)?;
    Ok(RollbackRestoreResult {
        registry,
        points: rollback::list(&game).points,
    })
}

#[tauri::command]
fn remove_rollback(state: State<'_, AppState>, id: String) -> Result<RollbackList, AppError> {
    let game = state.game();
    rollback::remove_point(&game, &id)?;
    Ok(rollback::list(&game))
}

#[tauri::command]
fn clear_rollbacks(state: State<'_, AppState>) -> Result<RollbackList, AppError> {
    let game = state.game();
    rollback::clear_points(&game)?;
    Ok(rollback::list(&game))
}

// ---------------------------------------------------------------- 系统

#[derive(Debug, Clone, Copy, Serialize)]
struct GameStatus {
    running: bool,
}

/// 存档来源目录的只读健康状态，供「新建备份」预检与工作区概览展示。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceStatus {
    path: String,
    exists: bool,
    file_count: u64,
    total_size: u64,
    /// 目录内最近一次文件写入时间（毫秒），0 表示未知
    modified_at: f64,
}

#[tauri::command]
fn source_status(state: State<'_, AppState>) -> SourceStatus {
    let game = state.game();
    let path = fsutil::absolutize(&game.source_path);
    let exists = fsutil::path_exists(&path);
    let (file_count, total_size) = if exists {
        fsutil::walk_stats(&path)
    } else {
        (0, 0)
    };
    let modified_at = if exists && file_count > 0 {
        fsutil::newest_file_ms(&path)
    } else {
        0.0
    };
    SourceStatus {
        path: game.source_path,
        exists,
        file_count,
        total_size,
        modified_at,
    }
}

#[tauri::command]
fn game_status(state: State<'_, AppState>) -> GameStatus {
    GameStatus {
        running: sys::is_game_running(&state.game().process_name),
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
struct KillResult {
    killed: bool,
}

#[tauri::command]
fn kill_game(state: State<'_, AppState>) -> KillResult {
    KillResult {
        killed: sys::kill_game(&state.game().process_name),
    }
}

/// 选择目录（设置页「浏览…」）；取消返回 null。
#[tauri::command]
async fn pick_directory(app: AppHandle, title: Option<String>) -> Option<String> {
    let mut builder = app.dialog().file();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        builder = builder.set_title(title);
    }
    builder
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// 「在资源管理器中打开」允许的范围：工作区、存档来源目录、回档点目录（含它们的子目录）。
fn open_roots(cfg: &AppConfig) -> Vec<String> {
    let mut roots: Vec<String> = cfg.workspaces.iter().map(|w| w.path.clone()).collect();
    roots.push(cfg.game.source_path.clone());
    roots.push(
        rollback::store_path(&cfg.game)
            .to_string_lossy()
            .into_owned(),
    );
    roots
}

/// 在资源管理器中打开路径。
///
/// 前端不直接调用 opener 插件：插件命令受 ACL scope 约束，而工作区是用户随时添加的，
/// 没法在 capabilities 里预先声明；改在这里校验范围后再调用插件 API，权限边界才可控。
#[tauri::command]
fn open_in_explorer(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), AppError> {
    let cfg = state.get();
    if !open_roots(&cfg)
        .iter()
        .any(|root| fsutil::is_within(root, &path))
    {
        return Err(AppError::from("只能打开工作区、存档来源目录或回档点目录"));
    }

    let target = fsutil::absolutize(&path);
    if !fsutil::path_exists(&target) {
        // 回档点目录要等第一次恢复才生成，提前建出来让「打开目录」随时可用
        let store = rollback::store_path(&cfg.game);
        if fsutil::is_within(&store.to_string_lossy(), &path) {
            fs::create_dir_all(&store)?;
        } else {
            return Err(AppError(format!("目录不存在：{path}")));
        }
    }

    app.opener()
        .open_path(target.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| AppError(format!("无法打开：{e}")))
}

fn progress_fn(app: &AppHandle, workspace_id: &str) -> Box<ProgressFn> {
    let app = app.clone();
    let workspace_id = workspace_id.to_string();
    Box::new(move |event: &ProgressEvent| {
        let payload = ProgressPayload::new(&workspace_id, event);
        let _ = app.emit(PROGRESS_EVENT, &payload);
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例插件必须第一个注册：第二次启动时聚焦已有窗口而不是再开一个
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = config::load(app.handle());
            app.manage(AppState::new(config));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_game,
            add_workspace,
            remove_workspace,
            list_backups,
            backup_detail,
            create_backup,
            restore_backup,
            delete_backup,
            rename_backup,
            set_note,
            list_rollbacks,
            restore_rollback,
            remove_rollback,
            clear_rollbacks,
            source_status,
            game_status,
            kill_game,
            pick_directory,
            open_in_explorer,
        ])
        .run(tauri::generate_context!())
        .expect("启动 EtG Save Vault 失败");
}
