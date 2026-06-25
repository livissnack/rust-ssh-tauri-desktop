<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { LOCAL_SERVER_ID } from '../utils/session.ts';
import type { SessionStatus } from '../utils/session.ts';
import { useI18n } from '../utils/i18n.ts';

const { tr } = useI18n();

const props = defineProps<{
  openSessions: Array<{ id: string; serverId: string; name: string }>;
  activeSessionId: string | null;
  sessionStatuses?: Record<string, SessionStatus>;
}>();

const emit = defineEmits<{
  (e: 'update:activeSessionId', id: string): void;
  (e: 'close', id: string): void;
  (e: 'cloneTab', id: string): void;
  (e: 'cloneWindow', id: string): void;
  (e: 'newLocalShell'): void;
  (e: 'reconnect', id: string): void;
}>();

const activeSessionId = computed({
  get: () => props.activeSessionId,
  set: (id) => emit('update:activeSessionId', id)
});

const menuVisible = ref(false);
const menuPos = ref({ x: 0, y: 0 });
const menuTargetTabId = ref<string | null>(null);
const menuElRef = ref<HTMLElement | null>(null);

const menuTargetTab = computed(() =>
  props.openSessions.find(tab => tab.id === menuTargetTabId.value) ?? null
);

const tabStatusClass = (tabId: string) => {
  const status = props.sessionStatuses?.[tabId];
  if (status === 'failed' || status === 'disconnected') return 'is-error';
  if (status === 'connecting') return 'is-connecting';
  if (status === 'connected') return 'is-connected';
  return '';
};

const canReconnect = (tabId: string) => {
  const status = props.sessionStatuses?.[tabId];
  return status === 'failed' || status === 'disconnected';
};

const clampMenuPosition = (anchorX: number, anchorY: number) => {
  const el = menuElRef.value;
  if (!el) return;
  const pad = 8;
  const { width, height } = el.getBoundingClientRect();
  let x = anchorX;
  let y = anchorY;
  if (x + width > window.innerWidth - pad) x = anchorX - width;
  if (y + height > window.innerHeight - pad) y = anchorY - height;
  x = Math.min(Math.max(x, pad), window.innerWidth - width - pad);
  y = Math.min(Math.max(y, pad), window.innerHeight - height - pad);
  menuPos.value = { x, y };
};

const openTabMenu = (e: MouseEvent, tabId: string) => {
  e.preventDefault();
  e.stopPropagation();
  menuTargetTabId.value = tabId;
  menuPos.value = { x: e.clientX, y: e.clientY };
  menuVisible.value = true;
  nextTick(() => clampMenuPosition(e.clientX, e.clientY));
};

const closeMenu = () => {
  menuVisible.value = false;
  menuTargetTabId.value = null;
};

const handleMenuAction = (action: 'cloneTab' | 'cloneWindow' | 'close' | 'reconnect') => {
  const id = menuTargetTabId.value;
  if (!id) return;
  closeMenu();
  if (action === 'cloneTab') emit('cloneTab', id);
  else if (action === 'cloneWindow') emit('cloneWindow', id);
  else if (action === 'reconnect') emit('reconnect', id);
  else emit('close', id);
};

const handlePointerDownOutside = (e: PointerEvent) => {
  if (!menuVisible.value) return;
  const target = e.target as HTMLElement;
  if (target.closest('.tab-context-menu')) return;
  closeMenu();
};

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDownOutside);
  window.addEventListener('blur', closeMenu);
});

onUnmounted(() => {
  document.removeEventListener('pointerdown', handlePointerDownOutside);
  window.removeEventListener('blur', closeMenu);
});
</script>

