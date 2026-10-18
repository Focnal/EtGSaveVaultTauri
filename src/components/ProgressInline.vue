<script setup lang="ts">
import { computed } from 'vue'
import { state } from '../store'

const p = computed(() => state.progress)
const pct = computed(() => {
  const cur = p.value
  if (!cur || !cur.total) return 0
  return Math.min(100, Math.round((cur.done / cur.total) * 100))
})
const indeterminate = computed(() => !!p.value && !p.value.total)
</script>

<template>
  <div v-if="p" class="progress-inline" role="status" aria-live="polite">
    <div class="progress-inline-head">
      <span>{{ p.label }}</span>
      <span v-if="p.total" class="dim">{{ p.done }}/{{ p.total }}</span>
    </div>
    <div class="progress-inline-track">
      <div
        class="progress-inline-fill"
        :class="{ indeterminate }"
        :style="indeterminate ? undefined : { width: pct + '%' }"
      ></div>
    </div>
  </div>
</template>
