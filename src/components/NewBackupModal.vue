<script setup lang="ts">
import { AlertTriangle, Gamepad2, HardDrive } from 'lucide-vue-next'
import { computed, onMounted, ref, watch } from 'vue'
import { formatSize, formatTime, timestampName, validateBackupName } from '../format'
import { createBackup, refreshGameStatus, refreshSourceStatus, state } from '../store'
import ModalShell from './ModalShell.vue'
import ProgressInline from './ProgressInline.vue'

const workspaceId = computed(() => (state.modal.kind === 'newBackup' ? state.modal.workspaceId : ''))
const name = ref(timestampName())
const note = ref('')
const busy = ref(false)
const overwritePrompt = ref(false)

const nameError = computed(() => (name.value ? validateBackupName(name.value) : null))
const status = computed(() => state.sourceStatus)
const sourceReady = computed(() => !!status.value?.exists && (status.value?.fileCount ?? 0) > 0)
const canSubmit = computed(
  () =>
    !!name.value.trim() &&
    !nameError.value &&
    !busy.value &&
    !state.sourceStatusLoading &&
    sourceReady.value
)
const duplicate = computed(() =>
  (state.scans[workspaceId.value]?.backups ?? []).some((b) => b.name === name.value.trim())
)

watch(name, () => {
  overwritePrompt.value = false
})

onMounted(() => {
  void refreshSourceStatus()
  void refreshGameStatus()
})

async function submit(): Promise<void> {
  if (!canSubmit.value) return
  // 第一次点击只提示覆盖，保留用户已填写的名称和备注
  if (duplicate.value && !overwritePrompt.value) {
    overwritePrompt.value = true
    return
  }
  busy.value = true
  const result = await createBackup(
    workspaceId.value,
    name.value.trim(),
    note.value.trim(),
    overwritePrompt.value
  )
  busy.value = false
  if (result === 'exists') {
    overwritePrompt.value = true
    return
  }
  if (result === 'ok' && state.modal.kind === 'newBackup') state.modal = { kind: 'none' }
}

function cancel(): void {
  if (busy.value) return
  state.modal = { kind: 'none' }
}

function selectName(e: FocusEvent): void {
  ;(e.target as HTMLInputElement).select()
}
</script>

<template>
  <ModalShell v-if="workspaceId" title="新建备份" :busy="busy" @close="cancel">
    <div class="source-check">
      <div class="source-check-head">
        <HardDrive :size="13" class="dim-icon" />
        <span>存档来源</span>
      </div>
      <code class="source-check-path">{{ status?.path || state.config?.game.sourcePath }}</code>
      <div v-if="state.sourceStatusLoading" class="hint compact">正在检查来源目录…</div>
      <div v-else-if="!status" class="warn-text compact">无法读取来源状态，请检查设置。</div>
      <div v-else-if="!status.exists" class="warn-text compact">来源目录不存在，请先在设置中重新选择。</div>
      <div v-else-if="status.fileCount === 0" class="warn-text compact">
        来源目录里没有可备份的存档文件，请先进入游戏保存一次。
      </div>
      <div v-else class="source-check-meta">
        <span>{{ status.fileCount }} 个文件</span>
        <span>{{ formatSize(status.totalSize) }}</span>
        <span v-if="status.modifiedAt">最后写入 {{ formatTime(status.modifiedAt) }}</span>
      </div>
      <div v-if="state.gameRunning" class="warn-text compact source-check-warn">
        <Gamepad2 :size="12" />
        游戏正在运行：存档可能正在写入，建议保存退出后再创建备份。
      </div>
    </div>

    <label class="field">
      <span>备份名称</span>
      <input
        v-model="name"
        type="text"
        data-autofocus
        spellcheck="false"
        @focus="selectName"
        @keyup.enter="submit"
      />
      <span v-if="nameError" class="field-error">{{ nameError }}</span>
    </label>
    <label class="field">
      <span>备注（可选）</span>
      <textarea v-model="note" rows="2" placeholder="例如：第一次通关前的进度"></textarea>
    </label>

    <div v-if="overwritePrompt" class="damage-banner">
      <AlertTriangle :size="14" class="warn-icon" />
      <span>备份「{{ name.trim() }}」已存在，继续将删除原备份并写入当前存档。</span>
    </div>

    <ProgressInline v-if="busy" />

    <template #actions>
      <button class="btn" :disabled="busy" @click="cancel">取消</button>
      <button
        class="btn"
        :class="overwritePrompt ? 'btn-danger' : 'btn-primary'"
        :disabled="busy || !canSubmit"
        @click="submit"
      >
        {{
          busy
            ? '创建中…'
            : overwritePrompt
              ? '确认覆盖并创建'
              : duplicate
                ? '覆盖并创建'
                : '创建备份'
        }}
      </button>
    </template>
  </ModalShell>
</template>
