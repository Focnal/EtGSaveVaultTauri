import { reactive, type Component } from 'vue'
import { api, errMsg } from './api'
import { formatSize, formatTime } from './format'
import type {
  AppConfig,
  BackupDetail,
  BackupProgress,
  BackupSummary,
  GameSettings,
  RegistryOutcome,
  RollbackList,
  RollbackPoint,
  SourceStatus,
  WorkspaceScan
} from './types'

export type BackupSort = 'newest' | 'oldest' | 'name' | 'size'

export type ModalState =
  | { kind: 'none' }
  | { kind: 'settings' }
  | { kind: 'rollback' }
  | { kind: 'newBackup'; workspaceId: string }
  | { kind: 'rename'; workspaceId: string; name: string }
  | {
      kind: 'confirm'
      title: string
      message: string
      confirmText: string
      danger: boolean
      onConfirm: () => void | Promise<void>
    }

export interface ToastAction {
  label: string
  run: () => void | Promise<void>
}

export interface Toast {
  id: number
  kind: 'ok' | 'err' | 'info'
  text: string
  action?: ToastAction
}

export interface MenuItem {
  label: string
  icon?: Component
  danger?: boolean
  disabled?: boolean
  /** 禁用原因或补充说明，作为 tooltip 展示 */
  hint?: string
  run: () => void | Promise<void>
}

export interface MenuDivider {
  divider: true
}

export type MenuEntry = MenuItem | MenuDivider

export interface MenuState {
  x: number
  y: number
  entries: MenuEntry[]
}

export const state = reactive({
  config: null as AppConfig | null,
  bootError: null as string | null,
  scans: {} as Record<string, WorkspaceScan>,
  scanning: {} as Record<string, boolean>,
  sourceStatus: null as SourceStatus | null,
  sourceStatusLoading: false,
  expanded: {} as Record<string, boolean>,
  filter: '',
  backupSort: 'newest' as BackupSort,
  selectedWorkspaceId: null as string | null,
  selectedBackupName: null as string | null,
  detail: null as BackupDetail | null,
  detailLoading: false,
  progress: null as BackupProgress | null,
  gameRunning: null as boolean | null,
  gameStatusLoading: false,
  rollbacks: null as RollbackList | null,
  menu: null as MenuState | null,
  modal: { kind: 'none' } as ModalState,
  toasts: [] as Toast[]
})

let toastId = 0
let unlistenProgress: (() => void) | null = null
let detailSeq = 0
let lastScanAt = 0
const scanSeq: Record<string, number> = {}

/** 备份搜索：匹配名称、备注和槽位。 */
export function visibleBackups(backups: BackupSummary[]): BackupSummary[] {
  const q = state.filter.trim().toLocaleLowerCase()
  const matched = q
    ? backups.filter((b) =>
        `${b.name}\n${b.note}\n${b.slots.join(' ')}`.toLocaleLowerCase().includes(q)
      )
    : backups.slice()
  switch (state.backupSort) {
    case 'oldest':
      return matched.sort((a, b) => a.createdAt - b.createdAt)
    case 'name':
      return matched.sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'))
    case 'size':
      return matched.sort((a, b) => b.totalSize - a.totalSize)
    default:
      return matched.sort((a, b) => b.createdAt - a.createdAt)
  }
}

export function matchCount(): number {
  let total = 0
  for (const scan of Object.values(state.scans)) total += visibleBackups(scan.backups).length
  return total
}

/** 复制文本；优先 Clipboard API，失败后退回临时 textarea。 */
export async function copyText(text: string, label = '路径'): Promise<void> {
  try {
    await navigator.clipboard.writeText(text)
  } catch {
    try {
      const area = document.createElement('textarea')
      area.value = text
      area.style.position = 'fixed'
      area.style.opacity = '0'
      document.body.appendChild(area)
      area.select()
      document.execCommand('copy')
      area.remove()
    } catch {
      toast('err', '复制失败，请手动选择文本复制')
      return
    }
  }
  toast('ok', `${label}已复制`)
}

