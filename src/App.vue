<script setup lang="ts">
import {
  AlertTriangle,
  Archive,
  CheckCircle2,
  Gamepad2,
  History,
  Info,
  LoaderCircle,
  Plus,
  Settings
} from 'lucide-vue-next'
import { computed, onMounted } from 'vue'
import BackupDetail from './components/BackupDetail.vue'
import ConfirmModal from './components/ConfirmModal.vue'
import ContextMenu from './components/ContextMenu.vue'
import NewBackupModal from './components/NewBackupModal.vue'
import RenameModal from './components/RenameModal.vue'
import RollbackPanel from './components/RollbackPanel.vue'
import SettingsModal from './components/SettingsModal.vue'
import Sidebar from './components/Sidebar.vue'
import WorkspaceView from './components/WorkspaceView.vue'
import { addWorkspace, init, refreshGameStatus, runToastAction, state } from './store'

onMounted(() => {
  void init()
})

const rollbackCount = computed(() => state.rollbacks?.points.length ?? 0)

const progressIndeterminate = computed(() => !!state.progress && !state.progress.total)

const progressPct = computed(() => {
  const p = state.progress
  if (!p || !p.total) return 0
  return Math.min(100, Math.round((p.done / p.total) * 100))
})

const gameStatusText = computed(() => {
  if (state.gameRunning === null) return '状态未知'
  return state.gameRunning ? '游戏运行中' : '游戏未运行'
})

const gameStatusClass = computed(() => {
  if (state.gameRunning === null) return 'chip-neutral'
  return state.gameRunning ? 'chip-warn' : 'chip-ok'
})
</script>

<template>
  <div v-if="state.bootError" class="boot-screen">
    <AlertTriangle :size="34" class="warn-icon" />
    <p class="boot-title">应用初始化失败</p>
    <p class="boot-message">{{ state.bootError }}</p>
    <button class="btn btn-primary" @click="init">重试</button>
  </div>

  <div v-else-if="!state.config" class="boot-screen">
    <LoaderCircle :size="28" class="spin" />
    <p class="boot-message">正在读取配置…</p>
  </div>

  <div v-else class="app">
    <header class="topbar">
      <div class="brand">
        <Archive :size="17" class="brand-icon" />
        <span>EtG Save Vault</span>
      </div>
      <div class="topbar-right">
        <button
          class="chip"
          title="回档点：恢复备份前自动保留的存档"
          @click="state.modal = { kind: 'rollback' }"
        >
          <History :size="13" />
          <span>回档点</span>
          <span v-if="rollbackCount" class="chip-count">{{ rollbackCount }}</span>
        </button>
        <button
          class="chip"
          :class="gameStatusClass"
          title="点击刷新游戏进程状态"
          :disabled="state.gameStatusLoading"
          @click="refreshGameStatus(true)"
        >
          <LoaderCircle v-if="state.gameStatusLoading" :size="13" class="spin" />
          <Gamepad2 v-else :size="13" />
          <span>{{ gameStatusText }}</span>
        </button>
        <button class="icon-btn" title="设置" @click="state.modal = { kind: 'settings' }">
          <Settings :size="16" />
        </button>
      </div>
    </header>

    <div v-if="state.progress" class="progress-strip">
      <span class="progress-label">
        {{ state.progress.label }}
        <template v-if="state.progress.total">
          {{ state.progress.done }}/{{ state.progress.total }}
        </template>
      </span>
      <div class="progress-track">
        <div
          class="progress-fill"
          :class="{ indeterminate: progressIndeterminate }"
          :style="progressIndeterminate ? undefined : { width: progressPct + '%' }"
        ></div>
      </div>
    </div>

    <div class="body">
      <Sidebar />
      <main class="main">
        <BackupDetail v-if="state.selectedBackupName && state.detail" />
        <div v-else-if="state.detailLoading" class="detail-loading">
          <LoaderCircle :size="22" class="spin" />
          <span>正在读取备份详情…</span>
        </div>
        <WorkspaceView v-else-if="state.selectedWorkspaceId" />
        <div v-else-if="state.config.workspaces.length === 0" class="empty-state">
          <Archive :size="42" :stroke-width="1.1" />
          <p>还没有工作区</p>
          <p class="dim">添加一个用于存放备份的目录，或选择已有的备份目录。</p>
          <button class="btn btn-primary" @click="addWorkspace">
            <Plus :size="14" />
            添加工作区目录
          </button>
        </div>
        <div v-else class="empty-state">
          <Archive :size="42" :stroke-width="1.1" />
          <p>在左侧选择一个备份查看详情</p>
          <p class="dim">也可以直接在工作区中新建备份。</p>
        </div>
      </main>
    </div>

    <SettingsModal v-if="state.modal.kind === 'settings'" />
    <RollbackPanel v-if="state.modal.kind === 'rollback'" />
    <NewBackupModal v-if="state.modal.kind === 'newBackup'" />
    <RenameModal v-if="state.modal.kind === 'rename'" />
    <ConfirmModal v-if="state.modal.kind === 'confirm'" />
    <ContextMenu v-if="state.menu" />

    <div class="toasts">
      <div v-for="t in state.toasts" :key="t.id" class="toast" :class="'toast-' + t.kind">
        <CheckCircle2 v-if="t.kind === 'ok'" :size="15" />
        <AlertTriangle v-else-if="t.kind === 'err'" :size="15" />
        <Info v-else :size="15" />
        <span class="toast-text">{{ t.text }}</span>
        <button v-if="t.action" class="toast-action" @click="runToastAction(t)">
          {{ t.action.label }}
        </button>
      </div>
    </div>
  </div>
</template>
