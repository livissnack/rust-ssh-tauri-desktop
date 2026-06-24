<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import { throttle } from '../utils/async.ts';
import { confirm } from '../utils/confirm.ts';
import RedisCreateModal from './RedisCreateModal.vue';

const KEY_TYPE_COLORS: Record<string, string> = {
  string: '#9ece6a',
  hash: '#7aa2f7',
  list: '#e0af68',
  set: '#bb9af7',
  zset: '#f7768e',
};

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
const topRef = ref<HTMLElement | null>(null);

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
  deleted: false,
});

const connectionMeta = computed(() =>
  `${connForm.value.host}:${connForm.value.port} / DB${connForm.value.db}`
);

const statusLabel = computed(() => {
  if (isConnecting.value) return 'Connecting';
  if (isConnected.value) return 'Connected';
  return 'Disconnected';
});

const statusClass = computed(() => {
  if (isConnecting.value) return 'is-connecting';
  if (isConnected.value) return 'is-connected';
  return 'is-disconnected';
});

const formattedTTL = computed(() => {
  if (selectedTTL.value === -1) return '永久';
  if (selectedTTL.value === -2) return '已过期';
  return `${selectedTTL.value}s`;
});

const typeColor = computed(() =>
  KEY_TYPE_COLORS[selectedKeyType.value] ?? 'var(--accent)'
);

const loadSavedConfigs = async () => {
  try {
    savedConfigs.value = await invoke('get_redis_configs');
  } catch (err) {
    console.error(err);
  }
};

const closePanels = () => {
  isConnectPanelVisible.value = false;
  isConfigListVisible.value = false;
};

const handlePointerDownOutside = (e: PointerEvent) => {
  if (topRef.value?.contains(e.target as Node)) return;
  closePanels();
};

const applySavedConfig = (cfg: any) => {
  connForm.value = { ...cfg };
  isConfigListVisible.value = false;
  handleConnect();
};

const handleConnect = throttle(async () => {
  isConnecting.value = true;
  try {
    await invoke('redis_connect', { config: connForm.value });
    await invoke('save_redis_config', { config: connForm.value });
    isConnected.value = true;
    toast.success('Redis 连接成功');
    isConnectPanelVisible.value = false;
    await refreshKeys();
    loadSavedConfigs();
  } catch (err) {
    isConnected.value = false;
    toast.error(`${err}`);
  } finally {
    isConnecting.value = false;
  }
}, 300);

const refreshKeys = async () => {
  if (!isConnected.value) return;

  try {
    keysList.value = await invoke('redis_get_keys', { pattern: searchQuery.value }) as string[];
  } catch {
    isConnected.value = false;
    keysList.value = [];
    selectedKey.value = null;
    keyValue.value = null;
    toast.error('刷新失败，请重新连接');
  }
};

const selectKey = async (key: string) => {
  selectedKey.value = key;
  try {
    const [val, type, ttl] = await Promise.all([
      invoke('redis_get_value', { key }),
      invoke('redis_get_type', { key }) as Promise<string>,
      invoke('redis_get_ttl', { key }) as Promise<number>,
    ]);
    keyValue.value = val;
    selectedKeyType.value = type;
    selectedTTL.value = ttl;
  } catch {
    toast.error('读取 Key 失败');
  }
};

const handleSave = async () => {
  if (!selectedKey.value) return;
  try {
    await invoke('redis_set_value', {
      key: selectedKey.value,
      value: String(keyValue.value),
      keyType: selectedKeyType.value,
      ttl: selectedTTL.value,
    });
    toast.success('保存成功');
  } catch {
    toast.error('保存失败');
  }
};

const handleDeleteKey = async () => {
  if (!selectedKey.value) return;
  const ok = await confirm.error(
    `确定要删除 Key "${selectedKey.value}" 吗？此操作无法恢复。`,
    '危险操作'
  );
  if (!ok) return;

  try {
    await invoke('redis_del_key', { key: selectedKey.value });
    toast.success('删除成功');
    selectedKey.value = null;
    keyValue.value = null;
    await refreshKeys();
  } catch {
    toast.error('删除失败');
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
  document.addEventListener('pointerdown', handlePointerDownOutside);
});

