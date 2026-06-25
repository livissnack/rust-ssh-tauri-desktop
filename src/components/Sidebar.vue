<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "../utils/confirm";
import { debounce } from "../utils/async.ts";
import { toast } from "../utils/toast.ts";
import { beginPointerDrag, findAttrFromPoint } from "../utils/pointerDrag.ts";

const UNGROUPED_KEY = "__ungrouped__";
const UNGROUPED_LABEL = "未分组";
const COLLAPSED_STORAGE_KEY = "host-group-collapsed";

type HostGroup = {
  key: string;
  label: string;
  servers: any[];
};

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

const searchQuery = ref("");
const dragServerId = ref<string | null>(null);
const dropTargetServerId = ref<string | null>(null);
const dropTargetGroupKey = ref<string | null>(null);
const isReordering = ref(false);

const menuVisible = ref(false);
const menuPos = ref({ x: 0, y: 0 });
const menuTargetId = ref<string | null>(null);

const loadCollapsedGroups = (): Set<string> => {
  try {
    const raw = localStorage.getItem(COLLAPSED_STORAGE_KEY);
    if (raw) return new Set(JSON.parse(raw) as string[]);
  } catch { /* ignore */ }
  return new Set();
};

const collapsedGroups = ref<Set<string>>(loadCollapsedGroups());

const persistCollapsedGroups = debounce(() => {
  localStorage.setItem(
    COLLAPSED_STORAGE_KEY,
    JSON.stringify([...collapsedGroups.value]),
  );
}, 300);

const menuTargetServer = computed(() =>
  props.servers.find((s) => s.id === menuTargetId.value) ?? null,
);

const hostMetaDisplay = (server: { username: string; host: string; port: number }) =>
  `${server.username} · ${server.host}:${server.port}`;

const matchesSearch = (server: any, query: string) => {
  const group = (server.group || "").toLowerCase();
  return (
    server.name.toLowerCase().includes(query) ||
    server.host.toLowerCase().includes(query) ||
    server.username.toLowerCase().includes(query) ||
    String(server.port).includes(query) ||
    group.includes(query)
  );
};

const filteredServers = computed(() => {
  const query = searchQuery.value.trim().toLowerCase();
  if (!query) return props.servers;
  return props.servers.filter((s) => matchesSearch(s, query));
});

const hasAnyGroup = computed(() =>
  props.servers.some((s) => s.group?.trim()),
);

const isSearching = computed(() => searchQuery.value.trim().length > 0);

const isFlatView = computed(() => isSearching.value || !hasAnyGroup.value);

const hostGroups = computed((): HostGroup[] => {
  const map = new Map<string, any[]>();
  for (const s of filteredServers.value) {
    const key = s.group?.trim() || UNGROUPED_KEY;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(s);
  }

  const named = [...map.entries()]
    .filter(([key]) => key !== UNGROUPED_KEY)
    .sort((a, b) => a[0].localeCompare(b[0], "zh"))
    .map(([key, servers]) => ({ key, label: key, servers }));

  const ungrouped = map.get(UNGROUPED_KEY);
  if (ungrouped?.length) {
    named.push({ key: UNGROUPED_KEY, label: UNGROUPED_LABEL, servers: ungrouped });
  }
  return named;
});

const isGroupCollapsed = (key: string) =>
  !isSearching.value && collapsedGroups.value.has(key);

const toggleGroup = (key: string) => {
  if (collapsedGroups.value.has(key)) {
    collapsedGroups.value.delete(key);
  } else {
    collapsedGroups.value.add(key);
  }
  collapsedGroups.value = new Set(collapsedGroups.value);
  persistCollapsedGroups();
};

watch(isSearching, (searching) => {
  if (searching) return;
  for (const group of hostGroups.value) {
    if (group.servers.some((s) => s.id === props.activeId)) {
      collapsedGroups.value.delete(group.key);
      collapsedGroups.value = new Set(collapsedGroups.value);
    }
  }
});

const openHostMenu = (e: MouseEvent, serverId: string) => {
  e.preventDefault();
  e.stopPropagation();
  menuTargetId.value = serverId;
  menuPos.value = { x: e.clientX, y: e.clientY };
  menuVisible.value = true;
};

