<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import { t } from '../utils/i18n.ts';

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits(['close', 'confirm']);

const redisTypes = computed(() => [
  { label: 'String', value: 'string', color: '#9ece6a', desc: t('redis.typeStringDesc') },
  { label: 'Hash', value: 'hash', color: '#7aa2f7', desc: t('redis.typeHashDesc') },
  { label: 'List', value: 'list', color: '#e0af68', desc: t('redis.typeListDesc') },
  { label: 'Set', value: 'set', color: '#bb9af7', desc: t('redis.typeSetDesc') },
]);

const placeholderMap = computed(() => ({
  string: { key: t('redis.phStringKey'), value: t('redis.phStringValue') },
  hash: {
    key: t('redis.phHashKey'),
    field: t('redis.phHashField'),
    value: t('redis.phHashValue'),
  },
  list: { key: t('redis.phListKey'), value: t('redis.phListValue') },
  set: { key: t('redis.phSetKey'), value: t('redis.phSetValue') },
}));

const formData = ref({
  key: '',
  value: '',
  type: 'string',
  field: '',
  ttl: -1,
});

const currentPlaceholder = computed(() =>
  placeholderMap.value[formData.value.type as keyof typeof placeholderMap.value],
);

const activeType = computed(() =>
  redisTypes.value.find((item) => item.value === formData.value.type),
);

watch(() => props.visible, (newVal) => {
  if (newVal) {
    formData.value = { key: '', value: '', type: 'string', field: '', ttl: -1 };
  }
});

const handleConfirm = async () => {
  if (!formData.value.key.trim()) {
    toast.warning(t('redis.keyRequired'));
    return;
  }
  if (formData.value.type === 'hash' && !formData.value.field.trim()) {
    toast.warning(t('redis.hashFieldRequired'));
    return;
  }
  try {
    await invoke('redis_set_value', {
      ...formData.value,
      keyType: formData.value.type,
    });
    toast.success(t('redis.saveSuccess'));
    emit('confirm', { ...formData.value });
    emit('close');
  } catch {
    toast.error(t('redis.saveFailed'));
  }
};
</script>

<template>
  <Transition name="modal-fade">
    <div v-if="visible" class="modal-overlay" @click.self="emit('close')">
      <div class="modal-card">
        <div class="modal-header">
          <div class="title">
            <i class="fas fa-plus-square"></i>
            <span>{{ t('redis.createTitle') }}</span>
          </div>
          <button type="button" class="close-x" @click="emit('close')">&times;</button>
        </div>

        <div class="modal-body">
          <div class="type-selector">
            <div
              v-for="typeItem in redisTypes"
              :key="typeItem.value"
              class="type-item"
              :class="{ active: formData.type === typeItem.value }"
              @click="formData.type = typeItem.value"
            >
              <span class="dot" :style="{ background: typeItem.color }"></span>
              {{ typeItem.label }}
            </div>
          </div>

          <div class="modal-form">
            <div class="input-row">
              <div class="form-group flex-3">
                <label>{{ t('redis.createKeyName') }}</label>
                <input v-model="formData.key" :placeholder="currentPlaceholder.key" class="dark-input" />
              </div>

              <div class="form-group flex-2">
                <label>{{ t('redis.createTtl') }} <span class="hint">{{ t('redis.createTtlHint') }}</span></label>
                <NumberInput v-model="formData.ttl" :min="-1" />
              </div>
            </div>

            <div class="expand-wrapper" :class="{ 'is-open': formData.type === 'hash' }">
              <div class="expand-content">
                <div class="form-group">
                  <label>{{ t('redis.createField') }}</label>
                  <input
                    v-model="formData.field"
                    :placeholder="currentPlaceholder.field"
                    class="dark-input"
                  />
                </div>
              </div>
            </div>

            <div class="form-group">
              <label>{{ t('redis.createValue') }}</label>
              <textarea
                v-model="formData.value"
                :placeholder="currentPlaceholder.value"
                class="dark-input value-area"
                :style="{ color: activeType?.color }"
              ></textarea>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <div class="type-hint">
            <i class="fas fa-info-circle"></i>
            {{ activeType?.desc }}
          </div>
          <div class="btns">
            <button type="button" class="btn-cancel" @click="emit('close')">{{ t('common.cancel') }}</button>
            <button type="button" class="btn-confirm" @click="handleConfirm">{{ t('redis.createNow') }}</button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.modal-card {
  background: var(--bg-card);
  width: 460px;
  border-radius: 8px;
  border: 1px solid var(--border);
  box-shadow: 0 20px 60px var(--shadow);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  padding: 16px 20px;
  background: var(--bg-secondary-60);
  border-bottom: 1px solid var(--border);
  display: flex;
  justify-content: space-between;
  align-items: center;

  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    color: var(--accent);
    font-weight: 600;
  }

  .close-x {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 22px;
    cursor: pointer;
    transition: color 0.2s;

    &:hover { color: var(--error); }
  }
}

.modal-body { padding: 20px; }

.type-selector {
  display: flex;
  gap: 4px;
  background: var(--bg-input);
  padding: 4px;
  border-radius: 8px;
  margin-bottom: 20px;

  .type-item {
    flex: 1;
    text-align: center;
    padding: 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: bold;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    transition: all 0.2s;

    &.active {
      background: var(--bg-card);
      color: var(--accent);
      box-shadow: 0 2px 8px var(--shadow);
    }

    &:hover:not(.active) {
      color: var(--text-main);
    }
  }
}

.modal-form {
  display: flex;
  flex-direction: column;
  gap: 16px;

  .input-row { display: flex; gap: 12px; .flex-3 { flex: 3; } .flex-2 { flex: 2; } }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 8px;

    label {
      font-size: 10px;
      color: var(--text-dim);
      font-weight: bold;
      text-transform: uppercase;

      .hint {
        font-weight: normal;
        color: var(--text-dim);
        opacity: 0.6;
        text-transform: none;
        margin-left: 4px;
      }
    }
  }
}

.dark-input {
  width: 100%;
  background: var(--bg-input) !important;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  color: var(--text-main);
  font-size: 13px;
  outline: none;
  transition: all 0.2s;

  &:focus {
    border-color: var(--accent);
    background: var(--accent-05) !important;
    box-shadow: 0 0 0 2px var(--accent-10);
  }
}

.value-area {
  padding: 10px 0 10px 12px;
  height: 120px;
  resize: none;
  font-family: var(--font-terminal);
  line-height: 1.5;
}

.expand-wrapper {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;

  &.is-open {
    grid-template-rows: 1fr;
    margin-bottom: 4px;
  }

  .expand-content { min-height: 0; }
}

.modal-footer {
  padding: 15px 20px;
  background: var(--bg-secondary-60);
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: space-between;
  align-items: center;

  .type-hint {
    font-size: 11px;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 6px;
    font-style: italic;
  }

  .btns { display: flex; gap: 10px; }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    padding: 8px 18px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      color: var(--text-main);
      background: var(--accent-05);
    }
  }

  .btn-confirm {
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    padding: 8px 22px;
    border-radius: 6px;
    font-weight: bold;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      transform: translateY(-1px);
      box-shadow: 0 4px 12px var(--accent-30);
      filter: brightness(1.1);
    }

    &:active { transform: translateY(0); }
  }
}

.modal-fade-enter-active, .modal-fade-leave-active { transition: opacity 0.3s ease; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }
</style>
