<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import RedisCreateModal from './RedisCreateModal.vue';

// --- 状态控制 ---
const isConnectPanelVisible = ref(false);
const isConnecting = ref(false);
const showPassword = ref(false);
const searchQuery = ref('*');
const keysList = ref<string[]>([]);
const selectedKey = ref<string | null>(null);
const keyValue = ref<any>(null);
const savedConfigs = ref<any[]>([]);
const isConfigListVisible = ref(false);
const isCreateModalVisible = ref(false);

const selectedKeyType = ref('string');
const selectedTTL = ref(-1);

const connForm = ref({
  id: '',
  name: '本地开发环境',
  host: '127.0.0.1',
  port: 6379,
  password: '',
  db: 0
});

// --- 逻辑函数 ---
const loadSavedConfigs = async () => {
  try { savedConfigs.value = await invoke('get_redis_configs'); } catch (err) { console.error(err); }
};

const handleConnect = async () => {
  isConnecting.value = true;
  try {
    await invoke('redis_connect', { config: connForm.value });
    await invoke('save_redis_config', { config: connForm.value });
    toast.success("Redis 连接成功");
    isConnectPanelVisible.value = false;
    refreshKeys();
    loadSavedConfigs();
  } catch (err) { toast.error(`${err}`); } finally { isConnecting.value = false; }
};

const refreshKeys = async () => {
  try { keysList.value = await invoke('redis_get_keys', { pattern: searchQuery.value }) as string[]; } catch (err) { toast.error("刷新失败"); }
};

const selectKey = async (key: string) => {
  selectedKey.value = key;
  try {
    const [val, type, ttl] = await Promise.all([
      invoke('redis_get_value', { key }),
      invoke('redis_get_type', { key }) as Promise<string>,
      invoke('redis_get_ttl', { key }) as Promise<number>
    ]);
    keyValue.value = val;
    selectedKeyType.value = type;
    selectedTTL.value = ttl;
  } catch (err) { toast.error("读取 Key 失败"); }
};

const handleSave = async () => {
  try {
    await invoke('redis_set_value', { key: selectedKey.value, value: String(keyValue.value), keyType: 'string', ttl: selectedTTL.value });
    toast.success("保存成功");
  } catch (err) { toast.error("保存失败"); }
};

const toggleConnectPanel = () => {
  isConfigListVisible.value = false; // 互斥：先关掉另一个
  isConnectPanelVisible.value = !isConnectPanelVisible.value;
};

const toggleConfigList = () => {
  isConnectPanelVisible.value = false; // 互斥：先关掉另一个
  isConfigListVisible.value = !isConfigListVisible.value;
};

onMounted(() => {
  loadSavedConfigs();
  refreshKeys();
});
</script>