/** 在资源管理器中打开路径；后端只放行工作区、存档来源与回档点目录 */
export async function openInExplorer(path: string): Promise<void> {
  try {
    await api.openInExplorer(path)
  } catch (e) {
    toast('err', errMsg(e))
  }
}

/** 带操作按钮的提示保留更久，方便点击 */
export function toast(kind: Toast['kind'], text: string, action?: ToastAction): void {
  const id = ++toastId
  state.toasts.push({ id, kind, text, action })
  // 限制同时展示的条数，避免连续失败时占满窗口；更早的提示仍保留在控制台/用户已看到
  if (state.toasts.length > 4) state.toasts.splice(0, state.toasts.length - 4)
  setTimeout(
    () => {
      state.toasts = state.toasts.filter((t) => t.id !== id)
    },
    action ? 15000 : kind === 'err' ? 8000 : 4200
  )
}

export function dismissToast(id: number): void {
  state.toasts = state.toasts.filter((t) => t.id !== id)
}

export async function runToastAction(t: Toast): Promise<void> {
  dismissToast(t.id)
  await t.action?.run()
}

export function openMenu(e: MouseEvent, entries: MenuEntry[]): void {
  state.menu = { x: e.clientX, y: e.clientY, entries }
}

export function closeMenu(): void {
  state.menu = null
}

/** 先收起菜单再执行动作 */
export async function runMenuEntry(entry: MenuEntry): Promise<void> {
  if ('divider' in entry) return
  closeMenu()
  try {
    await entry.run()
  } catch (e) {
    toast('err', errMsg(e))
  }
}

export async function init(): Promise<void> {
  state.bootError = null
  try {
    state.config = await api.getConfig()
    if (!unlistenProgress) {
      unlistenProgress = await api.onProgress((p) => {
        state.progress = p.phase === 'done' ? null : p
      })
    }
    for (const ws of state.config.workspaces) state.expanded[ws.id] = true
    // 先让界面可用，再把扫描放到后台：备份很多时不必等全部统计完才显示窗口
    const first = state.config.workspaces[0]
    if (first) selectWorkspace(first.id)
    void scanAll()
    void refreshGameStatus()
    void refreshSourceStatus()
    void loadRollbacks()
    startGameStatusWatcher()
  } catch (e) {
    state.bootError = errMsg(e)
  }
}

export async function scanAll(): Promise<void> {
  if (!state.config) return
  lastScanAt = Date.now()
  const ids = new Set(state.config.workspaces.map((w) => w.id))
  for (const key of Object.keys(state.scans)) {
    if (!ids.has(key)) delete state.scans[key]
  }
  for (const key of Object.keys(state.scanning)) {
    if (!ids.has(key)) delete state.scanning[key]
  }
  await Promise.all(state.config.workspaces.map((ws) => scanWorkspace(ws.id)))
}

export async function scanWorkspace(id: string): Promise<void> {
  const seq = (scanSeq[id] ?? 0) + 1
  scanSeq[id] = seq
  state.scanning[id] = true
  try {
    const scan = await api.listBackups(id)
    // 晚到的旧扫描结果不能覆盖新结果
    if (seq !== scanSeq[id]) return
    state.scans[id] = scan
  } catch (e) {
    if (seq === scanSeq[id]) toast('err', errMsg(e))
    return
  } finally {
    if (seq === scanSeq[id]) state.scanning[id] = false
  }
  if (state.selectedWorkspaceId === id && state.selectedBackupName) {
    await refreshDetail()
  }
}

/** expand=false 时只移动高亮，不改变展开状态（侧栏右键用） */
export function selectWorkspace(id: string, expand = true): void {
  detailSeq += 1
  state.selectedWorkspaceId = id
  state.selectedBackupName = null
  state.detail = null
  state.detailLoading = false
  if (expand) state.expanded[id] = true
}

export async function selectBackup(workspaceId: string, name: string): Promise<void> {
  state.selectedWorkspaceId = workspaceId
  state.selectedBackupName = name
  state.expanded[workspaceId] = true
  // 先清掉上一份详情，避免加载期间显示成新选中的备份
  state.detail = null
  state.detailLoading = true
  await refreshDetail()
}

