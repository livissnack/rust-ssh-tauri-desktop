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
  <nav class="session-tabs">
    <div
        v-for="tab in openSessions"
        :key="tab.id"
        :class="['tab-item', { active: activeSessionId === tab.id }]"
        @click="activeSessionId = tab.id"
    >
      <span class="tab-icon">🐚</span>
      <span class="tab-name">{{ tab.name }}</span>
      <span class="tab-close" @click.stop="emit('close', tab.id)">×</span>
    </div>
    <div class="tab-add" @click="emit('openAddModal')">+</div>
  </nav>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.session-tabs {
  height: 44px;
  background: var(--bg-sidebar);
  display: flex;
  align-items: flex-end;
  padding-left: 10px;
  gap: 4px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border);

  .tab-item {
    height: 34px;
    padding: 0 15px;
    // 修复点：使用预计算的 bg-primary 透明变量
    background: var(--bg-primary-60);
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    top: 1px; // 视觉技巧：压在底部的 border-bottom 上实现缝隙消除

    &:hover {
      background: var(--bg-primary-80); // 悬浮时稍亮
      color: var(--text-main);
    }

    &.active {
      background: var(--bg-primary);
      color: var(--accent);
      height: 35px; // 盖住底边框
      font-weight: 600;
      border-color: var(--border);

      // 现代 IDE 风格的顶部强调条
      &::before {
        content: '';
        position: absolute;
        top: 0; left: 0; right: 0;
        height: 2px;
        background: var(--accent);
        border-radius: 8px 8px 0 0;
      }
    }

    .tab-close {
      font-size: 14px;
      margin-left: 5px;
      padding: 2px;
      border-radius: 4px;
      transition: all 0.2s;
      display: flex;
      align-items: center;
      justify-content: center;

      &:hover {
        background: var(--error-15); // 修复点：使用 error 的透明变量
        color: var(--error);
      }
    }
  }

  .tab-add {
    height: 34px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 18px;
    transition: color 0.2s;

    &:hover {
      color: var(--accent);
    }
  }
}
</style>
