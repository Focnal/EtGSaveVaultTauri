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
  if (e.key === 'Escape') {
    closeMenu()
    return
  }
  if (!state.menu) return
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) return
  const items = menuButtons()
  if (items.length === 0) return
  e.preventDefault()
  const current = items.indexOf(document.activeElement as HTMLElement)
  let next = current
  if (e.key === 'ArrowDown') next = current < 0 ? 0 : (current + 1) % items.length
  else if (e.key === 'ArrowUp') next = current <= 0 ? items.length - 1 : current - 1
  else if (e.key === 'Home') next = 0
  else if (e.key === 'End') next = items.length - 1
  items[next]?.focus()
}

function menuButtons(): HTMLElement[] {
  const node = el.value
  if (!node) return []
  return Array.from(node.querySelectorAll<HTMLElement>('.menu-item:not([disabled])'))
}

function focusFirstItem(): void {
  menuButtons()[0]?.focus()
}

function onDismiss(): void {
  closeMenu()
}

function pick(entry: MenuEntry): void {
  void runMenuEntry(entry)
}

onMounted(() => {
  void nextTick(() => {
    place()
    focusFirstItem()
  })
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
  () =>
    void nextTick(() => {
      place()
      focusFirstItem()
    })
)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="state.menu"
      ref="el"
      class="context-menu"
      role="menu"
      :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
    >
      <template v-for="(entry, i) in state.menu.entries" :key="i">
        <div v-if="'divider' in entry" class="menu-divider" role="separator"></div>
        <button
          v-else
          class="menu-item"
          role="menuitem"
          :class="{ 'menu-item-danger': entry.danger }"
          :disabled="entry.disabled"
          :aria-disabled="entry.disabled || undefined"
          :title="entry.hint"
          @click="pick(entry)"
        >
          <component :is="entry.icon" v-if="entry.icon" :size="14" />
          <span>{{ entry.label }}</span>
        </button>
      </template>
    </div>
  </Teleport>
</template>
