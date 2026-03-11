<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

const props = defineProps<{
  isOpen: boolean;
  isEditing: boolean;
  server: any;
  servers: any[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', server: any): void;
}>();

const showPassword = ref(false);
const formData = ref({ ...props.server });

const filteredServers = computed(() => {
  return props.servers.filter(x => x.id !== formData.value.id);
});

watch(() => props.server, (newServer) => {
  formData.value = { ...newServer };
}, { deep: true });

watch(() => props.isOpen, (isOpen) => {
  if (isOpen) {
    formData.value = { ...props.server };
  }
});

const selectKeyFile = async () => {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'SSH Private Key', extensions: ['*', 'pem', 'key'] }]
  });
  if (selected) formData.value.private_key_path = selected as string;
};

const saveHost = async () => {
  if (formData.value.name && formData.value.host) {
    console.log({
      ...formData.value,
      port: Number(formData.value.port)
    }, 'mm-----')
    emit('save', {
      ...formData.value,
      port: Number(formData.value.port)
    });
  }
};

const closeModal = () => {
  showPassword.value = false;
  emit('close');
};
</script>

<template>
  <Transition name="scale">
    <div v-if="isOpen" class="modal-overlay" @click.self="closeModal">
      <div class="modal-content">
        <div class="modal-header">
          <h3>{{ isEditing ? 'Edit Host' : 'Add New Host' }}</h3>
          <button class="close-btn" @click="closeModal">×</button>
        </div>
        <div class="modal-body scrollable">
          <div class="form-group">
            <label>Display Name</label>
            <input v-model="formData.name" type="text" placeholder="Server Alpha" />
          </div>
          <div class="form-row">
            <div class="form-group flex-3">
              <label>Hostname / IP</label>
              <input v-model="formData.host" type="text" placeholder="127.0.0.1" />
            </div>
            <div class="form-group flex-1">
              <label>Port</label>
              <div class="port-input-wrapper">
                <button type="button" class="port-btn" @click="formData.port > 1 && formData.port--">-</button>
                <input v-model.number="formData.port" type="number" />
                <button type="button" class="port-btn" @click="formData.port < 65535 && formData.port++">+</button>
              </div>
            </div>
          </div>
          <div class="form-group">
            <label>Username</label>
            <input v-model="formData.username" type="text" />
          </div>
          <div class="form-group">
            <label>Auth Type</label>
            <select v-model="formData.auth_type" class="styled-select">
              <option value="password">Password</option>
              <option value="key">SSH Key</option>
            </select>
          </div>
          <div v-if="formData.auth_type === 'password'" class="form-group">
            <label>Password</label>
            <div class="password-wrapper">
              <input v-model="formData.password" :type="showPassword ? 'text' : 'password'" />
              <button type="button" class="eye-btn" @click="showPassword = !showPassword">
                <i class="fas" :class="showPassword ? 'fa-eye-slash' : 'fa-eye'"></i>
              </button>
            </div>
          </div>
          <div v-else class="form-group">
            <label>Private Key Path</label>
            <div class="file-picker">
              <input v-model="formData.private_key_path" type="text" readonly />
              <button @click="selectKeyFile">Browse</button>
            </div>
          </div>
          <div class="form-group">
            <label>Jump Host (Optional)</label>
            <select v-model="formData.jump_host_id" class="styled-select">
              <option value="">Direct Connection</option>
              <option v-for="s in filteredServers" :key="s.id" :value="s.id">{{ s.name }}</option>
            </select>
          </div>
        </div>
        <div class="modal-footer">
          <button class="cancel-btn" @click="closeModal">Cancel</button>
          <button class="save-btn" @click="saveHost">Save</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

/* --- 1. 遮罩层与弹窗主体 --- */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.modal-content {
  width: 460px;
  max-width: 95vw;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 20px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 25px 50px -12px var(--shadow);
  overflow: hidden;
}

/* --- 2. 头部样式 --- */
.modal-header {
  padding: 20px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--bg-secondary-60);
  border-bottom: 1px solid var(--border-30);

  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-main);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 20px;
    cursor: pointer;
    transition: color 0.2s;
    &:hover { color: var(--error); }
  }
}

