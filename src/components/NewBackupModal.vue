<script setup lang="ts">
import { computed, ref } from 'vue'
import { timestampName } from '../format'
import { createBackup, state } from '../store'

const workspaceId = computed(() => (state.modal.kind === 'newBackup' ? state.modal.workspaceId : ''))
const name = ref(timestampName())
const note = ref('')
const busy = ref(false)

async function submit(): Promise<void> {
  if (!name.value.trim() || busy.value) return
  busy.value = true
  const ok = await createBackup(workspaceId.value, name.value.trim(), note.value.trim(), false)
  busy.value = false
  if (ok && state.modal.kind === 'newBackup') state.modal = { kind: 'none' }
}

function cancel(): void {
  state.modal = { kind: 'none' }
}
</script>

<template>
  <div v-if="workspaceId" class="modal-mask" @click.self="cancel">
    <div class="modal">
      <div class="modal-title">新建备份</div>
      <label class="field">
        <span>备份名称</span>
        <input v-model="name" type="text" autofocus @keyup.enter="submit" />
      </label>
      <label class="field">
        <span>备注（可选）</span>
        <textarea v-model="note" rows="2" placeholder="例如：第一次通关前的进度"></textarea>
      </label>
      <div class="modal-actions">
        <button class="btn" @click="cancel">取消</button>
        <button class="btn btn-primary" :disabled="busy || !name.trim()" @click="submit">
          {{ busy ? '创建中…' : '创建备份' }}
        </button>
      </div>
    </div>
  </div>
</template>
