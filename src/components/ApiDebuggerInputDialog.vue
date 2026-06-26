<script setup lang="ts">
import { ref, watch } from 'vue';
import { t } from '../utils/i18n.ts';

const props = defineProps<{
  visible: boolean;
  title: string;
  label: string;
  placeholder?: string;
  icon?: string;
  initialValue?: string;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [value: string];
}>();

const inputValue = ref('');

watch(
  () => [props.visible, props.initialValue] as const,
  ([visible, initialValue]) => {
    if (visible) inputValue.value = initialValue ?? '';
  },
  { immediate: true },
);

const handleConfirm = () => {
  const value = inputValue.value.trim();
  if (!value) return;
  emit('confirm', value);
};
</script>

<template>
  <Teleport to="body">
    <Transition name="confirm-fade">
      <div v-if="visible" class="api-dialog-overlay" @click="emit('close')">
        <div class="api-dialog-box" @click.stop>
          <div class="api-dialog-header">
            <i class="fas" :class="icon || 'fa-pen-to-square'"></i>
            <span>{{ title }}</span>
          </div>

          <div class="form-section">
            <label class="field-label">{{ label }}</label>
            <input
              v-model="inputValue"
              class="field-input"
              :placeholder="placeholder"
              autofocus
              @keyup.enter="handleConfirm"
            />
          </div>

          <div class="api-dialog-footer">
            <button type="button" class="btn-cancel" @click="emit('close')">{{ t('common.cancel') }}</button>
            <button type="button" class="btn-confirm" :disabled="!inputValue.trim()" @click="handleConfirm">
              {{ t('common.confirm') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped lang="scss">
.api-dialog-overlay {
  position: fixed;
  inset: 0;
  background: var(--bg-secondary-60, rgba(0, 0, 0, 0.6));
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.api-dialog-box {
  width: min(400px, calc(100vw - 32px));
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 20px;
  box-shadow: 0 20px 25px -5px var(--shadow, rgba(0, 0, 0, 0.3));
}

.api-dialog-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
  font-size: 15px;
  font-weight: 700;
  color: var(--text-main);

  i {
    color: var(--accent);
    font-size: 14px;
  }
}

.form-section {
  margin-bottom: 18px;
}

.field-label {
  display: block;
  margin-bottom: 6px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
}

.field-input {
  width: 100%;
  height: 36px;
  padding: 0 12px;
  box-sizing: border-box;
  border-radius: 8px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 13px;

  &:focus {
    outline: none;
    border-color: var(--accent-30);
    box-shadow: 0 0 0 3px var(--accent-10);
  }
}

.api-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.btn-cancel,
.btn-confirm {
  height: 34px;
  padding: 0 16px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
}

.btn-cancel {
  background: transparent;
  border-color: var(--border-30);
  color: var(--text-dim);

  &:hover {
    background: var(--bg-input);
    color: var(--text-main);
  }
}

.btn-confirm {
  background: var(--accent);
  color: #fff;

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