/* --- 3. 表单主体逻辑 (修复重点) --- */
.modal-body {
  padding: 24px;
  overflow-y: auto;
  overflow-x: hidden;
  flex: 1;

  .form-group {
    margin-bottom: 20px;
    width: 100%;
    box-sizing: border-box;

    label {
      display: block;
      font-size: 11px;
      color: var(--text-dim);
      font-weight: 700;
      text-transform: uppercase;
      margin-bottom: 8px;
      letter-spacing: 0.8px;
    }

    /* [A] 共有基础样式：input 和 select 共享外壳 */
    input[type="text"],
    input[type="password"],
    input[type="number"],
    .styled-select {
      width: 100%;
      height: 40px;
      background: var(--bg-input);
      border: 1px solid var(--border);
      padding: 0 12px;
      border-radius: 10px;
      color: var(--text-main);
      font-size: 13px;
      outline: none;
      box-sizing: border-box;
      transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

      &:focus {
        border-color: var(--accent) !important;
        background-color: var(--accent-05);
        box-shadow: 0 0 0 3px var(--accent-15);
      }
    }

    /* [B] Input 专属样式：确保背景纯净，无箭头 */
    input[type="text"],
    input[type="password"],
    input[type="number"] {
      background-image: none !important;
    }

    /* [C] Select 专属样式：添加自定义箭头 */
    .styled-select {
      cursor: pointer;
      appearance: none;
      -webkit-appearance: none;
      /* 注入自定义箭头 SVG */
      background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 24 24' stroke='%23888' stroke-width='2.5'%3E%3Cpath stroke-linecap='round' stroke-linejoin='round' d='M19 9l-7 7-7-7'/%3E%3C/svg%3E") !important;
      background-repeat: no-repeat !important;
      background-position: calc(100% - 12px) center !important;
      background-size: 14px !important;
      padding-right: 36px !important;

      option {
        background-color: var(--bg-secondary);
        color: var(--text-main);
        padding: 12px;
      }
    }
  }

  /* 布局修复：IP 与 Port 并排 */
  .form-row {
    display: flex;
    gap: 16px;
    width: 100%;
    box-sizing: border-box;

    .flex-3 { flex: 3; min-width: 0; }
    .flex-1 { flex: 1.5; min-width: 110px; }
  }
}

/* --- 4. 增强型输入组件 --- */

/* 端口调节组件 */
.port-input-wrapper {
  display: flex;
  align-items: center;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 10px;
  height: 40px;
  overflow: hidden;
  box-sizing: border-box;

  &:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-15);
  }

  input {
    flex: 1;
    width: 0;
    border: none !important;
    text-align: center;
    background: transparent !important;
    color: var(--accent) !important;
    font-weight: 600;
    font-family: 'JetBrains Mono', monospace;
    padding: 0 !important;
    box-shadow: none !important;
    background-image: none !important; // 确保这里也没有箭头
  }

  .port-btn {
    width: 30px;
    height: 100%;
    background: transparent;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    &:hover {
      background: var(--accent-10);
      color: var(--accent);
    }
  }
}

/* 密码包装器 */
.password-wrapper {
  position: relative;
  width: 100%;
  input { padding-right: 40px !important; }
  .eye-btn {
    position: absolute;
    right: 4px; top: 50%;
    transform: translateY(-50%);
    width: 32px; height: 32px;
    background: transparent; border: none;
    color: var(--text-dim); cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    &:hover { color: var(--accent); }
  }
}

/* 文件选择器 */
.file-picker {
  display: flex;
  gap: 8px;
  width: 100%;
  input { flex: 1; min-width: 0; }
  button {
    flex-shrink: 0;
    padding: 0 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-main);
    border-radius: 10px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    &:hover {
      background: var(--accent-10);
      border-color: var(--accent);
      color: var(--accent);
    }
  }
}

/* --- 5. 底部按钮区 --- */
.modal-footer {
  padding: 16px 24px;
  background: var(--bg-secondary-30);
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  border-top: 1px solid var(--border-30);

  button {
    padding: 10px 24px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    &:hover {
      background: var(--bg-input);
      color: var(--text-main);
    }
  }

  .save-btn {
    background: var(--accent);
    border: none;
    color: #fff;
    box-shadow: 0 4px 12px var(--accent-20);
    &:hover {
      transform: translateY(-1px);
      filter: brightness(1.1);
      box-shadow: 0 6px 16px var(--accent-30);
    }
    &:active { transform: translateY(0); }
  }
}

/* --- 6. 动画与杂项 --- */
.scale-enter-active, .scale-leave-active {
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.scale-enter-from, .scale-leave-to {
  opacity: 0; transform: scale(0.95) translateY(10px);
}

/* 隐藏 Number 默认箭头 */
input::-webkit-outer-spin-button,
input::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
input[type="number"] { -moz-appearance: textfield; }
</style>
