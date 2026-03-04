<script setup lang="ts">
import { ref, watch } from 'vue';

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits(['close', 'confirm']);

// 内部状态
const redisTypes = [
  { label: 'String', value: 'string', color: '#9ece6a' },
  { label: 'Hash', value: 'hash', color: '#7aa2f7' },
  { label: 'List', value: 'list', color: '#e0af68' },
  { label: 'Set', value: 'set', color: '#bb9af7' }
];

const formData = ref({
  key: '',
  value: '',
  type: 'string',
  field: ''
});

// 每次打开时重置数据
watch(() => props.visible, (newVal) => {
  if (newVal) {
    formData.value = { key: '', value: '', type: 'string', field: '' };
  }
});

const handleConfirm = () => {
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
            <div class="form-group">
              <label>KEY 名称</label>
              <input v-model="formData.key" placeholder="user:profile:1001" class="dark-input" />
            </div>

            <div class="expand-wrapper" :class="{ 'is-open': formData.type === 'hash' }">
              <div class="expand-content">
                <div class="form-group">
                  <label>FIELD (字段名)</label>
                  <input v-model="formData.field" placeholder="username" class="dark-input" />
                </div>
              </div>
            </div>

            <div class="form-group">
              <label>VALUE 内容</label>
              <textarea
                  v-model="formData.value"
                  placeholder="输入值..."
                  class="dark-input value-area"
                  :style="{ color: redisTypes.find(t => t.value === formData.type)?.color }"
              ></textarea>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <div class="type-hint">
            <i class="fas fa-terminal"></i>
            {{ formData.type.toUpperCase() }} 模式
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
/* 定义局部变量，确保不污染全局 */
$bg-card: #1f2335;
$bg-input: #16161e;
$border-color: #292e42;
$text-main: #c0caf5;
$text-dim: #565f89;
$accent: #7aa2f7;

.modal-overlay {
  position: fixed; inset: 0;
  background: rgba(0, 0, 0, 0.8);
  backdrop-filter: blur(4px);
  display: flex; align-items: center; justify-content: center;
  z-index: 9999;
}

.modal-card {
  background: $bg-card;
  width: 420px;
  border-radius: 12px;
  border: 1px solid $border-color;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
  overflow: hidden;
  display: flex; flex-direction: column;
}

.modal-header {
  padding: 16px 20px;
  background: rgba(0, 0, 0, 0.2);
  border-bottom: 1px solid $border-color;
  display: flex; justify-content: space-between; align-items: center;

  .title {
    display: flex; align-items: center; gap: 10px;
    font-size: 14px; font-weight: bold; color: $accent;
  }
  .close-x {
    background: none; border: none; color: $text-dim;
    font-size: 24px; cursor: pointer; line-height: 1;
    &:hover { color: #f7768e; }
  }
}

.modal-body { padding: 20px; }

/* 类型选择器 Tab 样式 */
.type-selector {
  display: flex; gap: 4px; background: $bg-input; padding: 4px;
  border-radius: 8px; margin-bottom: 20px; border: 1px solid $border-color;

  .type-item {
    flex: 1; text-align: center; padding: 8px; border-radius: 6px;
    font-size: 11px; font-weight: bold; color: $text-dim;
    cursor: pointer; transition: 0.2s;
    display: flex; align-items: center; justify-content: center; gap: 6px;

    &.active { background: $border-color; color: $accent; }
    .dot { width: 6px; height: 6px; border-radius: 50%; }
  }
}

/* 输入框样式：强制解决大白底 */
.modal-form {
  display: flex; flex-direction: column; gap: 16px;

  .form-group {
    display: flex; flex-direction: column; gap: 8px;
    label { font-size: 10px; color: $text-dim; font-weight: bold; text-transform: uppercase; }
  }
}

.dark-input {
  width: 100%;
  background: $bg-input !important;
  border: 1px solid $border-color;
  border-radius: 6px;
  padding: 10px 12px;
  color: $text-main;
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s;

  &:focus { border-color: $accent; }
}

.value-area { height: 100px; resize: none; font-family: monospace; }

/* 核心：平滑高度过渡动画 */
.expand-wrapper {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.3s cubic-bezier(0.4, 0, 0.2, 1), margin 0.3s;
  overflow: hidden;

  &.is-open {
    grid-template-rows: 1fr;
    margin-bottom: 8px;
  }
  .expand-content { min-height: 0; }
}

.modal-footer {
  padding: 15px 20px;
  background: rgba(0, 0, 0, 0.15);
  border-top: 1px solid $border-color;
  display: flex; justify-content: space-between; align-items: center;

  .type-hint { font-size: 10px; color: $text-dim; display: flex; align-items: center; gap: 6px; }
  .btns { display: flex; gap: 10px; }

  .btn-cancel {
    background: transparent; border: 1px solid $border-color;
    color: $text-dim; padding: 8px 18px; border-radius: 6px; cursor: pointer;
    &:hover { color: $text-main; border-color: $text-dim; }
  }
  .btn-confirm {
    background: $accent; color: #1a1b26; border: none;
    padding: 8px 22px; border-radius: 6px; font-weight: bold; cursor: pointer;
    &:hover { opacity: 0.9; transform: translateY(-1px); }
  }
}

/* 进场动画 */
.modal-fade-enter-active, .modal-fade-leave-active { transition: opacity 0.3s; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }
</style>