const closeHostMenu = () => {
  menuVisible.value = false;
  menuTargetId.value = null;
};

const handleMenuEdit = () => {
  const server = menuTargetServer.value;
  closeHostMenu();
  if (server) emit('edit', server);
};

const handleMenuSetGroup = async () => {
  const server = menuTargetServer.value;
  closeHostMenu();
  if (!server) return;

  const input = prompt("设置分组名称（留空表示未分组）:", server.group || "");
  if (input === null) return;

  const group = input.trim() || null;
  try {
    await invoke("save_server", { server: { ...server, group } });
    const updated = props.servers.map((s) =>
      s.id === server.id ? { ...s, group } : s,
    );
    emit("update:servers", updated);
    toast.success("分组已更新");
  } catch {
    toast.error("保存分组失败");
  }
};

const handleMenuDelete = () => {
  const id = menuTargetId.value;
  closeHostMenu();
  if (id) deleteServer(id);
};

const handlePointerDownOutside = (e: PointerEvent) => {
  if (!menuVisible.value) return;
  const target = e.target as HTMLElement;
  if (target.closest('.host-context-menu')) return;
  closeHostMenu();
};

const normalizeGroup = (group?: string | null) => group?.trim() || null;

const resetReorderVisuals = () => {
  dragServerId.value = null;
  dropTargetServerId.value = null;
  dropTargetGroupKey.value = null;
  isReordering.value = false;
};

const updateDropTargetFromPoint = (x: number, y: number, sourceId: string) => {
  const hostId = findAttrFromPoint(x, y, "data-host-id", sourceId);
  if (hostId) {
    dropTargetServerId.value = hostId;
    dropTargetGroupKey.value = null;
    return;
  }
  const groupKey = findAttrFromPoint(x, y, "data-group-key");
  if (groupKey) {
    dropTargetGroupKey.value = groupKey;
    dropTargetServerId.value = null;
  }
};

const debouncedSaveOrder = debounce(async (newList: any[]) => {
  const ids = newList.map((s) => s.id);
  try {
    await invoke("update_server_order", { ids });
  } catch (err) {
    console.error("保存排序失败:", err);
  }
}, 800);

const commitReorderToServer = async (fromId: string, targetServerId: string) => {
  if (fromId === targetServerId) return;

  const newList = [...props.servers];
  const fromIdx = newList.findIndex((s) => s.id === fromId);
  const toIdx = newList.findIndex((s) => s.id === targetServerId);
  if (fromIdx < 0 || toIdx < 0) return;

  const targetGroup = normalizeGroup(newList[toIdx].group);
  const [movedItem] = newList.splice(fromIdx, 1);
  const insertIdx = newList.findIndex((s) => s.id === targetServerId);
  const updatedItem = { ...movedItem, group: targetGroup };
  newList.splice(insertIdx, 0, updatedItem);

  if (normalizeGroup(movedItem.group) !== targetGroup) {
    try {
      await invoke("save_server", { server: updatedItem });
    } catch {
      toast.error("保存分组失败");
      return;
    }
  }

  emit("update:servers", newList);
  debouncedSaveOrder(newList);
};

const commitReorderToGroup = async (fromId: string, groupKey: string) => {
  const targetGroup = groupKey === UNGROUPED_KEY ? null : groupKey;
  const newList = [...props.servers];
  const fromIdx = newList.findIndex((s) => s.id === fromId);
  if (fromIdx < 0) return;

  const movedItem = { ...newList[fromIdx], group: targetGroup };
  const groupChanged = normalizeGroup(newList[fromIdx].group) !== targetGroup;

  newList.splice(fromIdx, 1);

  let insertIdx = newList.length;
  if (groupKey === UNGROUPED_KEY) {
    const idx = newList.findIndex((s) => !normalizeGroup(s.group));
    insertIdx = idx >= 0 ? idx : newList.length;
  } else {
    const idx = newList.findIndex((s) => normalizeGroup(s.group) === groupKey);
    insertIdx = idx >= 0 ? idx : newList.length;
  }
  newList.splice(insertIdx, 0, movedItem);

  if (groupChanged) {
    try {
      await invoke("save_server", { server: movedItem });
    } catch {
      toast.error("保存分组失败");
      return;
    }
  }

  emit("update:servers", newList);
  debouncedSaveOrder(newList);
};