onUnmounted(() => {
  document.removeEventListener('pointerdown', handlePointerDownOutside);
});
</script>

<template>
  <div class="redis-manager">
    <div ref="topRef" class="redis-top">
      <header class="redis-header">
        <div class="redis-header__brand">
          <div class="redis-header__icon" :class="statusClass">
            <i class="fas fa-database" :class="{ 'fa-spin': isConnecting }"></i>
          </div>
          <div class="redis-header__info">
            <span class="redis-header__name">{{ connForm.name }}</span>
            <span class="redis-header__meta">{{ connectionMeta }}</span>
          </div>
        </div>

        <div class="redis-header__status">
          <span class="status-badge" :class="statusClass">
            <span class="status-badge__dot"></span>
            {{ statusLabel }}
          </span>
        </div>

        <div class="redis-header__actions">
          <Tooltip text="历史连接">
            <button
                type="button"
                class="icon-btn"
                :class="{ active: isConfigListVisible }"
                @click.stop="toggleConfigList"
            >
              <i class="fas fa-history"></i>
            </button>
          </Tooltip>
          <Tooltip text="连接设置">
            <button
                type="button"
                class="icon-btn"
                :class="{ active: isConnectPanelVisible }"
                @click.stop="toggleConnectPanel"
            >
              <i class="fas fa-plug"></i>
            </button>
          </Tooltip>
        </div>

        <Transition name="dropdown">
          <div v-if="isConfigListVisible" class="history-menu">
            <div v-if="savedConfigs.length === 0" class="history-menu__empty">暂无历史连接</div>
            <button
                v-for="cfg in savedConfigs"
                :key="cfg.id"
                type="button"
                class="history-menu__item"
                @click="applySavedConfig(cfg)"
            >
              <span class="history-menu__name">{{ cfg.name }}</span>
              <span class="history-menu__addr">{{ cfg.host }}:{{ cfg.port }} · DB{{ cfg.db ?? 0 }}</span>
            </button>
          </div>
        </Transition>
      </header>

      <Transition name="dropdown">
        <section v-if="isConnectPanelVisible" class="connect-panel">
          <div class="connect-panel__inner">
            <div class="connect-panel__head">
              <span class="section-title">连接配置</span>
              <button type="button" class="icon-btn" @click="isConnectPanelVisible = false">
                <i class="fas fa-xmark"></i>
              </button>
            </div>

            <div class="field">
              <label>连接名称</label>
              <div class="field-control">
                <i class="fas fa-tag field-icon"></i>
                <input v-model="connForm.name" placeholder="例如：生产环境主库" />
              </div>
            </div>

            <div class="field-row">
              <div class="field field--grow">
                <label>主机地址</label>
                <div class="field-control">
                  <i class="fas fa-globe field-icon"></i>
                  <input v-model="connForm.host" placeholder="127.0.0.1" />
                </div>
              </div>
              <div class="field field--port">
                <label>端口</label>
                <NumberInput v-model="connForm.port" :min="1" :max="65535" placeholder="6379" />
              </div>
            </div>

            <div class="field-row">
              <div class="field field--db">
                <label>数据库</label>
                <NumberInput v-model="connForm.db" :min="0" :max="15" />
              </div>
              <div class="field field--grow">
                <label>访问密码</label>
                <div class="field-control field-control--password">
                  <i class="fas fa-lock field-icon"></i>
                  <input
                      v-model="connForm.password"
                      :type="showPassword ? 'text' : 'password'"
                      placeholder="若无密码请留空"
                  />
                  <button type="button" class="eye-btn" @click="showPassword = !showPassword">
                    <i class="fas" :class="showPassword ? 'fa-eye-slash' : 'fa-eye'"></i>
                  </button>
                </div>
              </div>
            </div>

            <div class="connect-panel__footer">
              <button type="button" class="btn btn--primary" :disabled="isConnecting" @click="handleConnect">
                <i class="fas" :class="isConnecting ? 'fa-circle-notch fa-spin' : 'fa-bolt'"></i>
                {{ isConnecting ? '连接中...' : '测试并连接' }}
              </button>
            </div>
          </div>
        </section>
      </Transition>
    </div>

    <div class="redis-body">
      <aside class="keys-panel">
        <div class="keys-panel__toolbar">
          <div class="keys-search">
            <i class="fas fa-search"></i>
            <input
                v-model="searchQuery"
                placeholder="Key 过滤，如 user:*"
                @keyup.enter="refreshKeys"
            />
          </div>
          <Tooltip text="刷新列表">
            <button type="button" class="icon-btn" :disabled="!isConnected" @click="refreshKeys">
              <i class="fas fa-rotate"></i>
            </button>
          </Tooltip>
          <Tooltip text="新建 Key">
            <button
                type="button"
                class="icon-btn icon-btn--accent"
                :disabled="!isConnected"
                @click="isCreateModalVisible = true"
            >
              <i class="fas fa-plus"></i>
            </button>
          </Tooltip>
        </div>

        <div class="keys-panel__header">
          <span class="keys-panel__label">Keys</span>
          <span v-if="isConnected" class="keys-panel__count">{{ keysList.length }}</span>
        </div>

        <div v-if="!isConnected" class="keys-panel__empty">
          <i class="fas fa-plug"></i>
          <p>请先连接 Redis 实例</p>
        </div>

        <div v-else-if="keysList.length === 0" class="keys-panel__empty">
          <i class="fas fa-inbox"></i>
          <p>暂无匹配的 Key</p>
        </div>

        <div v-else class="keys-list">
          <Tooltip v-for="k in keysList" :key="k" :text="k" placement="right" block wrap>
            <button
                type="button"
                class="key-item"
                :class="{ 'is-active': selectedKey === k }"
                @click="selectKey(k)"
            >
              <span class="key-item__dot"></span>
              <span class="key-item__name">{{ k }}</span>
            </button>
          </Tooltip>
        </div>
      </aside>

      <main class="editor-panel">
        <template v-if="selectedKey">
          <div class="editor-panel__header">
            <span class="type-badge" :style="{ color: typeColor, borderColor: typeColor + '44', background: typeColor + '18' }">
              {{ selectedKeyType }}
            </span>
            <span class="editor-panel__key">{{ selectedKey }}</span>
            <span class="editor-panel__ttl">
              <i class="fas fa-clock"></i>
              TTL: {{ formattedTTL }}
            </span>
            <div class="editor-panel__actions">
              <Tooltip text="重新加载">
                <button type="button" class="icon-btn" @click="selectKey(selectedKey!)">
                  <i class="fas fa-rotate"></i>
                </button>
              </Tooltip>
              <Tooltip text="删除 Key">
                <button type="button" class="icon-btn icon-btn--danger" @click="handleDeleteKey">
                  <i class="fas fa-trash-can"></i>
                </button>
              </Tooltip>
            </div>
          </div>

          <div class="editor-panel__body">
            <textarea
                v-model="keyValue"
                spellcheck="false"
                placeholder="Value content..."
            ></textarea>
          </div>

          <div class="editor-panel__footer">
            <button type="button" class="btn btn--ghost" @click="selectedKey = null; keyValue = null">
              取消
            </button>
            <button type="button" class="btn btn--primary" @click="handleSave">
              <i class="fas fa-check"></i>
              保存修改
            </button>
          </div>
        </template>

        <div v-else class="editor-panel__empty">
          <div class="editor-panel__empty-icon">
            <i class="fas fa-key"></i>
          </div>
          <h3>选择 Key 进行编辑</h3>
          <p>从左侧列表选择一个键，或新建 Key 后开始操作</p>
        </div>
      </main>
    </div>

    <RedisCreateModal
        :visible="isCreateModalVisible"
        @close="isCreateModalVisible = false"
        @confirm="refreshKeys"
    />
  </div>
