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
/* 变量定义 */
$bg-primary: #1a1b26;
$bg-secondary: #16161e;
$accent: #bb9af7;
$border: #292e42;
$text-main: #c0caf5;
$text-dim: #565f89;
$success: #9ece6a;
$warning: #e0af68;

.sync-panel {
  display: flex;
  flex-direction: column;
  background: $bg-primary;
  border-radius: 10px;
  border: 1px solid $border;
  overflow: hidden;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
}

.panel-header {
  padding: 14px 18px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid $border;
  background: rgba($bg-secondary, 0.5);

  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    font-weight: 600;
    color: $accent;
    i { font-size: 16px; }
  }
}

.sync-form {
  padding: 20px;
  background: $bg-secondary;
  display: flex;
  flex-direction: column;
  gap: 15px;

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    label { font-size: 12px; color: $text-dim; font-weight: 500; }
  }

  .input-row { display: flex; gap: 12px; }

  .form-input {
    background: $bg-primary;
    border: 1px solid $border;
    border-radius: 6px;
    padding: 10px;
    color: $text-main;
    font-size: 13px;
    transition: all 0.2s;
    &:focus { outline: none; border-color: $accent; box-shadow: 0 0 0 2px rgba($accent, 0.2); }
    &::placeholder { color: #414868; }
  }

  /* 主密钥特殊样式 */
  .master-key-box {
    background: rgba($accent, 0.05);
    padding: 12px;
    border-radius: 8px;
    border: 1px dashed rgba($accent, 0.3);
    .master-label { color: $accent; font-weight: 600; }
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
      color: $text-dim;
      cursor: pointer;
      &:hover { color: $accent; }
    }
  }

  .form-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 10px;
    .auto-sync {
      font-size: 12px; color: $text-dim; display: flex; align-items: center; gap: 6px; cursor: pointer;
      input { cursor: pointer; accent-color: $accent; }
    }
    .btn-save {
      background: $accent; color: $bg-primary; border: none;
      padding: 8px 16px; border-radius: 6px; font-weight: 700; cursor: pointer;
      transition: opacity 0.2s;
      &:hover { opacity: 0.9; }
    }
  }
}

.sync-operations {
  padding: 15px;
  display: flex;
  flex-direction: column;
  gap: 12px;

  .op-card {
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 16px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid $border;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);

    &.is-disabled { opacity: 0.5; pointer-events: none; }

    &:hover {
      background: rgba($accent, 0.08);
      border-color: $accent;
      transform: translateY(-2px);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
      .op-icon { color: $accent; }
    }

    .op-icon { font-size: 22px; color: $text-dim; transition: color 0.2s; }
    .op-title { font-size: 14px; font-weight: 600; color: #c0caf5; }
    .op-desc { font-size: 11px; color: $text-dim; margin-top: 4px; line-height: 1.4; }

    &.upload:hover .op-icon { color: #7aa2f7; }
    &.download:hover .op-icon { color: $success; }
  }
}

.sync-footer {
  padding: 10px 18px;
  font-size: 11px;
  color: $text-dim;
  background: rgba($bg-secondary, 0.3);
  border-top: 1px solid $border;
  display: flex;
  align-items: center;
  gap: 8px;
  i { font-size: 12px; }
}

/* 抽屉式动画逻辑 */
.expand-container {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.35s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
  &.is-expanded { grid-template-rows: 1fr; }
  .expand-content { min-height: 0; width: 100%; }
}

.icon-btn {
  background: transparent; border: none; color: $text-dim; cursor: pointer;
  padding: 6px; border-radius: 50%;
  transition: all 0.3s ease;
  &:hover { background: rgba($text-dim, 0.1); color: $text-main; }
  &.is-active { transform: rotate(90deg); color: $accent; }
}
</style>