<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "../utils/confirm";
import { debounce } from "../utils/async.ts";
import { toast } from "../utils/toast.ts";

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
const dragOverIndex = ref<number | null>(null);

const onDragStart = (e: DragEvent, index: number) => {
  dragIndex.value = index;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move';
  }
};

const onDragEnd = () => {
  dragIndex.value = null;
  dragOverIndex.value = null;
};

const onDragOver = (e: DragEvent, index: number) => {
  e.preventDefault();
  if (e.dataTransfer) {
    e.dataTransfer.dropEffect = 'move';
  }
  if (dragIndex.value !== null && dragIndex.value !== index) {
    dragOverIndex.value = index;
  }
};

const onDragLeave = (e: DragEvent) => {
  const related = e.relatedTarget as Node | null;
  if (!related || !(e.currentTarget as HTMLElement).contains(related)) {
    dragOverIndex.value = null;
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

  dragOverIndex.value = null;
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
      <div class="brand__logo">
        <i class="fas fa-terminal"></i>
      </div>
      <div class="brand__info">
        <span class="brand__name">Hiphup</span>
        <span class="brand__tagline">SSH Client</span>
      </div>
    </div>

    <div class="sidebar-scroll-area">
      <nav class="nav-groups">
        <header class="hosts-header">
          <span class="hosts-header__label">Hosts</span>
          <span v-if="props.servers.length" class="hosts-header__count">{{ props.servers.length }}</span>
        </header>

        <div v-if="props.servers.length === 0" class="host-empty">
          <div class="host-empty__icon">
            <i class="fas fa-server"></i>
          </div>
          <p class="host-empty__title">暂无主机</p>
          <p class="host-empty__hint">添加 SSH 连接以开始使用</p>
        </div>

        <TransitionGroup v-else name="list" tag="div" class="host-list">
          <div
              v-for="(s, index) in props.servers"
              :key="s.id"
              :class="['host-item', {
                'is-active': props.activeId === s.id,
                'is-dragging': dragIndex === index,
                'is-drop-target': dragOverIndex === index
              }]"
              draggable="true"
              @dragstart="onDragStart($event, index)"
              @dragover="onDragOver($event, index)"
              @dragleave="onDragLeave"
              @drop="onDrop($event, index)"
              @dragend="onDragEnd"
              @click="emit('update:activeId', s.id)"
              @dblclick="handleDoubleClick"
          >
            <div class="host-item__accent" aria-hidden="true"></div>

            <div class="host-item__icon">
              <i class="fas fa-server"></i>
            </div>

            <div class="host-item__body">
              <span class="host-item__name">{{ s.name }}</span>
              <span class="host-item__meta">
                <span class="host-item__user">{{ s.username }}</span>
                <span class="host-item__sep">·</span>
                <span class="host-item__address">{{ s.host }}:{{ s.port }}</span>
                <Tooltip v-if="s.jump_host_id" text="跳板机连接">
                  <span class="host-item__badge">
                    <i class="fas fa-diagram-project"></i>
                  </span>
                </Tooltip>
              </span>
            </div>

            <div class="host-item__actions">
              <Tooltip text="编辑配置">
                <button
                    type="button"
                    class="host-item__action"
                    @click.stop="emit('edit', s)"
                >
                  <i class="fas fa-pen-to-square"></i>
                </button>
              </Tooltip>
              <Tooltip text="删除服务器">
                <button
                    type="button"
                    class="host-item__action host-item__action--danger"
                    @click.stop="deleteServer(s.id)"
                >
                  <i class="fas fa-trash-can"></i>
                </button>
              </Tooltip>
            </div>
          </div>
        </TransitionGroup>
      </nav>
    </div>

    <div class="sidebar-footer">
      <button class="add-host-btn" @click="emit('openAddModal')">
        <i class="fas fa-plus"></i>
        <span>Add New Host</span>
      </button>
      <p class="sidebar-footer__hint">双击主机可快速连接</p>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
.sidebar {
  width: 260px;
  flex-shrink: 0;
  background-color: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  height: 100%;
  user-select: none;

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 16px 14px;
    border-bottom: 1px solid var(--border-30);

    &__logo {
      width: 34px;
      height: 34px;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 9px;
      background: linear-gradient(135deg, var(--accent), var(--accent-purple));
      color: #fff;
      font-size: 14px;
      box-shadow: 0 4px 14px var(--accent-20);
    }

    &__info {
      min-width: 0;
      display: flex;
      flex-direction: column;
      gap: 2px;
    }

    &__name {
      font-size: 15px;
      font-weight: 700;
      color: var(--text-main);
      letter-spacing: 0.02em;
      line-height: 1.2;
    }

    &__tagline {
      font-size: 10px;
      font-weight: 600;
      color: var(--text-dim);
      letter-spacing: 0.06em;
      text-transform: uppercase;
      opacity: 0.75;
    }
  }

  .sidebar-scroll-area {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 8px 12px 16px;

    &::-webkit-scrollbar { width: 4px; }
    &::-webkit-scrollbar-thumb {
      background: var(--border);
      border-radius: 4px;

      &:hover { background: var(--scrollbar-thumb-hover); }
    }
  }

  .nav-groups {
    min-height: 0;
  }
}

