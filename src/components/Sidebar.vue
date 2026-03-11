<script setup lang="ts">
import { computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ask } from '@tauri-apps/plugin-dialog';

const props = defineProps<{
  activeId: string | null;
  servers: any[];
}>();

const emit = defineEmits<{
  (e: 'update:activeId', id: string): void;
  (e: 'connect'): void;
  (e: 'edit', server: any): void;
  (e: 'delete', id: string): void;
  (e: 'openAddModal'): void;
}>();

const activeId = computed({
  get: () => props.activeId,
  set: (id) => emit('update:activeId', id)
});

const deleteServer = async (id: string) => {
  const server = props.servers.find(s => s.id === id);
  const confirmed = await ask(`确定删除服务器配置 "${server?.name}"？`, {
    title: '确认删除',
    kind: 'warning',
    okLabel: '确定',
    cancelLabel: '取消',
  });

  if (confirmed) {
    await invoke("delete_server", { id });
    emit('delete', id);
  }
};

const handleDoubleClick = () => {
  emit('connect');
};
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="logo-hex">H</div>
      <span class="brand-text">Hiphup</span>
    </div>
    <div class="sidebar-scroll-area">
      <nav class="nav-groups">
        <div class="group-label">HOSTS</div>
        <div
            v-for="s in servers"
            :key="s.id"
            :class="['host-card', { active: activeId === s.id }]"
            @click="activeId = s.id"
            @dblclick="handleDoubleClick"
        >
          <div class="host-icon-wrapper">
            <div v-if="activeId === s.id" class="pulse-ring"></div>
            <span class="icon">🖥️</span>
          </div>
          <div class="host-meta">
            <div class="name">{{ s.name }}</div>
            <div class="ip">{{ s.host }}</div>
          </div>
          <div class="host-actions">
            <span @click.stop="emit('edit', s)">⚙️</span>
            <span @click.stop="deleteServer(s.id)">×</span>
          </div>
        </div>
      </nav>
    </div>
    <div class="sidebar-footer">
      <button class="add-host-btn" @click="emit('openAddModal')">+ Add New Host</button>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
@use "sass:color";
@use '../assets/css/base.scss';

.sidebar {
  width: 260px;
  background-color: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  height: 100%;

  .brand {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 24px 20px;

    .logo-hex {
      width: 32px;
      height: 32px;
      // 修复点：使用 CSS 变量构建渐变，注意这里的 --accent-purple 需在 base.scss 中定义
      background: linear-gradient(135deg, var(--accent), var(--accent-purple, #a78bfa));
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--bg-primary); // 通常 Logo 文字使用反差背景色
      font-weight: 800;
      box-shadow: 0 4px 12px var(--accent-30); // 修复点
    }

    .brand-text {
      font-weight: 700;
      color: var(--text-main);
      font-size: 18px;
      letter-spacing: -0.5px;
    }
  }

  .sidebar-scroll-area {
    flex: 1;
    overflow-y: auto;
    padding: 0 12px;

    /* 滚动条美化 */
    &::-webkit-scrollbar { width: 4px; }
    &::-webkit-scrollbar-thumb {
      background: var(--border);
      border-radius: 4px;
    }
  }

  .sidebar-footer {
    flex-shrink: 0;
    padding: 16px 12px;
    border-top: 1px solid var(--border);
    // 修复点：使用预计算的透明背景
    background: var(--bg-secondary-60);
  }
}

.host-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 12px;
  margin-bottom: 8px;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  border: 1px solid transparent;

  &:hover {
    background: var(--bg-secondary);
    border-color: var(--border);

    .host-actions {
      opacity: 1;
      transform: translateX(0);
    }
  }

  &.active {
    background: var(--bg-card);
    border-color: var(--accent-30); // 修复点
    box-shadow: 0 8px 20px var(--shadow); // 修复点

    .host-icon-wrapper {
      background: var(--accent);
      color: var(--bg-primary);
    }

    .pulse-ring {
      display: block;
    }

    .host-meta .name { color: var(--accent); }
  }

  .host-icon-wrapper {
    position: relative;
    width: 38px;
    height: 38px;
    background: var(--bg-input);
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-dim);
    transition: all 0.2s;
  }

  .host-meta {
    flex: 1;
    min-width: 0;
    .name {
      font-size: 13px;
      font-weight: 600;
      color: var(--text-main);
      margin-bottom: 2px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .ip {
      font-size: 11px;
      color: var(--text-dim);
      font-family: 'JetBrains Mono', monospace;
    }
  }

  .host-actions {
    position: absolute;
    right: 12px;
    display: flex;
    gap: 8px;
    opacity: 0;
    transform: translateX(5px);
    transition: all 0.2s ease;

    .action-item {
      cursor: pointer;
      color: var(--text-dim);
      font-size: 14px;
      &:hover {
        color: var(--accent);
      }
      &.del:hover { color: var(--error); }
    }
  }
}

.add-host-btn {
  width: 100%;
  background: transparent;
  border: 1px dashed var(--border);
  color: var(--text-dim);
  padding: 12px;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 13px;

  &:hover {
    border-color: var(--accent);
    background: var(--accent-05); // 修复点
    color: var(--accent);
  }
}

/* 激活态呼吸灯效果 */
.pulse-ring {
  display: none;
  position: absolute;
  inset: 0;
  border: 2px solid var(--accent);
  border-radius: 10px;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0% { transform: scale(1); opacity: 0.6; }
  100% { transform: scale(1.4); opacity: 0; }
}

.group-label {
  font-size: 10px;
  color: var(--text-dim); // 使用 text-dim 保持可读性
  text-transform: uppercase;
  letter-spacing: 1.5px;
  margin: 18px 0 8px 12px;
  font-weight: 700;
  opacity: 0.6; // 通过 opacity 控制视觉强度
}
</style>