</template>

<style lang="scss" scoped>
.redis-manager {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-main);
  overflow: hidden;
}

.redis-top {
  position: relative;
  flex-shrink: 0;
  z-index: 20;
}

.redis-header {
  position: relative;
  flex-shrink: 0;
  display: grid;
  grid-template-columns: 1fr auto auto;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-30);

  &__brand {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  &__icon {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    background: var(--bg-input);
    border: 1px solid var(--border-30);
    color: var(--text-dim);
    font-size: 14px;

    &.is-connected {
      border-color: var(--success);
      color: var(--success);
      background: var(--bg-input);
    }

    &.is-connecting {
      border-color: var(--accent-orange-20);
      color: var(--accent-orange);
      background: var(--accent-orange-10);
    }
  }

  &__info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  &__name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
    font-weight: 700;
    color: var(--text-main);
  }

  &__meta {
    font-family: var(--font-terminal);
    font-size: 10px;
    color: var(--text-dim);
    letter-spacing: 0.02em;
  }

  &__status {
    flex-shrink: 0;
  }

  &__actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 20px;
  padding: 0 8px;
  border-radius: 10px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  background: var(--bg-input);
  border: 1px solid var(--border-30);
  color: var(--text-dim);

  &__dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  &.is-connected {
    border-color: var(--success);
    color: var(--success);
  }

  &.is-connecting {
    border-color: var(--accent-orange-20);
    color: var(--accent-orange);

    .status-badge__dot {
      animation: pulse 1.2s infinite;
    }
  }
}

