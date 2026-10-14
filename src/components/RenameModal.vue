<script setup lang="ts">
import { computed, ref } from 'vue'
import { renameBackup, state } from '../store'

const m = computed(() => (state.modal.kind === 'rename' ? state.modal : null))
const newName = ref(m.value?.name ?? '')
const busy = ref(false)

async function submit(): Promise<void> {
  const mm = m.value
  if (!mm || !newName.value.trim() || busy.value) return
  busy.value = true
  const ok = await renameBackup(mm.workspaceId, mm.name, newName.value.trim())
  busy.value = false
  if (ok) state.modal = { kind: 'none' }
}

function cancel(): void {
  state.modal = { kind: 'none' }
}
</script>

<template>
  <div v-if="m" class="modal-mask" @click.self="cancel">
    <div class="modal">
      <div class="modal-title">重命名备份</div>
      <label class="field">
        <span>新名称</span>
        <input v-model="newName" type="text" autofocus @keyup.enter="submit" />
      </label>
      <div class="modal-actions">
        <button class="btn" @click="cancel">取消</button>
        <button class="btn btn-primary" :disabled="busy || !newName.trim()" @click="submit">
          {{ busy ? '保存中…' : '保存' }}
        </button>
      </div>
    </div>
  </div>
</template>
