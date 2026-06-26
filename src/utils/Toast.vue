<template>
  <Transition name="toast-fade" @after-leave="onAfterLeave">
    <div v-if="visible" class="toast-item" :class="type" role="status" aria-live="polite">
      <div class="toast-accent" aria-hidden="true"></div>
      <div class="toast-icon-wrap">
        <i :class="iconClass"></i>
      </div>
      <div class="toast-content">
        <div v-if="title" class="toast-title">{{ title }}</div>
        <div class="toast-message">{{ message }}</div>
      </div>
      <button type="button" class="toast-close" :aria-label="closeLabel" @click="dismiss">
        <i class="fas fa-xmark"></i>
      </button>
      <div v-if="showProgress" class="toast-progress">
        <span class="toast-progress__bar" :style="{ animationDuration: `${duration}ms` }"></span>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { t } from './i18n.ts';

const props = withDefaults(defineProps<{
  title?: string;
  message: string;
  type?: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
  onDismiss?: () => void;
}>(), {
  type: 'info',
  duration: 3200,
});

const visible = ref(false);
let timer: ReturnType<typeof setTimeout> | null = null;

const showProgress = computed(() => props.duration > 0);
const closeLabel = computed(() => t('common.close'));

const iconClass = computed(() => {
  switch (props.type) {
    case 'success':
      return 'fas fa-circle-check';
    case 'error':
      return 'fas fa-circle-xmark';
    case 'warning':
      return 'fas fa-triangle-exclamation';
    default:
      return 'fas fa-circle-info';
  }
});

const dismiss = () => {
  if (timer) {
    clearTimeout(timer);
    timer = null;
  }
  visible.value = false;
};

const onAfterLeave = () => {
  props.onDismiss?.();
};

onMounted(() => {
  visible.value = true;
  if (props.duration > 0) {
    timer = setTimeout(dismiss, props.duration);
  }
});
</script>

<style lang="scss" scoped>
.toast-item {
  --toast-accent: var(--accent);
  position: relative;
  width: min(360px, calc(100vw - 32px));
  min-height: 56px;
  background: color-mix(in srgb, var(--bg-card) 92%, transparent);
  border: 1px solid var(--border-50, var(--border));
  border-radius: 14px;
  padding: 14px 14px 16px 12px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  box-shadow:
    0 16px 40px -12px var(--shadow),
    0 0 0 1px color-mix(in srgb, var(--toast-accent) 12%, transparent);
  margin-top: 10px;
  pointer-events: auto;
  overflow: hidden;
  backdrop-filter: blur(14px) saturate(1.2);

  &.success { --toast-accent: var(--success); }
  &.error { --toast-accent: var(--error); }
  &.warning { --toast-accent: var(--accent-orange, #f59e0b); }
  &.info { --toast-accent: var(--accent); }

  .toast-accent {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: linear-gradient(180deg, var(--toast-accent), color-mix(in srgb, var(--toast-accent) 55%, transparent));
  }

  .toast-icon-wrap {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    margin-top: 1px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--toast-accent) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--toast-accent) 22%, transparent);
    color: var(--toast-accent);
    font-size: 15px;
  }

  .toast-content {
    flex: 1;
    min-width: 0;
    padding-top: 2px;

    .toast-title {
      font-size: 13px;
      font-weight: 700;
      color: var(--text-main);
      margin-bottom: 3px;
      line-height: 1.3;
    }

    .toast-message {
      font-size: 12px;
      color: var(--text-dim);
      line-height: 1.5;
      word-break: break-word;
    }
  }

  .toast-close {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    margin-top: 1px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 12px;
    opacity: 0.55;
    transition: opacity 0.2s, background 0.2s, color 0.2s;

    &:hover {
      opacity: 1;
      background: var(--bg-input);
      color: var(--text-main);
    }
  }

  .toast-progress {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 0;
    height: 2px;
    border-radius: 999px;
    overflow: hidden;
    background: color-mix(in srgb, var(--toast-accent) 10%, transparent);

    &__bar {
      display: block;
      height: 100%;
      width: 100%;
      transform-origin: left center;
      background: linear-gradient(90deg, var(--toast-accent), color-mix(in srgb, var(--toast-accent) 70%, white));
      animation: toast-progress-shrink linear forwards;
    }
  }
}

@keyframes toast-progress-shrink {
  from { transform: scaleX(1); }
  to { transform: scaleX(0); }
}

.toast-fade-enter-active {
  transition: transform 0.38s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.28s ease;
}

.toast-fade-leave-active {
  transition: transform 0.28s ease, opacity 0.22s ease;
}

.toast-fade-enter-from {
  transform: translateX(calc(100% + 24px)) scale(0.96);
  opacity: 0;
}

.toast-fade-leave-to {
  transform: translateX(calc(100% + 12px)) scale(0.98);
  opacity: 0;
}
</style>
