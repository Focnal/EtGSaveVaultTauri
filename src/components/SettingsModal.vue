<script setup lang="ts">
import { AlertTriangle, FolderOpen } from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import pkg from '../../package.json'
import { api } from '../api'
import {
  openInExplorer,
  refreshSourceStatus,
  saveSettings,
  state
} from '../store'
import ModalShell from './ModalShell.vue'

const game = state.config?.game
const sourcePath = ref(game?.sourcePath ?? '')
const registryKey = ref(game?.registryKey ?? '')
const processName = ref(game?.processName ?? '')
const rollbackKeep = ref(game?.rollbackKeep ?? 1)
const busy = ref(false)

const sourceError = computed(() =>
  sourcePath.value.trim() ? null : '存档来源目录不能为空'
)
const canSave = computed(() => !busy.value && !sourceError.value)

onMounted(() => {
  void refreshSourceStatus()
})

async function pickSource(): Promise<void> {
  const p = await api.pickDirectory('选择存档来源目录')
  if (p) sourcePath.value = p
}

async function save(): Promise<void> {
  if (!canSave.value) return
  busy.value = true
  try {
    const ok = await saveSettings({
      sourcePath: sourcePath.value.trim(),
      registryKey: registryKey.value.trim(),
      processName: processName.value.trim(),
      rollbackKeep: rollbackKeep.value
    })
    if (ok) state.modal = { kind: 'none' }
  } finally {
    busy.value = false
  }
}

function cancel(): void {
  if (busy.value) return
  state.modal = { kind: 'none' }
}
</script>

<template>
  <ModalShell title="设置" :busy="busy" @close="cancel">
    <label class="field">
      <span>存档来源目录</span>
      <div class="field-row">
        <input v-model="sourcePath" type="text" spellcheck="false" />
        <button class="btn" @click="pickSource">浏览…</button>
        <button
          class="btn"
          :disabled="!game?.sourcePath"
          title="打开当前已保存的来源目录"
          @click="openInExplorer(game?.sourcePath ?? '')"
        >
          <FolderOpen :size="14" />
          打开
        </button>
      </div>
      <span v-if="sourceError" class="field-error">{{ sourceError }}</span>
    </label>

    <div class="settings-status">
      <span v-if="state.sourceStatusLoading" class="dim">正在检查已保存的来源目录…</span>
      <span v-else-if="state.sourceStatus && !state.sourceStatus.exists" class="warn-text">
        <AlertTriangle :size="12" class="inline-icon" />
        已保存的来源目录不存在
      </span>
      <span v-else-if="state.sourceStatus && state.sourceStatus.fileCount === 0" class="warn-text">
        <AlertTriangle :size="12" class="inline-icon" />
        已保存的来源目录中没有存档文件
      </span>
      <span v-else-if="state.sourceStatus" class="dim">
        当前来源：{{ state.sourceStatus.fileCount }} 个文件 ·
        {{ state.sourceStatus.path }}
      </span>
    </div>

    <label class="field">
      <span>注册表键（备份/恢复时一并导出导入）</span>
      <input v-model="registryKey" type="text" spellcheck="false" />
    </label>
    <label class="field">
      <span>游戏进程名（恢复前检测）</span>
      <input v-model="processName" type="text" spellcheck="false" />
    </label>
    <label class="field">
      <span>恢复备份前保留的回档点</span>
      <select v-model.number="rollbackKeep">
        <option :value="0">关闭（不保留）</option>
        <option :value="1">仅保留最近 1 份（推荐）</option>
        <option :value="3">保留最近 3 份</option>
        <option :value="10">保留最近 10 份</option>
      </select>
    </label>
    <p class="field-hint">
      回档点用于“撤销本次恢复”。它存放在应用数据目录，不占用工作区，超出份数的会自动删除。
    </p>

    <div class="settings-about">
      <span>EtG Save Vault v{{ pkg.version }}</span>
      <span class="dim">配置与回档点保存在应用数据目录</span>
    </div>

    <template #actions>
      <button class="btn" :disabled="busy" @click="cancel">取消</button>
      <button class="btn btn-primary" :disabled="!canSave" @click="save">
        {{ busy ? '保存中…' : '保存' }}
      </button>
    </template>
  </ModalShell>
</template>
