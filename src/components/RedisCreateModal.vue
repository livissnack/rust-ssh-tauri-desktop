<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import {invoke} from "@tauri-apps/api/core";
import {toast} from "../utils/toast.ts";

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits(['close', 'confirm']);

// --- 数据类型配置 ---
const redisTypes = [
  { label: 'String', value: 'string', color: '#9ece6a', desc: '字符串 (SET)' },
  { label: 'Hash', value: 'hash', color: '#7aa2f7', desc: '哈希表 (HSET)' },
  { label: 'List', value: 'list', color: '#e0af68', desc: '列表 (RPUSH)' },
  { label: 'Set', value: 'set', color: '#bb9af7', desc: '集合 (SADD)' }
];

// --- 动态提示文字映射 ---
const placeholderMap: Record<string, { key: string; field?: string; value: string }> = {
  string: { key: '例如: config:settings', value: '输入字符串内容...' },
  hash: { key: '例如: user:1001', field: '例如: nickname', value: '输入字段值...' },
  list: { key: '例如: task:queue', value: '输入要插入列表的值...' },
  set: { key: '例如: tags:active', value: '输入要加入集合的成员...' }
};

// --- 表单状态 ---
const formData = ref({
  key: '',
  value: '',
  type: 'string',
  field: '',
  ttl: -1
});

const currentPlaceholder = computed(() => placeholderMap[formData.value.type]);

watch(() => props.visible, (newVal) => {
  if (newVal) {
    formData.value = { key: '', value: '', type: 'string', field: '', ttl: -1 };
  }
});

const handleConfirm = async () => {
  if (!formData.value.key.trim()) return alert("KEY 名称不能为空");
  if (formData.value.type === 'hash' && !formData.value.field.trim()) return alert("Hash 必须填写 Field");
  try {
    await invoke('redis_set_value', {
      ...formData.value,
      keyType: formData.value.type
    });
    toast.success("保存成功");
  } catch (err) {
    toast.error("保存失败");
  }
  emit('confirm', { ...formData.value });
};
</script>

<template>
  <Transition name="modal-fade">
    <div v-if="visible" class="modal-overlay" @click.self="emit('close')">
      <div class="modal-card">
        <div class="modal-header">
          <div class="title">
            <i class="fas fa-plus-square"></i>
            <span>新建 Redis 数据</span>
          </div>
          <button class="close-x" @click="emit('close')">&times;</button>
        </div>

        <div class="modal-body">
          <div class="type-selector">
            <div
                v-for="t in redisTypes"
                :key="t.value"
                class="type-item"
                :class="{ active: formData.type === t.value }"
                @click="formData.type = t.value"
            >
              <span class="dot" :style="{ background: t.color }"></span>
              {{ t.label }}
            </div>
          </div>

          <div class="modal-form">
            <div class="input-row">
              <div class="form-group flex-3">
                <label>KEY 名称</label>
                <input v-model="formData.key" :placeholder="currentPlaceholder.key" class="dark-input" />
              </div>

              <div class="form-group flex-2">
                <label>过期 (秒) <span class="hint">(-1永久)</span></label>
                <div class="ttl-input-container">
                  <input
                      v-model.number="formData.ttl"
                      type="number"
                      class="dark-input ttl-input"
                  />
                  <i class="fas fa-clock clock-icon"></i>
                </div>
              </div>
            </div>

            <div class="expand-wrapper" :class="{ 'is-open': formData.type === 'hash' }">
              <div class="expand-content">
                <div class="form-group">
                  <label>FIELD (字段名)</label>
                  <input v-model="formData.field" :placeholder="currentPlaceholder.field" class="dark-input" />
                </div>
              </div>
            </div>

            <div class="form-group">
              <label>VALUE 内容</label>
              <textarea
                  v-model="formData.value"
                  :placeholder="currentPlaceholder.value"
                  class="dark-input value-area"
                  :style="{ color: redisTypes.find(t => t.value === formData.type)?.color }"
              ></textarea>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <div class="type-hint">
            <i class="fas fa-info-circle"></i>
            {{ redisTypes.find(t => t.value === formData.type)?.desc }}
          </div>
          <div class="btns">
            <button class="btn-cancel" @click="emit('close')">取消</button>
            <button class="btn-confirm" @click="handleConfirm">立即创建</button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

/* 基础结构 */
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
  border-radius: 12px;
  border: 1px solid var(--border);
  box-shadow: 0 20px 60px var(--shadow);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Header & Body */
.modal-header {
  padding: 16px 20px;
  // 使用预计算的透明变量，避免 rgba(var) 报错
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

/* 类型选择器 */
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

/* 表单布局 */
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
    background: var(--accent-05) !important; // 修复点
    box-shadow: 0 0 0 2px var(--accent-10); // 修复点
  }
}

/* TTL 数字输入框专项美化 */
.ttl-input-container {
  position: relative;
  display: flex;
  align-items: center;

  .clock-icon {
    position: absolute;
    right: 12px;
    font-size: 12px;
    color: var(--text-dim);
    pointer-events: none;
  }

  &:focus-within .clock-icon { color: var(--accent); }
}

.ttl-input {
  padding-right: 32px !important;

  &::-webkit-inner-spin-button {
    opacity: 0.7;
    cursor: pointer;
    /* 亮色主题下保持默认，暗色主题下通过滤镜翻转箭头颜色 */
    filter: var(--spin-button-filter, invert(0.8));
    height: 18px;
  }

  -moz-appearance: textfield;
}

.value-area {
  padding: 10px 0 10px 12px;
  height: 120px;
  resize: none;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  line-height: 1.5;
}

/* 展开动画容器 */
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

/* Footer */
.modal-footer {
  padding: 15px 20px;
  background: var(--bg-secondary-60); // 修复点
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
      box-shadow: 0 4px 12px var(--accent-30); // 使用预定义的 accent-30
      filter: brightness(1.1);
    }

    &:active { transform: translateY(0); }
  }
}

/* 弹窗渐变动画 */
.modal-fade-enter-active, .modal-fade-leave-active { transition: opacity 0.3s ease; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }
</style>