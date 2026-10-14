<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { closeMenu, runMenuEntry, state, type MenuEntry } from '../store'

const el = ref<HTMLElement | null>(null)
const pos = ref({ x: 0, y: 0 })

/** 贴边时向内收，避免被裁掉 */
function place(): void {
  const node = el.value
  const menu = state.menu
  if (!node || !menu) return
  const pad = 8
  const maxX = Math.max(pad, window.innerWidth - node.offsetWidth - pad)
  const maxY = Math.max(pad, window.innerHeight - node.offsetHeight - pad)
  pos.value = { x: Math.min(Math.max(pad, menu.x), maxX), y: Math.min(Math.max(pad, menu.y), maxY) }
}

function onPointerDown(e: MouseEvent): void {
  const node = el.value
  if (node && e.target instanceof Node && node.contains(e.target)) return
  closeMenu()
}

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') closeMenu()
}

function onDismiss(): void {
  closeMenu()
}

function pick(entry: MenuEntry): void {
  void runMenuEntry(entry)
}

onMounted(() => {
  void nextTick(place)
  // 捕获阶段监听，点击任何地方都能收起
  window.addEventListener('mousedown', onPointerDown, true)
  window.addEventListener('keydown', onKey)
  window.addEventListener('resize', onDismiss)
  window.addEventListener('blur', onDismiss)
  // 树视图滚动时菜单不应停在原地
  window.addEventListener('scroll', onDismiss, true)
})

onBeforeUnmount(() => {
  window.removeEventListener('mousedown', onPointerDown, true)
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('resize', onDismiss)
  window.removeEventListener('blur', onDismiss)
  window.removeEventListener('scroll', onDismiss, true)
})

// 已打开时右键另一行不会重新挂载，需要重新定位
watch(
  () => state.menu,
  () => void nextTick(place)
)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="state.menu"
      ref="el"
      class="context-menu"
      :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
    >
      <template v-for="(entry, i) in state.menu.entries" :key="i">
        <div v-if="'divider' in entry" class="menu-divider"></div>
        <button
          v-else
          class="menu-item"
          :class="{ 'menu-item-danger': entry.danger }"
          :disabled="entry.disabled"
          @click="pick(entry)"
        >
          <component :is="entry.icon" v-if="entry.icon" :size="14" />
          <span>{{ entry.label }}</span>
        </button>
      </template>
    </div>
  </Teleport>
</template>
