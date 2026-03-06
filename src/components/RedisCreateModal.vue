<script setup lang="ts">
import { ref, watch, computed } from 'vue';

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

const handleConfirm = () => {
  if (!formData.value.key.trim()) return alert("KEY 名称不能为空");
  if (formData.value.type === 'hash' && !formData.value.field.trim()) return alert("Hash 必须填写 Field");
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
$bg-card: #1f2335;
$bg-input: #16161e;
$border-color: #292e42;
$text-main: #c0caf5;
$text-dim: #565f89;
$accent: #7aa2f7;

/* 基础结构 */
.modal-overlay {
  position: fixed; inset: 0; background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px); display: flex; align-items: center; justify-content: center; z-index: 9999;
}
.modal-card {
  background: $bg-card; width: 460px; border-radius: 12px; border: 1px solid $border-color;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5); display: flex; flex-direction: column; overflow: hidden;
}

/* Header & Body */
.modal-header {
  padding: 16px 20px; background: rgba(0, 0, 0, 0.2); border-bottom: 1px solid $border-color;
  display: flex; justify-content: space-between; align-items: center;
  .title { display: flex; align-items: center; gap: 10px; font-size: 14px; color: $accent; font-weight: 600; }
  .close-x { background: none; border: none; color: $text-dim; font-size: 22px; cursor: pointer; &:hover { color: #f7768e; } }
}
.modal-body { padding: 20px; }

/* 类型选择 */
.type-selector {
  display: flex; gap: 4px; background: $bg-input; padding: 4px; border-radius: 8px; margin-bottom: 20px;
  .type-item {
    flex: 1; text-align: center; padding: 8px; border-radius: 6px; font-size: 11px; font-weight: bold;
    color: $text-dim; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 6px;
    &.active { background: $border-color; color: $accent; }
    .dot { width: 6px; height: 6px; border-radius: 50%; }
  }
}

/* 表单布局 */
.modal-form {
  display: flex; flex-direction: column; gap: 16px;
  .input-row { display: flex; gap: 12px; .flex-3 { flex: 3; } .flex-2 { flex: 2; } }
  .form-group {
    display: flex; flex-direction: column; gap: 8px;
    label { font-size: 10px; color: $text-dim; font-weight: bold; text-transform: uppercase; .hint { font-weight: normal; opacity: 0.5; text-transform: none; } }
  }
}

.dark-input {
  width: 100%; background: $bg-input !important; border: 1px solid $border-color;
  border-radius: 6px; padding: 10px 12px; color: $text-main; font-size: 13px; outline: none;
  transition: all 0.2s;
  &:focus { border-color: $accent; background: rgba($accent, 0.02) !important; }
}

/* TTL 数字输入框专项美化 */
.ttl-input-container {
  position: relative; display: flex; align-items: center;
  .clock-icon { position: absolute; right: 12px; font-size: 12px; color: $text-dim; pointer-events: none; }
  &:focus-within .clock-icon { color: $accent; }
}

.ttl-input {
  padding-right: 32px !important;
  /* 针对 Chrome/Edge/Safari 优化箭头样式 */
  &::-webkit-inner-spin-button {
    opacity: 1;
    cursor: pointer;
    filter: invert(0.8) sepia(1) saturate(5) hue-rotate(175deg); /* 使箭头在暗色下变为亮蓝色/白色 */
    height: 20px;
  }
  /* 火狐隐藏默认箭头保持整洁 */
  -moz-appearance: textfield;
}

.value-area { height: 120px; resize: none; font-family: 'Fira Code', monospace; line-height: 1.5; }

/* 展开动画 */
.expand-wrapper {
  display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.3s cubic-bezier(0.4, 0, 0.2, 1); overflow: hidden;
  &.is-open { grid-template-rows: 1fr; margin-bottom: 4px; }
  .expand-content { min-height: 0; }
}

/* Footer */
.modal-footer {
  padding: 15px 20px; background: rgba(0, 0, 0, 0.15); border-top: 1px solid $border-color;
  display: flex; justify-content: space-between; align-items: center;
  .type-hint { font-size: 11px; color: $text-dim; display: flex; align-items: center; gap: 6px; font-style: italic; }
  .btns { display: flex; gap: 10px; }
  .btn-cancel { background: transparent; border: 1px solid $border-color; color: $text-dim; padding: 8px 18px; border-radius: 6px; cursor: pointer; &:hover { color: $text-main; } }
  .btn-confirm { background: $accent; color: #1a1b26; border: none; padding: 8px 22px; border-radius: 6px; font-weight: bold; cursor: pointer; &:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba($accent, 0.3); } }
}

.modal-fade-enter-active, .modal-fade-leave-active { transition: opacity 0.3s ease; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }
</style>