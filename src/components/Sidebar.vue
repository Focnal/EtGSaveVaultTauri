<script setup lang="ts">
import {
  AlertTriangle,
  Archive,
  Camera,
  ChevronRight,
  Folder,
  FolderOpen,
  FolderSearch,
  LoaderCircle,
  Plus,
  RefreshCw,
  Search,
  X
} from 'lucide-vue-next'
import { computed, nextTick, watch } from 'vue'
import { formatCompactTime } from '../format'
import { openBackupMenu, openWorkspaceMenu } from '../menus'
import {
  addWorkspace,
  scanAll,
  scanWorkspace,
  selectBackup,
  selectWorkspace,
  state,
  visibleBackups
} from '../store'
import type { BackupSummary, Workspace } from '../types'

const workspaces = computed(() => state.config?.workspaces ?? [])
const backupsOf = (id: string) => state.scans[id]?.backups ?? []
const missingOf = (id: string) => state.scans[id]?.missing ?? false
const scanningOf = (id: string) => !!state.scanning[id]
const scanningAll = computed(() => workspaces.value.some((ws) => scanningOf(ws.id)))
const filtering = computed(() => state.filter.trim().length > 0)

const visibleByWorkspace = computed(() => {
  const map: Record<string, BackupSummary[]> = {}
  for (const ws of workspaces.value) map[ws.id] = visibleBackups(backupsOf(ws.id))
  return map
})
const visibleOf = (id: string): BackupSummary[] => visibleByWorkspace.value[id] ?? []
const matchTotal = computed(() =>
  filtering.value
    ? workspaces.value.reduce((sum, ws) => sum + visibleOf(ws.id).length, 0)
    : null
)

function clearFilter(): void {
  state.filter = ''
}

watch(
  () => [state.selectedWorkspaceId, state.selectedBackupName],
  async () => {
    await nextTick()
    document
      .querySelector<HTMLElement>('.bk-row.selected')
      ?.scrollIntoView({ block: 'nearest' })
  }
)

function toggle(id: string): void {
  state.expanded[id] = !state.expanded[id]
  if (state.expanded[id] && !state.scans[id]) void scanWorkspace(id)
}

/** 左键点击只负责选中/展开；折叠交给 chevron，避免误收起列表 */
function onWorkspaceClick(ws: Workspace): void {
  selectWorkspace(ws.id, true)
}

/** 右键先选中，菜单项与右侧详情才对应用户点到的那一项 */
function onWorkspaceContext(e: MouseEvent, ws: Workspace): void {
  // 折叠状态保持不变：右键只移动高亮
  selectWorkspace(ws.id, false)
  openWorkspaceMenu(e, ws)
}

function onBackupContext(e: MouseEvent, ws: Workspace, b: BackupSummary): void {
  // 详情是异步加载的，不阻塞菜单弹出
  void selectBackup(ws.id, b.name)
  openBackupMenu(e, ws.id, b)
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-head">
      <span class="sidebar-title">工作区</span>
      <div class="sidebar-head-actions">
        <button class="icon-btn" title="全部刷新" :disabled="scanningAll" @click="scanAll">
          <LoaderCircle v-if="scanningAll" :size="14" class="spin" />
          <RefreshCw v-else :size="14" />
        </button>
        <button class="icon-btn" title="添加工作区" @click="addWorkspace">
          <Plus :size="16" />
        </button>
      </div>
    </div>

    <div class="sidebar-search">
      <Search :size="13" class="search-icon" />
      <input
        id="backup-search"
        v-model="state.filter"
        type="text"
        placeholder="搜索备份名称、备注或槽位"
        spellcheck="false"
        aria-label="搜索备份"
      />
      <button
        v-if="filtering"
        class="icon-btn xs"
        title="清除搜索"
        aria-label="清除搜索"
        @click="clearFilter"
      >
        <X :size="12" />
      </button>
    </div>

    <div class="tree">
      <div v-if="workspaces.length === 0" class="tree-empty">
        <FolderSearch :size="30" :stroke-width="1.3" />
        <p>还没有工作区</p>
        <p class="tree-empty-hint">工作区是存放备份的目录，可以添加多个。</p>
        <button class="btn btn-primary" @click="addWorkspace">
          <Plus :size="14" />
          添加工作区目录
        </button>
      </div>

      <div v-else-if="filtering && matchTotal === 0" class="tree-hint filter-empty">
        没有匹配「{{ state.filter.trim() }}」的备份
      </div>

      <div v-for="ws in workspaces" :key="ws.id" class="ws-node">
        <div
          class="tree-row ws-row"
          :class="{
            selected: state.selectedWorkspaceId === ws.id && !state.selectedBackupName,
            'selected-parent': state.selectedWorkspaceId === ws.id && !!state.selectedBackupName
          }"
          @click="onWorkspaceClick(ws)"
          @contextmenu.prevent="onWorkspaceContext($event, ws)"
        >
          <button
            class="chevron"
            :class="{ open: state.expanded[ws.id] }"
            title="展开/收起"
            @click.stop="toggle(ws.id)"
          >
            <ChevronRight :size="13" />
          </button>
          <FolderOpen v-if="state.expanded[ws.id]" :size="15" class="row-icon ws-icon" />
          <Folder v-else :size="15" class="row-icon ws-icon" />
          <span class="row-label" :title="ws.path">{{ ws.name }}</span>
          <AlertTriangle v-if="missingOf(ws.id)" :size="12" class="warn-icon" />
          <LoaderCircle v-else-if="scanningOf(ws.id)" :size="11" class="spin row-spinner" />
          <span v-else class="row-count">
            {{ filtering ? visibleOf(ws.id).length : backupsOf(ws.id).length }}
          </span>
        </div>

        <div v-if="state.expanded[ws.id] || filtering" class="bk-list">
          <div
            v-for="b in visibleOf(ws.id)"
            :key="b.name"
            class="tree-row bk-row"
            :class="{
              selected: state.selectedWorkspaceId === ws.id && state.selectedBackupName === b.name
            }"
            @click="selectBackup(ws.id, b.name)"
            @contextmenu.prevent="onBackupContext($event, ws, b)"
          >
            <Camera v-if="b.isSnapshot" :size="13" class="row-icon dim-icon" />
            <Archive v-else :size="13" class="row-icon" />
            <span class="row-label" :title="b.note ? `${b.name} — ${b.note}` : b.name">
              {{ b.name }}
            </span>
            <span v-if="b.slots.length" class="slot-badge" :title="`存档槽位 ${b.slots.join('、')}`">
              {{ b.slots.join('') }}
            </span>
            <AlertTriangle v-if="!b.valid" :size="11" class="warn-icon" />
            <span class="row-date">{{ formatCompactTime(b.createdAt) }}</span>
          </div>
          <div v-if="scanningOf(ws.id) && !state.scans[ws.id]" class="tree-hint">
            正在扫描…
          </div>
          <div
            v-else-if="filtering && visibleOf(ws.id).length === 0"
            class="tree-hint"
          >
            无匹配
          </div>
          <div
            v-else-if="!missingOf(ws.id) && backupsOf(ws.id).length === 0"
            class="tree-hint"
          >
            暂无备份
          </div>
          <div v-if="missingOf(ws.id)" class="tree-hint warn-text">目录不存在</div>
        </div>
      </div>
    </div>
  </aside>
</template>
