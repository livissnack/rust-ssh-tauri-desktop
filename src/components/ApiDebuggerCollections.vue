<script setup lang="ts">
import { ref } from 'vue';
import { toast } from '../utils/toast.ts';
import { confirm } from '../utils/confirm.ts';
import ApiDebuggerInputDialog from './ApiDebuggerInputDialog.vue';
import type { ApiCollection, RequestSnapshot, SavedRequest } from '../utils/apiDebuggerStorage.ts';
import {
  createId,
  getSnapshotProtocol,
  snapshotTagLabel,
} from '../utils/apiDebuggerStorage.ts';

const props = defineProps<{
  collections: ApiCollection[];
  currentSnapshot: RequestSnapshot;
}>();

const emit = defineEmits<{
  update: [collections: ApiCollection[]];
  load: [snapshot: RequestSnapshot];
  'open-save': [collectionId?: string];
  'open-edit': [payload: { collectionId: string; request: SavedRequest }];
}>();

const expandedIds = ref<Set<string>>(new Set());
const showCreateDialog = ref(false);

const toggleExpand = (id: string) => {
  const next = new Set(expandedIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expandedIds.value = next;
};

const methodClass = (snapshot: RequestSnapshot) => {
  const protocol = getSnapshotProtocol(snapshot);
  if (protocol !== 'http') return `protocol-${protocol}`;
  const key = snapshot.method.toUpperCase();
  if (key === 'GET') return 'method-get';
  if (key === 'POST') return 'method-post';
  if (key === 'PUT') return 'method-put';
  if (key === 'PATCH') return 'method-patch';
  if (key === 'DELETE') return 'method-delete';
  if (key === 'HEAD') return 'method-head';
  if (key === 'OPTIONS') return 'method-options';
  return 'method-other';
};

const tagLabel = (snapshot: RequestSnapshot) => snapshotTagLabel(snapshot);

const addCollection = () => {
  showCreateDialog.value = true;
};

const confirmCreateCollection = (name: string) => {
  const collection: ApiCollection = {
    id: createId(),
    name,
    requests: [],
    updatedAt: Date.now(),
  };
  emit('update', [...props.collections, collection]);
  showCreateDialog.value = false;
  expandedIds.value = new Set([...expandedIds.value, collection.id]);
  toast.success('集合已创建');
};

const deleteCollection = async (collection: ApiCollection) => {
  const ok = await confirm(`删除集合「${collection.name}」？`, 'warning', '删除集合');
  if (!ok) return;
  emit('update', props.collections.filter((item) => item.id !== collection.id));
  toast.success('集合已删除');
};

const openSaveForm = (collectionId?: string) => {
  if (props.collections.length === 0) {
    toast.warning('请先创建集合');
    return;
  }
  emit('open-save', collectionId);
};

const deleteRequest = async (collectionId: string, requestId: string) => {
  const ok = await confirm('删除此请求？', 'warning', '删除请求');
  if (!ok) return;
  const next = props.collections.map((collection) => {
    if (collection.id !== collectionId) return collection;
    return {
      ...collection,
      updatedAt: Date.now(),
      requests: collection.requests.filter((item) => item.id !== requestId),
    };
  });
  emit('update', next);
};

const loadRequest = (snapshot: RequestSnapshot) => {
  emit('load', snapshot);
};

const editRequest = (collectionId: string, request: SavedRequest) => {
  emit('open-edit', { collectionId, request });
};
</script>

<template>
  <section class="manager-view collections-view">
    <div class="collections-toolbar">
      <span v-if="collections.length" class="collections-meta">
        {{ collections.length }} 个集合 ·
        {{ collections.reduce((n, c) => n + c.requests.length, 0) }} 个请求
      </span>
      <span v-else class="collections-meta">组织并复用 API 请求</span>
      <div class="collections-toolbar__actions">
        <button type="button" class="btn-ghost" @click="addCollection">
          <i class="fas fa-folder-plus"></i>
          新建集合
        </button>
        <button type="button" class="btn-primary" @click="openSaveForm()">
          <i class="fas fa-save"></i>
          保存当前
        </button>
      </div>
    </div>

    <p v-if="collections.length" class="hint-box">
      点击请求条目加载到 Request 面板；支持 HTTP / WS / SSE / Socket.IO / MQTT。
    </p>

    <div v-if="!collections.length" class="empty-state">
      <div class="empty-state__icon">
        <i class="fas fa-folder-open"></i>
      </div>
      <p class="empty-state__title">暂无集合</p>
      <p class="empty-state__desc">创建集合后，可将当前 HTTP 请求保存并按项目分组管理</p>
      <button type="button" class="btn-primary empty-state__action" @click="addCollection">
        <i class="fas fa-folder-plus"></i>
        新建集合
      </button>
    </div>

    <div v-else class="item-list">
      <div
        v-for="collection in collections"
        :key="collection.id"
        class="group-card"
        :class="{ 'is-expanded': expandedIds.has(collection.id) }"
      >
        <div class="group-head">
          <button
            type="button"
            class="group-expand-btn"
            :aria-expanded="expandedIds.has(collection.id)"
            @click="toggleExpand(collection.id)"
          >
            <i class="fas" :class="expandedIds.has(collection.id) ? 'fa-chevron-down' : 'fa-chevron-right'"></i>
          </button>

          <div class="group-head-main" @click="toggleExpand(collection.id)">
            <i class="fas fa-folder group-folder"></i>
            <span class="group-name">{{ collection.name }}</span>
            <span class="group-count">{{ collection.requests.length }}</span>
          </div>

          <div class="group-head-actions">
            <Tooltip text="保存当前请求到此集合" placement="top">
              <button
                type="button"
                class="icon-btn icon-btn--accent"
                @click.stop="openSaveForm(collection.id)"
              >
                <i class="fas fa-plus"></i>
              </button>
            </Tooltip>
            <Tooltip text="删除集合" placement="top">
              <button type="button" class="icon-btn" @click.stop="deleteCollection(collection)">
                <i class="fas fa-trash-alt"></i>
              </button>
            </Tooltip>
          </div>
        </div>

        <ul v-if="expandedIds.has(collection.id)" class="sub-list">
          <li
            v-for="request in collection.requests"
            :key="request.id"
            class="request-item"
            @click="loadRequest(request.snapshot)"
          >
            <div class="request-item__main">
              <div class="request-item__title-row">
                <span class="method-tag" :class="methodClass(request.snapshot)">
                  {{ tagLabel(request.snapshot) }}
                </span>
                <span class="request-name">{{ request.name }}</span>
              </div>
              <p v-if="request.description" class="request-desc">{{ request.description }}</p>
              <p class="request-url">{{ request.snapshot.url || '（未设置 URL）' }}</p>
            </div>

            <div class="request-item__actions">
              <Tooltip text="编辑名称和备注" placement="top">
                <button
                  type="button"
                  class="icon-btn icon-btn--muted"
                  @click.stop="editRequest(collection.id, request)"
                >
                  <i class="fas fa-pen"></i>
                </button>
              </Tooltip>
              <Tooltip text="删除请求" placement="top">
                <button
                  type="button"
                  class="icon-btn"
                  @click.stop="deleteRequest(collection.id, request.id)"
                >
                  <i class="fas fa-times"></i>
                </button>
              </Tooltip>
            </div>
          </li>

          <li v-if="!collection.requests.length" class="request-empty">
            <i class="fas fa-inbox"></i>
            <span>暂无请求</span>
            <button type="button" class="request-empty__link" @click.stop="openSaveForm(collection.id)">
              保存当前请求
            </button>
          </li>
        </ul>
      </div>
    </div>
  </section>

  <ApiDebuggerInputDialog
    :visible="showCreateDialog"
    title="新建集合"
    label="集合名称"
    placeholder="例如 用户中心 API"
    icon="fa-folder-plus"
    initial-value="New Collection"
    @close="showCreateDialog = false"
    @confirm="confirmCreateCollection"
  />
</template>

<style scoped lang="scss">
@use './api-debugger-manager.scss';

.collections-view {
  gap: 12px;
}

.collections-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  flex-wrap: wrap;

  &__actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
}

