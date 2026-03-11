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
  background: base.$bg-sidebar; // 状态栏通常与侧边栏色调一致，保持底座稳重
  border-top: 1px solid base.$border; // 修改：统一使用 $border 变量
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 15px;
  font-size: 11px;
  color: base.$text-dim;
  user-select: none; // 状态栏建议禁止文本选中

  .status-left,
  .status-right {
    display: flex;
    gap: 15px;
    align-items: center;
  }

  /* 延迟显示：根据数值高低建议使用 $accent 或 $warning */
  .latency {
    color: base.$accent-orange; // 修改：使用主题中的橙色/警告色
    font-family: 'JetBrains Mono', monospace;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 4px;

    i {
      font-size: 10px;
      // 修改：发光颜色跟随变量
      filter: drop-shadow(0 0 3px rgba(base.$accent-orange, 0.5));
    }
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
    transition: color 0.2s;

    &:hover {
      color: base.$text-main;
    }
  }

  /* 状态指示灯 */
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: base.$border; // 离线状态使用边框色或深色

    &.online {
      background: base.$success;
      // 修改：发光半径微调，使用 rgba 确保柔和
      box-shadow: 0 0 8px rgba(base.$success, 0.6);
      animation: status-pulse 3s infinite; // 增加一个极其微弱的呼吸感
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
