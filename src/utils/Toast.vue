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
import {ref, onMounted, computed} from 'vue';

const props = defineProps<{
  title?: string;
  message: string;
  type?: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
}>();

const visible = ref(false);

const iconClass = computed(() => {
  switch (props.type) {
    case 'success':
      return 'fas fa-check-circle';
    case 'error':
      return 'fas fa-exclamation-circle';
    case 'warning':
      return 'fas fa-exclamation-triangle';
    default:
      return 'fas fa-info-circle';
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
@use "sass:color";
@use '../assets/css/base.scss';

.toast-item {
  width: 280px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px 16px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  box-shadow: 0 10px 15px -3px var(--shadow);
  margin-top: 10px;
  pointer-events: auto;
  position: relative;
  overflow: hidden;

  &.success {
    border-left: 4px solid var(--success);

    .toast-icon {
      color: var(--success);
    }
  }

  &.error {
    border-left: 4px solid var(--error);

    .toast-icon {
      color: var(--error);
    }
  }

  &.warning {
    border-left: 4px solid var(--accent-orange);

    .toast-icon {
      color: var(--accent-orange);
    }
  }

  &.info {
    border-left: 4px solid var(--accent);

    .toast-icon {
      color: var(--accent);
    }
  }

  .toast-icon {
    font-size: 18px;
    margin-top: 2px;
    filter: drop-shadow(0 0 4px currentColor);
  }

  .toast-content {
    flex: 1;

    .toast-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--text-main);
      margin-bottom: 2px;
    }

    .toast-message {
      font-size: 12px;
      color: var(--text-dim);
      line-height: 1.4;
    }
  }

  .toast-close {
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    transition: all 0.2s;
    opacity: 0.6;

    &:hover {
      color: var(--accent);
      opacity: 1;
    }
  }
}

.toast-fade-enter-active, .toast-fade-leave-active {
  transition: all 0.4s cubic-bezier(0.18, 0.89, 0.32, 1.28);
}

.toast-fade-enter-from {
  transform: translateX(120%);
  opacity: 0;
}

.toast-fade-leave-to {
  transform: translateX(120%);
  opacity: 0;
}
</style>