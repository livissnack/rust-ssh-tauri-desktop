<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  openSessions: Array<{ id: string; serverId: string; name: string }>;
  activeSessionId: string | null;
}>();

const emit = defineEmits<{
  (e: 'update:activeSessionId', id: string): void;
  (e: 'close', id: string): void;
  (e: 'openAddModal'): void;
}>();

const activeSessionId = computed({
  get: () => props.activeSessionId,
  set: (id) => emit('update:activeSessionId', id)
});
</script>

<template>
  <nav class="session-tabs custom-scrollbar">
    <div
        v-for="tab in openSessions"
        :key="tab.id"
        :class="['tab-item', { active: activeSessionId === tab.id }]"
        @click="activeSessionId = tab.id"
    >
      <i class="fas fa-terminal tab-icon"></i>

      <span class="tab-name">{{ tab.name }}</span>

      <div class="tab-close" @click.stop="emit('close', tab.id)">
        <i class="fas fa-times"></i>
      </div>
    </div>

    <button class="tab-add-btn" @click="emit('openAddModal')" title="新建会话">
      <i class="fas fa-plus"></i>
    </button>
  </nav>
</template>

<style lang="scss" scoped>
@use "sass:color";
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
    }

    .tab-name {
      flex: 1;
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
      border-radius: 4px;
      font-size: 10px;
      color: var(--text-dim);
      opacity: 0;
      transition: all 0.2s;

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
</style>
