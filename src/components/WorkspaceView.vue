<script setup lang="ts">
import { Archive, Camera, ExternalLink, FolderOpen, HardDrive, Plus, RefreshCw } from 'lucide-vue-next'
import { computed } from 'vue'
import { formatSize, formatTime } from '../format'
import { openBackupMenu } from '../menus'
import { openInExplorer, openNewBackup, scanWorkspace, selectBackup, state } from '../store'

const ws = computed(
  () => state.config?.workspaces.find((w) => w.id === state.selectedWorkspaceId) ?? null
)
const scan = computed(() => (ws.value ? state.scans[ws.value.id] : undefined))
const backups = computed(() => scan.value?.backups ?? [])
const totalSize = computed(() => backups.value.reduce((s, b) => s + b.totalSize, 0))
const snapshotCount = computed(() => backups.value.filter((b) => b.isSnapshot).length)
const latest = computed(() =>
  backups.value.length ? Math.max(...backups.value.map((b) => b.createdAt)) : null
)
const game = computed(() => state.config?.game ?? null)
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
        <button class="btn" @click="scanWorkspace(ws.id)">
          <RefreshCw :size="14" />
          刷新
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
        <div class="stat-value">{{ snapshotCount }}</div>
        <div class="stat-label">自动快照</div>
      </div>
      <div class="stat">
        <div class="stat-value">{{ formatSize(totalSize) }}</div>
        <div class="stat-label">总占用</div>
      </div>
      <div class="stat">
        <div class="stat-value">{{ latest ? formatTime(latest).slice(5) : '—' }}</div>
        <div class="stat-label">最近备份</div>
      </div>
    </div>

    <div v-if="game" class="source-line">
      <HardDrive :size="14" class="dim-icon" />
      <span class="dim">存档来源</span>
      <code>{{ game.sourcePath }}</code>
      <button class="icon-btn xs" title="打开来源目录" @click="openInExplorer(game.sourcePath)">
        <ExternalLink :size="12" />
      </button>
    </div>

    <div class="section-title">备份列表</div>
    <div v-if="scan?.missing" class="hint warn-text">目录不存在：{{ ws.path }}</div>
    <div v-else-if="backups.length === 0" class="hint">还没有备份，点击「新建备份」创建第一份。</div>
    <div v-else class="bk-table">
      <div
        v-for="b in backups"
        :key="b.name"
        class="bk-table-row"
        @click="selectBackup(ws.id, b.name)"
        @contextmenu.prevent="openBackupMenu($event, ws.id, b)"
      >
        <Camera v-if="b.isSnapshot" :size="14" class="dim-icon" />
        <Archive v-else :size="14" class="accent-icon" />
        <span class="bk-name" :title="b.note || b.name">{{ b.name }}</span>
        <span v-if="!b.hasData" class="badge badge-warn">数据缺失</span>
        <span v-if="!b.hasRegistry" class="badge badge-warn">注册表缺失</span>
        <span class="bk-meta">{{ formatTime(b.createdAt) }}</span>
        <span class="bk-meta">{{ b.fileCount }} 个文件</span>
        <span class="bk-meta">{{ formatSize(b.totalSize) }}</span>
      </div>
    </div>
  </div>
</template>