.host-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.host-item {
  position: relative;
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 10px;
  padding: 10px 10px 10px 12px;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  cursor: grab;
  overflow: hidden;
  transition:
    background 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease,
    opacity 0.2s ease;

  &:active { cursor: grabbing; }

  &:hover {
    background: var(--bg-primary-30);
    border-color: var(--border-30);

    .host-item__actions {
      opacity: 1;
      transform: translateX(0);
      pointer-events: auto;
    }

    .host-item__icon {
      border-color: var(--accent-20);
      color: var(--accent);
    }
  }

  &.is-active {
    background: var(--accent-08);
    border-color: var(--accent-20);
    box-shadow: inset 0 0 0 1px var(--accent-10);

    .host-item__accent {
      opacity: 1;
      transform: scaleY(1);
    }

    .host-item__icon {
      background: var(--accent-15);
      border-color: var(--accent-30);
      color: var(--accent);
    }

    .host-item__name {
      color: var(--accent);
      font-weight: 600;
    }

    .host-item__actions {
      opacity: 1;
      transform: translateX(0);
      pointer-events: auto;
    }
  }

  &.is-dragging {
    opacity: 0.45;
    border-style: dashed;
    border-color: var(--accent-30);
    background: var(--accent-05);
  }

  &.is-drop-target {
    border-color: var(--accent-30);
    background: var(--accent-08);
    box-shadow: 0 0 0 1px var(--accent-15);

    &::after {
      content: '';
      position: absolute;
      inset: 0;
      border-radius: inherit;
      border: 1px dashed var(--accent-30);
      pointer-events: none;
    }
  }

  &__accent {
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
    opacity: 0;
    transform: scaleY(0.4);
    transition: opacity 0.2s ease, transform 0.2s ease;
  }

  &__icon {
    width: 34px;
    height: 34px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: var(--bg-input);
    border: 1px solid var(--border-30);
    color: var(--text-dim);
    font-size: 13px;
    transition: all 0.2s ease;
  }

  &__body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  &__name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-main);
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__meta {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    font-size: 11px;
    line-height: 1.2;
    color: var(--text-dim);
  }

  &__user {
    flex-shrink: 0;
    max-width: 72px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__sep {
    flex-shrink: 0;
    opacity: 0.5;
  }

  &__address {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-terminal);
    font-size: 10px;
    letter-spacing: 0.02em;
  }

  &__badge {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    margin-left: 2px;
    border-radius: 4px;
    background: var(--accent-orange-10);
    color: var(--accent-orange);
    font-size: 8px;
  }

  &__actions {
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transform: translateX(4px);
    pointer-events: none;
    transition: opacity 0.2s ease, transform 0.2s ease;
  }

  &__action {
    width: 26px;
    height: 26px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;

    &:hover {
      background: var(--accent-10);
      color: var(--accent);
    }

    &--danger:hover {
      background: var(--error-15);
      color: var(--error);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 2px var(--accent-glow);
    }
  }
}

.list-move {
  transition: transform 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.list-enter-active,
.list-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}

.list-enter-from,
.list-leave-to {
  opacity: 0;
  transform: translateX(-8px);
}

.sidebar-footer {
  padding: 10px 12px 12px;
  border-top: 1px solid var(--border-30);
  background: var(--bg-secondary-60);

  &__hint {
    margin: 8px 0 0;
    text-align: center;
    font-size: 10px;
    color: var(--text-dim);
    opacity: 0.65;
    letter-spacing: 0.02em;
  }
}

.add-host-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px dashed var(--border-50);
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;

  i {
    font-size: 11px;
    transition: transform 0.2s ease;
  }

  &:hover {
    border-color: var(--accent-30);
    color: var(--accent);
    background: var(--accent-05);

    i { transform: rotate(90deg); }
  }

  &:focus-visible {
    outline: none;
    border-color: var(--accent-30);
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
}

.host-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 28px 16px 20px;
  text-align: center;

  &__icon {
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 12px;
    border-radius: 12px;
    background: var(--bg-input);
    border: 1px dashed var(--border-50);
    color: var(--text-dim);
    font-size: 16px;
    opacity: 0.8;
  }

  &__title {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-main);
  }

  &__hint {
    margin: 0;
    font-size: 11px;
    line-height: 1.5;
    color: var(--text-dim);
    opacity: 0.75;
  }
}

.hosts-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 2px 6px 8px;
  margin-bottom: 2px;

  &__label {
    position: relative;
    padding-left: 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    letter-spacing: 0.02em;
    line-height: 1;

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
      opacity: 0.6;
    }
  }

  &__count {
    min-width: 22px;
    height: 20px;
    padding: 0 7px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    background: var(--accent-12);
    border: 1px solid var(--accent-20);
    font-size: 12px;
    font-weight: 700;
    font-family: var(--font-terminal);
    font-variant-numeric: tabular-nums;
    color: var(--accent);
    text-align: center;
    line-height: 1;
  }
}
</style>