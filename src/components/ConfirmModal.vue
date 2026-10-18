<script setup lang="ts">
import { AlertTriangle } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { state } from '../store'
import ModalShell from './ModalShell.vue'

const m = computed(() => (state.modal.kind === 'confirm' ? state.modal : null))
const busy = ref(false)

async function confirm(): Promise<void> {
  if (!m.value || busy.value) return
  busy.value = true
  try {
    await m.value.onConfirm()
  } finally {
    busy.value = false
    state.modal = { kind: 'none' }
  }
}

function cancel(): void {
  if (busy.value) return
  state.modal = { kind: 'none' }
}
</script>

<template>
  <ModalShell v-if="m" :title="m.title" :busy="busy" @close="cancel">
    <template #icon>
      <AlertTriangle :size="16" :class="m.danger ? 'danger-icon' : 'warn-icon'" />
    </template>
    <p class="modal-msg">{{ m.message }}</p>
    <template #actions>
      <button class="btn" data-autofocus :disabled="busy" @click="cancel">取消</button>
      <button
        class="btn"
        :class="m.danger ? 'btn-danger' : 'btn-primary'"
        :disabled="busy"
        @click="confirm"
      >
        {{ busy ? '处理中…' : m.confirmText }}
      </button>
    </template>
  </ModalShell>
</template>
