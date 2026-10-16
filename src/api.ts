import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  AppConfig,
  BackupDetail,
  BackupProgress,
  CreateResult,
  GameSettings,
  GameStatus,
  RestoreResult,
  RollbackList,
  RollbackRestoreResult,
  WorkspaceScan
} from './types'

/**
 * Tauri 的 invoke 与 Electron 的 ipcRenderer.invoke 语义一致：
 * 入参按名字解构，所以这里的参数名必须与 Rust 命令签名对齐。
 */
export const api = {
  getConfig: () => invoke<AppConfig>('get_config'),
  updateGame: (patch: Partial<GameSettings>) => invoke<AppConfig>('update_game', { patch }),
  addWorkspace: () => invoke<AppConfig>('add_workspace'),
  removeWorkspace: (id: string) => invoke<AppConfig>('remove_workspace', { id }),
  listBackups: (workspaceId: string) => invoke<WorkspaceScan>('list_backups', { workspaceId }),
  backupDetail: (workspaceId: string, name: string) =>
    invoke<BackupDetail | null>('backup_detail', { workspaceId, name }),
  createBackup: (workspaceId: string, name: string, note: string, overwrite: boolean) =>
    invoke<CreateResult>('create_backup', { workspaceId, name, note, overwrite }),
  restoreBackup: (workspaceId: string, name: string) =>
    invoke<RestoreResult>('restore_backup', { workspaceId, name }),
  deleteBackup: (workspaceId: string, name: string) =>
    invoke<void>('delete_backup', { workspaceId, name }),
  renameBackup: (workspaceId: string, name: string, newName: string) =>
    invoke<void>('rename_backup', { workspaceId, name, newName }),
  setNote: (workspaceId: string, name: string, note: string) =>
    invoke<void>('set_note', { workspaceId, name, note }),
  listRollbacks: () => invoke<RollbackList>('list_rollbacks'),
  restoreRollback: (id: string) => invoke<RollbackRestoreResult>('restore_rollback', { id }),
  removeRollback: (id: string) => invoke<RollbackList>('remove_rollback', { id }),
  clearRollbacks: () => invoke<RollbackList>('clear_rollbacks'),
  gameStatus: () => invoke<GameStatus>('game_status'),
  killGame: () => invoke<{ killed: boolean }>('kill_game'),
  pickDirectory: (title?: string) => invoke<string | null>('pick_directory', { title }),
  /** 在资源管理器中打开路径；允许范围由后端校验 */
  openInExplorer: (path: string) => invoke<void>('open_in_explorer', { path }),
  /** 返回取消订阅函数 */
  onProgress: (cb: (p: BackupProgress) => void) =>
    listen<BackupProgress>('vault:progress', (event) => cb(event.payload))
}

/** 剥掉 invoke 错误的前后缀，便于直接展示给用户 */
export function errMsg(e: unknown): string {
  const m = e instanceof Error ? e.message : String(e)
  return m
    .replace(/^Error invoking remote method '[^']+': (Error: )?/, '')
    .replace(/^error invoking command `[^`]+`: /i, '')
    .replace(/^Error: /, '')
}
