<script setup lang="ts">
import { computed, ref } from 'vue'
import { renameBackup, state } from '../store'
import ModalShell from './ModalShell.vue'

const m = computed(() => (state.modal.kind === 'rename' ? state.modal : null))
const newName = ref(m.value?.name ?? '')
const busy = ref(false)

async function submit(): Promise<void> {
  const mm = m.value
  if (!mm || !newName.value.trim() || busy.value) return
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
</script>

<template>
  <ModalShell v-if="m" title="重命名备份" :busy="busy" @close="cancel">
    <label class="field">
      <span>新名称</span>
      <input v-model="newName" type="text" data-autofocus spellcheck="false" @keyup.enter="submit" />
    </label>
    <template #actions>
      <button class="btn" :disabled="busy" @click="cancel">取消</button>
      <button class="btn btn-primary" :disabled="busy || !newName.trim()" @click="submit">
        {{ busy ? '保存中…' : '保存' }}
      </button>
    </template>
  </ModalShell>
</template>
