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
$accent: #7aa2f7;
$border-color: #292e42;
$text-dim: #565f89;

.workspace-header {
  height: 56px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  flex-shrink: 0;
  background: #0f111a;

  .breadcrumb {
    font-size: 13px;

    .current {
      color: #fff;
      font-weight: 600;
    }

    .sep {
      color: $text-dim;
      margin: 0 8px;
    }

    .root {
      color: $text-dim;
    }
  }
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  background: rgba(26, 27, 38, 0.5);
  border-radius: 10px;
  border: 1px solid $border-color;

  .separator {
    width: 1px;
    height: 20px;
    background: $border-color;
    margin: 0 4px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid transparent;
    background: transparent;
    color: #a9b1d6;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

    i {
      font-size: 14px;
    }

    &:hover:not(:disabled) {
      background: rgba(122, 162, 247, 0.1);
      color: $accent;
      border-color: rgba(122, 162, 247, 0.2);
    }

    &:disabled {
      opacity: 0.3;
      cursor: not-allowed;
    }
  }

  .mode-toggle.is-sftp {
    background: rgba(224, 175, 104, 0.1);
    color: #e0af68;
    border-color: rgba(224, 175, 104, 0.3);

    &:hover {
      background: rgba(224, 175, 104, 0.2);
    }
  }

  .connect-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    border-radius: 6px;
    border: none;
    background: $accent;
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 2px 8px rgba(122, 162, 247, 0.3);

    &:hover:not(:disabled) {
      background: #89ddff;
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(122, 162, 247, 0.4);
    }

    &:active {
      transform: translateY(0);
    }

    &:disabled {
      background: #24283b;
      color: $text-dim;
      box-shadow: none;
      cursor: not-allowed;
    }

    &.loading i {
      animation: spin 1s linear infinite;
    }
  }
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