.collections-meta {
  flex: 1;
  min-width: 0;
  font-size: 11px;
  color: var(--text-dim);
  line-height: 1.4;
}

.empty-state {
  padding: 40px 16px;
  gap: 10px;

  &__icon {
    width: 52px;
    height: 52px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-08);
    border: 1px solid var(--accent-15);

    i {
      font-size: 22px;
      color: var(--accent);
      opacity: 0.85;
    }
  }

  &__title {
    margin: 4px 0 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-main);
  }

  &__desc {
    margin: 0;
    max-width: 240px;
    font-size: 11px;
    line-height: 1.55;
    color: var(--text-dim);
  }

  &__action {
    margin-top: 6px;
  }
}

.group-card {
  transition: border-color 0.15s, box-shadow 0.15s;

  &.is-expanded {
    border-color: var(--accent-20);
    box-shadow: 0 1px 0 var(--accent-08);
  }
}

.group-head {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px 8px 6px 6px;
  cursor: default;
  user-select: none;

  &:hover {
    background: var(--bg-input);
  }
}

.group-expand-btn {
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;

  i {
    font-size: 9px;
  }

  &:hover {
    background: var(--accent-08);
    color: var(--accent);
  }
}

.group-head-main {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 6px;
}

.group-folder {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--accent);
  opacity: 0.85;
}

