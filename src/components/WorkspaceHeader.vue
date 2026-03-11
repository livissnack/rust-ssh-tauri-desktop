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
  background: base.$bg-primary; // 修改：使用主背景色，确保与内容区连贯

  .breadcrumb {
    font-size: 13px;
    display: flex;
    align-items: center;

    .root {
      color: base.$text-dim;
      transition: color 0.2s;
      &:hover { color: base.$text-main; cursor: pointer; }
    }

    .sep {
      color: base.$border;
      margin: 0 10px;
      font-size: 10px;
    }

    .current {
      color: base.$text-main; // 修改：跟随主题文字色
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
  background: base.$bg-secondary; // 修改：使用次要背景色营造工具栏感
  border-radius: 10px;
  border: 1px solid base.$border;

  .separator {
    width: 1px;
    height: 18px;
    background: base.$border;
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
    color: base.$text-muted;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

    i { font-size: 14px; }

    &:hover:not(:disabled) {
      background: rgba(base.$accent, 0.1);
      color: base.$accent;
      border-color: rgba(base.$accent, 0.15);
    }

    &:disabled {
      opacity: 0.3;
      cursor: not-allowed;
    }

    /* SFTP 模式特定样式 - 使用主题橙色/警告色 */
    &.mode-toggle.is-sftp {
      background: rgba(base.$accent-orange, 0.1);
      color: base.$accent-orange;
      border-color: rgba(base.$accent-orange, 0.2);

      &:hover {
        background: rgba(base.$accent-orange, 0.2);
        border-color: base.$accent-orange;
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
    background: base.$accent;
    color: base.$bg-primary; // 修改：按钮文字使用背景深色以获得最高对比度
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 4px 12px rgba(base.$accent, 0.2);

    &:hover:not(:disabled) {
      filter: brightness(1.1);
      transform: translateY(-1px);
      box-shadow: 0 6px 16px rgba(base.$accent, 0.3);
    }

    &:active {
      transform: translateY(0);
      filter: brightness(0.9);
    }

    &:disabled {
      background: base.$bg-input;
      color: base.$text-dim;
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
