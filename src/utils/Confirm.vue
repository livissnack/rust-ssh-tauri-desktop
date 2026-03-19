<template>
  <Transition name="confirm-fade">
    <div v-if="visible" class="confirm-overlay" @click="handleCancel">
      <div class="confirm-box" @click.stop>
        <div class="confirm-header">
          <i :class="[iconClass, type]"></i>
          <span class="confirm-title">{{ title || '确认操作' }}</span>
        </div>
        <div class="confirm-body">
          {{ message }}
        </div>
        <div class="confirm-footer">
          <button class="btn-cancel" @click="handleCancel">取消</button>
          <button class="btn-confirm" :class="type" @click="handleConfirm">确定</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';

const props = defineProps<{
  message: string;
  title?: string;
  type?: 'info' | 'success' | 'warning' | 'error';
  onResolve: (result: boolean) => void; // 这里的回调用于 Promise
}>();

const visible = ref(false);

const iconClass = computed(() => {
  switch (props.type) {
    case 'success': return 'fas fa-check-circle';
    case 'error': return 'fas fa-circle-xmark';
    case 'warning': return 'fas fa-exclamation-triangle';
    default: return 'fas fa-info-circle';
  }
});

const handleConfirm = () => {
  visible.value = false;
  setTimeout(() => props.onResolve(true), 300); // 等动画播完再销毁
};

const handleCancel = () => {
  visible.value = false;
  setTimeout(() => props.onResolve(false), 300);
};

onMounted(() => {
  visible.value = true;
});
</script>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.confirm-overlay {
  position: fixed;
  inset: 0;
  background: var(--bg-secondary-60, rgba(0, 0, 0, 0.6));
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  pointer-events: auto;
}

.confirm-box {
  width: 340px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 24px;
  box-shadow: 0 20px 25px -5px var(--shadow, rgba(0, 0, 0, 0.3));
  position: relative;
  overflow: hidden;

  &::before {
    content: "";
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 3px;
    background: var(--accent);
    opacity: 0.8;
  }

  .confirm-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;

    i {
      font-size: 20px;
      &.error   { color: var(--error); }
      &.warning { color: var(--accent-orange); }
      &.success { color: var(--success); }
      &.info    { color: var(--accent); }

      filter: drop-shadow(0 0 4px var(--accent-30));
    }

    .confirm-title {
      font-size: 16px;
      font-weight: 700;
      color: var(--text-main);
      letter-spacing: 0.5px;
    }
  }

  .confirm-body {
    font-size: 14px;
    color: var(--text-dim);
    line-height: 1.6;
    margin-bottom: 28px;
    padding-left: 32px;
  }

  .confirm-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;

    button {
      padding: 10px 20px;
      border-radius: 8px;
      font-size: 13px;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
      border: 1px solid transparent;
      outline: none;
    }

    .btn-cancel {
      background: var(--bg-input);
      color: var(--text-dim);
      border-color: var(--border);

      &:hover {
        background: var(--bg-secondary);
        color: var(--text-main);
        border-color: var(--text-dim-40, var(--border));
      }
    }

    .btn-confirm {
      background: var(--accent);
      color: #fff;

      &.error {
        background: var(--error);
        &:hover {
          background: color.mix(#000, #f7768e, 10%); // 稍微深一点
          box-shadow: 0 4px 12px var(--error-30);
        }
      }

      &:hover {
        transform: translateY(-1px);
        box-shadow: 0 4px 12px var(--accent-30);
        filter: brightness(1.1);
      }

      &:active {
        transform: translateY(0);
      }
    }
  }
}

.confirm-fade-enter-active, .confirm-fade-leave-active {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1); // 增加一点点回弹感
}

.confirm-fade-enter-from, .confirm-fade-leave-to {
  opacity: 0;
  transform: scale(0.9) translateY(10px);
}
</style>