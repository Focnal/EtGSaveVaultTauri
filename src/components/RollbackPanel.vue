<script setup lang="ts">
import {
  AlertTriangle,
  FolderOpen,
  History,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
  Trash2
} from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import { formatSize, formatTime } from '../format'
import {
  loadRollbacks,
  openInExplorer,
  requestClearRollbacks,
  requestRemoveRollback,
  requestRestoreRollback,
  state
} from '../store'
import ModalShell from './ModalShell.vue'

const list = computed(() => state.rollbacks)
const points = computed(() => list.value?.points ?? [])
const refreshing = ref(false)

async function refresh(): Promise<void> {
  if (refreshing.value) return
  refreshing.value = true
  try {
    await loadRollbacks()
  } finally {
    refreshing.value = false
  }
}

onMounted(() => {
  void refresh()
})

function close(): void {
  state.modal = { kind: 'none' }
}
</script>

<template>
  <ModalShell title="回档点" wide actions-class="rp-footer" @close="close">
    <template #icon>
      <History :size="16" />
    </template>

    <p class="modal-msg">
      恢复备份前自动保留的当前存档。它存放在应用数据目录，不占用工作区，也不会计入备份列表。
      <template v-if="list">
        <template v-if="list.keep > 0">当前策略：只保留最近 {{ list.keep }} 份，多余的自动删除。</template>
        <template v-else>当前策略：已关闭自动保留（可在设置中开启）。</template>
      </template>
    </p>

    <div v-if="!list" class="hint">读取中…</div>
    <div v-else-if="points.length === 0" class="rp-empty">
      <History :size="26" :stroke-width="1.3" />
      <p>暂无回档点</p>
      <p class="dim">执行一次「恢复备份」后，覆盖前的存档会出现在这里。</p>
    </div>

    <div v-else class="rp-list">
      <div v-for="p in points" :key="p.id" class="rp-row">
        <div class="rp-main">
          <div class="rp-head">
            <span class="rp-time">{{ formatTime(p.createdAt) }}</span>
            <span class="badge">{{ p.mode === 'move' ? '零拷贝' : '复制' }}</span>
            <span v-if="!p.hasData" class="badge badge-warn">数据缺失</span>
            <span v-else-if="!p.hasRegistry" class="badge badge-warn">注册表缺失</span>
          </div>
          <div v-if="p.sourceBackup" class="rp-source" :title="p.sourceBackup">
            来自恢复「{{ p.sourceBackup }}」之前
          </div>
          <div class="rp-note" :title="p.note">{{ p.note || p.id }}</div>
        </div>
        <div class="rp-meta">
          <span>{{ formatSize(p.totalSize) }}</span>
          <span>{{ p.fileCount }} 个文件</span>
        </div>
        <div class="rp-actions">
          <button
            class="icon-btn"
            title="在资源管理器中打开这份回档点"
            aria-label="打开这份回档点目录"
            @click="openInExplorer(p.path)"
          >
            <FolderOpen :size="14" />
          </button>
          <button
            class="btn"
            :disabled="!p.hasData"
            :title="p.hasData ? '用这份回档点覆盖当前存档' : '缺少存档数据，无法还原'"
            @click="requestRestoreRollback(p)"
          >
            <RotateCcw :size="13" />
            还原
          </button>
          <button
            class="icon-btn"
            title="删除这份回档点"
            aria-label="删除这份回档点"
            @click="requestRemoveRollback(p)"
          >
            <Trash2 :size="14" />
          </button>
        </div>
      </div>
    </div>

    <div v-if="list && list.keep === 0" class="damage-banner">
      <AlertTriangle :size="14" />
      <span>自动保留已关闭，之后执行的恢复不会再生成回档点。</span>
    </div>

    <template #actions>
      <button
        v-if="list"
        class="btn"
        title="在资源管理器中打开存放目录"
        @click="openInExplorer(list.storePath)"
      >
        <FolderOpen :size="14" />
        打开目录
      </button>
      <button class="btn" :disabled="refreshing" @click="refresh">
        <LoaderCircle v-if="refreshing" :size="14" class="spin" />
        <RefreshCw v-else :size="14" />
        刷新
      </button>
      <button class="btn btn-danger" :disabled="points.length === 0" @click="requestClearRollbacks">
        <Trash2 :size="14" />
        全部清空
      </button>
      <button class="btn btn-primary" data-autofocus @click="close">关闭</button>
    </template>
  </ModalShell>
</template>