<template>
  <div class="rd-mg-container">
    <header class="rd-mg-header">
      <div class="rd-mg-brand">
        <i class="fas fa-database" :class="{'fa-spin': isConnecting}"></i>
        <span>{{ connForm.name }}</span>
      </div>
      <div class="rd-mg-toolbar">
        <button class="rd-mg-btn-icon" @click.stop="toggleConfigList" title="历史连接">
          <i class="fas fa-history"></i>
        </button>
        <button class="rd-mg-btn-icon" :class="{'active': isConnectPanelVisible}" @click.stop="toggleConnectPanel" title="连接设置">
          <i class="fas fa-plug"></i>
        </button>
      </div>

      <div v-if="isConfigListVisible" class="rd-mg-dropdown">
        <div v-for="cfg in savedConfigs" :key="cfg.id" class="rd-mg-dropdown-item" @click="connForm = {...cfg}; handleConnect(); isConfigListVisible=false;">
          <span class="name">{{ cfg.name }}</span>
          <span class="addr">{{ cfg.host }}:{{ cfg.port }}</span>
        </div>
      </div>
    </header>

    <div class="rd-mg-expand-panel" :class="{'is-open': isConnectPanelVisible}">
      <div class="rd-mg-form-scroll">
        <div class="rd-mg-form-vertical">
          <div class="rd-mg-field">
            <label>连接名称</label>
            <input v-model="connForm.name" placeholder="请输入连接名称..." />
          </div>

          <div class="rd-mg-field-row">
            <div class="rd-mg-field">
              <label>主机地址</label>
              <input v-model="connForm.host" placeholder="127.0.0.1" />
            </div>
            <div class="rd-mg-field" style="width: 120px; flex: none;">
              <label>端口</label>
              <input v-model.number="connForm.port" type="number" />
            </div>
          </div>

          <div class="rd-mg-field">
            <label>数据库 (Database Index)</label>
            <input v-model.number="connForm.db" type="number" min="0" max="15" />
          </div>

          <div class="rd-mg-field">
            <label>访问密码</label>
            <div class="rd-mg-password-box">
              <input :type="showPassword ? 'text' : 'password'" v-model="connForm.password" placeholder="若无密码请留空" />
              <button class="rd-mg-eye-btn" @click="showPassword = !showPassword">
                <i class="fas" :class="showPassword ? 'fa-eye-slash' : 'fa-eye'"></i>
              </button>
            </div>
          </div>

          <div class="rd-mg-form-footer">
            <button class="rd-mg-btn-submit" @click="handleConnect">
              <i class="fas fa-paper-plane"></i> 测试并连接
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="rd-mg-main">
      <aside class="rd-mg-sidebar">
        <div class="rd-mg-search">
          <div class="rd-mg-search-inner">
            <i class="fas fa-search"></i>
            <input v-model="searchQuery" @keyup.enter="refreshKeys" placeholder="过滤 Key..." />
          </div>
          <button @click="isCreateModalVisible = true" class="rd-mg-add-btn"><i class="fas fa-plus"></i></button>
        </div>
        <div class="rd-mg-list">
          <div v-for="k in keysList" :key="k" class="rd-mg-item" :title="k" :class="{'is-active': selectedKey === k}" @click="selectKey(k)">
            <i class="fas fa-hashtag"></i>
            <span class="truncate">{{ k }}</span>
          </div>
        </div>
      </aside>

      <main class="rd-mg-content">
        <template v-if="selectedKey">
          <div class="rd-mg-detail-header">
            <span class="rd-mg-tag">{{ selectedKeyType }}</span>
            <strong class="truncate">{{ selectedKey }}</strong>
            <span class="ttl">TTL: {{ selectedTTL }}s</span>
          </div>
          <div class="rd-mg-editor">
            <textarea v-model="keyValue" spellcheck="false" placeholder="Value..."></textarea>
          </div>
          <div class="rd-mg-footer">
            <button class="rd-mg-btn-primary" @click="handleSave">保存修改</button>
          </div>
        </template>
        <div v-else class="rd-mg-empty">
          <i class="fas fa-inbox"></i>
          <p>请选择左侧列表中的键值进行操作</p>
        </div>
      </main>
    </div>

    <RedisCreateModal :visible="isCreateModalVisible" @close="isCreateModalVisible = false" @confirm="refreshKeys" />
  </div>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.rd-mg-container {
  height: 100%; width: 100%; display: flex; flex-direction: column;
  background: var(--bg-primary); color: var(--text-main); font-size: 13px;
  position: relative; overflow: hidden;

  /* 移除数字输入框原生样式 */
  input[type="number"] {
    -moz-appearance: textfield;
    &::-webkit-outer-spin-button, &::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
  }

  .rd-mg-header {
    height: 52px; display: flex; align-items: center; justify-content: space-between;
    padding: 0 16px; background: var(--bg-secondary); border-bottom: 1px solid var(--border);
    z-index: 30;
    .rd-mg-brand { display: flex; align-items: center; gap: 10px; font-weight: 600; color: var(--accent); }
  }

  /* --- 纵向表单面板 --- */
  .rd-mg-expand-panel {
    backface-visibility: hidden;
    will-change: max-height;
    max-height: 0; overflow: hidden; transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);

    // 修复点 1：移除静态 @if，改用动态背景
    background: var(--bg-card-95); // 之前的 95% 透明变量派上用场了

    border-bottom: 0px solid var(--border);
    &.is-open { max-height: 450px; border-bottom-width: 1px; }

    .rd-mg-form-scroll { max-height: 450px; overflow-y: auto; padding: 20px 0; }

    .rd-mg-form-vertical {
      max-width: 480px; margin: 0 auto; display: flex; flex-direction: column; gap: 16px;
      padding: 0 20px;

      .rd-mg-field-row { display: flex; gap: 12px; }

      .rd-mg-field {
        display: flex; flex-direction: column; gap: 8px;
        label { font-size: 12px; font-weight: 600; color: var(--text-dim); }
        input {
          height: 36px; padding: 0 12px; background: var(--bg-input);
          border: 1px solid var(--border); color: var(--text-main); border-radius: 6px;
          font-size: 13px; transition: all 0.2s ease;
          &:focus { border-color: var(--accent); outline: none; box-shadow: 0 0 0 3px var(--accent-15); }
        }
      }
    }
  }

  /* 密码框样式 */
  .rd-mg-password-box {
    position: relative; display: flex;
    input { flex: 1; padding-right: 40px !important; }
    .rd-mg-eye-btn {
      position: absolute; right: 0; top: 0; bottom: 0; width: 40px;
      background: transparent; border: none; color: var(--text-dim);
      cursor: pointer; display: flex; align-items: center; justify-content: center;
      &:hover { color: var(--accent); }
    }
  }

  /* 提交按钮区域 */
  .rd-mg-form-footer {
    margin-top: 8px; display: flex; justify-content: flex-end;
    .rd-mg-btn-submit {
      background: var(--accent); color: var(--bg-primary); border: none; padding: 10px 24px;
      border-radius: 6px; font-weight: 600; cursor: pointer; transition: all 0.2s;
      &:hover { filter: brightness(1.1); transform: translateY(-1px); }
      &:active { transform: translateY(0); }
    }
  }

  /* --- 主体部分 --- */
  .rd-mg-main { flex: 1; display: flex; overflow: hidden; }

  /* 侧边栏列表 */
  .rd-mg-sidebar {
    width: 180px; border-right: 1px solid var(--border); display: flex; flex-direction: column;
    background: var(--bg-sidebar);
    .rd-mg-search {
      padding: 12px; display: flex; gap: 8px;
      .rd-mg-search-inner {
        position: relative; flex: 1;
        i { position: absolute; left: 10px; top: 10px; font-size: 12px; color: var(--text-dim); }
        input { width: 100%; height: 32px; padding: 0 10px 0 32px; background: var(--bg-input); border: 1px solid var(--border); color: var(--text-main); border-radius: 4px; }
      }
      .rd-mg-add-btn { width: 32px; height: 32px; border: 1px solid var(--border); background: transparent; color: var(--accent); border-radius: 4px; cursor: pointer; &:hover { background: var(--accent-10); } }
    }
    .rd-mg-list {
      flex: 1; overflow-y: auto; padding: 4px 8px;
      .rd-mg-item {
        display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-radius: 6px;
        cursor: pointer; color: var(--text-dim); margin-bottom: 2px;
        &:hover { background: var(--accent-05); color: var(--text-main); }

        // 修复点 2：使用混合变量处理激活态背景
        &.is-active {
          background: var(--accent-20); // 亮色下是淡蓝/淡紫，暗色下是深色叠加，效果统一
          color: var(--accent);
          font-weight: 600;
        }

        i { font-size: 11px; opacity: 0.5; }
      }
    }
  }

  /* 编辑器内容区 */
  .rd-mg-content {
    flex: 1; display: flex; flex-direction: column;
    .rd-mg-detail-header {
      padding: 14px 20px; border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 12px;
      .rd-mg-tag { background: var(--accent-10); color: var(--accent); padding: 2px 8px; border-radius: 4px; font-size: 10px; font-weight: bold; }
      .ttl { font-size: 12px; color: var(--text-dim); margin-left: auto; }
    }
    .rd-mg-editor {
      flex: 1;
      textarea {
        width: 100%; height: 100%; border: none; outline: none; padding: 20px; background: transparent;

        // 修复点 3：编辑器文字颜色，使用 accent 或 success 确保可读性
        color: var(--text-main);
        &:focus { color: var(--accent); } // 聚焦时变色提示编辑

        font-family: 'JetBrains Mono', monospace; font-size: 14px; resize: none; line-height: 1.6;
      }
    }
    .rd-mg-footer { padding: 14px 20px; border-top: 1px solid var(--border); display: flex; justify-content: flex-end; }
  }

  /* 通用组件样式 */
  .rd-mg-btn-icon {
    background: transparent; border: none; color: var(--text-dim);
    width: 36px; height: 36px; border-radius: 6px; cursor: pointer;
    &:hover { background: var(--accent-10); color: var(--accent); }
    &.active { background: var(--accent); color: var(--bg-primary); }
  }

  .rd-mg-btn-primary {
    background: var(--accent); color: var(--bg-primary); border: none; padding: 8px 18px; border-radius: 4px; font-weight: 600; cursor: pointer;
    &:hover { filter: brightness(1.1); }
  }

  .rd-mg-dropdown {
    position: absolute; top: 55px; right: 16px; width: 260px; z-index: 100;
    background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px;
    box-shadow: 0 10px 30px var(--shadow);
    &-item {
      padding: 12px 16px; cursor: pointer; border-bottom: 1px solid var(--border-50);
      &:last-child { border-bottom: none; }
      &:hover { background: var(--accent-05); }
      .name { font-weight: 600; color: var(--accent); display: block; }
      .addr { font-size: 11px; color: var(--text-dim); }
    }
  }

  .rd-mg-empty { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-dim); opacity: 0.5; i { font-size: 50px; margin-bottom: 16px; } }
  .truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
}
</style>