.group-name {
  font-size: 12px;
  font-weight: 600;
}

.group-count {
  flex-shrink: 0;
  min-width: 20px;
  padding: 1px 7px;
  font-size: 10px;
  font-weight: 600;
  text-align: center;
}

.group-head-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.sub-list {
  padding: 4px 8px 8px;
  background: var(--bg-primary);
}

.request-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 10px 10px 12px;
  margin-bottom: 4px;
  border-radius: 8px;
  border: 1px solid var(--border-30);
  background: var(--bg-secondary);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;

  &:last-child {
    margin-bottom: 0;
  }

  &:hover {
    border-color: var(--accent-20);
    background: var(--accent-08);
  }

  &__main {
    flex: 1;
    min-width: 0;
  }

  &__title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 3px;
  }

  &__actions {
    display: flex;
    flex-shrink: 0;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
  }

  &:hover &__actions {
    opacity: 1;
  }
}

.request-name {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-main);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.request-desc {
  margin: 0 0 5px;
  padding-left: 1px;
  font-size: 11px;
  line-height: 1.45;
  color: var(--text-dim);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.request-url {
  margin: 0;
  padding: 4px 8px;
  border-radius: 5px;
  background: var(--bg-input);
  border: 1px solid var(--border-30);
  font-family: var(--font-terminal);
  font-size: 10px;
  line-height: 1.45;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.request-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 20px 12px;
  border-radius: 8px;
  border: 1px dashed var(--border-30);
  background: var(--bg-secondary);
  font-size: 11px;
  color: var(--text-dim);
  list-style: none;

  i {
    font-size: 16px;
    opacity: 0.4;
  }

  &__link {
    border: none;
    background: none;
    padding: 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    cursor: pointer;

    &:hover {
      text-decoration: underline;
    }
  }
}

.method-tag {
  min-width: 44px;
  padding: 2px 6px;
  font-size: 9px;
  letter-spacing: 0.04em;

  &.method-get {
    background: color-mix(in srgb, var(--success) 18%, transparent);
    color: var(--success);
  }

  &.method-post {
    background: color-mix(in srgb, #f59e0b 18%, transparent);
    color: #d97706;
  }

  &.method-put {
    background: color-mix(in srgb, #3b82f6 18%, transparent);
    color: #2563eb;
  }

  &.method-patch {
    background: color-mix(in srgb, #8b5cf6 18%, transparent);
    color: #7c3aed;
  }

  &.method-delete {
    background: var(--error-15);
    color: var(--error);
  }

  &.method-head,
  &.method-options {
    background: var(--bg-input);
    color: var(--text-dim);
  }

  &.method-other {
    background: var(--accent-10);
    color: var(--accent);
  }

  &.protocol-ws {
    background: color-mix(in srgb, #06b6d4 18%, transparent);
    color: #0891b2;
  }

  &.protocol-sse {
    background: color-mix(in srgb, #8b5cf6 18%, transparent);
    color: #7c3aed;
  }

  &.protocol-socketio {
    background: color-mix(in srgb, #f59e0b 18%, transparent);
    color: #d97706;
  }

  &.protocol-mqtt {
    background: color-mix(in srgb, #10b981 18%, transparent);
    color: #059669;
  }
}

.icon-btn--accent:hover {
  color: var(--accent) !important;
  background: var(--accent-08) !important;
}

.icon-btn--muted:hover {
  color: var(--text-main) !important;
  background: var(--bg-input) !important;
}

.btn-ghost,
.btn-primary {
  height: 32px;
}
</style>
