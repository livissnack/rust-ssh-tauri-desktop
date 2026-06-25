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
  isLocalSession?: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggleViewMode'): void;
  (e: 'connect'): void;
}>();

const activeServer = computed(() => {
  if (props.activeSessionId) {
    const session = props.openSessions?.find(s => s.id === props.activeSessionId);
    if (session && props.isLocalSession) {
      return { name: session.name };
    }
    if (session) {
      return props.servers?.find(s => s.id === session.serverId) ?? null;
    }
  }
  return props.currentServer ?? null;
});

const displayServerName = computed(() => {
  if (props.isLocalSession && props.activeSessionId) {
    const session = props.openSessions?.find(s => s.id === props.activeSessionId);
    if (session) return session.name;
  }
  if (activeServer.value?.name) return activeServer.value.name;
  return 'Select a host';
});

const displayHostMeta = computed(() => {
  if (props.isLocalSession) return 'Local Shell';
  const server = activeServer.value;
  if (!server?.host) return null;
  return `${server.username}@${server.host}:${server.port}`;
});

const statusClass = computed(() => {
  if (props.isConnecting) return 'is-connecting';
  if (props.isError) return 'is-error';
  if (props.activeSessionId) return 'is-active';
  return 'is-idle';
});

const statusLabel = computed(() => {
  if (props.isConnecting) return 'Connecting';
  if (props.isError) return 'Failed';
  if (props.activeSessionId) return 'Connected';
  return 'Ready';
});

const connectButtonText = computed(() => {
  if (props.isConnecting) return 'Connecting';
  return 'Connect';
});

const connectButtonIcon = computed(() => {
  if (props.isConnecting) return 'fa-circle-notch fa-spin';
  return 'fa-plug';
});

const viewModeLabel = computed(() => {
  if (props.currentViewMode === 'sftp') return 'Terminal';
  return props.isLocalSession ? 'Files' : 'SFTP';
});

const viewModeIcon = computed(() =>
  props.currentViewMode === 'sftp' ? 'fa-terminal' : 'fa-folder-open'
);

const viewModeTooltip = computed(() => {
  if (!props.activeSessionId) return '';
  if (props.isLocalSession) {
    return props.currentViewMode === 'sftp' ? 'Switch to Terminal' : 'Switch to Files';
  }
  return `Switch to ${viewModeLabel.value}`;
});
</script>

<template>
  <header class="workspace-header">
    <div class="header-left">
      <div class="header-icon" :class="statusClass">
        <i class="fas fa-server"></i>
      </div>

      <div class="header-info">
        <nav class="breadcrumb" aria-label="Breadcrumb">
          <span class="breadcrumb__root">Hosts</span>
          <i class="fas fa-chevron-right breadcrumb__sep" aria-hidden="true"></i>
          <span class="breadcrumb__current" :class="{ 'is-placeholder': !activeServer }">
            {{ displayServerName }}
          </span>
        </nav>

        <div v-if="displayHostMeta" class="header-meta">
          <span class="status-badge" :class="statusClass">
            <span class="status-badge__dot"></span>
            {{ statusLabel }}
          </span>
          <span class="header-meta__address">{{ displayHostMeta }}</span>
        </div>
        <p v-else class="header-hint">从侧栏选择主机以建立连接</p>
      </div>
    </div>

    <div class="toolbar">
      <Tooltip :text="viewModeTooltip">
        <button
            type="button"
            class="tool-btn tool-btn--mode"
            :class="{ 'is-sftp': currentViewMode === 'sftp' }"
            :disabled="!activeSessionId"
            @click="emit('toggleViewMode')"
        >
          <i class="fas" :class="viewModeIcon"></i>
          <span>{{ viewModeLabel }}</span>
        </button>
      </Tooltip>

      <button
          type="button"
          class="connect-btn"
          :class="{ 'is-loading': isConnecting }"
          :disabled="!activeId || isConnecting"
          @click="emit('connect')"
      >
        <i class="fas" :class="connectButtonIcon"></i>
        <span>{{ connectButtonText }}</span>
      </button>
    </div>
  </header>