<template>
  <nav class="session-tabs custom-scrollbar">
    <div
        v-for="tab in openSessions"
        :key="tab.id"
        :class="['tab-item', tabStatusClass(tab.id), { active: activeSessionId === tab.id }]"
        @click="activeSessionId = tab.id"
        @contextmenu="openTabMenu($event, tab.id)"
    >
      <i :class="['fas', tab.serverId === LOCAL_SERVER_ID ? 'fa-laptop' : 'fa-terminal', 'tab-icon']"></i>
      <span class="tab-name">{{ tab.name }}</span>

      <Tooltip :text="tr.tabs.closeSession" placement="bottom">
        <button type="button" class="tab-close" @click.stop="emit('close', tab.id)">
          <i class="fas fa-times"></i>
        </button>
      </Tooltip>
    </div>

    <Tooltip :text="tr.tabs.newLocalShell">
      <button class="tab-add-btn" @click="emit('newLocalShell')">
        <i class="fas fa-plus"></i>
      </button>
    </Tooltip>

    <Teleport to="body">
      <Transition name="tab-menu">
        <div
            v-if="menuVisible && menuTargetTab"
            ref="menuElRef"
            class="tab-context-menu"
            :style="{ top: `${menuPos.y}px`, left: `${menuPos.x}px` }"
            @contextmenu.prevent
            @click.stop
        >
          <div class="tab-context-menu__title">{{ menuTargetTab.name }}</div>
          <button
              v-if="canReconnect(menuTargetTab.id) && menuTargetTab.serverId !== LOCAL_SERVER_ID"
              type="button"
              class="tab-context-menu__item"
              @click="handleMenuAction('reconnect')"
          >
            <i class="fas fa-rotate-right"></i>
            <span>{{ tr.tabs.reconnect }}</span>
          </button>
          <button type="button" class="tab-context-menu__item" @click="handleMenuAction('cloneTab')">
            <i class="fas fa-copy"></i>
            <span>{{ tr.tabs.cloneTab }}</span>
          </button>
          <button type="button" class="tab-context-menu__item" @click="handleMenuAction('cloneWindow')">
            <i class="fas fa-window-restore"></i>
            <span>{{ tr.tabs.cloneWindow }}</span>
          </button>
          <div class="tab-context-menu__divider"></div>
          <button type="button" class="tab-context-menu__item tab-context-menu__item--danger" @click="handleMenuAction('close')">
            <i class="fas fa-times"></i>
            <span>{{ tr.tabs.closeSession }}</span>
          </button>
        </div>
      </Transition>
    </Teleport>
  </nav>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

.session-tabs {
  height: 40px;
  background: var(--bg-sidebar);
  display: flex;
  align-items: flex-end;
  padding: 0 12px;
  gap: 2px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border);
  overflow-x: auto;
  overflow-y: hidden;

  &::-webkit-scrollbar { height: 0; }

  .tab-item {
    height: 32px;
    min-width: 120px;
    max-width: 200px;
    padding: 0 10px 0 12px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px 6px 0 0;
    display: flex;
    align-items: center;
    position: relative;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    top: 1px;

    .tab-icon {
      font-size: 12px;
      margin-right: 8px;
      color: var(--text-dim);
      opacity: 0.7;
      flex-shrink: 0;
    }

    .tab-name {
      flex: 1;
      min-width: 0;
      font-size: 12px;
      color: var(--text-dim);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .tab-close {
      width: 18px;
      height: 18px;
      display: flex;
      align-items: center;
      justify-content: center;
      border: none;
      border-radius: 4px;
      background: transparent;
      color: var(--text-dim);
      font-size: 10px;
      opacity: 0;
      cursor: pointer;
      transition: all 0.2s;
      flex-shrink: 0;

      &:hover {
        background: var(--error-15);
        color: var(--error);
      }
    }

    &:hover {
      background: var(--bg-primary-30);
      .tab-name { color: var(--text-main); }
      .tab-close { opacity: 1; }
      .tab-icon { color: var(--accent); opacity: 1; }
    }

    &.active {
      background: var(--bg-primary);
      border-color: var(--border) var(--border) transparent var(--border);
      z-index: 2;

      .tab-name {
        color: var(--accent);
        font-weight: 500;
      }
      .tab-icon { color: var(--accent); opacity: 1; }
      .tab-close { opacity: 1; }

      &::before {
        content: '';
        position: absolute;
        top: -1px; left: -1px; right: -1px;
        height: 2px;
        background: var(--accent);
        border-radius: 6px 6px 0 0;
      }
    }

    &.is-error:not(.active) .tab-name { color: var(--error); }
    &.is-connecting:not(.active) .tab-name { color: var(--accent-orange); }
  }

  .tab-add-btn {
    width: 28px;
    height: 28px;
    margin-bottom: 3px;
    margin-left: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      background: var(--accent-10);
      color: var(--accent);
      transform: rotate(90deg);
    }

    i { font-size: 14px; }
  }
}

.tab-context-menu {
  position: fixed;
  z-index: 4000;
  min-width: 180px;
  padding: 6px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 12px 28px var(--shadow);

  &__title {
    padding: 6px 10px 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border-bottom: 1px solid var(--border-30);
    margin-bottom: 4px;
  }

  &__item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text-main);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;

    i {
      width: 14px;
      font-size: 12px;
      color: var(--text-dim);
      text-align: center;
    }

    &:hover {
      background: var(--accent-08);
      color: var(--accent);

      i { color: var(--accent); }
    }

    &--danger {
      color: var(--error);

      i { color: var(--error); }

      &:hover {
        background: var(--error-10);
        color: var(--error);
      }
    }
  }

  &__divider {
    height: 1px;
    margin: 4px 6px;
    background: var(--border-30);
  }
}

.tab-menu-enter-active,
.tab-menu-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.tab-menu-enter-from,
.tab-menu-leave-to {
  opacity: 0;
  transform: scale(0.96);
}
</style>
