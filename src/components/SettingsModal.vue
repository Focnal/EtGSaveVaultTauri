<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api'
import { saveSettings, state } from '../store'

const game = state.config?.game
const sourcePath = ref(game?.sourcePath ?? '')
const registryKey = ref(game?.registryKey ?? '')
const processName = ref(game?.processName ?? '')
const rollbackKeep = ref(game?.rollbackKeep ?? 1)
const busy = ref(false)

async function pickSource(): Promise<void> {
  const p = await api.pickDirectory('选择存档来源目录')
  if (p) sourcePath.value = p
}

async function save(): Promise<void> {
  if (busy.value) return
  busy.value = true
  await saveSettings({
    sourcePath: sourcePath.value.trim(),
    registryKey: registryKey.value.trim(),
    processName: processName.value.trim(),
    rollbackKeep: rollbackKeep.value
  })
  busy.value = false
  state.modal = { kind: 'none' }
}

function cancel(): void {
  state.modal = { kind: 'none' }
}
</script>

<template>
  <div class="modal-mask" @click.self="cancel">
    <div class="modal">
      <div class="modal-title">设置</div>
      <label class="field">
        <span>存档来源目录</span>
        <div class="field-row">
          <input v-model="sourcePath" type="text" />
          <button class="btn" @click="pickSource">浏览…</button>
        </div>
      </label>
      <label class="field">
        <span>注册表键（备份/恢复时一并导出导入）</span>
        <input v-model="registryKey" type="text" />
      </label>
      <label class="field">
        <span>游戏进程名（恢复前检测）</span>
        <input v-model="processName" type="text" />
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
      <div class="modal-actions">
        <button class="btn" @click="cancel">取消</button>
        <button class="btn btn-primary" :disabled="busy" @click="save">
          {{ busy ? '保存中…' : '保存' }}
        </button>
      </div>
    </div>
  </div>
</template>