const onHostNamePointerDown = (e: PointerEvent, serverId: string) => {
  beginPointerDrag(e, {
    onActivate: () => {
      isReordering.value = true;
      dragServerId.value = serverId;
    },
    onMove: (x, y) => updateDropTargetFromPoint(x, y, serverId),
    onFinish: async (_x, _y, activated) => {
      const targetId = dropTargetServerId.value;
      const targetGroup = dropTargetGroupKey.value;
      resetReorderVisuals();
      if (!activated) return;
      if (targetId) await commitReorderToServer(serverId, targetId);
      else if (targetGroup) await commitReorderToGroup(serverId, targetGroup);
    },
    onCancel: resetReorderVisuals,
  });
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

const clearSearch = () => {
  searchQuery.value = "";
};

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDownOutside);
  window.addEventListener('blur', closeHostMenu);
});

onUnmounted(() => {
  document.removeEventListener('pointerdown', handlePointerDownOutside);
  window.removeEventListener('blur', closeHostMenu);
});
</script>

<template>
  <aside class="sidebar" :class="{ 'is-reordering': isReordering }">
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
          <span v-if="props.servers.length" class="hosts-header__count">
            {{ filteredServers.length }}<template v-if="isSearching">/{{ props.servers.length }}</template>
          </span>
        </header>

        <div v-if="props.servers.length" class="hosts-search">
          <div class="hosts-search__wrapper">
            <i class="fas fa-search"></i>
            <input
              v-model="searchQuery"
              type="text"
              placeholder="搜索名称、地址、分组..."
              @keyup.esc="clearSearch"
            />
            <button
              v-if="searchQuery"
              type="button"
              class="hosts-search__clear"
              aria-label="清除搜索"
              @click="clearSearch"
            >
              <i class="fas fa-xmark"></i>
            </button>
          </div>
        </div>

        <div v-if="props.servers.length === 0" class="host-empty">
          <div class="host-empty__icon">
            <i class="fas fa-server"></i>
          </div>
          <p class="host-empty__title">暂无主机</p>
          <p class="host-empty__hint">添加 SSH 连接以开始使用</p>
        </div>

        <div v-else-if="filteredServers.length === 0" class="host-empty host-empty--compact">
          <div class="host-empty__icon">
            <i class="fas fa-magnifying-glass"></i>
          </div>
          <p class="host-empty__title">无匹配主机</p>
          <p class="host-empty__hint">试试其他关键词</p>
        </div>

        <TransitionGroup v-else-if="isFlatView" name="list" tag="div" class="host-list">
          <div
              v-for="s in filteredServers"
              :key="s.id"
              :data-host-id="s.id"
              :class="['host-item', {
                'is-active': props.activeId === s.id,
                'is-dragging': dragServerId === s.id,
                'is-drop-target': dropTargetServerId === s.id
              }]"
              @click="emit('update:activeId', s.id)"
              @dblclick="handleDoubleClick"
              @contextmenu="openHostMenu($event, s.id)"
          >
            <div class="host-item__accent" aria-hidden="true"></div>

            <div class="host-item__icon">
              <i class="fas fa-server"></i>
            </div>

            <div class="host-item__body">
              <span
                class="host-item__name"
                @pointerdown="onHostNamePointerDown($event, s.id)"
              >{{ s.name }}</span>
              <span class="host-item__meta">
                <span class="host-item__meta-text">{{ hostMetaDisplay(s) }}</span>
                <span v-if="s.jump_host_id" class="host-item__badge">
                  <i class="fas fa-diagram-project"></i>
                </span>
              </span>
            </div>
          </div>
        </TransitionGroup>

        <div v-else class="host-groups">
          <section v-for="group in hostGroups" :key="group.key" class="host-group">
            <button
              type="button"
              class="host-group__header"
              :data-group-key="group.key"
              :class="{
                'is-collapsed': isGroupCollapsed(group.key),
                'is-drop-target': dropTargetGroupKey === group.key,
              }"
              @click="toggleGroup(group.key)"
            >
              <i class="fas fa-chevron-right host-group__chevron"></i>
              <i class="fas fa-folder host-group__folder"></i>
              <span class="host-group__name">{{ group.label }}</span>
              <span class="host-group__count">{{ group.servers.length }}</span>
            </button>

            <div
              v-show="!isGroupCollapsed(group.key)"
              class="host-list host-list--nested"
            >
              <div
                  v-for="s in group.servers"
                  :key="s.id"
                  :data-host-id="s.id"
                  :class="['host-item', {
                    'is-active': props.activeId === s.id,
                    'is-dragging': dragServerId === s.id,
                    'is-drop-target': dropTargetServerId === s.id
                  }]"
                  @click="emit('update:activeId', s.id)"
                  @dblclick="handleDoubleClick"
                  @contextmenu="openHostMenu($event, s.id)"
              >
                <div class="host-item__accent" aria-hidden="true"></div>

                <div class="host-item__icon">
                  <i class="fas fa-server"></i>
                </div>

                <div class="host-item__body">
                  <span
                    class="host-item__name"
                    @pointerdown="onHostNamePointerDown($event, s.id)"
                  >{{ s.name }}</span>
                  <span class="host-item__meta">
                    <span class="host-item__meta-text">{{ hostMetaDisplay(s) }}</span>
                    <span v-if="s.jump_host_id" class="host-item__badge">
                      <i class="fas fa-diagram-project"></i>
                    </span>
                  </span>
                </div>
              </div>
            </div>
          </section>
        </div>
      </nav>
    </div>

    <Teleport to="body">
      <Transition name="host-menu">
        <div
          v-if="menuVisible && menuTargetServer"
          class="host-context-menu"
          :style="{ top: `${menuPos.y}px`, left: `${menuPos.x}px` }"
          @contextmenu.prevent
          @click.stop
        >
          <div class="host-context-menu__title">{{ menuTargetServer.name }}</div>
          <div class="host-context-menu__subtitle">
            <template v-if="menuTargetServer.group">{{ menuTargetServer.group }} · </template>
            {{ menuTargetServer.username }}@{{ menuTargetServer.host }}:{{ menuTargetServer.port }}
          </div>
          <button type="button" class="host-context-menu__item" @click="handleMenuEdit">
            <i class="fas fa-pen-to-square"></i>
            <span>编辑配置</span>
          </button>
          <button type="button" class="host-context-menu__item" @click="handleMenuSetGroup">
            <i class="fas fa-folder"></i>
            <span>设置分组</span>
          </button>
          <div class="host-context-menu__divider"></div>
          <button type="button" class="host-context-menu__item host-context-menu__item--danger" @click="handleMenuDelete">
            <i class="fas fa-trash-can"></i>
            <span>删除主机</span>
          </button>
        </div>
      </Transition>
    </Teleport>

    <div class="sidebar-footer">
      <button class="add-host-btn" @click="emit('openAddModal')">
        <i class="fas fa-plus"></i>
        <span>Add New Host</span>
      </button>
      <p class="sidebar-footer__hint">双击连接 · 按住名称拖动排序或改分组</p>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss' as *;

