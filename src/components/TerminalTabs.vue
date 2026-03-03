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
$bg-sidebar: #16161e;
$accent: #7aa2f7;
$text-dim: #565f89;

.session-tabs {
  height: 44px;
  background: $bg-sidebar;
  display: flex;
  align-items: flex-end;
  padding-left: 10px;
  gap: 4px;
  flex-shrink: 0;

  .tab-item {
    height: 34px;
    padding: 0 15px;
    background: #1a1b26;
    border-radius: 10px 10px 0 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: $text-dim;
    cursor: pointer;

    &.active {
      background: #0f111a;
      color: $accent;
      border-bottom: 2px solid $accent;
    }

    .tab-close {
      font-size: 16px;
      margin-left: 5px;

      &:hover {
        color: #ff5f56;
      }
    }
  }

  .tab-add {
    padding: 0 12px 10px;
    color: $text-dim;
    cursor: pointer;
    font-size: 20px;

    &:hover {
      color: $accent;
    }
  }
}
</style>
