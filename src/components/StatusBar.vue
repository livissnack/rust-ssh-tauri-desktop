<script setup lang="ts">
import { computed, watch, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  openSessions: Array<{ id: string; serverId: string; name: string }>;
  currentServer: any; // 包含 host, port, id 等信息
}>();

// 内部维护一个响应式的延迟值
const currentLatency = ref<number | string | null>(null);

const connectionStatus = computed(() => {
  return props.openSessions.length > 0 ? 'Connected' : 'Idle';
});

const isOnline = computed(() => props.openSessions.length > 0);

// 核心逻辑：监听服务器切换并测速
watch(() => props.currentServer?.id, async (newId) => {
  if (!newId || !props.currentServer?.host) {
    currentLatency.value = null;
    return;
  }

  currentLatency.value = "..."; // 正在测速的占位符

  try {
    // 调用我们刚才拆分出的独立后端功能
    const ms = await invoke<number>("get_server_latency", {
      host: props.currentServer.host,
      port: props.currentServer.port || 22
    });
    currentLatency.value = ms;
  } catch (err) {
    console.error("测速失败:", err);
    currentLatency.value = "ERR";
  }
}, { immediate: true }); // 立即执行一次以处理初始状态
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
      <span v-if="currentServer?.jump_host_id" class="status-item">
        <i class="fas fa-project-diagram" style="font-size: 10px; color: #bb9af7;"></i>
        Jump: Active
      </span>
      <span class="status-item">UTF-8</span>
    </div>
  </footer>
</template>

<style lang="scss" scoped>
$bg-sidebar: #16161e;
$border-color: #292e42;
$text-dim: #565f89;
$success: #9ece6a;

.status-bar {
  height: 30px;
  background: $bg-sidebar;
  border-top: 1px solid $border-color;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 15px;
  font-size: 11px;
  color: $text-dim;

  .status-left,
  .status-right {
    display: flex;
    gap: 15px;
  }

  .latency {
    color: #e0af68;
    font-family: 'JetBrains Mono', monospace;
    font-weight: 500;

    i {
      font-size: 10px;
      filter: drop-shadow(0 0 2px rgba(224, 175, 104, 0.4));
    }
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #414868;

    &.online {
      background: $success;
      box-shadow: 0 0 8px $success;
    }
  }
}
</style>
