<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import RedisCreateModal from './RedisCreateModal.vue';

const isConnectPanelVisible = ref(false);
const isConnecting = ref(false);
const searchQuery = ref('*');
const keysList = ref<string[]>([]);
const selectedKey = ref<string | null>(null);
const keyValue = ref<any>(null);
const savedConfigs = ref<any[]>([]);
const isConfigListVisible = ref(false);

const isCreateModalVisible = ref(false);
// --- 新增：当前选中键的元数据 ---
const selectedKeyType = ref('string'); // 默认为 string
const selectedField = ref<string | null>(null); // Hash 专用
const selectedTTL = ref(-1); // 过期时间
const newKeyData = ref({
  key: '',
  value: '',
  type: 'string',
  field: ''
});

const connForm = ref({
  id: '',
  name: '本地开发环境',
  host: '127.0.0.1',
  port: 2552,
  password: '',
  db: 0
});

const loadSavedConfigs = async () => {
  try {
    savedConfigs.value = await invoke('get_redis_configs');
  } catch (err) {
    console.error("加载 Redis 配置失败", err);
  }
};

const selectSavedConfig = (config: any) => {
  connForm.value = { ...config };
  isConfigListVisible.value = false;
  handleConnect();
};

const handleConnect = async () => {
  isConnecting.value = true;
  try {
    // await invoke('clear_all_redis_configs');
    await invoke('redis_connect', { config: connForm.value });

    const savedConfig = await invoke('save_redis_config', { config: connForm.value }) as any;

    connForm.value.id = savedConfig.id;

    toast.success("连接成功并已保存配置", "成功");
    isConnectPanelVisible.value = false;

    await loadSavedConfigs();
    refreshKeys();
  } catch (err) {
    toast.error(`${err}`, "连接失败");
  } finally {
    isConnecting.value = false;
  }
};

const handleDeleteConfig = async (id: string, event: Event) => {
  event.stopPropagation();
  if (!confirm("确定要删除此连接配置吗？")) return;
  try {
    await invoke('delete_redis_config', { id });
    await loadSavedConfigs();
    toast.success("配置已删除");
  } catch (err) {
    toast.error("删除失败");
  }
};

const refreshKeys = async () => {
  try {
    keysList.value = await invoke('redis_get_keys', { pattern: searchQuery.value }) as string[];
  } catch (err) {
    toast.error("获取 Key 列表失败");
  }
};

const selectKey = async (key: string) => {
  selectedKey.value = key;
  try {
    // 1. 并发获取数据，提高效率
    const [value, type, ttl] = await Promise.all([
      invoke('redis_get_value', { key }),
      invoke('redis_get_type', { key }) as Promise<string>, // 需要后端实现
      invoke('redis_get_ttl', { key }) as Promise<number>   // 需要后端实现
    ]);

    keyValue.value = value;
    selectedKeyType.value = type;
    selectedTTL.value = ttl;

    // 如果是 Hash 类型，初始化 field 为空（或者根据你的 UI 逻辑调整）
    selectedField.value = null;

  } catch (err) {
    console.error("加载 Key 详情失败:", err);
    toast.error("读取内容失败");
  }
};

const handleSave = async () => {
  if (!selectedKey.value) return;

  try {
    await invoke('redis_set_value', {
      key: selectedKey.value,
      value: String(keyValue.value),
      keyType: selectedKeyType.value, // 现在有了
      field: selectedField.value,      // 现在有了
      ttl: selectedTTL.value          // 现在有了
    });

    toast.success("数据已成功保存", "更新成功");
    // 保存后刷新列表，以防过期时间或类型发生变化
    await refreshKeys();
  } catch (err) {
    console.error(err);
    toast.error(`保存失败: ${err}`);
  }
};

const handleDeleteKey = async () => {
  if (!selectedKey.value) return;
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
    await invoke('redis_set_value', {
      key: data.key,
      value: data.value,
      keyType: data.type,
      field: data.type === 'hash' ? data.field : null,
      ttl: data.ttl
    });

    toast.success(`Key "${data.key}" 创建成功`, "操作成功");
    isCreateModalVisible.value = false;

    await refreshKeys();

    setTimeout(() => {
      selectKey(data.key);
    }, 100);

  } catch (err) {
    toast.error(`创建失败: ${err}`, "错误");
  }
};

onMounted(() => {
  loadSavedConfigs();
  refreshKeys();
});
</script>

<template>
  <div class="redis-manager">
    <div class="panel-header">
      <div class="title">
        <i class="fas fa-database" :class="{ 'fa-spin': isConnecting }"></i>
        <span>{{ connForm.name }}</span>
      </div>

      <div class="actions">
        <button class="icon-btn" @click="isConfigListVisible = !isConfigListVisible" title="历史连接">
          <i class="fas fa-history"></i>
        </button>
        <button class="icon-btn" @click="isConnectPanelVisible = !isConnectPanelVisible" :class="{ 'is-active': isConnectPanelVisible }">
          <i class="fas fa-plug"></i>
        </button>
      </div>

      <div v-if="isConfigListVisible" class="saved-configs-dropdown animate-slide">
        <div v-for="cfg in savedConfigs" :key="cfg.id" class="config-item" @click="selectSavedConfig(cfg)">
          <div class="cfg-info">
            <span class="cfg-name">{{ cfg.name }}</span>
            <span class="cfg-addr">{{ cfg.host }}:{{ cfg.port }}</span>
          </div>
          <i class="fas fa-times-circle delete-cfg-icon" @click="handleDeleteConfig(cfg.id, $event)"></i>
        </div>
        <div v-if="savedConfigs.length === 0" class="empty-hint">暂无连接</div>
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
            <h3 class="redis-key-name" :title="selectedKey">{{ selectedKey }}</h3>
            <button class="delete-btn" @click="handleDeleteKey" title="删除">
              <i class="fas fa-trash"></i>
            </button>
          </div>

          <div class="value-editor">
            <textarea v-model="keyValue" spellcheck="false" placeholder="Value is empty..."></textarea>
          </div>

          <div class="detail-footer">
            <button class="btn-save" v-if="selectedKeyType === 'string'" @click="handleSave">
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
      h3 { font-size: 14px; margin: 0; flex: 1; font-family: monospace; width: 120px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;}
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

/* 已保存配置下拉列表 */
.saved-configs-dropdown {
  background: $bg-secondary;
  border: 1px solid $border;
  border-radius: 8px;
  margin: 10px 18px;
  max-height: 200px;
  overflow-y: auto;
  box-shadow: 0 10px 20px rgba(0,0,0,0.3);

  .config-item {
    padding: 10px 15px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    border-bottom: 1px solid rgba($border, 0.5);
    &:hover { background: rgba($accent, 0.1); }

    .cfg-info {
      display: flex;
      flex-direction: column;
      .cfg-name { font-size: 13px; font-weight: bold; color: $accent; }
      .cfg-addr { font-size: 11px; color: $text-dim; }
    }

    .delete-cfg-icon {
      color: $text-dim;
      font-size: 14px;
      opacity: 0;
      transition: opacity 0.2s;
      &:hover { color: $danger; }
    }
    &:hover .delete-cfg-icon { opacity: 1; }
  }

  .empty-hint { padding: 20px; text-align: center; color: $text-dim; font-size: 12px; }
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