</template>

<style lang="scss" scoped>
.workspace-header {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 20px;
  flex-shrink: 0;
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-30);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  flex: 1;
}

.header-icon {
  width: 38px;
  height: 38px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: var(--bg-input);
  border: 1px solid var(--border-30);
  color: var(--text-dim);
  font-size: 14px;
  transition: all 0.25s ease;

  &.is-active {
    background: var(--bg-input);
    border-color: var(--success);
    color: var(--success);
  }

  &.is-connecting {
    background: var(--accent-orange-10);
    border-color: var(--accent-orange-20);
    color: var(--accent-orange);
  }

  &.is-error {
    background: var(--error-15);
    border-color: var(--error-30);
    color: var(--error);
  }
}

.header-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;

  &__root {
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    letter-spacing: 0.02em;
  }

  &__sep {
    flex-shrink: 0;
    font-size: 8px;
    color: var(--text-dim);
    opacity: 0.45;
  }

  &__current {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 15px;
    font-weight: 700;
    color: var(--text-main);
    letter-spacing: -0.01em;
    line-height: 1.2;

    &.is-placeholder {
      font-weight: 500;
      color: var(--text-dim);
      font-size: 14px;
    }
  }
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;

  &__address {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-terminal);
    font-size: 11px;
    color: var(--text-dim);
    letter-spacing: 0.02em;
  }
}

.header-hint {
  margin: 0;
  font-size: 11px;
  color: var(--text-dim);
  opacity: 0.75;
}

.status-badge {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 18px;
  padding: 0 8px;
  border-radius: 9px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  background: var(--bg-input);
  border: 1px solid var(--border-30);
  color: var(--text-dim);

  &__dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  &.is-active {
    background: var(--bg-input);
    border-color: var(--success);
    color: var(--success);

    .status-badge__dot {
      box-shadow: 0 0 6px var(--success-60);
    }
  }

  &.is-connecting {
    background: var(--accent-orange-10);
    border-color: var(--accent-orange-20);
    color: var(--accent-orange);

    .status-badge__dot {
      animation: status-pulse 1.5s infinite;
    }
  }

  &.is-error {
    background: var(--error-15);
    border-color: var(--error-30);
    color: var(--error);

    .status-badge__dot {
      animation: error-shake 0.4s ease-in-out;
    }
  }
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  padding: 4px;
  border-radius: 11px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-30);
}

.tool-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;

  i { font-size: 12px; }

  &:hover:not(:disabled) {
    background: var(--accent-08);
    border-color: var(--accent-15);
    color: var(--accent);
  }

  &:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  &--mode.is-sftp {
    background: var(--accent-orange-10);
    border-color: var(--accent-orange-20);
    color: var(--accent-orange);

    &:hover {
      background: var(--accent-orange-20);
      border-color: var(--accent-orange-50);
    }
  }

  &:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
}

.connect-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 32px;
  padding: 0 16px;
  border: none;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 3px 12px var(--accent-20);

  i { font-size: 11px; }

  &:hover:not(:disabled) {
    filter: brightness(1.08);
    transform: translateY(-1px);
    box-shadow: 0 5px 16px var(--accent-30);
  }

  &:active:not(:disabled) {
    transform: translateY(0);
  }

  &:disabled {
    background: var(--bg-input);
    color: var(--text-dim);
    box-shadow: none;
    opacity: 0.55;
    cursor: not-allowed;
  }

  &.is-loading i {
    animation: spin 1s linear infinite;
  }

  &:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
}

@keyframes status-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.45; transform: scale(0.85); }
}

@keyframes error-shake {
  0%, 100% { transform: translateX(0); }
  25% { transform: translateX(-2px); }
  75% { transform: translateX(2px); }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
