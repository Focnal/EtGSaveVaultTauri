import { ExternalLink, Pencil, Plus, RefreshCw, RotateCcw, Trash2 } from 'lucide-vue-next'
import type { BackupSummary, Workspace } from './types'
import {
  openInExplorer,
  openMenu,
  openNewBackup,
  openRename,
  requestDeleteBackup,
  requestRemoveWorkspace,
  restoreBackup,
  scanWorkspace,
  type MenuEntry
} from './store'

/** 备份菜单同时出现在侧栏与工作区视图，集中定义避免两处不一致 */

/** 工作区文件夹的菜单 */
export function openWorkspaceMenu(e: MouseEvent, ws: Workspace): void {
  const entries: MenuEntry[] = [
    { label: '新建备份', icon: Plus, run: () => openNewBackup(ws.id) },
    { label: '刷新', icon: RefreshCw, run: () => scanWorkspace(ws.id) },
    { label: '在资源管理器中打开', icon: ExternalLink, run: () => openInExplorer(ws.path) },
    { divider: true },
    {
      label: '移除工作区',
      icon: Trash2,
      danger: true,
      run: () => requestRemoveWorkspace(ws.id, ws.name)
    }
  ]
  openMenu(e, entries)
}

/** 备份条目的菜单，与详情页头部操作一致 */
export function openBackupMenu(e: MouseEvent, workspaceId: string, b: BackupSummary): void {
  const entries: MenuEntry[] = [
    { label: '重命名', icon: Pencil, run: () => openRename(workspaceId, b.name) },
    // 数据或注册表缺失时无法恢复，与详情页按钮一致
    {
      label: '恢复此存档',
      icon: RotateCcw,
      disabled: !b.valid,
      run: () => restoreBackup(workspaceId, b.name)
    },
    { label: '在资源管理器中打开', icon: ExternalLink, run: () => openInExplorer(b.path) },
    { divider: true },
    { label: '删除', icon: Trash2, danger: true, run: () => requestDeleteBackup(workspaceId, b.name) }
  ]
  openMenu(e, entries)
}
