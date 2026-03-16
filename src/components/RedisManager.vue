<script setup lang="ts">
import {ref, onMounted} from 'vue';
import {invoke} from '@tauri-apps/api/core';
import {toast} from '../utils/toast.ts';
import {throttle} from '../utils/async.ts';
import RedisCreateModal from './RedisCreateModal.vue';

const isConnectPanelVisible = ref(false);
const isConnecting = ref(false);
const isConnected = ref(false);
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
  db: 0,
  updated_at: 0,
  deleted: false
});

// --- 逻辑函数 ---
const loadSavedConfigs = async () => {
  try {
    savedConfigs.value = await invoke('get_redis_configs');
  } catch (err) {
    console.error(err);
  }
};


const handleConnect = throttle(async () => {
  isConnecting.value = true;
  try {
    await invoke('redis_connect', {config: connForm.value});
    await invoke('save_redis_config', {config: connForm.value});
    isConnected.value = true;
    toast.success("Redis 连接成功");
    isConnectPanelVisible.value = false;
    refreshKeys();
    loadSavedConfigs();
  } catch (err) {
    isConnected.value = false;
    toast.error(`${err}`);
  } finally {
    isConnecting.value = false;
  }
}, 300);

const refreshKeys = async () => {
  if (!isConnected.value) {
    return;
  }

  try {
    keysList.value = await invoke('redis_get_keys', {pattern: searchQuery.value}) as string[];
  } catch (err) {
    isConnected.value = false;
    keysList.value = [];
    toast.error("刷新失败，请重新连接");
  }
};

const selectKey = async (key: string) => {
  selectedKey.value = key;
  try {
    const [val, type, ttl] = await Promise.all([
      invoke('redis_get_value', {key}),
      invoke('redis_get_type', {key}) as Promise<string>,
      invoke('redis_get_ttl', {key}) as Promise<number>
    ]);
    keyValue.value = val;
    selectedKeyType.value = type;
    selectedTTL.value = ttl;
  } catch (err) {
    toast.error("读取 Key 失败");
  }
};

const handleSave = async () => {
  try {
    await invoke('redis_set_value', {
      key: selectedKey.value,
      value: String(keyValue.value),
      keyType: selectedKeyType.value,
      ttl: selectedTTL.value
    });
    toast.success("保存成功");
  } catch (err) {
    toast.error("保存失败");
  }
};

const toggleConnectPanel = throttle(() => {
  isConfigListVisible.value = false;
  isConnectPanelVisible.value = !isConnectPanelVisible.value;
}, 300);

const toggleConfigList = throttle(() => {
  isConnectPanelVisible.value = false;
  isConfigListVisible.value = !isConfigListVisible.value;
}, 300);

