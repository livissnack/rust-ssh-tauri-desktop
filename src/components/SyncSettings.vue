<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
const emit = defineEmits(['refresh-data']);

const isConfigVisible = ref(false);
const isSyncing = ref(false);
const showMasterKey = ref(false); // 控制主密钥明文显示

const syncForm = ref({
  endpoint: '',
  username: '',
  password: '',
  remote_filename: 'ssh_sync_backup.enc',
  master_key: '', // 本地加密用的主密钥
  auto_sync: false
});

const lastSyncTime = ref<string | null>(null);

// 加载持久化的 WebDAV 配置
const loadSettings = async () => {
  try {
    const saved = await invoke('get_sync_settings') as any;
    if (saved) {
      // 合并配置，保留默认文件名
      syncForm.value = { ...syncForm.value, ...saved };
    }
  } catch (e) {
    console.error("加载配置失败:", e);
  }
};

// 核心同步逻辑
const handleSync = async (type: 'upload' | 'download') => {
  // 验证必填项
  if (!syncForm.value.endpoint || !syncForm.value.password) {
    toast.warning("请完善 WebDAV 服务器配置", "配置缺失");
    return;
  }
  if (!syncForm.value.master_key) {
    toast.warning("请输入主加密密钥，否则无法加解密数据", "安全验证");
    return;
  }

  isSyncing.value = true;
  // 对应 Rust 端的 #[command] 名称
  const command = type === 'upload' ? 'sync_to_cloud' : 'sync_from_cloud';

  try {
    const msg = await invoke(command, { config: syncForm.value }) as string;
    lastSyncTime.value = new Date().toLocaleTimeString();
    toast.success(msg, type === 'upload' ? "备份成功" : "恢复成功");
  } catch (err) {
    // 捕获解密失败、网络错误或 401 认证失败
    toast.error(`${err}`, "同步操作失败");
  } finally {
    isSyncing.value = false;
  }
};

// 保存配置到本地数据库
const saveSettings = async () => {
  try {
    await invoke('save_sync_settings', { config: syncForm.value });
    toast.success("同步配置已保存至本地");
    isConfigVisible.value = false;
  } catch (err) {
    toast.error("保存失败");
  }
};

onMounted(loadSettings);
</script>

<template>
  <div class="sync-panel">
    <div class="panel-header">
      <div class="title">
        <i class="fas fa-shield-halved" :class="{ 'fa-spin': isSyncing }"></i>
        <span>安全同步</span>
      </div>
      <button class="icon-btn" @click="isConfigVisible = !isConfigVisible" :class="{ 'is-active': isConfigVisible }">
        <i class="fas fa-cog"></i>
      </button>
    </div>

    <div class="expand-container" :class="{ 'is-expanded': isConfigVisible }">
      <div class="expand-content">
        <div class="sync-form">
          <div class="input-group">
            <label>WebDAV 地址</label>
            <input v-model="syncForm.endpoint" placeholder="http://ip:port/" class="form-input" />
          </div>

          <div class="input-row">
            <div class="input-group">
              <label>用户名</label>
              <input v-model="syncForm.username" placeholder="Username" class="form-input" />
            </div>
          </div>

          <div class="input-row">
            <div class="input-group">
              <label>WebDAV 密码</label>
              <input v-model="syncForm.password" type="password" placeholder="Password" class="form-input" />
            </div>
          </div>

          <div class="input-group master-key-box">
            <label class="master-label">
              <i class="fas fa-key"></i> 主加密密钥 (E2EE)
            </label>
            <div class="password-wrapper">
              <input
                  v-model="syncForm.master_key"
                  :type="showMasterKey ? 'text' : 'password'"
                  placeholder="此密钥不上传，丢失将无法解密云端数据"
                  class="form-input"
              />
              <i class="fas" :class="showMasterKey ? 'fa-eye-slash' : 'fa-eye'" @click="showMasterKey = !showMasterKey"></i>
            </div>
          </div>

          <div class="form-actions">
            <label class="auto-sync">
              <input type="checkbox" v-model="syncForm.auto_sync"> 自动同步
            </label>
            <button @click="saveSettings" class="btn-save">保存配置</button>
          </div>
        </div>
      </div>
    </div>

    <div class="sync-operations">
      <div class="op-card upload" @click="handleSync('upload')" :class="{ 'is-disabled': isSyncing }">
        <div class="op-icon"><i class="fas fa-lock"></i></div>
        <div class="op-info">
          <div class="op-title">本地加密并上传</div>
          <div class="op-desc">推送 AES 密文到自建 WebDAV</div>
        </div>
      </div>

      <div class="op-card download" @click="handleSync('download')" :class="{ 'is-disabled': isSyncing }">
        <div class="op-icon"><i class="fas fa-unlock-keyhole"></i></div>
        <div class="op-info">
          <div class="op-title">下载并本地解密</div>
          <div class="op-desc">从云端拉取并恢复本地配置</div>
        </div>
      </div>
    </div>

    <div class="sync-footer" v-if="lastSyncTime">
      <i class="fas fa-clock-rotate-left"></i>
      最近同步: {{ lastSyncTime }}
    </div>
  </div>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.sync-panel {
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  border-radius: 10px;
  border: 1px solid var(--border);
  overflow: hidden;
  box-shadow: 0 4px 25px var(--shadow);
}

