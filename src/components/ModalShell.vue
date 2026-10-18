<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

/**
 * 统一弹窗外壳：
 * - Esc 关闭（busy 时忽略）
 * - 打开时聚焦首个可交互元素，Tab 在弹窗内循环
 * - 关闭后把焦点还给触发元素
 * - 遮罩点击可配置为关闭，busy 时始终阻止
 */
const props = withDefaults(
  defineProps<{
    title: string
    wide?: boolean
    busy?: boolean
    closeOnMask?: boolean
    actionsClass?: string
  }>(),
  { wide: false, busy: false, closeOnMask: true, actionsClass: '' }
)

const emit = defineEmits<{ close: [] }>()

const panel = ref<HTMLElement | null>(null)
let restoreFocus: HTMLElement | null = null

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'

function focusables(): HTMLElement[] {
  const node = panel.value
  if (!node) return []
  return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) => el.offsetParent !== null || el === document.activeElement
  )
}

function requestClose(): void {
  if (props.busy) return
  emit('close')
}

function onMaskDown(): void {
  if (!props.closeOnMask) return
  requestClose()
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    // busy 时不让 Esc 把正在执行的操作「藏起来」
    if (!props.busy) {
      e.preventDefault()
      e.stopPropagation()
      requestClose()
    }
    return
  }
  if (e.key !== 'Tab') return
  const list = focusables()
  if (list.length === 0) {
    e.preventDefault()
    return
  }
  const first = list[0]
  const last = list[list.length - 1]
  const active = document.activeElement as HTMLElement | null
  if (!panel.value?.contains(active)) {
    e.preventDefault()
    first.focus()
    return
  }
  if (!e.shiftKey && active === last) {
    e.preventDefault()
    first.focus()
  } else if (e.shiftKey && active === first) {
    e.preventDefault()
    last.focus()
  }
}

onMounted(() => {
  restoreFocus = document.activeElement as HTMLElement | null
  document.addEventListener('keydown', onKeydown, true)
  void nextTick(() => {
    const preferred =
      panel.value?.querySelector<HTMLElement>('[data-autofocus]') ?? focusables()[0] ?? null
    preferred?.focus()
  })
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown, true)
  if (restoreFocus?.isConnected) restoreFocus.focus()
})
</script>

<template>
  <Teleport to="body">
    <div class="modal-mask" @mousedown.self="onMaskDown">
      <div
        ref="panel"
        class="modal"
        :class="{ 'modal-wide': wide }"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
      >
        <div class="modal-title">
          <slot name="icon" />
          {{ title }}
        </div>
        <slot />
        <div class="modal-actions" :class="actionsClass">
          <slot name="actions" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
