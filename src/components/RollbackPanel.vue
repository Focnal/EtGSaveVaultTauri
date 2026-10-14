<script setup lang="ts">
import { AlertTriangle, FolderOpen, History, RotateCcw, Trash2 } from 'lucide-vue-next'
import { computed, onMounted } from 'vue'
import { formatSize, formatTime } from '../format'
import {
  loadRollbacks,
  openInExplorer,
  requestClearRollbacks,
  requestRemoveRollback,
  requestRestoreRollback,
  state
} from '../store'

const list = computed(() => state.rollbacks)
const points = computed(() => list.value?.points ?? [])

onMounted(() => {
  void loadRollbacks()
})

function close(): void {
  state.modal = { kind: 'none' }
}
</script>

<template>
  <div class="modal-mask" @click.self="close">
    <div class="modal modal-wide">
      <div class="modal-title">
        <History :size="16" />
        回档点
      </div>

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
            <div class="rp-note" :title="p.note">{{ p.note || p.id }}</div>
          </div>
          <div class="rp-meta">
            <span>{{ formatSize(p.totalSize) }}</span>
            <span>{{ p.fileCount }} 个文件</span>
          </div>
          <div class="rp-actions">
            <button
              class="btn"
              :disabled="!p.hasData"
              :title="p.hasData ? '用这份回档点覆盖当前存档' : '缺少存档数据，无法还原'"
              @click="requestRestoreRollback(p)"
            >
              <RotateCcw :size="13" />
              还原
            </button>
            <button class="icon-btn" title="删除这份回档点" @click="requestRemoveRollback(p)">
              <Trash2 :size="14" />
            </button>
          </div>
        </div>
      </div>

      <div v-if="list && list.keep === 0" class="damage-banner">
        <AlertTriangle :size="14" />
        <span>自动保留已关闭，之后执行的恢复不会再生成回档点。</span>
      </div>

      <div class="modal-actions rp-footer">
        <button
          v-if="list"
          class="btn"
          title="在资源管理器中打开存放目录"
          @click="openInExplorer(list.storePath)"
        >
          <FolderOpen :size="14" />
          打开目录
        </button>
        <button class="btn btn-danger" :disabled="points.length === 0" @click="requestClearRollbacks">
          <Trash2 :size="14" />
          全部清空
        </button>
        <button class="btn btn-primary" @click="close">关闭</button>
      </div>
    </div>
  </div>
</template>
