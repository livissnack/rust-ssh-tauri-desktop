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
    emit('save', { ...formData.value, port: Number(formData.value.port) });
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
              <button type="button" class="eye-btn" @click="showPassword = !showPassword">👁️</button>
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
$bg-dark: #0f111a;
$border-color: #292e42;
$accent: #7aa2f7;
$text-dim: #565f89;

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.modal-content {
  width: 440px;
  background: #1a1b26;
  border: 1px solid $border-color;
  border-radius: 20px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;

  .modal-header {
    padding: 20px 24px;
    display: flex;
    justify-content: space-between;

    h3 {
      margin: 0;
      color: #fff;
    }

    .close-btn {
      background: none;
      border: none;
      color: #fff;
      font-size: 20px;
      cursor: pointer;
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
        color: $text-dim;
        margin-bottom: 8px;
      }

      input,
      .styled-select {
        width: 100%;
        background: $bg-dark;
        border: 1px solid $border-color;
        padding: 12px;
        border-radius: 10px;
        color: #c0caf5;
        outline: none;
        box-sizing: border-box;

        &:focus {
          border-color: $accent !important;
          box-shadow: 0 0 0 2px rgba(122, 162, 247, 0.2);
        }
      }
    }

    .form-row {
      display: flex;
      gap: 15px;

      .flex-3 {
        flex: 3;
      }

      .flex-1 {
        flex: 1;
      }
    }

    .file-picker {
      display: flex;
      gap: 8px;

      input {
        flex: 1;
      }

      button {
        background: #24283b;
        border: 1px solid $border-color;
        color: #c0caf5;
        padding: 0 15px;
        border-radius: 10px;
        cursor: pointer;
      }
    }
  }

  .modal-footer {
    padding: 20px 24px;
    background: #16161e;
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    border-radius: 0 0 20px 20px;

    button {
      padding: 10px 20px;
      border-radius: 8px;
      cursor: pointer;
    }

    .cancel-btn {
      background: transparent;
      border: 1px solid $border-color;
      color: #a9b1d6;
    }

    .save-btn {
      background: $accent;
      border: none;
      color: #fff;
      font-weight: 600;
    }
  }
}

.scale-enter-active,
.scale-leave-active {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.scale-enter-from,
.scale-leave-to {
  opacity: 0;
  transform: scale(0.9);
}

.port-input-wrapper {
  display: flex;
  align-items: center;
  background: $bg-dark;
  border: 1px solid $border-color;
  border-radius: 8px;
  overflow: hidden;
  transition: border-color 0.2s;

  &:focus-within {
    border-color: $accent;
  }

  input {
    border: none !important;
    text-align: center;
    padding: 10px 0 !important;
    width: 100%;
    background: transparent !important;

    &:focus {
      outline: none;
    }
  }

  .port-btn {
    background: transparent;
    color: $text-dim;
    padding: 0 10px;
    font-size: 14px;
    height: 100%;
    cursor: pointer;
    border: none;

    &:hover {
      color: $accent;
      background: rgba(122, 162, 247, 0.1);
    }
  }
}

.password-wrapper {
  position: relative;
  display: flex;
  align-items: center;

  input {
    padding-right: 40px !important;
    width: 100%;
  }

  .eye-btn {
    position: absolute;
    right: 8px;
    background: transparent;
    border: none;
    padding: 4px;
    cursor: pointer;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.1s;

    &:hover {
      transform: scale(1.1);
    }
  }
}

/* 隐藏数字输入框的箭头 */
input::-webkit-outer-spin-button,
input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

input[type="number"] {
  -moz-appearance: textfield;
}
</style>
