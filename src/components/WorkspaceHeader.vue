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

const hasActiveHost = computed(() => !!activeServer.value?.name && activeServer.value.name !== 'Select a host');

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

const headerIconClass = computed(() => {
  if (props.isLocalSession) return 'fa-laptop-code';
  if (props.currentViewMode === 'sftp' && props.activeSessionId) {
    return props.isLocalSession ? 'fa-folder-open' : 'fa-folder-tree';
  }
  return 'fa-server';
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
  <header class="workspace-header" :class="statusClass">
    <div class="header-accent" aria-hidden="true"></div>

    <div class="header-body">
      <div class="header-left">
        <div class="header-icon" :class="[statusClass, { 'is-local': isLocalSession }]">
          <i class="fas" :class="headerIconClass"></i>
        </div>

        <div class="header-info">
          <div class="header-title-row">
            <span class="header-title" :class="{ 'is-placeholder': !hasActiveHost }">
              {{ displayServerName }}
            </span>
          </div>

          <div v-if="displayHostMeta" class="header-meta">
            <span class="status-dot" :class="statusClass" aria-hidden="true"></span>
            <span class="status-text" :class="statusClass">{{ statusLabel }}</span>
            <span class="meta-sep">·</span>
            <span class="header-meta__address">{{ displayHostMeta }}</span>
          </div>
          <p v-else class="header-hint">Select a host from the sidebar</p>
        </div>
      </div>

      <div class="toolbar">
        <Tooltip :text="viewModeTooltip">
          <button
            type="button"
            class="mode-toggle"
            :class="{ 'is-alt': currentViewMode === 'sftp', 'is-disabled': !activeSessionId }"
            :disabled="!activeSessionId"
            @click="emit('toggleViewMode')"
          >
            <i class="fas" :class="viewModeIcon"></i>
            <span>{{ viewModeLabel }}</span>
          </button>
        </Tooltip>

        <span class="toolbar-split" aria-hidden="true"></span>

        <button
          type="button"
          class="connect-btn"
          :class="{ 'is-loading': isConnecting }"
          :disabled="!activeId || isConnecting"
          @click="emit('connect')"
        >
          <span class="connect-btn__shine" aria-hidden="true"></span>
          <i class="fas" :class="isConnecting ? 'fa-circle-notch fa-spin' : 'fa-plug'"></i>
          <span>{{ connectButtonText }}</span>
        </button>
      </div>
    </div>
  </header>
</template>

<style lang="scss" scoped>
@use 'sass:color';

.workspace-header {
  position: relative;
  flex-shrink: 0;
  background: linear-gradient(
    180deg,
    color-mix(in srgb, var(--bg-secondary) 88%, var(--accent) 12%) 0%,
    var(--bg-primary) 100%
  );
  border-bottom: 1px solid var(--border-30);
  overflow: hidden;

  &.is-active .header-accent {
    background: linear-gradient(90deg, transparent, var(--success), transparent);
    opacity: 0.85;
  }

  &.is-connecting .header-accent {
    background: linear-gradient(90deg, transparent, var(--accent-orange), transparent);
    animation: accent-flow 1.8s ease-in-out infinite;
  }

  &.is-error .header-accent {
    background: linear-gradient(90deg, transparent, var(--error), transparent);
    opacity: 0.9;
  }
}

.header-accent {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, transparent, var(--accent-30), transparent);
  opacity: 0.55;
  pointer-events: none;
}

.header-body {
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 16px 0 18px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  flex: 1;
}

.header-icon {
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border-30);
  color: var(--text-dim);
  font-size: 12px;
  transition: border-color 0.2s ease, color 0.2s ease;

  &.is-local {
    color: var(--accent-alt);
  }

  &.is-active {
    color: var(--success);
    border-color: color-mix(in srgb, var(--success) 30%, var(--border-30));
  }

  &.is-connecting {
    color: var(--accent-orange);
    border-color: color-mix(in srgb, var(--accent-orange) 30%, var(--border-30));
  }

  &.is-error {
    color: var(--error);
    border-color: color-mix(in srgb, var(--error) 30%, var(--border-30));
  }
}

.header-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 2px;
}

.header-title-row {
  display: flex;
  align-items: center;
  min-width: 0;
}

.header-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main);
  line-height: 1.2;

  &.is-placeholder {
    font-weight: 500;
    color: var(--text-dim);
  }
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  font-size: 11px;
  line-height: 1.2;
}

.status-dot {
  width: 5px;
  height: 5px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--text-dim);

  &.is-active {
    background: var(--success);
    box-shadow: 0 0 6px var(--success-60);
  }

  &.is-connecting {
    background: var(--accent-orange);
    animation: status-pulse 1.5s infinite;
  }

  &.is-error {
    background: var(--error);
  }
}

.status-text {
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 500;
  color: var(--text-dim);

  &.is-active { color: var(--success); }
  &.is-connecting { color: var(--accent-orange); }
  &.is-error { color: var(--error); }
}

.meta-sep {
  flex-shrink: 0;
  color: var(--text-dim);
  opacity: 0.45;
  user-select: none;
}

.header-meta__address {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-terminal);
  font-size: 10px;
  color: var(--text-dim);
}

.header-hint {
  margin: 0;
  font-size: 10px;
  color: var(--text-dim);
  opacity: 0.8;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  padding: 4px 6px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-secondary) 70%, transparent);
  border: 1px solid var(--border-30);
  backdrop-filter: blur(8px);
}

.toolbar-split {
  width: 1px;
  height: 22px;
  background: var(--border-30);
  flex-shrink: 0;
}

.mode-toggle {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--border-30);
  border-radius: 7px;
  background: var(--bg-card);
  color: var(--text-main);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.22s ease;

  i {
    font-size: 10px;
    color: var(--accent);
  }

  &:hover:not(:disabled) {
    border-color: var(--accent-30);
    box-shadow: 0 2px 10px color-mix(in srgb, var(--accent) 10%, transparent);
  }

  &.is-alt {
    border-color: color-mix(in srgb, var(--accent-orange) 30%, var(--border-30));
    background: color-mix(in srgb, var(--accent-orange) 6%, var(--bg-card));

    i {
      color: var(--accent-orange);
    }
  }

  &.is-disabled,
  &:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    box-shadow: none;
  }

  &:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
}

.connect-btn {
  position: relative;
  overflow: hidden;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 12px;
  border: none;
  border-radius: 7px;
  background: linear-gradient(
    135deg,
    var(--accent) 0%,
    color-mix(in srgb, var(--accent) 78%, var(--accent-alt) 22%) 100%
  );
  color: #fff;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease, filter 0.2s ease;
  box-shadow:
    0 1px 0 color-mix(in srgb, #fff 18%, transparent) inset,
    0 4px 12px color-mix(in srgb, var(--accent) 24%, transparent);

  &__shine {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      105deg,
      transparent 40%,
      color-mix(in srgb, #fff 16%, transparent) 50%,
      transparent 60%
    );
    transform: translateX(-120%);
    transition: transform 0.5s ease;
  }

  i {
    font-size: 10px;
  }

  &:hover:not(:disabled) {
    filter: brightness(1.06);
    transform: translateY(-1px);
    box-shadow:
      0 1px 0 color-mix(in srgb, #fff 20%, transparent) inset,
      0 8px 22px color-mix(in srgb, var(--accent) 36%, transparent);

    .connect-btn__shine {
      transform: translateX(120%);
    }
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

  &:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
}

@keyframes status-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.45; transform: scale(0.85); }
}

@keyframes accent-flow {
  0%, 100% { opacity: 0.45; }
  50% { opacity: 1; }
}
</style>