onMounted(() => {
  loadSavedConfigs();
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
        <button class="rd-mg-btn-icon" :class="{'active': isConnectPanelVisible}" @click.stop="toggleConnectPanel"
                title="连接设置">
          <i class="fas fa-plug"></i>
        </button>
      </div>

      <div v-if="isConfigListVisible && savedConfigs.length > 0" class="rd-mg-dropdown">
        <div v-for="cfg in savedConfigs" :key="cfg.id" class="rd-mg-dropdown-item"
             @click="connForm = {...cfg}; handleConnect(); isConfigListVisible=false;">
          <span class="name">{{ cfg.name }}</span>
          <span class="addr">{{ cfg.host }}:{{ cfg.port }}</span>
        </div>
      </div>
    </header>

    <div class="rd-mg-expand-panel" :class="{'is-open': isConnectPanelVisible}">
      <div class="rd-mg-form-scroll">
        <div class="rd-mg-form-vertical">
          <div class="form-header-hint">
            <div class="dot-deco"></div>
            <span>实例连接配置</span>
          </div>

          <div class="rd-mg-field">
            <label><i class="fas fa-tag"></i> 连接名称</label>
            <div class="input-control">
              <input v-model="connForm.name" placeholder="例如：生产环境主库"/>
            </div>
          </div>

          <div class="rd-mg-field-row group-box">
            <div class="rd-mg-field flex-3">
              <label><i class="fas fa-network-wired"></i> 主机地址</label>
              <div class="input-control">
                <input v-model="connForm.host" placeholder="127.0.0.1"/>
              </div>
            </div>
            <div class="rd-mg-field flex-1">
              <label><i class="fas fa-door-open"></i> 端口</label>
              <div class="input-control">
                <input v-model.number="connForm.port" type="number" placeholder="6379"/>
              </div>
            </div>
          </div>

          <div class="rd-mg-field-row">
            <div class="rd-mg-field" style="width: 140px; flex: none;">
              <label><i class="fas fa-layer-group"></i> 数据库</label>
              <div class="input-control">
                <input v-model.number="connForm.db" type="number" min="0" max="15"/>
              </div>
            </div>

            <div class="rd-mg-field flex-1">
              <label><i class="fas fa-key"></i> 访问密码</label>
              <div class="input-control rd-mg-password-box">
                <input :type="showPassword ? 'text' : 'password'" v-model="connForm.password"
                       placeholder="若无密码请留空"/>
                <button class="rd-mg-eye-btn" @click="showPassword = !showPassword">
                  <i class="fas" :class="showPassword ? 'fa-eye-slash' : 'fa-eye'"></i>
                </button>
              </div>
            </div>
          </div>

          <div class="rd-mg-form-footer">
            <button class="rd-mg-btn-submit" @click="handleConnect" :disabled="isConnecting">
              <i class="fas" :class="isConnecting ? 'fa-circle-notch fa-spin' : 'fa-bolt'"></i>
              <span>{{ isConnecting ? '正在连接...' : '测试并连接' }}</span>
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
            <input v-model="searchQuery" @keyup.enter="refreshKeys" placeholder="过滤 Key..."/>
          </div>
          <button @click="isCreateModalVisible = true" class="rd-mg-add-btn"><i class="fas fa-plus"></i></button>
        </div>
        <div class="rd-mg-list">
          <div v-for="k in keysList" :key="k" class="rd-mg-item" :title="k" :class="{'is-active': selectedKey === k}"
               @click="selectKey(k)">
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

    <RedisCreateModal :visible="isCreateModalVisible" @close="isCreateModalVisible = false" @confirm="refreshKeys"/>
  </div>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.rd-mg-container {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-main);
  font-size: 13px;
  position: relative;
  overflow: hidden;

  /* 基础组件对齐修正 */
  input, textarea, button {
    font-family: inherit;
    box-sizing: border-box; /* 核心修复：防止宽度溢出 */
  }

  .rd-mg-header {
    height: 52px;
    flex-shrink: 0; /* 防止被下方撑开的面板挤压 */
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    z-index: 30;

    .rd-mg-brand {
      display: flex;
      align-items: center;
      gap: 10px;
      font-weight: 600;
      color: var(--accent);

      .fa-database { transition: all 0.3s; }
    }

    .rd-mg-toolbar {
      display: flex;
      gap: 8px;
    }
  }

  /* 连接配置面板 - 增强过渡动画 */
  .rd-mg-expand-panel {
    flex-shrink: 0;
    will-change: max-height;
    max-height: 0;
    overflow: hidden;
    transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    background: var(--bg-card);
    border-bottom: 0 solid var(--border);

    &.is-open {
      max-height: 480px;
      border-bottom-width: 1px;
      box-shadow: inset 0 -10px 20px -10px rgba(0,0,0,0.1);
    }

    .rd-mg-form-scroll {
      padding: 24px 0;
    }

    .rd-mg-form-vertical {
      max-width: 500px;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      gap: 20px;
      padding: 0 20px;

      .form-header-hint {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 4px;
        .dot-deco {
          width: 3px;
          height: 14px;
          background: var(--accent);
          border-radius: 2px;
        }
        span {
          font-size: 11px;
          color: var(--text-dim);
          font-weight: bold;
          letter-spacing: 1px;
        }
      }

      .rd-mg-field-row {
        display: flex;
        gap: 12px;

        &.group-box {
          background: var(--bg-primary);
          padding: 16px;
          border-radius: 12px;
          border: 1px solid var(--border);
        }
      }

      .rd-mg-field {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 6px;

        label {
          font-size: 11px;
          color: var(--text-dim);
          font-weight: 600;
        }

        input {
          height: 36px;
          padding: 0 12px;
          background: var(--bg-input);
          border: 1px solid var(--border);
          color: var(--text-main);
          border-radius: 6px;
          width: 100%;
          transition: border-color 0.2s;

          &:focus {
            outline: none;
            border-color: var(--accent);
            background: var(--bg-primary);
          }
        }

        .input-control.rd-mg-password-box {
          position: relative;
          display: flex;
          align-items: center;
          width: 100%; /* 确保撑满父容器 */

          input {
            flex: 1;
            width: 100%;
            padding-right: 40px !important; /* 必须给右侧留出眼睛图标的位置 */
          }

          .rd-mg-eye-btn {
            position: absolute;
            right: 4px; /* 距离输入框右边界的距离 */
            top: 50%;
            transform: translateY(-50%); /* 垂直绝对居中 */
            width: 32px;
            height: 32px;
            padding: 0;
            background: transparent;
            border: none;
            color: var(--text-dim);
            cursor: pointer;
            display: flex;
            align-items: center;
            justify-content: center;
            z-index: 10;
            transition: color 0.2s;

            &:hover {
              color: var(--accent);
            }

            i {
              font-size: 14px;
            }
          }
        }
      }
    }

    /* 补充到 .rd-mg-form-vertical 内部或下方 */
    .rd-mg-form-footer {
      margin-top: 8px;
      padding-top: 16px;
      border-top: 1px solid var(--border-50);
      display: flex;
      justify-content: flex-end;

      .rd-mg-btn-submit {
        height: 36px;
        padding: 0 24px;
        background: var(--accent);
        color: white;
        border: none;
        border-radius: 6px;
        font-weight: 600;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 8px;
        transition: all 0.2s;
        box-shadow: 0 4px 12px var(--accent-30);

        &:hover:not(:disabled) {
          filter: brightness(1.1);
          transform: translateY(-1px);
        }

        &:disabled {
          opacity: 0.6;
          cursor: not-allowed;
          background: var(--text-dim);
        }

        i { font-size: 12px; }
      }
    }
  }

  /* 主体区域 */
  .rd-mg-main {
    flex: 1;
    display: flex;
    overflow: hidden; /* 保证内部滚动条生效 */
  }

  /* 侧边栏 - 修复搜索框和按钮显示问题 */
  .rd-mg-sidebar {
    width: 240px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    background: var(--bg-sidebar);

    .rd-mg-search {
      padding: 12px;
      display: flex;
      gap: 8px;
      align-items: center;

      .rd-mg-search-inner {
        position: relative;
        flex: 1;
        min-width: 0; /* 允许内部元素缩小而不撑破 Flex */

        i {
          position: absolute;
          left: 10px;
          top: 50%;
          transform: translateY(-50%); /* 垂直居中修正 */
          font-size: 12px;
          color: var(--text-dim);
          pointer-events: none;
        }

        input {
          width: 100%;
          height: 32px;
          padding: 0 10px 0 32px;
          background: var(--bg-input);
          border: 1px solid var(--border);
          color: var(--text-main);
          border-radius: 6px;
          font-size: 12px;

          &:focus {
            outline: none;
            border-color: var(--accent);
          }
        }
      }

      .rd-mg-add-btn {
        flex-shrink: 0; /* 强制不被压缩，解决消失问题 */
        width: 32px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 1px solid var(--border);
        background: var(--bg-input);
        color: var(--accent);
        border-radius: 6px;
        cursor: pointer;
        transition: all 0.2s;

        &:hover {
          background: var(--accent);
          color: white;
          border-color: var(--accent);
        }
      }
    }

    .rd-mg-list {
      flex: 1;
      overflow-y: auto;
      padding: 0 8px 12px;

      /* 滚动条美化 */
      &::-webkit-scrollbar { width: 4px; }
      &::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

      .rd-mg-item {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 12px;
        border-radius: 6px;
        cursor: pointer;
        color: var(--text-dim);
        margin-bottom: 2px;
        transition: all 0.2s;

        &:hover {
          background: var(--bg-input);
          color: var(--text-main);
        }

        &.is-active {
          background: var(--accent-15);
          color: var(--accent);
          font-weight: 600;
        }

        .truncate {
          flex: 1;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
      }
    }
  }

  /* 内容编辑区 */
  .rd-mg-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);

    .rd-mg-detail-header {
      padding: 12px 20px;
      height: 48px;
      border-bottom: 1px solid var(--border);
      display: flex;
      align-items: center;
      gap: 12px;
      flex-shrink: 0;

      .rd-mg-tag {
        background: var(--accent-10);
        color: var(--accent);
        padding: 2px 6px;
        border-radius: 4px;
        font-size: 10px;
        font-weight: 800;
        text-transform: uppercase;
      }

      strong { font-size: 14px; }

      .ttl {
        margin-left: auto;
        font-size: 11px;
        color: var(--text-dim);
        font-family: monospace;
      }
    }

    .rd-mg-editor {
      flex: 1;
      padding: 0;
      position: relative;

      textarea {
        width: 100%;
        height: 100%;
        border: none;
        outline: none;
        padding: 20px;
        background: transparent;
        color: var(--text-main);
        font-family: 'JetBrains Mono', 'Cascadia Code', monospace;
        font-size: 14px;
        line-height: 1.6;
        resize: none;

        &::placeholder { color: var(--text-dim); opacity: 0.3; }
      }
    }

    .rd-mg-footer {
      padding: 12px 20px;
      border-top: 1px solid var(--border);
      display: flex;
      justify-content: flex-end;
      background: var(--bg-secondary);
    }
  }

  /* 通用按钮 */
  .rd-mg-btn-icon {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.2s;

    &:hover { background: var(--bg-input); color: var(--accent); }
    &.active { background: var(--accent); color: white; }
  }

  .rd-mg-btn-primary {
    background: var(--accent);
    color: white;
    border: none;
    padding: 6px 20px;
    border-radius: 6px;
    font-weight: 600;
    font-size: 12px;
    cursor: pointer;
    box-shadow: 0 4px 12px var(--accent-30);

    &:hover { filter: brightness(1.1); transform: translateY(-1px); }
    &:active { transform: translateY(0); }
  }

  /* 历史记录下拉菜单 */
  .rd-mg-dropdown {
    position: absolute;
    top: 56px;
    right: 16px;
    width: 280px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 30px rgba(0,0,0,0.2);
    z-index: 100;
    overflow: hidden;

    &-item {
      padding: 12px 16px;
      cursor: pointer;
      border-bottom: 1px solid var(--border-50);
      transition: background 0.2s;

      &:hover { background: var(--accent-10); }
      &:last-child { border-bottom: none; }

      .name { display: block; font-weight: 600; color: var(--accent); margin-bottom: 2px; }
      .addr { font-size: 11px; color: var(--text-dim); }
    }
  }

  .rd-mg-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-dim);
    gap: 16px;
    opacity: 0.6;
    i { font-size: 40px; }
  }
}
</style>