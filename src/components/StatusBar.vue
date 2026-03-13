<script setup lang="ts">
import {computed, onMounted, ref, watch} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {check} from '@tauri-apps/plugin-updater';
import {relaunch} from '@tauri-apps/plugin-process';

const props = defineProps<{
  openSessions: Array<{ id: string; serverId: string; name: string }>;
  currentServer: any; // 包含 host, port, id 等信息
}>();

const hasUpdate = ref(false);
const updateInfo = ref<any>(null);
const isDownloading = ref(false);

const currentLatency = ref<number | string | null>(null);

const connectionStatus = computed(() => {
  return props.openSessions.length > 0 ? 'Connected' : 'Idle';
});

const isOnline = computed(() => props.openSessions.length > 0);

const checkUpdate = async () => {
  try {
    const update = await check();
    if (update) {
      hasUpdate.value = true;
      updateInfo.value = update;
    }
  } catch (e) {
    console.error("更新检查失败:", e);
  }
};

const startUpdate = async () => {
  if (!updateInfo.value) return;
  isDownloading.value = true;
  try {
    await updateInfo.value.downloadAndInstall();
    await relaunch();
  } catch (e) {
    console.error("下载更新失败:", e);
    isDownloading.value = false;
  }
};

watch(() => props.currentServer?.id, async (newId) => {
  if (!newId || !props.currentServer?.host) {
    currentLatency.value = null;
    return;
  }

  currentLatency.value = "...";

  try {
    currentLatency.value = await invoke<number>("get_server_latency", {
      host: props.currentServer.host,
      port: props.currentServer.port || 22
    });
  } catch (err) {
    console.error("测速失败:", err);
    currentLatency.value = "ERR";
  }
}, { immediate: true });

onMounted(() => {
  checkUpdate();
});
</script>

<template>
  <footer class="status-bar">
    <div class="status-left">
      <span class="status-item">
        <i :class="['dot', { online: isOnline }]"></i>
        {{ connectionStatus }}
      </span>

      <span v-if="isOnline && currentLatency !== null" class="status-item latency">
        <i class="fas fa-bolt"></i>
        {{ currentLatency }}{{ typeof currentLatency === 'number' ? 'ms' : '' }}
      </span>
    </div>

    <div class="status-right">
      <span v-if="hasUpdate" class="status-item update-badge" @click="startUpdate">
        <i :class="['fas', isDownloading ? 'fa-spinner fa-spin' : 'fa-arrow-alt-circle-up']"></i>
        {{ isDownloading ? 'Updating...' : `Update to v${updateInfo?.version}` }}
      </span>
      <span v-if="currentServer?.jump_host_id" class="status-item">
        <i class="fas fa-project-diagram" style="font-size: 10px; color: #bb9af7;"></i>
        Jump: Active
      </span>
      <span class="status-item">UTF-8</span>
    </div>
  </footer>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.status-bar {
  height: 30px;
  background: var(--bg-sidebar);
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 15px;
  font-size: 11px;
  color: var(--text-dim);
  user-select: none;

  .status-left,
  .status-right {
    display: flex;
    gap: 15px;
    align-items: center;
  }

  .latency {
    color: var(--accent-orange, #f97316);
    font-family: 'JetBrains Mono', monospace;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 4px;

    i {
      font-size: 10px;
      filter: drop-shadow(0 0 3px var(--accent-orange-50, rgba(249, 115, 22, 0.5)));
    }
  }

  .update-badge {
    color: var(--accent-blue, #3b82f6) !important;
    font-weight: bold;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.1);
    transition: all 0.2s;

    &:hover {
      background: rgba(59, 130, 246, 0.2);
      color: var(--text-main) !important;
    }

    i {
      font-size: 12px;
    }
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
    transition: color 0.2s;

    &:hover {
      color: var(--text-main);
    }
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--border);

    &.online {
      background: var(--success);
      box-shadow: 0 0 8px var(--success-60, rgba(16, 185, 129, 0.6));
      animation: status-pulse 3s infinite;
    }
  }
}

@keyframes status-pulse {
  0% { opacity: 1; }
  50% { opacity: 0.7; }
  100% { opacity: 1; }
}
</style>