.sidebar {
  width: 260px;
  flex-shrink: 0;
  background-color: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  height: 100%;
  user-select: none;

  &.is-reordering {
    cursor: grabbing;
  }

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
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 8px 12px 16px;
    @include custom-scrollbar(5px);
  }

  .nav-groups {
    min-height: 0;
  }
}

.host-list {
  display: flex;
  flex-direction: column;
  gap: 6px;

  &--nested {
    margin-top: 4px;
    padding-left: 4px;
  }
}

.host-groups {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.host-group {
  &__header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;

    &:hover {
      background: var(--bg-primary-30);
      color: var(--text-main);
    }

    &.is-collapsed .host-group__chevron {
      transform: rotate(0deg);
    }

    &:not(.is-collapsed) .host-group__chevron {
      transform: rotate(90deg);
    }

    &.is-drop-target {
      background: var(--accent-08);
      color: var(--accent);
      box-shadow: inset 0 0 0 1px var(--accent-20);

      .host-group__folder,
      .host-group__chevron {
        color: var(--accent);
      }
    }
  }

  &__chevron {
    width: 10px;
    font-size: 9px;
    flex-shrink: 0;
    transition: transform 0.2s ease;
    opacity: 0.7;
  }

  &__folder {
    font-size: 10px;
    color: var(--accent-orange);
    opacity: 0.85;
    flex-shrink: 0;
  }

  &__name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  &__count {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    background: var(--bg-input);
    border: 1px solid var(--border-30);
    font-size: 10px;
    font-family: var(--font-terminal);
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
    flex-shrink: 0;
  }
}

