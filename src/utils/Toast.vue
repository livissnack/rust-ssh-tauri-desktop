<template>
  <Transition name="toast-fade">
    <div v-if="visible" class="toast-item" :class="type">
      <div class="toast-icon">
        <i :class="iconClass"></i>
      </div>
      <div class="toast-content">
        <div class="toast-title" v-if="title">{{ title }}</div>
        <div class="toast-message">{{ message }}</div>
      </div>
      <button class="toast-close" @click="visible = false">
        <i class="fas fa-times"></i>
      </button>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';

const props = defineProps<{
  title?: string;
  message: string;
  type?: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
}>();

const visible = ref(false);

const iconClass = computed(() => {
  switch (props.type) {
    case 'success': return 'fas fa-check-circle';
    case 'error': return 'fas fa-exclamation-circle';
    case 'warning': return 'fas fa-exclamation-triangle';
    default: return 'fas fa-info-circle';
  }
});

onMounted(() => {
  visible.value = true;
  if (props.duration !== 0) {
    setTimeout(() => {
      visible.value = false;
    }, props.duration || 3000);
  }
});
</script>

<style lang="scss" scoped>
.toast-item {
  width: 200px;
  background: #24283b; // Tokyo Night 风格
  border: 1px solid #292e42;
  border-radius: 8px;
  padding: 12px 16px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.4);
  margin-top: 10px;
  pointer-events: auto;
  position: relative;

  &.success { border-left: 4px solid #9ece6a; .toast-icon { color: #9ece6a; } }
  &.error { border-left: 4px solid #f7768e; .toast-icon { color: #f7768e; } }
  &.warning { border-left: 4px solid #e0af68; .toast-icon { color: #e0af68; } }
  &.info { border-left: 4px solid #7aa2f7; .toast-icon { color: #7aa2f7; } }

  .toast-icon { font-size: 18px; margin-top: 2px; }
  .toast-content {
    flex: 1;
    .toast-title { font-size: 13px; font-weight: 600; color: #c0caf5; margin-bottom: 2px; }
    .toast-message { font-size: 12px; color: #a9b1d6; line-height: 1.4; }
  }
  .toast-close {
    background: transparent; border: none; color: #565f89; cursor: pointer;
    font-size: 12px; &:hover { color: #bb9af7; }
  }
}

/* 动画：从右侧滑入 */
.toast-fade-enter-active, .toast-fade-leave-active {
  transition: all 0.4s cubic-bezier(0.18, 0.89, 0.32, 1.28);
}
.toast-fade-enter-from { transform: translateX(100%); opacity: 0; }
.toast-fade-leave-to { transform: translateX(100%); opacity: 0; }
</style>