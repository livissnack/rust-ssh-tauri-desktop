<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "../utils/confirm";
import {debounce} from "../utils/async.ts";

const props = defineProps<{
  activeId: string | null;
  servers: any[];
}>();

const emit = defineEmits<{
  (e: 'update:activeId', id: string): void;
  (e: 'update:servers', newList: any[]): void;
  (e: 'connect'): void;
  (e: 'edit', server: any): void;
  (e: 'delete', id: string): void;
  (e: 'openAddModal'): void;
}>();

const dragIndex = ref<number | null>(null);
const isDragging = ref(false);

const onDragStart = (e: DragEvent, index: number) => {
  dragIndex.value = index;
  isDragging.value = true;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move';
    const target = e.target as HTMLElement;
    target.style.opacity = '0.5';
  }
};

const onDragEnd = (e: DragEvent) => {
  isDragging.value = false;
  dragIndex.value = null;
  const target = e.target as HTMLElement;
  target.style.opacity = '1';
};

const onDragOver = (e: DragEvent) => {
  e.preventDefault();
  if (e.dataTransfer) {
    e.dataTransfer.dropEffect = 'move';
  }
};

const debouncedSaveOrder = debounce(async (newList: any[]) => {
  const ids = newList.map(s => s.id);
  try {
    await invoke("update_server_order", { ids });
  } catch (err) {
    console.error("保存排序失败:", err);
  }
}, 800);

const onDrop = async (e: DragEvent, targetIndex: number) => {
  e.preventDefault();
  if (dragIndex.value === null || dragIndex.value === targetIndex) return;

  const newList = [...props.servers];
  const [movedItem] = newList.splice(dragIndex.value, 1);
  newList.splice(targetIndex, 0, movedItem);

  emit('update:servers', newList);

  debouncedSaveOrder(newList);
};

const deleteServer = async (id: string) => {
  const server = props.servers.find(s => s.id === id);
  const ok = await confirm.error(
      `确定要删除 "${server?.name}" 吗？此操作无法恢复。`,
      '危险操作'
  );
  if (ok) {
    try {
      await invoke("delete_server", { id });
      emit('delete', id);
      toast.success("删除成功", "配置已移除");
    } catch (e) {
      toast.error("删除失败");
    }
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

        <TransitionGroup name="list" tag="div" class="drag-container">
          <div
              v-for="(s, index) in props.servers"
              :key="s.id"
              :class="['host-card', {
                active: props.activeId === s.id,
                'dragging-source': dragIndex === index
              }]"
              draggable="true"
              @dragstart="onDragStart($event, index)"
              @dragover="onDragOver($event)"
              @drop="onDrop($event, index)"
              @dragend="onDragEnd"
              @click="emit('update:activeId', s.id)"
              @dblclick="handleDoubleClick"
          >
            <div class="host-icon-wrapper">
              <div v-if="props.activeId === s.id" class="pulse-ring"></div>
              <span class="icon">🖥️</span>
            </div>

            <div class="host-meta">
              <div class="name">{{ s.name }}</div>
              <div class="ip">{{ s.host }}</div>
            </div>

            <div class="host-actions">
              <span class="action-item" @click.stop="emit('edit', s)" title="编辑配置">
                <i class="fas fa-gear"></i>
              </span>

              <span class="action-item del" @click.stop="deleteServer(s.id)" title="删除服务器">
                <i class="fas fa-xmark"></i>
              </span>
            </div>
          </div>
        </TransitionGroup>
      </nav>
    </div>

    <div class="sidebar-footer">
      <button class="add-host-btn" @click="emit('openAddModal')">
        + Add New Host
      </button>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
@use "sass:color";

.sidebar {
  width: 260px;
  background-color: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  height: 100%;
  user-select: none;

  .brand {
    padding: 24px 20px;
    display: flex;
    align-items: center;
    gap: 12px;
    .logo-hex {
      width: 32px; height: 32px;
      background: linear-gradient(135deg, var(--accent), #a78bfa);
      border-radius: 8px;
      display: flex; align-items: center; justify-content: center;
      color: white; font-weight: 800;
      box-shadow: 0 4px 12px var(--accent-30);
    }
    .brand-text { font-weight: 700; color: var(--text-main); font-size: 18px; }
  }

  .sidebar-scroll-area {
    flex: 1; overflow-y: auto; padding: 0 12px;
    &::-webkit-scrollbar { width: 4px; }
    &::-webkit-scrollbar-thumb { background: var(--border); border-radius: 4px; }
  }
}

.drag-container {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.host-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 12px;
  cursor: grab;
  transition: background 0.2s, border 0.2s, transform 0.2s;
  border: 1px solid transparent;
  position: relative;

  &:active { cursor: grabbing; }

  &:hover {
    background: var(--bg-secondary);
    .host-actions { opacity: 1; transform: translateX(0); }
  }

  &.active {
    background: var(--bg-card);
    border-color: var(--accent-30);
    .host-meta .name { color: var(--accent); }
    .host-icon-wrapper { background: var(--accent); color: white; }
  }

  &.dragging-source {
    opacity: 0.2;
    border: 1px dashed var(--accent);
  }

  .host-icon-wrapper {
    width: 38px; height: 38px;
    background: var(--bg-input);
    border-radius: 10px;
    display: flex; align-items: center; justify-content: center;
    color: var(--text-dim);
  }

  .host-meta {
    flex: 1; min-width: 0;
    .name { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .ip { font-size: 11px; color: var(--text-dim); font-family: monospace; }
  }

  .host-actions {
    position: absolute;
    right: 12px;
    display: flex;
    align-items: center;
    gap: 4px; // 稍微收紧间距
    opacity: 0;
    transform: translateX(5px);
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    z-index: 10;

    .action-item, .drag-handle {
      width: 28px;
      height: 28px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 6px;
      color: var(--text-dim);
      transition: all 0.2s;
      font-size: 14px; // FontAwesome 图标在这个尺寸最精致

      &:hover {
        background: rgba(255, 255, 255, 0.08);
        color: var(--accent);
        cursor: pointer;
      }
    }

    .drag-handle {
      cursor: grab;
      font-size: 13px; // 抓取手柄可以稍微小一点，显得精致
      &:active { cursor: grabbing; }
    }

    .del:hover {
      color: #ef4444; // 稍微亮一点的红色
      background: rgba(239, 68, 68, 0.1);
    }
  }
}

/* 过渡动画：当数组顺序改变时，其他卡片会平滑滑动 */
.list-move {
  transition: transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

.add-host-btn {
  width: calc(100% - 24px); margin: 16px 12px;
  background: transparent; border: 1px dashed var(--border);
  color: var(--text-dim); padding: 12px; border-radius: 12px;
  cursor: pointer;
  &:hover { border-color: var(--accent); color: var(--accent); background: var(--accent-05); }
}

@keyframes pulse {
  0% { transform: scale(1); opacity: 0.6; }
  100% { transform: scale(1.3); opacity: 0; }
}

.group-label {
  font-size: 10px; color: var(--text-dim); font-weight: 700;
  margin: 18px 0 8px 12px; opacity: 0.6; letter-spacing: 1px;
}
</style>