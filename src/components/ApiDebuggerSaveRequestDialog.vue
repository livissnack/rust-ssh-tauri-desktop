<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { t } from '../utils/i18n.ts';
import type { ApiCollection, RequestSnapshot } from '../utils/apiDebuggerStorage.ts';
import {
  defaultRequestName,
  getSnapshotProtocol,
  snapshotPreviewText,
  snapshotTagLabel,
} from '../utils/apiDebuggerStorage.ts';

export type SaveRequestPayload = {
  collectionId: string;
  name: string;
  description: string;
  requestId?: string;
};

const props = defineProps<{
  visible: boolean;
  collections: ApiCollection[];
  snapshot: RequestSnapshot;
  initialCollectionId?: string;
  editRequest?: {
    id: string;
    collectionId: string;
    name: string;
    description?: string;
  } | null;
}>();

const emit = defineEmits<{
  close: [];
  save: [payload: SaveRequestPayload];
}>();

const name = ref('');
const description = ref('');
const collectionId = ref('');

const isEdit = computed(() => !!props.editRequest);
const title = computed(() => t(isEdit.value ? 'apiDebugger.saveDialog.editTitle' : 'apiDebugger.saveDialog.saveTitle'));
const previewTag = computed(() => snapshotTagLabel(props.snapshot));
const previewExtra = computed(() => snapshotPreviewText(props.snapshot));
const previewProtocol = computed(() => getSnapshotProtocol(props.snapshot));

watch(
  () => [props.visible, props.initialCollectionId, props.editRequest, props.snapshot] as const,
  ([visible]) => {
    if (!visible) return;
    if (props.editRequest) {
      name.value = props.editRequest.name;
      description.value = props.editRequest.description ?? '';
      collectionId.value = props.editRequest.collectionId;
      return;
    }
    name.value = defaultRequestName(props.snapshot);
    description.value = '';
    collectionId.value =
      props.initialCollectionId
      ?? props.collections[0]?.id
      ?? '';
  },
  { immediate: true },
);

const handleSave = () => {
  if (!name.value.trim()) return;
  if (!collectionId.value) return;
  emit('save', {
    collectionId: collectionId.value,
    name: name.value.trim(),
    description: description.value.trim(),
    requestId: props.editRequest?.id,
  });
};
</script>

<template>
  <Teleport to="body">
    <Transition name="confirm-fade">
      <div v-if="visible" class="save-dialog-overlay" @click="emit('close')">
        <div class="save-dialog-box" @click.stop>
          <div class="save-dialog-header">
            <i class="fas" :class="isEdit ? 'fa-pen' : 'fa-save'"></i>
            <span>{{ title }}</span>
          </div>

          <div class="url-preview">
            <span class="method-tag" :class="`protocol-${previewProtocol}`">{{ previewTag }}</span>
            <div class="url-preview__content">
              <span class="url-text">{{ snapshot.url || t('apiDebugger.saveDialog.noUrl') }}</span>
              <span v-if="previewExtra" class="url-extra">{{ previewExtra }}</span>
            </div>
          </div>

          <div class="form-section">
            <label class="field-label">{{ t('apiDebugger.saveDialog.requestName') }}</label>
            <input
              v-model="name"
              class="field-input"
              :placeholder="t('apiDebugger.saveDialog.requestNamePlaceholder')"
              @keyup.enter="handleSave"
            />
          </div>

          <div class="form-section">
            <label class="field-label">{{ t('apiDebugger.saveDialog.description') }}</label>
            <textarea
              v-model="description"
              class="field-textarea"
              :placeholder="t('apiDebugger.saveDialog.descriptionPlaceholder')"
            ></textarea>
          </div>

          <div class="form-section">
            <label class="field-label">{{ t('apiDebugger.saveDialog.collection') }}</label>
            <div class="collection-picker">
              <button
                v-for="collection in collections"
                :key="collection.id"
                type="button"
                class="collection-chip"
                :class="{ active: collectionId === collection.id, disabled: isEdit }"
                :disabled="isEdit"
                @click="collectionId = collection.id"
              >
                <i class="fas fa-folder"></i>
                <span>{{ collection.name }}</span>
                <span class="chip-count">{{ collection.requests.length }}</span>
              </button>
            </div>
          </div>

          <div class="save-dialog-footer">
            <button type="button" class="btn-cancel" @click="emit('close')">{{ t('common.cancel') }}</button>
            <button
              type="button"
              class="btn-confirm"
              :disabled="!name.trim() || !collectionId"
              @click="handleSave"
            >
              {{ isEdit ? t('apiDebugger.saveDialog.saveChanges') : t('common.save') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped lang="scss">
.save-dialog-overlay {
  position: fixed;
  inset: 0;
  background: var(--bg-secondary-60, rgba(0, 0, 0, 0.6));
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.save-dialog-box {
  width: min(440px, calc(100vw - 32px));
  max-height: calc(100vh - 40px);
  overflow-y: auto;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 20px;
  box-shadow: 0 20px 25px -5px var(--shadow, rgba(0, 0, 0, 0.3));
}

.save-dialog-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
  font-size: 15px;
  font-weight: 700;
  color: var(--text-main);

  i { color: var(--accent); font-size: 14px; }
}

.url-preview {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px;
  margin-bottom: 14px;
  border-radius: 8px;
  background: var(--bg-input);
  border: 1px solid var(--border-30);

  &__content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
}

.url-text {
  font-family: var(--font-terminal);
  font-size: 10px;
  line-height: 1.45;
  color: var(--text-dim);
  word-break: break-all;
}

.url-extra {
  font-size: 10px;
  line-height: 1.45;
  color: var(--text-dim);
  opacity: 0.9;
}

.method-tag {
  flex-shrink: 0;
  min-width: 42px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--accent-10);
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  text-align: center;

  &.protocol-ws {
    background: color-mix(in srgb, #06b6d4 18%, transparent);
    color: #0891b2;
  }

  &.protocol-sse {
    background: color-mix(in srgb, #8b5cf6 18%, transparent);
    color: #7c3aed;
  }

  &.protocol-socketio {
    background: color-mix(in srgb, #f59e0b 18%, transparent);
    color: #d97706;
  }

  &.protocol-mqtt {
    background: color-mix(in srgb, #10b981 18%, transparent);
    color: #059669;
  }
}

.form-section {
  margin-bottom: 14px;
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

.field-textarea {
  width: 100%;
  min-height: 72px;
  padding: 10px 12px;
  box-sizing: border-box;
  border-radius: 8px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 12px;
  line-height: 1.5;
  resize: vertical;

  &:focus {
    outline: none;
    border-color: var(--accent-30);
    box-shadow: 0 0 0 3px var(--accent-10);
  }
}

.collection-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.collection-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 100%;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
  color: var(--text-dim);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s;

  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip-count {
    flex-shrink: 0;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--bg-secondary);
    font-size: 9px;
  }

  &.active {
    border-color: var(--accent-30);
    background: var(--accent-10);
    color: var(--accent);
  }

  &.disabled {
    cursor: default;
    opacity: 0.85;
  }

  &:hover:not(.disabled):not(.active) {
    border-color: var(--accent-20);
    color: var(--text-main);
  }
}

.save-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
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