.history-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 14px;
  width: 260px;
  padding: 6px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 12px 28px var(--shadow);
  z-index: 50;

  &__empty {
    padding: 12px;
    text-align: center;
    font-size: 12px;
    color: var(--text-dim);
  }

  &__item {
    width: 100%;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
    border: none;
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease;

    &:hover {
      background: var(--accent-08);
    }
  }

  &__name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-main);
  }

  &__addr {
    font-family: var(--font-terminal);
    font-size: 10px;
    color: var(--text-dim);
  }
}

.connect-panel {
  position: absolute;
  top: calc(100% + 8px);
  left: 12px;
  right: 12px;
  z-index: 30;
  max-height: min(420px, calc(100vh - 180px));
  overflow-y: auto;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: 0 16px 40px var(--shadow);

  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 4px;
  }

  &__inner {
    padding: 14px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  &__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  &__footer {
    display: flex;
    justify-content: flex-end;
    padding-top: 4px;
  }
}

.section-title {
  padding-left: 10px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
  position: relative;

  &::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--accent);
    opacity: 0.65;
  }
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;

  &--grow { flex: 1; min-width: 0; }
  &--port { width: 112px; flex-shrink: 0; }
  &--db { width: 100px; flex-shrink: 0; }

  label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-main);
    opacity: 0.85;
  }
}

.field-row {
  display: flex;
  gap: 10px;
}

.field-control {
  position: relative;

  .field-icon {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--text-dim);
    opacity: 0.65;
    pointer-events: none;
  }

  input {
    width: 100%;
    height: 36px;
    padding: 0 10px 0 32px;
    box-sizing: border-box;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-main);
    font-size: 13px;
    outline: none;
    transition: border-color 0.2s, box-shadow 0.2s;

    &:focus {
      border-color: var(--accent);
      box-shadow: 0 0 0 3px var(--accent-15);
    }
  }

  &--password input {
    padding-right: 36px;
  }
}

.eye-btn {
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;

  &:hover {
    background: var(--accent-10);
    color: var(--accent);
  }
}

.redis-body {
  flex: 1;
  display: flex;
  min-height: 0;
  overflow: hidden;
  position: relative;
  z-index: 1;
}

.keys-panel {
  width: 248px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-30);

  &__toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 10px 8px;
  }

  &__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 8px;
  }

  &__label {
    padding-left: 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    position: relative;

    &::before {
      content: '';
      position: absolute;
      left: 0;
      top: 50%;
      transform: translateY(-50%);
      width: 3px;
      height: 3px;
      border-radius: 50%;
      background: var(--accent);
      opacity: 0.65;
    }
  }

  &__count {
    min-width: 20px;
    height: 18px;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    background: var(--accent-12);
    border: 1px solid var(--accent-20);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-terminal);
    color: var(--accent);
  }

  &__empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px 16px;
    text-align: center;
    color: var(--text-dim);

    i {
      font-size: 28px;
      opacity: 0.45;
    }

    p {
      margin: 0;
      font-size: 12px;
      opacity: 0.75;
    }
  }
}

