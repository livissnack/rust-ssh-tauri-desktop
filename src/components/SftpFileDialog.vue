<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { formatSize } from '../utils/async';

export interface SftpFileDetail {
  path: string;
  name: string;
  isDir: boolean;
  size: number;
  permissions?: string;
  permissionsText?: string;
  modifiedAt?: number;
  accessedAt?: number;
  uid?: number;
  gid?: number;
  user?: string;
  group?: string;
}

const props = defineProps<{
  visible: boolean;
  mode: 'info' | 'rename' | 'chmod' | 'createFile' | 'createFolder';
  source: 'local' | 'remote';
  loading?: boolean;
  detail: SftpFileDetail | null;
  inputValue?: string;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'confirm', value: string): void;
}>();

const localInput = ref('');

watch(
  () => [props.visible, props.inputValue] as const,
  ([visible, value]) => {
    if (visible) localInput.value = value ?? '';
  },
  { immediate: true }
);

const title = computed(() => {
  if (props.mode === 'info') return '文件信息';
  if (props.mode === 'rename') return '重命名';
  if (props.mode === 'chmod') return '修改权限';
  if (props.mode === 'createFile') return '新建文件';
  return '新建文件夹';
});

const isCreateMode = computed(() => props.mode === 'createFile' || props.mode === 'createFolder');

const formatTime = (timestamp?: number) => {
  if (!timestamp) return '-';
  return new Date(timestamp * 1000).toLocaleString();
};

const handleConfirm = () => {
  if (props.mode === 'info') {
    emit('close');
    return;
  }
  emit('confirm', localInput.value.trim());
};
</script>

<template>
  <Transition name="confirm-fade">
    <div v-if="visible" class="sftp-dialog-overlay" @click="emit('close')">
      <div class="sftp-dialog-box" @click.stop>
        <div class="sftp-dialog-header">
          <i class="fas" :class="mode === 'info' ? 'fa-circle-info' : mode === 'rename' ? 'fa-pen' : mode === 'chmod' ? 'fa-key' : mode === 'createFile' ? 'fa-file-medical' : 'fa-folder-plus'"></i>
          <span>{{ title }}</span>
        </div>

        <div v-if="loading" class="sftp-dialog-loading">
          <i class="fas fa-spinner fa-spin"></i>
          <span>加载中...</span>
        </div>

        <template v-else-if="detail">
          <div v-if="mode === 'info'" class="info-grid">
            <div class="info-row">
              <span class="label">名称</span>
              <span class="value">{{ detail.name }}</span>
            </div>
            <div class="info-row">
              <span class="label">类型</span>
              <span class="value">{{ detail.isDir ? '目录' : '文件' }}</span>
            </div>
            <div class="info-row">
              <span class="label">大小</span>
              <span class="value">{{ detail.isDir ? '-' : formatSize(detail.size) }}</span>
            </div>
            <div class="info-row">
              <span class="label">路径</span>
              <span class="value path-value">{{ detail.path }}</span>
            </div>
            <div v-if="source === 'remote'" class="info-row">
              <span class="label">权限</span>
              <span class="value">
                <template v-if="detail.permissions">
                  {{ detail.permissionsText }} ({{ detail.permissions }})
                </template>
                <template v-else>-</template>
              </span>
            </div>
            <div v-if="source === 'remote' && (detail.user || detail.uid != null)" class="info-row">
              <span class="label">所有者</span>
              <span class="value">
                {{ detail.user || '-' }}
                <template v-if="detail.uid != null"> (UID: {{ detail.uid }})</template>
              </span>
            </div>
            <div v-if="source === 'remote' && (detail.group || detail.gid != null)" class="info-row">
              <span class="label">所属组</span>
              <span class="value">
                {{ detail.group || '-' }}
                <template v-if="detail.gid != null"> (GID: {{ detail.gid }})</template>
              </span>
            </div>
            <div class="info-row">
              <span class="label">修改时间</span>
              <span class="value">{{ formatTime(detail.modifiedAt) }}</span>
            </div>
            <div class="info-row">
              <span class="label">访问时间</span>
              <span class="value">{{ formatTime(detail.accessedAt) }}</span>
            </div>
          </div>

          <div v-else class="form-section">
            <label class="field-label">
              {{ mode === 'rename' ? '新名称' : mode === 'chmod' ? '权限 (八进制)' : '名称' }}
            </label>
            <input
              v-model="localInput"
              class="field-input"
              :placeholder="mode === 'rename' ? (detail?.name || '') : mode === 'chmod' ? '例如 755 或 0644' : mode === 'createFile' ? '例如 notes.txt' : '例如 backup'"
              @keyup.enter="handleConfirm"
            />
            <p v-if="mode === 'chmod'" class="field-hint">
              当前权限：
              <template v-if="detail.permissions">
                {{ detail.permissionsText }} ({{ detail.permissions }})
              </template>
              <template v-else>未知</template>
            </p>
          </div>
        </template>

        <div class="sftp-dialog-footer">
          <button class="btn-cancel" @click="emit('close')">
            {{ mode === 'info' ? '关闭' : '取消' }}
          </button>
          <button
            v-if="mode !== 'info'"
            class="btn-confirm"
            :disabled="loading || !localInput.trim()"
            @click="handleConfirm"
          >
            确定
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.sftp-dialog-overlay {
  position: fixed;
  inset: 0;
  background: var(--bg-secondary-60, rgba(0, 0, 0, 0.6));
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.sftp-dialog-box {
  width: min(460px, calc(100vw - 32px));
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 24px;
  box-shadow: 0 20px 25px -5px var(--shadow, rgba(0, 0, 0, 0.3));
}

.sftp-dialog-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-main);

  i {
    color: var(--accent);
  }
}

.sftp-dialog-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 120px;
  color: var(--text-dim);
}

.info-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 24px;
}

.info-row {
  display: grid;
  grid-template-columns: 88px 1fr;
  gap: 12px;
  font-size: 13px;
  line-height: 1.5;

  .label {
    color: var(--text-dim);
  }

  .value {
    color: var(--text-main);
    word-break: break-all;
  }

  .path-value {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12px;
  }
}

.form-section {
  margin-bottom: 24px;
}

.field-label {
  display: block;
  margin-bottom: 8px;
  font-size: 13px;
  color: var(--text-dim);
}

.field-input {
  width: 100%;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 14px;
  outline: none;

  &:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-10);
  }
}

.field-hint {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-dim);
}

.sftp-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;

  button {
    padding: 10px 20px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid transparent;
  }

  .btn-cancel {
    background: var(--bg-input);
    color: var(--text-dim);
    border-color: var(--border);
  }

  .btn-confirm {
    background: var(--accent);
    color: #fff;

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}

.confirm-fade-enter-active,
.confirm-fade-leave-active {
  transition: all 0.25s ease;
}

.confirm-fade-enter-from,
.confirm-fade-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(8px);
}
</style>
