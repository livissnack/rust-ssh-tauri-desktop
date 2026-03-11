<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  currentServer: any;
  activeId: string | null;
  activeSessionId: string | null;
  isConnecting: boolean;
  currentViewMode: 'terminal' | 'sftp';
  openSessions: Array<{ id: string; serverId: string; name: string }>;
  servers: any[];
}>();

const emit = defineEmits<{
  (e: 'toggleViewMode'): void;
  (e: 'cloneSession'): void;
  (e: 'connect'): void;
}>();

const displayServerName = computed(() => {
  // 优先显示当前会话对应的服务器名称
  if (props.activeSessionId) {
    const session = props.openSessions?.find(s => s.id === props.activeSessionId);
    if (session) {
      const server = props.servers?.find(s => s.id === session.serverId);
      if (server) {
        return server.name;
      }
    }
  }

  // 如果没有活动会话，显示选中的服务器
  if (props.currentServer) {
    return props.currentServer.name;
  }

  return 'Select a host';
});

const connectButtonText = computed(() => {
  if (props.isConnecting) return 'Connecting';
  return 'Connect';
});

const connectButtonIcon = computed(() => {
  if (props.isConnecting) return 'fa-circle-notch fa-spin';
  return 'fa-plug';
});
</script>

<template>
  <header class="workspace-header">
    <div class="breadcrumb">
      <span class="root">Hosts</span>
      <span class="sep">/</span>
      <span class="current">{{ displayServerName }}</span>
    </div>
    <div class="toolbar">
      <button
          class="action-btn mode-toggle"
          :class="{ 'is-sftp': currentViewMode === 'sftp' }"
          @click="emit('toggleViewMode')"
      >
        <i class="fas" :class="currentViewMode === 'sftp' ? 'fa-terminal' : 'fa-folder-open'"></i>
        <span>{{ currentViewMode === 'sftp' ? 'Terminal' : 'SFTP' }}</span>
      </button>

      <div class="separator"></div>

      <button
          class="action-btn clone-btn"
          @click="emit('cloneSession')"
          :disabled="!activeSessionId"
      >
        <i class="fas fa-copy"></i>
        <span>Clone</span>
      </button>

      <button
          class="connect-btn"
          @click="emit('connect')"
          :disabled="!activeId || isConnecting"
          :class="{ 'loading': isConnecting }"
      >
        <i class="fas" :class="connectButtonIcon"></i>
        <span>{{ connectButtonText }}</span>
      </button>
    </div>
  </header>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.workspace-header {
  height: 56px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  flex-shrink: 0;
  background: var(--bg-primary); // 保持与内容区连贯

  .breadcrumb {
    font-size: 13px;
    display: flex;
    align-items: center;

    .root {
      color: var(--text-dim);
      transition: color 0.2s;
      &:hover {
        color: var(--text-main);
        cursor: pointer;
      }
    }

    .sep {
      color: var(--border);
      margin: 0 10px;
      font-size: 10px;
    }

    .current {
      color: var(--text-main);
      font-weight: 600;
      letter-spacing: 0.3px;
    }
  }
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  background: var(--bg-secondary); // 工具栏使用次要背景
  border-radius: 10px;
  border: 1px solid var(--border);

  .separator {
    width: 1px;
    height: 18px;
    background: var(--border);
    margin: 0 4px;
    opacity: 0.6;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-dim); // 使用统一 dim 变量
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

    i { font-size: 14px; }

    &:hover:not(:disabled) {
      background: var(--accent-10); // 修复点：使用预计算透明变量
      color: var(--accent);
      border-color: var(--accent-20);
    }

    &:disabled {
      opacity: 0.3;
      cursor: not-allowed;
    }

    /* SFTP 模式特定样式 - 使用橙色/警告色变量 */
    &.mode-toggle.is-sftp {
      background: var(--accent-orange-10, rgba(249, 115, 22, 0.1));
      color: var(--accent-orange, #f97316);
      border-color: var(--accent-orange-20, rgba(249, 115, 22, 0.2));

      &:hover {
        background: var(--accent-orange-20);
        border-color: var(--accent-orange);
      }
    }
  }

  /* 主连接按钮 */
  .connect-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    border-radius: 6px;
    border: none;
    background: var(--accent);
    color: var(--bg-primary); // 按钮文字使用深色以增强对比
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 12px var(--accent-20); // 修复点

    &:hover:not(:disabled) {
      filter: brightness(1.1);
      transform: translateY(-1px);
      box-shadow: 0 6px 16px var(--accent-30);
    }

    &:active {
      transform: translateY(0);
      filter: brightness(0.9);
    }

    &:disabled {
      background: var(--bg-input);
      color: var(--text-dim);
      box-shadow: none;
      cursor: not-allowed;
      opacity: 0.6;
    }

    &.loading i {
      animation: spin 1s linear infinite;
    }
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
