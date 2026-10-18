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
import { computed, onBeforeUnmount, ref, watch } from 'vue'
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

interface NoteTarget {
  workspaceId: string
  name: string
}

const note = ref('')
const noteSaving = ref(false)
let noteTarget: NoteTarget | null = null
let persistedNote = ''
let noteTimer: number | undefined

function clearNoteTimer(): void {
  if (noteTimer !== undefined) {
    window.clearTimeout(noteTimer)
    noteTimer = undefined
  }
}

/** 切换备份前把上一份未保存的备注写回，避免静默丢失 */
function flushPreviousNote(): void {
  const target = noteTarget
  if (!target) return
  const value = note.value
  if (value === persistedNote) return
  clearNoteTimer()
  void saveNote(target.workspaceId, target.name, value, { silent: true }).then((ok) => {
    if (ok) persistedNote = value
  })
}

function applyCurrentDetail(): void {
  const dd = state.detail
  if (!dd) {
    noteTarget = null
    persistedNote = ''
    note.value = ''
    noteSaving.value = false
    return
  }
  noteTarget = { workspaceId: dd.workspaceId, name: dd.name }
  persistedNote = dd.note
  note.value = dd.note
  noteSaving.value = false
}

// 同步侦听：store 里切换选中项的同一拍就能取到旧目标并落盘
watch(
  () => (state.detail ? `${state.detail.workspaceId}\u0000${state.detail.name}` : ''),
  (_, oldKey) => {
    if (oldKey) flushPreviousNote()
    applyCurrentDetail()
  },
  { immediate: true, flush: 'sync' }
)

const noteDirty = computed(() => note.value !== (state.detail?.note ?? ''))

async function persistNote(value: string, target: NoteTarget): Promise<void> {
  noteSaving.value = true
  const ok = await saveNote(target.workspaceId, target.name, value, { silent: true })
  if (ok) persistedNote = value
  noteSaving.value = false
}

// 输入停顿后自动保存；显式「保存」按钮仍保留
watch(note, (value) => {
  if (!noteTarget || value === persistedNote) return
  clearNoteTimer()
  const target = noteTarget
  noteTimer = window.setTimeout(() => {
    noteTimer = undefined
    void persistNote(value, target)
  }, 700)
})

onBeforeUnmount(() => {
  flushPreviousNote()
})

const noteStatus = computed(() => {
  if (noteSaving.value) return '保存中…'
  if (noteDirty.value) return '未保存'
  return noteTarget ? '已自动保存' : ''
})

const damageText = computed(() => {
  const dd = state.detail
  if (!dd || dd.valid) return ''
  if (!dd.hasData) return '备份中缺少存档数据目录，无法用于恢复。'
  return '备份中缺少注册表文件，可以只恢复存档数据，或补齐注册表后再完整恢复。'
})

async function onSaveNote(): Promise<void> {
  const dd = state.detail
  if (!dd || !noteTarget) return
  clearNoteTimer()
  noteSaving.value = true
  const ok = await saveNote(dd.workspaceId, dd.name, note.value)
  noteSaving.value = false
  if (ok) persistedNote = note.value
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
          :disabled="!d.hasData"
          :title="
            !d.hasData
              ? '缺少存档数据目录，无法恢复'
              : d.hasRegistry
                ? '用该备份覆盖当前存档并导入注册表'
                : '仅恢复存档数据，不导入注册表'
          "
          @click="restoreBackup(d.workspaceId, d.name, d.hasRegistry)"
        >
          <RotateCcw :size="14" />
          {{ d.hasRegistry ? '恢复此备份' : '仅恢复存档数据' }}
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

    <div class="section-title">
      备注
      <span class="note-status" :class="{ 'warn-text': noteDirty && !noteSaving }">{{ noteStatus }}</span>
    </div>
    <div class="note-editor">
      <textarea
        v-model="note"
        rows="2"
        placeholder="给这个备份写点备注，例如当时的进度、解锁情况等"
        @blur="flushPreviousNote"
      ></textarea>
      <button class="btn" :disabled="!noteDirty || noteSaving" @click="onSaveNote">
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
