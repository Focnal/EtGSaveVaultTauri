/** 与 src-tauri 中 serde 序列化结构一一对应（后端统一 camelCase） */

export interface Workspace {
  id: string
  name: string
  path: string
}

export interface GameSettings {
  /** 游戏存档目录（备份来源） */
  sourcePath: string
  /** 随存档导出的注册表键 */
  registryKey: string
  /** 恢复前检测用的游戏进程名 */
  processName: string
  /** 备份内数据目录名 */
  dataDirName: string
  /** 备份内注册表文件名 */
  regFileName: string
  /** 回档点保留份数，0 表示关闭 */
  rollbackKeep: number
}

export interface AppConfig {
  workspaces: Workspace[]
  game: GameSettings
}

export interface BackupSummary {
  name: string
  path: string
  workspaceId: string
  createdAt: number
  fileCount: number
  totalSize: number
  hasRegistry: boolean
  hasData: boolean
  isSnapshot: boolean
  note: string
  /** 数据目录与注册表齐全，否则视为损坏 */
  valid: boolean
}

export interface WorkspaceScan {
  workspaceId: string
  /** 工作区目录已丢失 */
  missing: boolean
  backups: BackupSummary[]
}

export interface FileEntry {
  name: string
  isDir: boolean
  size: number
  mtime: number
}

export interface BackupDetail extends BackupSummary {
  /** 数据目录的顶层条目 */
  entries: FileEntry[]
}

export type BackupPhase = 'copy' | 'registry' | 'snapshot' | 'done'

/** move = 同卷 rename（零拷贝）；copy = 跨卷复制 */
export type RollbackMode = 'move' | 'copy'

/** 恢复备份前自动捕获的当前存档，存放在应用数据目录而非工作区 */
export interface RollbackPoint {
  id: string
  path: string
  createdAt: number
  note: string
  /** 触发该回档点的备份 */
  sourceBackup: string | null
  mode: RollbackMode
  fileCount: number
  totalSize: number
  hasData: boolean
  hasRegistry: boolean
}

export interface RollbackList {
  /** 回档点存放目录 */
  storePath: string
  /** 保留份数，0 表示已关闭 */
  keep: number
  points: RollbackPoint[]
}

export interface RollbackRestoreResult {
  registry: RegistryOutcome
  points: RollbackPoint[]
}

export type RegistryOutcome = 'ok' | 'missing' | 'failed'

export interface BackupProgress {
  workspaceId: string
  phase: BackupPhase
  done: number
  total: number
  label: string
}

export interface GameStatus {
  running: boolean
}

export interface RestoreResult {
  /** 恢复前捕获的回档点，null 表示未捕获 */
  rollback: { id: string; mode: RollbackMode } | null
  registry: RegistryOutcome
}

export interface CreateResult {
  hasRegistry: boolean
}
