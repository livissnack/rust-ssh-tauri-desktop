<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { toast } from '../utils/toast.ts';
import type { SessionStatus } from "../utils/session.ts";
import { sessionStatusLabel, useI18n } from "../utils/i18n.ts";
const { tr, t } = useI18n();

const props = defineProps<{
  openSessions: Array<{ id: string; serverId: string; name: string; kind?: string }>;
  latencyServer: { id: string; jump_host_id?: string | null } | null;
  isActiveLocalSession?: boolean;
  sessionStatus: SessionStatus;
  servers?: Array<{ id: string; name: string }>;
}>();

const hasUpdate = ref(false);
const updateInfo = ref<any>(null);
const isDownloading = ref(false);

const currentLatency = ref<number | string | null>(null);
const latencyCache = new Map<string, number | string>();

const isOnline = computed(() => props.openSessions.length > 0);

const connectionStatus = computed(() => {
  if (!isOnline.value) return tr.value.statusBar.idle;
  if (props.isActiveLocalSession) return sessionStatusLabel(props.sessionStatus, true);
  return sessionStatusLabel(props.sessionStatus, false);
});

const showOnlineDot = computed(() =>
  props.sessionStatus === 'connected' || (props.isActiveLocalSession && props.sessionStatus !== 'failed'),
);

const latencyServerId = computed(() => {
  if (props.isActiveLocalSession) return null;
  return props.latencyServer?.id ?? null;
});

const jumpHostLabel = computed(() => {
  const jumpId = props.latencyServer?.jump_host_id;
  if (!jumpId || !props.servers) return null;
  const jump = props.servers.find((s) => s.id === jumpId);
  return jump ? t('statusBar.jumpHost', { name: jump.name }) : t('statusBar.jumpEnabled');
});

const latencyTitle = computed(() => {
  if (currentLatency.value === 'ERR') {
    return tr.value.statusBar.latencyErr;
  }
  if (typeof currentLatency.value === 'number') {
    return tr.value.statusBar.latencyOk;
  }
  return '';
});

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
    toast.error(t('statusBar.updateFailed'));
    isDownloading.value = false;
  }
};

watch(
  latencyServerId,
  async (serverId) => {
    if (!serverId) {
      currentLatency.value = null;
      return;
    }

    const cached = latencyCache.get(serverId);
    if (cached !== undefined) {
      currentLatency.value = cached;
      return;
    }

    currentLatency.value = "...";

    try {
      const ms = await invoke<number>("get_server_latency", { serverId });
      latencyCache.set(serverId, ms);
      if (latencyServerId.value === serverId) {
        currentLatency.value = ms;
      }
    } catch (err) {
      console.error("测速失败:", err);
      latencyCache.set(serverId, "ERR");
      if (latencyServerId.value === serverId) {
        currentLatency.value = "ERR";
      }
    }
  },
  { immediate: true },
);

onMounted(() => {
  checkUpdate();
});
</script>

<template>
  <footer class="status-bar">
    <div class="status-left">
      <span class="status-item">
        <i :class="['dot', { online: showOnlineDot }]"></i>
        {{ connectionStatus }}
      </span>

      <Tooltip
          v-if="isOnline && !isActiveLocalSession && sessionStatus === 'connected' && currentLatency !== null"
          :text="latencyTitle"
          placement="top"
          wrap
          :disabled="!latencyTitle"
      >
        <span class="status-item latency">
          <i class="fas fa-bolt"></i>
          {{ currentLatency }}{{ typeof currentLatency === 'number' ? 'ms' : '' }}
        </span>
      </Tooltip>
    </div>

    <div class="status-right">
      <span v-if="hasUpdate" class="status-item update-badge" @click="startUpdate">
        <i :class="['fas', isDownloading ? 'fa-spinner fa-spin' : 'fa-arrow-alt-circle-up']"></i>
        {{ isDownloading ? tr.statusBar.updating : t('statusBar.updateTo', { version: updateInfo?.version ?? '' }) }}
      </span>
      <span v-if="jumpHostLabel" class="status-item" :title="jumpHostLabel">
        <i class="fas fa-project-diagram" style="font-size: 10px; color: #bb9af7;"></i>
        {{ jumpHostLabel }}
      </span>
      <span class="status-item">UTF-8</span>
    </div>
  </footer>
</template>

<style lang="scss" scoped>
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
    font-family: var(--font-terminal);
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: help;

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

    i { font-size: 12px; }
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
    transition: color 0.2s;

    &:hover { color: var(--text-main); }
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
