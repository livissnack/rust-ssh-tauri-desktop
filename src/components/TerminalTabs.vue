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
  background: base.$bg-sidebar; // 选项卡槽背景，通常与侧边栏一致
  display: flex;
  align-items: flex-end;
  padding-left: 10px;
  gap: 4px;
  flex-shrink: 0;
  border-bottom: 1px solid base.$border; // 增加底部细线，增强与内容区的隔离感

  .tab-item {
    height: 34px;
    padding: 0 15px;
    background: rgba(base.$bg-primary, 0.4); // 非激活状态略透明
    border: 1px solid base.$border;
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: base.$text-dim;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    top: 1px; // 压在 border-bottom 上

    &:hover {
      background: rgba(base.$bg-primary, 0.8);
      color: base.$text-main;
    }

    &.active {
      background: base.$bg-primary; // 激活态与主内容区背景融合
      color: base.$accent;
      height: 35px; // 略高一点点，覆盖住底边框
      font-weight: 600;
      border-color: base.$border;

      // 使用伪元素做顶部发光条，比直接底边框更符合现代 IDE 审美
      &::before {
        content: '';
        position: absolute;
        top: 0; left: 0; right: 0;
        height: 2px;
        background: base.$accent;
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
        background: rgba(base.$error, 0.15);
        color: base.$error; // 修改：使用主题错误色
      }
    }
  }

  .tab-add {
    height: 34px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    color: base.$text-dim;
    cursor: pointer;
    font-size: 18px;
    transition: color 0.2s;

    &:hover {
      color: base.$accent;
    }
  }
}
</style>
