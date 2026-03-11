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

/* 遮罩层 */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.7); // 保持固定深色以增强对比
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

/* 弹窗主体 */
.modal-content {
  width: 440px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 20px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 25px 50px -12px var(--shadow);

  .modal-header {
    padding: 20px 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-secondary-60); // 修复点：使用预计算透明变量

    h3 {
      margin: 0;
      font-size: 16px;
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

  .modal-body {
    padding: 24px;
    overflow-y: auto;
    flex: 1;

    .form-group {
      margin-bottom: 18px;

      label {
        display: block;
        font-size: 11px;
        color: var(--text-dim); // 使用 text-dim 替代 muted
        font-weight: 600;
        text-transform: uppercase;
        margin-bottom: 8px;
        letter-spacing: 0.5px;
      }

      input,
      .styled-select {
        width: 100%;
        background: var(--bg-input);
        border: 1px solid var(--border);
        padding: 12px;
        border-radius: 10px;
        color: var(--text-main);
        font-size: 13px;
        outline: none;
        box-sizing: border-box;
        transition: all 0.2s ease;

        &:focus {
          border-color: var(--accent) !important;
          background: var(--accent-05); // 修复点
          box-shadow: 0 0 0 3px var(--accent-10); // 修复点
        }
      }
    }

    .form-row {
      display: flex;
      gap: 15px;
      .flex-3 { flex: 3; }
      .flex-1 { flex: 1; }
    }

    /* 文件选择器 */
    .file-picker {
      display: flex;
      gap: 8px;
      input { flex: 1; }

      button {
        background: var(--bg-secondary);
        border: 1px solid var(--border);
        color: var(--text-main);
        padding: 0 15px;
        border-radius: 10px;
        cursor: pointer;
        transition: all 0.2s;
        &:hover {
          background: var(--accent-10); // 修复点
          border-color: var(--accent);
          color: var(--accent);
        }
      }
    }
  }

  .modal-footer {
    padding: 20px 24px;
    background: var(--bg-secondary);
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    border-radius: 0 0 20px 20px;
    border-top: 1px solid var(--border);

    button {
      padding: 10px 22px;
      border-radius: 8px;
      cursor: pointer;
      font-size: 13px;
      transition: all 0.2s;
    }

    .cancel-btn {
      background: transparent;
      border: 1px solid var(--border);
      color: var(--text-dim);
      &:hover {
        background: var(--accent-05);
        color: var(--text-main);
      }
    }

    .save-btn {
      background: var(--accent);
      border: none;
      color: var(--bg-primary);
      font-weight: 600;
      &:hover {
        filter: brightness(1.1);
        transform: translateY(-1px);
        box-shadow: 0 4px 12px var(--accent-30); // 修复点
      }
      &:active { transform: translateY(0); }
    }
  }
}

/* 端口输入框微调组件 */
.port-input-wrapper {
  display: flex;
  align-items: center;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  transition: all 0.2s;

  &:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-10);
  }

  input {
    border: none !important;
    text-align: center;
    padding: 10px 0 !important;
    width: 100%;
    background: transparent !important;
    color: var(--accent) !important;
    font-weight: bold;
    font-family: 'JetBrains Mono', monospace;
    &:focus { outline: none; }
  }

  .port-btn {
    background: transparent;
    color: var(--text-dim);
    padding: 0 10px;
    font-size: 14px;
    height: 100%;
    cursor: pointer;
    border: none;
    transition: all 0.2s;

    &:hover {
      color: var(--accent);
      background: var(--accent-10);
    }
  }
}

/* 密码显隐组件 */
.password-wrapper {
  position: relative;
  display: flex;
  align-items: center;

  input { padding-right: 40px !important; width: 100%; }

  .eye-btn {
    position: absolute;
    right: 8px;
    background: transparent;
    border: none;
    padding: 4px;
    cursor: pointer;
    font-size: 16px;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;

    &:hover {
      color: var(--accent);
      transform: scale(1.1);
    }
  }
}

/* 动画效果保持不变 */
.scale-enter-active,
.scale-leave-active {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.scale-enter-from,
.scale-leave-to {
  opacity: 0;
  transform: scale(0.9);
}

input::-webkit-outer-spin-button,
input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
input[type="number"] { -moz-appearance: textfield; }
</style>
