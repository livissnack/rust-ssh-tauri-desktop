<script setup lang="ts">
import { computed } from "vue";
import type { SessionStatus } from "../utils/session.ts";
import { sessionStatusLabel, useI18n } from "../utils/i18n.ts";
const { tr } = useI18n();

const props = defineProps<{
  currentServer: any;
  activeId: string | null;
  activeSessionId: string | null;
  sessionStatus: SessionStatus;
  sessionError?: string;
  currentViewMode: 'terminal' | 'sftp';
  openSessions: Array<{ id: string; serverId: string; name: string }>;
  servers: any[];
  isLocalSession?: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggleViewMode'): void;
  (e: 'connect'): void;
  (e: 'reconnect'): void;
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

const hasActiveHost = computed(() => !!activeServer.value?.name && activeServer.value.name !== tr.value.session.selectHost);

const displayServerName = computed(() => {
  if (props.isLocalSession && props.activeSessionId) {
    const session = props.openSessions?.find(s => s.id === props.activeSessionId);
    if (session) return session.name;
  }
  if (activeServer.value?.name) return activeServer.value.name;
  return tr.value.session.selectHost;
});

const displayHostMeta = computed(() => {
  if (props.isLocalSession) return tr.value.session.localTerminal;
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
  if (props.sessionStatus === 'connecting') return 'is-connecting';
  if (props.sessionStatus === 'failed' || props.sessionStatus === 'disconnected') return 'is-error';
  if (props.sessionStatus === 'connected') return 'is-active';
  if (props.activeSessionId) return 'is-idle';
  return 'is-idle';
});

const statusLabel = computed(() =>
  sessionStatusLabel(props.sessionStatus, !!props.isLocalSession),
);

const showConnectButton = computed(() =>
  !props.isLocalSession &&
  props.sessionStatus !== 'connected' &&
  props.sessionStatus !== 'connecting',
);

const showReconnectButton = computed(() =>
  !props.isLocalSession &&
  (props.sessionStatus === 'failed' || props.sessionStatus === 'disconnected'),
);

const connectButtonText = computed(() => {
  if (props.sessionStatus === 'connecting') return tr.value.session.connecting;
  return tr.value.session.connect;
});

const viewModeLabel = computed(() => {
  if (props.currentViewMode === 'sftp') return tr.value.session.terminal;
  return props.isLocalSession ? tr.value.session.files : tr.value.session.sftp;
});

const viewModeIcon = computed(() =>
  props.currentViewMode === 'sftp' ? 'fa-terminal' : 'fa-folder-open'
);

const viewModeTooltip = computed(() => {
  if (!props.activeSessionId) return '';
  if (props.isLocalSession) {
    return props.currentViewMode === 'sftp' ? tr.value.session.switchToTerminal : tr.value.session.switchToFiles;
  }
  return props.currentViewMode === 'sftp' ? tr.value.session.switchToTerminal : tr.value.session.switchToSftp;
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
          <p v-else class="header-hint">{{ tr.session.pickHostHint }}</p>
        </div>
      </div>

      <div class="header-actions">
        <Tooltip :text="viewModeTooltip" :disabled="!activeSessionId">
          <button
              type="button"
              class="action-btn action-btn--ghost"
              :class="{ 'is-alt': currentViewMode === 'sftp', 'is-disabled': !activeSessionId }"
              :disabled="!activeSessionId"
              @click="emit('toggleViewMode')"
          >
            <i class="fas" :class="viewModeIcon"></i>
            <span>{{ viewModeLabel }}</span>
          </button>
        </Tooltip>

        <button
            v-if="showReconnectButton"
            type="button"
            class="action-btn action-btn--primary"
            @click="emit('reconnect')"
        >
          <i class="fas fa-rotate-right"></i>
          <span>{{ tr.session.reconnect }}</span>
        </button>

        <button
            v-else-if="showConnectButton"
            type="button"
            class="action-btn action-btn--primary"
            :disabled="!activeId && !activeSessionId"
            @click="emit('connect')"
        >
          <i class="fas" :class="sessionStatus === 'connecting' ? 'fa-spinner fa-spin' : 'fa-plug'"></i>
          <span>{{ connectButtonText }}</span>
        </button>
      </div>
    </div>
  </header>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

.workspace-header {
  position: relative;
  flex-shrink: 0;
  background: var(--bg-secondary);
  overflow: hidden;

  .header-accent {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--accent), var(--accent-purple));
    opacity: 0.6;
    transition: opacity 0.3s, background 0.3s;
  }

  &.is-active .header-accent {
    opacity: 1;
    background: linear-gradient(
      90deg,
      var(--success) 0%,
      color-mix(in srgb, var(--success) 65%, white) 28%,
      rgba(255, 255, 255, 0.5) 50%,
      color-mix(in srgb, var(--success) 75%, var(--accent)) 72%,
      var(--success) 100%
    );
  }
  &.is-error .header-accent { background: var(--error); opacity: 0.9; }
  &.is-connecting .header-accent {
    opacity: 1;
    overflow: hidden;
    background: linear-gradient(
      90deg,
      var(--accent) 0%,
      var(--accent-orange) 22%,
      rgba(255, 255, 255, 0.9) 50%,
      var(--accent-purple) 78%,
      var(--accent) 100%
    );
    background-size: 220% 100%;
    animation: header-accent-gradient 1.6s linear infinite;

    &::after {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      width: 40%;
      height: 100%;
      background: linear-gradient(
        90deg,
        transparent 0%,
        rgba(255, 255, 255, 0.15) 30%,
        rgba(255, 255, 255, 0.55) 50%,
        rgba(255, 255, 255, 0.15) 70%,
        transparent 100%
      );
      mix-blend-mode: overlay;
      animation: header-accent-sweep 1.6s cubic-bezier(0.45, 0, 0.25, 1) infinite;
    }
  }

  .header-body {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    gap: 16px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
    flex: 1;
  }

  .header-icon {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    background: var(--bg-input);
    border: 1px solid var(--border-30);
    color: var(--text-dim);
    font-size: 16px;
    transition: all 0.25s;

    &.is-active {
      color: var(--success);
      border-color: var(--success-30);
      background: var(--success-10);
    }
    &.is-error {
      color: var(--error);
      border-color: var(--error-30);
      background: var(--error-10);
    }
    &.is-connecting {
      color: var(--accent-orange);
      border-color: var(--accent-orange-30);
    }
    &.is-local {
      color: var(--accent);
      border-color: var(--accent-30);
      background: var(--accent-08);
    }
  }

  .header-info {
    min-width: 0;
  }

  .header-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;

    &.is-placeholder { color: var(--text-dim); font-weight: 500; }
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
    font-size: 11px;
    color: var(--text-dim);
    font-family: var(--font-terminal);

    .meta-sep { opacity: 0.5; }
  }

  .header-hint {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--text-dim);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--border);

    &.is-active { background: var(--success); box-shadow: 0 0 6px var(--success-60); }
    &.is-error { background: var(--error); }
    &.is-connecting { background: var(--accent-orange); animation: header-pulse 1s infinite; }
  }

  .status-text {
    &.is-active { color: var(--success); }
    &.is-error { color: var(--error); }
    &.is-connecting { color: var(--accent-orange); }
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 14px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.2s;

    &--ghost {
      background: var(--bg-input);
      border-color: var(--border-30);
      color: var(--text-dim);

      &:hover:not(.is-disabled) {
        color: var(--accent);
        border-color: var(--accent-30);
        background: var(--accent-08);
      }

      &.is-alt {
        color: var(--accent);
        border-color: var(--accent-30);
      }
    }

    &--primary {
      background: var(--accent);
      color: #fff;

      &:hover:not(:disabled) {
        filter: brightness(1.08);
      }

      &:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
    }

    &.is-disabled,
    &:disabled {
      opacity: 0.45;
      cursor: not-allowed;
    }
  }
}

@keyframes header-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

@keyframes header-accent-gradient {
  0% { background-position: 0% 50%; }
  100% { background-position: 200% 50%; }
}

@keyframes header-accent-sweep {
  0% { transform: translateX(-120%); }
  100% { transform: translateX(420%); }
}
</style>
