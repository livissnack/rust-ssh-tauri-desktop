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

  /* 延迟显示：根据数值高低建议使用强调色或警告色 */
  .latency {
    // 假设 base.scss 映射了 --accent-orange，若无，请确保主题配置中包含该颜色
    color: var(--accent-orange, #f97316);
    font-family: 'JetBrains Mono', monospace;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 4px;

    i {
      font-size: 10px;
      // 修复点：使用预计算的带透明度的变量处理发光
      filter: drop-shadow(0 0 3px var(--accent-orange-50, rgba(249, 115, 22, 0.5)));
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

  /* 状态指示灯 */
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--border); // 离线状态

    &.online {
      background: var(--success);
      // 修复点：使用预计算的 --success-60 处理发光
      box-shadow: 0 0 8px var(--success-60, rgba(16, 185, 129, 0.6));
      animation: status-pulse 3s infinite;
    }
  }
}

/* 状态栏专用微弱动画 */
@keyframes status-pulse {
  0% { opacity: 1; }
  50% { opacity: 0.7; }
  100% { opacity: 1; }
}
</style>
