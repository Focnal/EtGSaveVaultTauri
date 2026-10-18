<script setup lang="ts">
import {
  Archive,
  Camera,
  Copy,
  ExternalLink,
  FolderOpen,
  HardDrive,
  LoaderCircle,
  Plus,
  RefreshCw
} from 'lucide-vue-next'
import { computed } from 'vue'
import { formatCompactTime, formatSize, formatTime } from '../format'
import { openBackupMenu } from '../menus'
import {
  copyText,
  openInExplorer,
  openNewBackup,
  scanWorkspace,
  selectBackup,
  state,
  visibleBackups
} from '../store'
import type { BackupSort } from '../store'

const ws = computed(
  () => state.config?.workspaces.find((w) => w.id === state.selectedWorkspaceId) ?? null
)
const scan = computed(() => (ws.value ? state.scans[ws.value.id] : undefined))
const backups = computed(() => scan.value?.backups ?? [])
const filtered = computed(() => visibleBackups(backups.value))
const filtering = computed(() => state.filter.trim().length > 0)
const scanning = computed(() => (ws.value ? !!state.scanning[ws.value.id] : false))
const totalSize = computed(() => backups.value.reduce((s, b) => s + b.totalSize, 0))
const validCount = computed(() => backups.value.filter((b) => b.valid).length)
const latest = computed(() =>
  backups.value.length ? Math.max(...backups.value.map((b) => b.createdAt)) : null
)
const game = computed(() => state.config?.game ?? null)
const source = computed(() => state.sourceStatus)

function onSortChange(e: Event): void {
  state.backupSort = (e.target as HTMLSelectElement).value as BackupSort
}
</script>

<template>
  <div v-if="ws" class="workspace-view">
    <div class="view-head">
      <div class="view-head-text">
        <h2 class="view-title">{{ ws.name }}</h2>
        <button class="path-line" :title="'在资源管理器中打开 ' + ws.path" @click="openInExplorer(ws.path)">
          <FolderOpen :size="13" />
          <span>{{ ws.path }}</span>
        </button>
      </div>
      <div class="view-actions">
        <button class="btn" :disabled="scanning" @click="scanWorkspace(ws.id)">
          <LoaderCircle v-if="scanning" :size="14" class="spin" />
          <RefreshCw v-else :size="14" />
          {{ scanning ? '扫描中…' : '刷新' }}
        </button>
        <button class="btn btn-primary" @click="openNewBackup(ws.id)">
          <Plus :size="14" />
          新建备份
        </button>
      </div>
    </div>

    <div class="stat-row">
      <div class="stat">
        <div class="stat-value">{{ backups.length }}</div>
        <div class="stat-label">备份</div>
      </div>
      <div class="stat">
        <div class="stat-value">{{ validCount }}</div>
        <div class="stat-label">可完整恢复</div>
      </div>
      <div class="stat">
        <div class="stat-value">{{ formatSize(totalSize) }}</div>
        <div class="stat-label">数据占用</div>
      </div>
      <div class="stat">
        <div class="stat-value">{{ latest ? formatCompactTime(latest) : '—' }}</div>
        <div class="stat-label">最近备份</div>
      </div>
    </div>

    <div v-if="game" class="source-line">
      <HardDrive :size="14" class="dim-icon" />
      <span class="dim">存档来源</span>
      <code>{{ game.sourcePath }}</code>
      <span v-if="state.sourceStatusLoading" class="source-meta">检测中…</span>
      <span v-else-if="source && !source.exists" class="source-meta warn-text">目录不存在</span>
      <span v-else-if="source && source.fileCount === 0" class="source-meta warn-text">目录为空</span>
      <span v-else-if="source" class="source-meta">
        {{ source.fileCount }} 个文件 · {{ formatSize(source.totalSize) }}
        <template v-if="source.modifiedAt"> · 最后写入 {{ formatTime(source.modifiedAt) }}</template>
      </span>
      <button
        class="icon-btn xs"
        title="复制来源路径"
        aria-label="复制来源路径"
        @click="copyText(game.sourcePath, '来源路径')"
      >
        <Copy :size="12" />
      </button>
      <button
        class="icon-btn xs"
        title="打开来源目录"
        aria-label="打开来源目录"
        @click="openInExplorer(game.sourcePath)"
      >
        <ExternalLink :size="12" />
      </button>
    </div>

    <div class="list-toolbar">
      <div class="section-title">
        备份列表
        <span v-if="filtering" class="dim">
          · 匹配 {{ filtered.length }}/{{ backups.length }} 项
        </span>
      </div>
      <label class="sort-control">
        <span class="dim">排序</span>
        <select :value="state.backupSort" aria-label="备份排序" @change="onSortChange">
          <option value="newest">最新优先</option>
          <option value="oldest">最早优先</option>
          <option value="name">名称</option>
          <option value="size">占用大小</option>
        </select>
      </label>
    </div>

    <div v-if="scan?.missing" class="hint warn-text">目录不存在：{{ ws.path }}</div>
    <div v-else-if="scanning && !scan" class="hint">
      <LoaderCircle :size="13" class="spin inline-icon" />
      正在扫描备份…
    </div>
    <div v-else-if="filtering && filtered.length === 0" class="hint">
      没有匹配「{{ state.filter.trim() }}」的备份。
    </div>
    <div v-else-if="backups.length === 0" class="hint">还没有备份，点击「新建备份」创建第一份。</div>
    <div v-else class="bk-table">
      <div class="bk-table-head">
        <span class="bk-head-name">名称 / 备注</span>
        <span class="bk-meta bk-col-date">创建时间</span>
        <span class="bk-meta bk-col-count">文件数</span>
        <span class="bk-meta bk-col-size">占用</span>
      </div>
      <div
        v-for="b in filtered"
        :key="b.name"
        class="bk-table-row"
        @click="selectBackup(ws.id, b.name)"
        @contextmenu.prevent="openBackupMenu($event, ws.id, b)"
      >
        <Camera v-if="b.isSnapshot" :size="14" class="dim-icon" />
        <Archive v-else :size="14" class="accent-icon" />
        <div class="bk-name-block">
          <span class="bk-name" :title="b.name">{{ b.name }}</span>
          <span v-if="b.note" class="bk-note" :title="b.note">{{ b.note }}</span>
        </div>
        <span v-if="b.slots.length" class="slot-badge" :title="`存档槽位 ${b.slots.join('、')}`">
          槽位 {{ b.slots.join(' / ') }}
        </span>
        <span v-if="!b.hasData" class="badge badge-warn">数据缺失</span>
        <span v-if="!b.hasRegistry" class="badge badge-warn">注册表缺失</span>
        <span class="bk-meta bk-col-date">{{ formatTime(b.createdAt) }}</span>
        <span class="bk-meta bk-col-count">{{ b.fileCount }} 个文件</span>
        <span class="bk-meta bk-col-size">{{ formatSize(b.totalSize) }}</span>
      </div>
    </div>
  </div>
</template>
