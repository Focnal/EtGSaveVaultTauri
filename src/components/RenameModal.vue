<script setup lang="ts">
import { computed, ref } from 'vue'
import { validateBackupName } from '../format'
import { renameBackup, state } from '../store'
import ModalShell from './ModalShell.vue'

const m = computed(() => (state.modal.kind === 'rename' ? state.modal : null))
const newName = ref(m.value?.name ?? '')
const busy = ref(false)

const nameError = computed(() => (newName.value ? validateBackupName(newName.value) : null))
const unchanged = computed(() => newName.value.trim() === (m.value?.name ?? ''))
const canSubmit = computed(
  () => !!newName.value.trim() && !nameError.value && !unchanged.value && !busy.value
)

async function submit(): Promise<void> {
  const mm = m.value
  if (!mm || !canSubmit.value) return
  busy.value = true
  try {
    const ok = await renameBackup(mm.workspaceId, mm.name, newName.value.trim())
    if (ok) state.modal = { kind: 'none' }
  } finally {
    busy.value = false
  }
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
  <ModalShell v-if="m" title="重命名备份" :busy="busy" @close="cancel">
    <label class="field">
      <span>新名称</span>
      <input
        v-model="newName"
        type="text"
        data-autofocus
        spellcheck="false"
        @focus="selectName"
        @keyup.enter="submit"
      />
      <span v-if="nameError" class="field-error">{{ nameError }}</span>
      <span v-else-if="unchanged" class="field-hint">名称没有变化</span>
    </label>
    <template #actions>
      <button class="btn" :disabled="busy" @click="cancel">取消</button>
      <button class="btn btn-primary" :disabled="!canSubmit" @click="submit">
        {{ busy ? '保存中…' : '保存' }}
      </button>
    </template>
  </ModalShell>
</template>