.hosts-search {
  margin-bottom: 8px;

  &__wrapper {
    position: relative;
    display: flex;
    align-items: center;

    i.fa-search {
      position: absolute;
      left: 10px;
      font-size: 11px;
      color: var(--text-dim);
      opacity: 0.65;
      pointer-events: none;
    }

    input {
      width: 100%;
      height: 32px;
      padding: 0 28px 0 30px;
      box-sizing: border-box;
      background: var(--bg-input);
      border: 1px solid var(--border-30);
      border-radius: 8px;
      color: var(--text-main);
      font-size: 12px;
      outline: none;
      transition: border-color 0.2s, box-shadow 0.2s;

      &::placeholder {
        color: var(--text-dim);
        opacity: 0.55;
      }

      &:focus {
        border-color: var(--accent-30);
        box-shadow: 0 0 0 2px var(--accent-10);
      }
    }
  }

  &__clear {
    position: absolute;
    right: 4px;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-dim);
    font-size: 10px;
    cursor: pointer;

    &:hover {
      background: var(--accent-10);
      color: var(--accent);
    }
  }
}

.host-item {
  position: relative;
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: 10px;
  padding: 9px 10px 9px 12px;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  cursor: default;
  overflow: hidden;
  transition:
    background 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease,
    opacity 0.2s ease;

  &:hover {
    background: var(--bg-primary-30);
    border-color: var(--border-30);

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
  }

  &.is-dragging {
    opacity: 0.45;
    border-style: dashed;
    border-color: var(--accent-30);
    background: var(--accent-05);
    pointer-events: none;
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
    pointer-events: none;
  }

  &__icon {
    width: 32px;
    height: 32px;
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
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  &__name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-main);
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: grab;
    touch-action: none;

    &:active {
      cursor: grabbing;
    }
  }

  &__meta {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    font-size: 10px;
    line-height: 1.25;
    color: var(--text-dim);
  }

  &__meta-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-terminal);
    letter-spacing: 0.02em;
  }

  &__badge {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 15px;
    height: 15px;
    border-radius: 4px;
    background: var(--accent-orange-10);
    color: var(--accent-orange);
    font-size: 8px;
  }
}

.host-context-menu {
  position: fixed;
  z-index: 10050;
  min-width: 196px;
  padding: 6px;
  border-radius: 10px;
  background: var(--bg-card-95);
  backdrop-filter: blur(16px);
  border: 1px solid var(--border-30);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);

  &__title {
    padding: 8px 12px 2px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-main);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__subtitle {
    padding: 0 12px 8px;
    font-size: 10px;
    color: var(--text-dim);
    font-family: var(--font-terminal);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__divider {
    height: 1px;
    margin: 4px 6px;
    background: var(--border-30);
  }

  &__item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-main);
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;

    i {
      width: 14px;
      font-size: 12px;
      color: var(--text-dim);
      text-align: center;
    }

    &:hover {
      background: var(--accent-10);
      color: var(--accent);

      i { color: var(--accent); }
    }

    &--danger {
      color: var(--error);

      i { color: var(--error); }

      &:hover {
        background: var(--error-10);
        color: var(--error);

        i { color: var(--error); }
      }
    }
  }
}

.host-menu-enter-active,
.host-menu-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.host-menu-enter-from,
.host-menu-leave-to {
  opacity: 0;
  transform: scale(0.96);
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

  &--compact {
    padding: 20px 16px 12px;

    .host-empty__icon {
      width: 36px;
      height: 36px;
      margin-bottom: 10px;
      font-size: 14px;
    }
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