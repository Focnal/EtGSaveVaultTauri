<script setup lang="ts">
import { AlertTriangle } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { state } from '../store'

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
  state.modal = { kind: 'none' }
}
</script>

<template>
  <div v-if="m" class="modal-mask" @click.self="cancel">
    <div class="modal">
      <div class="modal-title">
        <AlertTriangle v-if="m.danger" :size="16" class="warn-icon" />
        {{ m.title }}
      </div>
      <p class="modal-msg">{{ m.message }}</p>
      <div class="modal-actions">
        <button class="btn" @click="cancel">取消</button>
        <button
          class="btn"
          :class="m.danger ? 'btn-danger' : 'btn-primary'"
          :disabled="busy"
          @click="confirm"
        >
          {{ busy ? '处理中…' : m.confirmText }}
        </button>
      </div>
    </div>
  </div>
</template>
