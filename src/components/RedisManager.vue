<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import RedisCreateModal from './RedisCreateModal.vue';

// --- 状态管理 ---
const isConnectPanelVisible = ref(false);
const isConnecting = ref(false);
const searchQuery = ref('*');
const keysList = ref<string[]>([]);
const selectedKey = ref<string | null>(null);
const keyValue = ref<any>(null);

// --- 新增 Key 弹窗状态 ---
const isCreateModalVisible = ref(false);
const newKeyData = ref({
  key: '',
  value: '',
  type: 'string', // 默认类型
  field: ''       // Hash 专属字段
});

// Redis 类型配置
const redisTypes = [
  { label: 'String', value: 'string', color: '#9ece6a' },
  { label: 'Hash', value: 'hash', color: '#7aa2f7' },
  { label: 'List', value: 'list', color: '#e0af68' },
  { label: 'Set', value: 'set', color: '#bb9af7' }
];

// 表单数据
const connForm = ref({
  name: '本地开发环境',
  host: '127.0.0.1',
  port: 2552,
  password: '',
  db: 0
});

// 1. 连接 Redis
const handleConnect = async () => {
  isConnecting.value = true;
  try {
    await invoke('redis_connect', { config: connForm.value });
    toast.success("已成功连接至 Redis 服务器", "连接成功");
    isConnectPanelVisible.value = false;
    refreshKeys();
  } catch (err) {
    console.error(err);
    toast.error(`${err}`, "连接失败");
  } finally {
    isConnecting.value = false;
  }
};

// 2. 获取 Key 列表
const refreshKeys = async () => {
  try {
    keysList.value = await invoke('redis_get_keys', { pattern: searchQuery.value }) as string[];
  } catch (err) {
    toast.error("获取 Key 列表失败");
  }
};

// 3. 查看 Key 详情
const selectKey = async (key: string) => {
  selectedKey.value = key;
  try {
    keyValue.value = await invoke('redis_get_value', { key });
  } catch (err) {
    toast.error("读取内容失败");
  }
};

// 4. 保存/更新 Key (String)
const handleSave = async () => {
  if (!selectedKey.value) return;
  try {
    await invoke('redis_set_value', {
      key: selectedKey.value,
      value: String(keyValue.value)
    });
    toast.success("数据已成功保存", "更新成功");
  } catch (err) {
    toast.error(`保存失败: ${err}`);
  }
};

// --- 5. 执行新增 Key (支持多类型) ---
const handleCreateKey = async () => {
  const { key, value, type, field } = newKeyData.value;

  if (!key.trim()) return toast.error("Key 名称不能为空");
  if (type === 'hash' && !field.trim()) return toast.error("Hash 类型必须填写 Field");

  try {
    // 调用后端多类型指令 (需确保后端已实现 redis_add_key)
    await invoke('redis_add_key', {
      key,
      value,
      keyType: type,
      field: type === 'hash' ? field : null
    });

    toast.success(`Key "${key}" 创建成功`);
    isCreateModalVisible.value = false;

    // 重置数据
    newKeyData.value = { key: '', value: '', type: 'string', field: '' };

    // 刷新并选中
    await refreshKeys();
    selectKey(key);
  } catch (err) {
    toast.error(`创建失败: ${err}`);
  }
};

// 6. 删除 Key
const handleDeleteKey = async () => {
  if (!selectedKey.value) return;
  const confirmed = confirm(`确定要删除键 "${selectedKey.value}" 吗？`);
  if (!confirmed) return;

  try {
    await invoke('redis_del_key', { key: selectedKey.value });
    toast.success("删除成功");
    selectedKey.value = null;
    keyValue.value = null;
    refreshKeys();
  } catch (err) {
    toast.error(`删除失败: ${err}`);
  }
};

const onConfirmCreate = async (data: any) => {
  try {
    await invoke('redis_add_key', {
      key: data.key,
      value: data.value,
      keyType: data.type,
      field: data.type === 'hash' ? data.field : null
    });

    toast.success(`Key "${data.key}" 创建成功`);
    isCreateModalVisible.value = false; // 关闭弹窗
    await refreshKeys();
    selectKey(data.key);
  } catch (err) {
    toast.error(`创建失败: ${err}`);
  }
};

onMounted(refreshKeys);
</script>

