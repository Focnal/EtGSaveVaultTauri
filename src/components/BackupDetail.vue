<script setup lang="ts">
import {
  AlertTriangle,
  Archive,
  Camera,
  Clock,
  File,
  Folder,
  FolderOpen,
  HardDrive,
  Pencil,
  RotateCcw,
  Save,
  Trash2
} from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { formatSize, formatTime } from '../format'
import {
  openInExplorer,
  openRename,
  requestDeleteBackup,
  restoreBackup,
  saveNote,
  state
} from '../store'

const ENTRY_SHOW = 100

const d = computed(() => state.detail)
const wsName = computed(
  () => state.config?.workspaces.find((w) => w.id === state.detail?.workspaceId)?.name ?? ''
)
const shownEntries = computed(() => (state.detail?.entries ?? []).slice(0, ENTRY_SHOW))

const note = ref('')
watch(
  () => state.detail?.name,
  () => {
    note.value = state.detail?.note ?? ''
  },
  { immediate: true }
)
const noteDirty = computed(() => note.value !== (state.detail?.note ?? ''))

const damageText = computed(() => {
  const dd = state.detail
  if (!dd || dd.valid) return ''
  if (!dd.hasData && !dd.hasRegistry) return '备份中缺少存档数据目录和注册表文件，备份已损坏，无法用于恢复。'
  if (!dd.hasData) return '备份中缺少存档数据目录，备份已损坏，无法用于恢复。'
  return '备份中缺少注册表文件，备份已损坏，无法用于恢复。'
})

function onSaveNote(): void {
  const dd = state.detail
  if (dd) void saveNote(dd.workspaceId, dd.name, note.value)
}

function onRename(): void {
  const dd = state.detail
  if (dd) openRename(dd.workspaceId, dd.name)
}
</script>

<template>
  <div v-if="d" class="detail-view">
    <div class="view-head">
      <div class="view-head-text">
        <div class="detail-title-row">
          <Camera v-if="d.isSnapshot" :size="17" class="dim-icon" />
          <Archive v-else :size="17" class="accent-icon" />
          <h2 class="view-title">{{ d.name }}</h2>
          <span v-if="d.isSnapshot" class="badge">自动快照</span>
          <span v-if="!d.hasData" class="badge badge-warn">数据缺失</span>
          <span v-if="!d.hasRegistry" class="badge badge-warn">注册表缺失</span>
        </div>
        <button class="path-line" :title="'在资源管理器中打开 ' + d.path" @click="openInExplorer(d.path)">
          <FolderOpen :size="13" />
          <span>{{ d.path }}</span>
        </button>
      </div>
      <div class="view-actions">
        <button class="btn" @click="onRename">
          <Pencil :size="14" />
          重命名
        </button>
        <button
          class="btn btn-warn"
          :disabled="!d.valid"
          :title="d.valid ? '用该备份覆盖当前存档' : '备份已损坏，无法恢复'"
          @click="restoreBackup(d.workspaceId, d.name)"
        >
          <RotateCcw :size="14" />
          恢复此备份
        </button>
        <button class="btn btn-danger" @click="requestDeleteBackup(d.workspaceId, d.name)">
          <Trash2 :size="14" />
          删除
        </button>
      </div>
    </div>

    <div class="kv-grid">
      <div class="kv">
        <Clock :size="13" class="dim-icon" />
        <span class="kv-label">创建时间</span>
        <span class="kv-value">{{ formatTime(d.createdAt) }}</span>
      </div>
      <div class="kv">
        <File :size="13" class="dim-icon" />
        <span class="kv-label">文件数</span>
        <span class="kv-value">{{ d.fileCount }}</span>
      </div>
      <div class="kv">
        <HardDrive :size="13" class="dim-icon" />
        <span class="kv-label">总大小</span>
        <span class="kv-value">{{ formatSize(d.totalSize) }}</span>
      </div>
      <div class="kv">
        <Folder :size="13" class="dim-icon" />
        <span class="kv-label">工作区</span>
        <span class="kv-value">{{ wsName }}</span>
      </div>
    </div>

    <div v-if="damageText" class="damage-banner">
      <AlertTriangle :size="14" class="warn-icon" />
      <span>{{ damageText }}</span>
    </div>

    <div class="section-title">备注</div>
    <div class="note-editor">
      <textarea v-model="note" rows="2" placeholder="给这个备份写点备注，例如当时的进度、解锁情况等"></textarea>
      <button class="btn" :disabled="!noteDirty" @click="onSaveNote">
        <Save :size="14" />
        保存
      </button>
    </div>

    <div class="section-title">
      存档内容
      <span v-if="d.entries.length > ENTRY_SHOW" class="dim">
        （仅显示前 {{ ENTRY_SHOW }} 项，共 {{ d.entries.length }} 项）
      </span>
    </div>
    <div v-if="!d.hasData" class="hint warn-text">缺少存档数据目录，不显示内容。</div>
    <div v-else-if="shownEntries.length === 0" class="hint">数据目录为空。</div>
    <div v-else class="file-list">
      <div v-for="e in shownEntries" :key="e.name" class="file-row">
        <Folder v-if="e.isDir" :size="13" class="ws-icon" />
        <File v-else :size="13" class="dim-icon" />
        <span class="file-name">{{ e.name }}</span>
        <span class="file-size">{{ e.isDir ? '—' : formatSize(e.size) }}</span>
        <span class="file-time">{{ formatTime(e.mtime) }}</span>
      </div>
    </div>
  </div>
</template>