.panel-header {
  padding: 14px 18px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--border);
  // 修复点：使用预计算的背景透明度变量
  background: var(--bg-secondary-60);

  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    font-weight: 600;
    color: var(--accent);
    i { font-size: 16px; }
  }
}

.sync-form {
  padding: 20px;
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  gap: 15px;

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    label {
      font-size: 11px;
      color: var(--text-dim); // 统一使用 text-dim
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
  }

  .input-row { display: flex; gap: 12px; }

  .form-input {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    color: var(--text-main);
    font-size: 13px;
    transition: all 0.2s;

    &:focus {
      outline: none;
      border-color: var(--accent);
      box-shadow: 0 0 0 2px var(--accent-10);
    }

    &::placeholder {
      color: var(--text-dim);
      opacity: 0.5;
    }
  }

  /* 主密钥特殊样式 */
  .master-key-box {
    // 修复点：使用 accent 的透明变量
    background: var(--accent-05);
    padding: 12px;
    border-radius: 8px;
    border: 1px dashed var(--accent-30);

    .master-label {
      color: var(--accent);
      font-weight: 700;
      font-size: 12px;
      display: flex;
      align-items: center;
      gap: 6px;
    }
  }

  .password-wrapper {
    position: relative;
    display: flex;
    align-items: center;

    .form-input { width: 100%; padding-right: 35px; }

    i {
      position: absolute;
      right: 12px;
      font-size: 14px;
      color: var(--text-dim);
      cursor: pointer;
      transition: color 0.2s;
      &:hover { color: var(--accent); }
    }
  }

  .form-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 10px;

    .auto-sync {
      font-size: 12px;
      color: var(--text-dim);
      display: flex;
      align-items: center;
      gap: 8px;
      cursor: pointer;
      transition: color 0.2s;

      &:hover { color: var(--text-main); }

      input {
        cursor: pointer;
        accent-color: var(--accent);
      }
    }

    .btn-save {
      background: var(--accent);
      color: var(--bg-primary);
      border: none;
      padding: 8px 18px;
      border-radius: 6px;
      font-weight: 700;
      cursor: pointer;
      transition: all 0.2s;

      &:hover {
        filter: brightness(1.1);
        box-shadow: 0 4px 12px var(--accent-30);
      }
    }
  }
}

.sync-operations {
  padding: 15px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--bg-primary);

  .op-card {
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 16px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);

    &.is-disabled {
      opacity: 0.4;
      pointer-events: none;
      filter: grayscale(1);
    }

    &:hover {
      background: var(--accent-08);
      border-color: var(--accent);
      transform: translateY(-2px);
      box-shadow: 0 6px 15px var(--shadow);

      .op-icon { color: var(--accent); }
      .op-title { color: var(--accent); }
    }

    .op-icon {
      font-size: 24px;
      color: var(--text-dim);
      transition: all 0.3s;
    }

    .op-info {
      flex: 1;
      .op-title {
        font-size: 14px;
        font-weight: 600;
        color: var(--text-main);
        transition: color 0.2s;
      }
      .op-desc {
        font-size: 11px;
        color: var(--text-dim);
        margin-top: 4px;
        line-height: 1.4;
      }
    }

    /* 悬浮时的特定颜色微调 - 建议 download 使用 success */
    &.download:hover .op-icon { color: var(--success, #10b981); }
  }
}

.sync-footer {
  padding: 12px 18px;
  font-size: 11px;
  color: var(--text-dim);
  background: var(--bg-secondary-60);
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 8px;

  i {
    font-size: 12px;
    color: var(--success);
  }
}

/* 抽屉式动画保持逻辑不变 */
.expand-container {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.35s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;

  &.is-expanded { grid-template-rows: 1fr; }

  .expand-content {
    min-height: 0;
    width: 100%;
  }
}

.icon-btn {
  background: transparent;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 6px;
  border-radius: 50%;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);

  &:hover {
    background: var(--accent-10);
    color: var(--text-main);
  }

  &.is-active {
    transform: rotate(90deg);
    color: var(--accent);
  }
}
</style>