<template>
  <div class="redis-manager">
    <div class="panel-header">
      <div class="title">
        <i class="fas fa-database" :class="{ 'fa-spin': isConnecting }"></i>
        <span>Redis 控制台 - {{ connForm.name }}</span>
      </div>
      <div class="actions">
        <button class="icon-btn" @click="isConnectPanelVisible = !isConnectPanelVisible" :class="{ 'is-active': isConnectPanelVisible }">
          <i class="fas fa-plug"></i>
        </button>
        <button class="icon-btn" @click="refreshKeys" title="刷新列表"><i class="fas fa-sync"></i></button>
      </div>
    </div>

    <div class="expand-container" :class="{ 'is-expanded': isConnectPanelVisible }">
      <div class="expand-content">
        <div class="config-form">
          <div class="input-row">
            <div class="input-group">
              <label>连接名称</label>
              <input v-model="connForm.name" placeholder="连接名称" class="form-input" />
            </div>
          </div>
          <div class="input-row">
            <div class="input-group">
              <label>主机地址</label>
              <input v-model="connForm.host" placeholder="127.0.0.1" class="form-input" />
            </div>
            <div class="input-group" style="flex: 0.4;">
              <label>端口</label>
              <input v-model.number="connForm.port" type="number" class="form-input" />
            </div>
          </div>
          <div class="input-row">
            <div class="input-group">
              <label>密码</label>
              <input v-model="connForm.password" type="password" placeholder="无密码请留空" class="form-input" />
            </div>
            <div class="input-group" style="flex: 0.4;">
              <label>DB</label>
              <input v-model.number="connForm.db" type="number" class="form-input" />
            </div>
          </div>
          <div class="form-actions">
            <span class="status-tag" :class="{ 'connected': !isConnecting }">
              <i class="fas fa-circle"></i> {{ isConnecting ? '正在连接...' : '服务就绪' }}
            </span>
            <button @click="handleConnect" class="btn-connect" :disabled="isConnecting">
              {{ isConnecting ? '连接中...' : '确认并保存' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="main-content">
      <div class="sidebar">
        <div class="sidebar-header-actions">
          <div class="search-box">
            <input v-model="searchQuery" @keyup.enter="refreshKeys" placeholder="过滤 (Key*)" />
            <i class="fas fa-filter"></i>
          </div>
          <button class="add-key-btn" @click="isCreateModalVisible = true" title="新增 Key">
            <i class="fas fa-plus"></i>
          </button>
        </div>

        <div class="sidebar-info">
          <span>{{ keysList.length }} KEYS LOADED</span>
        </div>
        <div class="key-list">
          <div
              v-for="key in keysList"
              :key="key"
              class="key-item"
              :class="{ 'is-active': selectedKey === key }"
              @click="selectKey(key)"
          >
            <i class="fas fa-file-code"></i>
            <span class="key-name">{{ key }}</span>
          </div>
        </div>
      </div>

      <div class="detail-view">
        <template v-if="selectedKey">
          <div class="detail-header">
            <span class="tag">String</span>
            <h3>{{ selectedKey }}</h3>
            <button class="delete-btn" @click="handleDeleteKey" title="删除键">
              <i class="fas fa-trash"></i>
            </button>
          </div>

          <div class="value-editor">
            <textarea v-model="keyValue" spellcheck="false" placeholder="Value is empty..."></textarea>
          </div>

          <div class="detail-footer">
            <button class="btn-save" @click="handleSave">
              <i class="fas fa-save"></i> 保存
            </button>
          </div>
        </template>

        <div v-else class="welcome-screen">
          <div class="hint-card">
            <div class="brand-logo"><i class="fas fa-terminal"></i></div>
            <h2>Redis 就绪</h2>
            <p>从左侧选择一个键开始编辑，或点击搜索框旁的 "+" 按钮新增数据。</p>
          </div>
        </div>
      </div>
    </div>

    <RedisCreateModal
        :visible="isCreateModalVisible"
        @close="isCreateModalVisible = false"
        @confirm="onConfirmCreate"
    />
  </div>
</template>

<style lang="scss" scoped>
@use "sass:color";

$bg-primary: #1a1b26;
$bg-secondary: #16161e;
$accent: #7aa2f7;
$border: #292e42;
$text-main: #c0caf5;
$text-dim: #565f89;
$success: #9ece6a;
$danger: #f7768e;

.redis-manager {
  height: 100%; width: 100%; display: flex; flex-direction: column;
  background: $bg-primary; color: $text-main; overflow: hidden;
  position: absolute; top: 0; left: 0;
  * { box-sizing: border-box; }
}

/* Header */
.panel-header {
  padding: 12px 18px; display: flex; justify-content: space-between; align-items: center;
  border-bottom: 1px solid $border; background: $bg-secondary;
  .title { display: flex; align-items: center; gap: 10px; color: $accent; font-weight: 600; font-size: 14px; }
  .actions { display: flex; gap: 8px; }
}

/* Sidebar */
.sidebar-header-actions {
  padding: 12px; display: flex; gap: 8px;
  .search-box { flex: 1; position: relative;
    input { width: 100%; background: $bg-secondary; border: 1px solid $border; border-radius: 6px; padding: 8px 12px; color: $text-main; font-size: 12px; }
    i { position: absolute; right: 10px; top: 50%; transform: translateY(-50%); color: $text-dim; font-size: 11px; }
  }
  .add-key-btn {
    background: rgba($accent, 0.1); border: 1px solid rgba($accent, 0.3); color: $accent;
    width: 32px; height: 32px; border-radius: 6px; cursor: pointer; transition: all 0.2s;
    &:hover { background: $accent; color: $bg-primary; }
  }
}

.main-content {
  flex: 1; display: flex; overflow: hidden;
  .sidebar {
    width: 220px; border-right: 1px solid $border; display: flex; flex-direction: column;
    .sidebar-info { padding: 6px 15px; font-size: 10px; color: $text-dim; border-bottom: 1px solid $border; }
    .key-list { flex: 1; overflow-y: auto;
      .key-item {
        padding: 10px 15px; font-size: 13px; display: flex; align-items: center; gap: 10px; cursor: pointer;
        &.is-active { background: rgba($accent, 0.12); color: $accent; border-left: 3px solid $accent; }
        .key-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
      }
    }
  }

  /* Detail View */
  .detail-view {
    flex: 1; display: flex; flex-direction: column; background: $bg-primary; height: 100%; min-width: 0;
    .detail-header { padding: 14px 20px; display: flex; align-items: center; gap: 12px; border-bottom: 1px solid $border;
      h3 { font-size: 14px; margin: 0; flex: 1; font-family: monospace; }
      .tag { background: rgba($accent, 0.15); color: $accent; padding: 2px 8px; border-radius: 4px; font-size: 10px; }
      .delete-btn { background: transparent; border: none; color: $text-dim; cursor: pointer; &:hover { color: $danger; } }
    }
    .value-editor { flex: 1; min-height: 0; textarea { width: 100%; height: 100%; background: transparent; border: none; color: $success; font-family: monospace; padding: 20px; resize: none; outline: none; } }
    .detail-footer {
      padding: 12px 20px 28px; border-top: 1px solid $border; background: color.adjust($bg-secondary, $lightness: 1%); display: flex; justify-content: flex-end;
      .btn-save { background: $accent; color: $bg-primary; border: none; padding: 10px 24px; border-radius: 6px; font-weight: bold; cursor: pointer; &:hover { transform: translateY(-2px); } }
    }
  }
}

.modal-form {
  display: flex; flex-direction: column; gap: 14px;
  .modal-input-group {
    display: flex; flex-direction: column; gap: 6px;
    label { font-size: 11px; color: $text-dim; font-weight: bold; }
    .value-area { height: 100px; resize: none; font-family: monospace; }
  }
}

.modal-footer-btns {
  padding: 15px 20px; background: rgba(0, 0, 0, 0.15); display: flex; justify-content: space-between; align-items: center;
  .type-hint { font-size: 10px; color: $text-dim; font-style: italic; }
  .btns-row { display: flex; gap: 10px; }
  .btn-cancel { background: transparent; border: 1px solid $border; color: $text-main; padding: 8px 16px; border-radius: 6px; cursor: pointer; }
  .btn-confirm { background: $accent; color: $bg-primary; border: none; padding: 8px 20px; border-radius: 6px; font-weight: bold; cursor: pointer; }
}

/* Animations */
.animate-slide { animation: slideDown 0.2s ease-out; }
@keyframes slideDown { from { opacity: 0; transform: translateY(-8px); } to { opacity: 1; transform: translateY(0); } }

/* Shared Components */
.welcome-screen { flex: 1; display: flex; align-items: center; justify-content: center; .hint-card { text-align: center; .brand-logo { font-size: 40px; color: rgba($accent, 0.15); margin-bottom: 20px; } } }
.config-form { padding: 20px; background: color.adjust($bg-secondary, $lightness: 2%); border-bottom: 1px solid $border; display: flex; flex-direction: column; gap: 15px; .input-row { display: flex; gap: 12px; } .input-group { display: flex; flex-direction: column; gap: 6px; flex: 1; min-width: 0; label { font-size: 11px; color: $text-dim; font-weight: 600; } } .form-input { background: $bg-primary; border: 1px solid $border; border-radius: 6px; padding: 10px; color: $text-main; width: 100%; &:focus { outline: none; border-color: $accent; } } .form-actions { display: flex; justify-content: space-between; align-items: center; } }
.icon-btn { background: transparent; border: none; color: $text-dim; cursor: pointer; padding: 8px; border-radius: 6px; &.is-active { color: $accent; background: rgba($accent, 0.1); } }
.btn-connect { background: $accent; color: $bg-primary; border: none; padding: 8px 20px; border-radius: 6px; font-weight: bold; cursor: pointer; &:disabled { opacity: 0.5; } }
.status-tag { font-size: 12px; color: $text-dim; display: flex; align-items: center; gap: 6px; &.connected i { color: $success; } }
.expand-container { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.35s ease; overflow: hidden; &.is-expanded { grid-template-rows: 1fr; } .expand-content { min-height: 0; } }
</style>