export async function refreshDetail(): Promise<void> {
  const workspaceId = state.selectedWorkspaceId
  const name = state.selectedBackupName
  if (!workspaceId || !name) {
    state.detail = null
    state.detailLoading = false
    return
  }
  const seq = ++detailSeq
  state.detailLoading = true
  try {
    const detail = await api.backupDetail(workspaceId, name)
    if (seq !== detailSeq) return
    if (state.selectedWorkspaceId !== workspaceId || state.selectedBackupName !== name) return
    state.detail = detail
    if (!detail) state.selectedBackupName = null
  } catch (e) {
    if (seq === detailSeq) toast('err', errMsg(e))
  } finally {
    if (seq === detailSeq) state.detailLoading = false
  }
}

export async function addWorkspace(): Promise<void> {
  try {
    const result = await api.addWorkspace()
    state.config = result.config
    if (result.added) {
      state.expanded[result.added.id] = true
      await scanWorkspace(result.added.id)
      selectWorkspace(result.added.id)
      toast('ok', `已添加工作区「${result.added.name}」`)
    } else if (result.duplicate) {
      toast('info', '该目录已经在工作区列表中')
    }
  } catch (e) {
    toast('err', errMsg(e))
  }
}

export function requestRemoveWorkspace(id: string, name: string): void {
  state.modal = {
    kind: 'confirm',
    title: '移除工作区',
    message: `将「${name}」从列表中移除。目录里的备份文件不会被删除。`,
    confirmText: '移除',
    danger: false,
    onConfirm: async () => {
      state.config = await api.removeWorkspace(id)
      delete state.scans[id]
      delete state.scanning[id]
      delete scanSeq[id]
      if (state.selectedWorkspaceId === id) {
        detailSeq += 1
        state.selectedWorkspaceId = null
        state.selectedBackupName = null
        state.detail = null
        state.detailLoading = false
      }
      toast('ok', '工作区已移除')
    }
  }
}

export function openNewBackup(workspaceId: string): void {
  state.modal = { kind: 'newBackup', workspaceId }
}

export function openRename(workspaceId: string, name: string): void {
  state.modal = { kind: 'rename', workspaceId, name }
}

/** 命令失败时后端不会再发 done 事件，主动收起可能残留的进度条 */
function clearStaleProgress(): void {
  state.progress = null
}

export type CreateBackupOutcome = 'ok' | 'exists' | 'error'

export async function createBackup(
  workspaceId: string,
  name: string,
  note: string,
  overwrite: boolean
): Promise<CreateBackupOutcome> {
  try {
    const r = await api.createBackup(workspaceId, name, note, overwrite)
    toast('ok', `备份「${name}」已创建${r.hasRegistry ? '' : '（注册表未导出）'}`)
    await scanWorkspace(workspaceId)
    return 'ok'
  } catch (e) {
    const msg = errMsg(e)
    clearStaleProgress()
    // 交给调用方内联展示覆盖确认，避免把「新建备份」弹窗整个替换掉
    if (msg.includes('已存在') && !overwrite) return 'exists'
    // 覆盖失败时旧备份可能已移入回收站，重新扫描避免列表停留在旧状态
    void scanWorkspace(workspaceId)
    toast('err', msg)
    return 'error'
  }
}

/** 恢复类操作共用：游戏运行中先提示结束进程，否则退出时会把旧存档写回 */
async function withGameStopped(opts: {
  title: string
  message: string
  confirmText: string
  danger: boolean
  run: () => Promise<void>
}): Promise<void> {
  const status = await api.gameStatus()
  state.gameRunning = status.running
  if (!status.running) {
    state.modal = {
      kind: 'confirm',
      title: opts.title,
      message: opts.message,
      confirmText: opts.confirmText,
      danger: opts.danger,
      onConfirm: opts.run
    }
    return
  }
  state.modal = {
    kind: 'confirm',
    title: '游戏正在运行',
    message: '恢复前需要结束游戏进程，否则游戏退出时可能把内存中的旧存档写回，覆盖恢复结果。',
    confirmText: '结束游戏并恢复',
    danger: true,
    onConfirm: async () => {
      const k = await api.killGame()
      if (!k.killed) {
        toast('err', '结束游戏进程失败，请手动关闭游戏后重试')
        return
      }
      await new Promise((r) => setTimeout(r, 800))
      await opts.run()
    }
  }
}