.keys-search {
  flex: 1;
  min-width: 0;
  position: relative;

  i {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--text-dim);
    opacity: 0.65;
  }

  input {
    width: 100%;
    height: 32px;
    padding: 0 10px 0 30px;
    box-sizing: border-box;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-main);
    font-size: 12px;
    outline: none;

    &:focus {
      border-color: var(--accent);
    }

    &::placeholder {
      color: var(--text-dim);
      opacity: 0.55;
    }
  }
}

.keys-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 10px;

  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 4px;
  }
}

.key-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  margin-bottom: 2px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  text-align: left;
  transition: all 0.15s ease;

  &__dot {
    width: 6px;
    height: 6px;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--border);
    transition: background 0.15s ease;
  }

  &__name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-terminal);
    font-size: 11px;
  }

  &:hover {
    background: var(--bg-primary-30);
    border-color: var(--border-30);
    color: var(--text-main);
  }

  &.is-active {
    background: var(--accent-08);
    border-color: var(--accent-20);
    color: var(--accent);
    font-weight: 600;

    .key-item__dot {
      background: var(--accent);
      box-shadow: 0 0 6px var(--accent-30);
    }
  }
}

.editor-panel {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);

  &__header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-30);
    background: var(--bg-secondary-60);
  }

  &__key {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-terminal);
    font-size: 13px;
    font-weight: 600;
  }

  &__ttl {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-dim);
    font-family: var(--font-terminal);

    i { font-size: 10px; opacity: 0.7; }
  }

  &__actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  &__body {
    flex: 1;
    min-height: 0;

    textarea {
      width: 100%;
      height: 100%;
      padding: 16px;
      box-sizing: border-box;
      border: none;
      outline: none;
      resize: none;
      background: transparent;
      color: var(--text-main);
      font-family: var(--font-terminal);
      font-size: 13px;
      line-height: 1.6;

      &::placeholder {
        color: var(--text-dim);
        opacity: 0.45;
      }
    }
  }

  &__footer {
    flex-shrink: 0;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 14px;
    border-top: 1px solid var(--border-30);
    background: var(--bg-secondary-60);
  }

  &__empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 24px;
    text-align: center;

    &-icon {
      width: 52px;
      height: 52px;
      display: flex;
      align-items: center;
      justify-content: center;
      margin-bottom: 14px;
      border-radius: 14px;
      background: var(--bg-input);
      border: 1px dashed var(--border-50);
      color: var(--text-dim);
      font-size: 22px;
      opacity: 0.75;
    }

    h3 {
      margin: 0 0 8px;
      font-size: 15px;
      font-weight: 600;
      color: var(--text-main);
    }

    p {
      margin: 0;
      font-size: 12px;
      color: var(--text-dim);
      opacity: 0.75;
      line-height: 1.5;
    }
  }
}

.type-badge {
  flex-shrink: 0;
  padding: 3px 8px;
  border-radius: 6px;
  border: 1px solid;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.icon-btn {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover:not(:disabled) {
    background: var(--accent-08);
    border-color: var(--accent-15);
    color: var(--accent);
  }

  &.active {
    background: var(--accent-12);
    border-color: var(--accent-20);
    color: var(--accent);
  }

  &--accent:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  &--danger:hover:not(:disabled) {
    background: var(--error-15);
    border-color: var(--error-30);
    color: var(--error);
  }

  &:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 34px;
  padding: 0 16px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;

  &--ghost {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-dim);

    &:hover {
      background: var(--bg-input);
      color: var(--text-main);
    }
  }

  &--primary {
    border: none;
    background: var(--accent);
    color: #fff;
    box-shadow: 0 3px 12px var(--accent-20);

    &:hover:not(:disabled) {
      filter: brightness(1.08);
      transform: translateY(-1px);
    }

    &:disabled {
      opacity: 0.55;
      cursor: not-allowed;
    }
  }
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}
</style>
