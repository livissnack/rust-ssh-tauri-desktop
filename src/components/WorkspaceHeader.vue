<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  currentServer: any;
  activeId: string | null;
  activeSessionId: string | null;
  isConnecting: boolean;
  isError?: boolean;
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

const statusClass = computed(() => {
  if (props.isConnecting) return 'is-connecting';
  if (props.isError) return 'is-error';
  if (props.activeSessionId) return 'is-active';
  return '';
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
      <div class="breadcrumb-item">
        <span>Hosts</span>
      </div>
      <span class="sep">/</span>
      <div class="breadcrumb-item current">
        <span
            class="status-indicator"
            v-if="activeSessionId || isConnecting || isError"
            :class="statusClass"
        ></span>
        <span class="name">{{ displayServerName }}</span>
      </div>
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
  background: var(--bg-primary);

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px; // 整体间距缩小，依靠 padding 撑开点击区域
    font-family: 'Inter', sans-serif;

    .breadcrumb-item {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 6px 10px;
      border-radius: 6px;
      transition: all 0.2s ease;

      &.current {
        color: var(--text-main);
        font-weight: 700;
        letter-spacing: -0.2px; // 略微收紧字间距，更有现代感
        font-size: 14px;

        /* 在 .breadcrumb-item.current 内部修改 status-indicator */
        .status-indicator {
          width: 8px;  // 稍微调大一点点更醒目
          height: 8px;
          border-radius: 50%;
          margin-right: 8px;
          transition: all 0.3s ease;

          // 1. 在线状态 (绿色)
          &.is-active {
            background: var(--success);
            box-shadow: 0 0 8px var(--success-60, rgba(34, 197, 94, 0.4));
          }

          // 2. 失败状态 (红色)
          &.is-error {
            background: var(--danger, #ef4444);
            box-shadow: 0 0 8px rgba(239, 68, 68, 0.5);
            animation: error-shake 0.4s ease-in-out; // 失败时抖动一下
          }

          // 3. 连接中状态 (黄色/橙色)
          &.is-connecting {
            background: var(--accent-orange, #f97316);
            box-shadow: 0 0 8px rgba(249, 115, 22, 0.4);
            animation: status-pulse 1.5s infinite; // 正在连接时有呼吸效果
          }
        }

        // 呼吸灯动画
        @keyframes status-pulse {
          0% { opacity: 1; transform: scale(1); }
          50% { opacity: 0.5; transform: scale(1.1); }
          100% { opacity: 1; transform: scale(1); }
        }

        // 错误时的轻微抖动
        @keyframes error-shake {
          0%, 100% { transform: translateX(0); }
          25% { transform: translateX(-2px); }
          75% { transform: translateX(2px); }
        }
      }
    }

    .sep {
      color: var(--text-dim);
      font-family: "JetBrains Mono", "Cascadia Code", monospace;
      font-size: 14px;
      font-weight: 300;
      user-select: none;
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }
  }
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  background: var(--bg-secondary);
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