function registryText(registry: RegistryOutcome): string {
  if (registry === 'ok') return '注册表已导入'
  if (registry === 'failed') return '注册表导入失败'
  if (registry === 'skipped') return '已跳过注册表'
  return '备份中无注册表文件'
}

export async function restoreBackup(
  workspaceId: string,
  name: string,
  includeRegistry = true
): Promise<void> {
  try {
    const summary = state.scans[workspaceId]?.backups.find((b) => b.name === name)
    const sourcePath = state.config?.game.sourcePath ?? '（未配置）'
    const meta = summary
      ? `${formatTime(summary.createdAt)} · ${summary.fileCount} 个文件 · ${formatSize(summary.totalSize)}`
      : '未知时间'
    const registryLine = includeRegistry
      ? '备份中的注册表将一并导入。'
      : '只恢复存档数据，不会导入注册表。'
    await withGameStopped({
      title: '恢复备份',
      message:
        `将用「${name}」（${meta}）覆盖当前存档：\n${sourcePath}\n\n` +
        `${registryLine}覆盖前的存档会自动保留为回档点，可随时还原。`,
      confirmText: includeRegistry ? '确认恢复' : '仅恢复数据',
      danger: false,
      run: () => doRestore(workspaceId, name, includeRegistry)
    })
  } catch (e) {
    toast('err', errMsg(e))
  }
}

async function doRestore(
  workspaceId: string,
  name: string,
  includeRegistry: boolean
): Promise<void> {
  try {
    const r = await api.restoreBackup(workspaceId, name, includeRegistry)
    const kept = r.rollback ? '，原存档已存为回档点' : ''
    const point = r.rollback
    toast(
      'ok',
      `恢复完成：${registryText(r.registry)}${kept}`,
      point ? { label: '撤销本次恢复', run: () => restoreRollback(point.id) } : undefined
    )
    await scanWorkspace(workspaceId)
    await loadRollbacks()
    void refreshGameStatus()
    void refreshSourceStatus()
  } catch (e) {
    clearStaleProgress()
    toast('err', errMsg(e))
    void refreshGameStatus()
  }
}

export async function loadRollbacks(): Promise<void> {
  try {
    state.rollbacks = await api.listRollbacks()
  } catch (e) {
    toast('err', errMsg(e))
  }
}

export async function restoreRollback(id: string): Promise<void> {
  try {
    const r = await api.restoreRollback(id)
    state.rollbacks = state.rollbacks ? { ...state.rollbacks, points: r.points } : state.rollbacks
    toast('ok', `已还原到该回档点：${registryText(r.registry)}`)
    void refreshGameStatus()
    void refreshSourceStatus()
  } catch (e) {
    clearStaleProgress()
    toast('err', errMsg(e))
    void refreshGameStatus()
  }
}

export async function requestRestoreRollback(point: RollbackPoint): Promise<void> {
  try {
    await withGameStopped({
      title: '还原回档点',
      message: '将用这份回档点覆盖当前存档。回档点本身不会被删除，可以再次还原。',
      confirmText: '确认还原',
      danger: false,
      run: () => restoreRollback(point.id)
    })
  } catch (e) {
    toast('err', errMsg(e))
  }
}

export function requestRemoveRollback(point: RollbackPoint): void {
  state.modal = {
    kind: 'confirm',
    title: '删除回档点',
    message: '将把这份回档点移入系统回收站，之后可从回收站还原。',
    confirmText: '移入回收站',
    danger: true,
    onConfirm: async () => {
      try {
        state.rollbacks = await api.removeRollback(point.id)
        toast('ok', '回档点已移入回收站')
      } catch (e) {
        toast('err', errMsg(e))
      }
    }
  }
}

export function requestClearRollbacks(): void {
  const n = state.rollbacks?.points.length ?? 0
  state.modal = {
    kind: 'confirm',
    title: '清空回档点',
    message: `将把全部 ${n} 份回档点移入系统回收站，工作区里的备份不受影响。`,
    confirmText: '全部移入回收站',
    danger: true,
    onConfirm: async () => {
      try {
        state.rollbacks = await api.clearRollbacks()
        toast('ok', '回档点已移入回收站')
      } catch (e) {
        toast('err', errMsg(e))
      }
    }
  }
}

export function requestDeleteBackup(workspaceId: string, name: string): void {
  state.modal = {
    kind: 'confirm',
    title: '删除备份',
    message: `备份「${name}」将被移入系统回收站，之后可从回收站还原。`,
    confirmText: '移入回收站',
    danger: true,
    onConfirm: async () => {
      try {
        await api.deleteBackup(workspaceId, name)
        toast('ok', '备份已移入回收站')
        if (state.selectedBackupName === name) {
          detailSeq += 1
          state.selectedBackupName = null
          state.detail = null
          state.detailLoading = false
        }
        await scanWorkspace(workspaceId)
      } catch (e) {
        toast('err', errMsg(e))
      }
    }
  }
}

export async function renameBackup(
  workspaceId: string,
  name: string,
  newName: string
): Promise<boolean> {
  try {
    await api.renameBackup(workspaceId, name, newName)
    toast('ok', '已重命名')
    if (state.selectedBackupName === name) state.selectedBackupName = newName
    await scanWorkspace(workspaceId)
    await refreshDetail()
    return true
  } catch (e) {
    toast('err', errMsg(e))
    return false
  }
}

export async function saveNote(
  workspaceId: string,
  name: string,
  note: string,
  opts: { silent?: boolean } = {}
): Promise<boolean> {
  try {
    await api.setNote(workspaceId, name, note)
    // 只更新内存中的对应项，不再触发整库扫描
    if (state.detail?.workspaceId === workspaceId && state.detail.name === name) {
      state.detail.note = note
    }
    const summary = state.scans[workspaceId]?.backups.find((b) => b.name === name)
    if (summary) summary.note = note
    if (!opts.silent) toast('ok', '备注已保存')
    return true
  } catch (e) {
    toast('err', errMsg(e))
    return false
  }
}

export async function saveSettings(patch: Partial<GameSettings>): Promise<boolean> {
  try {
    state.config = await api.updateGame(patch)
    toast('ok', '设置已保存')
    void loadRollbacks()
    void refreshSourceStatus()
    return true
  } catch (e) {
    toast('err', errMsg(e))
    return false
  }
}

export async function refreshSourceStatus(): Promise<void> {
  state.sourceStatusLoading = true
  try {
    state.sourceStatus = await api.sourceStatus()
  } catch (e) {
    state.sourceStatus = null
    toast('err', errMsg(e))
  } finally {
    state.sourceStatusLoading = false
  }
}

export async function refreshGameStatus(showLoading = false): Promise<void> {
  if (showLoading) state.gameStatusLoading = true
  try {
    state.gameRunning = (await api.gameStatus()).running
  } catch {
    state.gameRunning = null
  } finally {
    if (showLoading) state.gameStatusLoading = false
  }
}

let gameWatcherStarted = false

/** 定期刷新进程状态；窗口重新获得焦点时也刷新一次，避免展示过期状态。 */
export function startGameStatusWatcher(): void {
  if (gameWatcherStarted) return
  gameWatcherStarted = true
  window.setInterval(() => {
    if (!document.hidden) void refreshGameStatus()
  }, 5000)
  window.addEventListener('focus', () => {
    void refreshGameStatus()
    // 外部可能新增/删除了备份；节流后刷新，避免频繁 Alt-Tab 触发全量统计
    if (state.modal.kind === 'none' && Date.now() - lastScanAt > 30000) void scanAll()
  })
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) void refreshGameStatus()
  })
}
