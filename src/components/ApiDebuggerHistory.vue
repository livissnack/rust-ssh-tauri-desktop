<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { confirm } from '../utils/confirm.ts';
import { toast } from '../utils/toast.ts';
import { t } from '../utils/i18n.ts';
import type { HistoryEntry, HttpRequestSnapshot } from '../utils/apiDebuggerStorage.ts';
import { formatHistoryTime } from '../utils/apiDebuggerStorage.ts';

const HISTORY_PAGE_SIZE = 40;
const HISTORY_MAX = 100;

const props = defineProps<{
  history: HistoryEntry[];
}>();

const emit = defineEmits<{
  update: [history: HistoryEntry[]];
  load: [snapshot: HttpRequestSnapshot];
}>();

const visibleCount = ref(HISTORY_PAGE_SIZE);

watch(
  () => props.history.length,
  (len, prev) => {
    if (len < prev) {
      visibleCount.value = HISTORY_PAGE_SIZE;
    }
  },
);

const visibleHistory = computed(() => props.history.slice(0, visibleCount.value));
const hasMore = computed(() => props.history.length > visibleCount.value);
const remainingCount = computed(() => props.history.length - visibleCount.value);

const successCount = computed(() =>
  props.history.filter((entry) => entry.status !== undefined && entry.status < 400).length,
);

const errorCount = computed(() =>
  props.history.filter((entry) => entry.status !== undefined && entry.status >= 400).length,
);

const methodClass = (method: string) => {
  const key = method.toUpperCase();
  if (key === 'GET') return 'method-get';
  if (key === 'POST') return 'method-post';
  if (key === 'PUT') return 'method-put';
  if (key === 'PATCH') return 'method-patch';
  if (key === 'DELETE') return 'method-delete';
  if (key === 'HEAD') return 'method-head';
  if (key === 'OPTIONS') return 'method-options';
  return 'method-other';
};

const statusLabel = (entry: HistoryEntry) => {
  if (entry.status === undefined) return t('apiDebugger.history.failed');
  return String(entry.status);
};

const statusClass = (entry: HistoryEntry) => {
  if (entry.status === undefined) return 'pending';
  return entry.status < 400 ? 'ok' : 'err';
};

const loadMore = () => {
  visibleCount.value = Math.min(visibleCount.value + HISTORY_PAGE_SIZE, props.history.length);
};

const clearHistory = async () => {
  if (!props.history.length) return;
  const ok = await confirm(
    t('apiDebugger.history.clearConfirm'),
    'warning',
    t('apiDebugger.history.clearTitle'),
  );
  if (!ok) return;
  emit('update', []);
  visibleCount.value = HISTORY_PAGE_SIZE;
  toast.success(t('apiDebugger.history.cleared'));
};

const deleteEntry = (id: string) => {
  emit('update', props.history.filter((item) => item.id !== id));
};

const loadEntry = (entry: HistoryEntry) => {
  emit('load', entry.snapshot);
};
</script>

<template>
  <section class="manager-view history-view">
    <div class="history-toolbar">
      <div class="history-toolbar__meta">
        <span v-if="history.length" class="history-count">
          {{ t('apiDebugger.history.recordCount', { count: history.length }) }}
          <template v-if="successCount || errorCount">
            · <span class="history-stat history-stat--ok">{{ t('apiDebugger.history.successCount', { count: successCount }) }}</span>
            <template v-if="errorCount"> · <span class="history-stat history-stat--err">{{ t('apiDebugger.history.errorCount', { count: errorCount }) }}</span></template>
          </template>
        </span>
        <span v-else class="history-count">{{ t('apiDebugger.history.emptyMeta') }}</span>
        <span v-if="history.length >= HISTORY_MAX" class="history-limit">{{ t('apiDebugger.history.limitReached', { max: HISTORY_MAX }) }}</span>
      </div>
      <button type="button" class="btn-ghost" :disabled="!history.length" @click="clearHistory">
        <i class="fas fa-trash-alt"></i>
        {{ t('apiDebugger.history.clear') }}
      </button>
    </div>

    <p v-if="history.length" class="hint-box">
      {{ t('apiDebugger.history.hint', { max: HISTORY_MAX }) }}
    </p>

    <div v-if="!history.length" class="empty-state">
      <div class="empty-state__icon">
        <i class="fas fa-history"></i>
      </div>
      <p class="empty-state__title">{{ t('apiDebugger.history.emptyTitle') }}</p>
      <p class="empty-state__desc">{{ t('apiDebugger.history.emptyDesc') }}</p>
    </div>

    <template v-else>
      <ul class="history-list">
        <li
          v-for="entry in visibleHistory"
          :key="entry.id"
          class="history-item"
          @click="loadEntry(entry)"
        >
          <div class="history-item__main">
            <div class="history-item__top">
              <span class="history-time">{{ formatHistoryTime(entry.timestamp) }}</span>
              <span v-if="entry.elapsedMs !== undefined" class="history-elapsed">{{ entry.elapsedMs }}ms</span>
              <span class="status-tag" :class="statusClass(entry)">{{ statusLabel(entry) }}</span>
            </div>
            <div class="history-item__body">
              <span class="method-tag" :class="methodClass(entry.snapshot.method)">
                {{ entry.snapshot.method }}
              </span>
              <p class="history-url">{{ entry.snapshot.url || t('apiDebugger.history.noUrl') }}</p>
            </div>
          </div>

          <div class="history-item__actions">
            <Tooltip :text="t('apiDebugger.history.deleteEntry')" placement="top">
              <button type="button" class="icon-btn" @click.stop="deleteEntry(entry.id)">
                <i class="fas fa-times"></i>
              </button>
            </Tooltip>
          </div>
        </li>
      </ul>

      <div v-if="hasMore" class="history-load-more">
        <button type="button" class="btn-ghost history-load-more__btn" @click="loadMore">
          <i class="fas fa-chevron-down"></i>
          {{ t('apiDebugger.history.loadMore', { count: remainingCount }) }}
        </button>
      </div>
    </template>
  </section>
</template>

<style scoped lang="scss">
@use './api-debugger-manager.scss';

.history-view {
  gap: 12px;
}

.history-toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  position: sticky;
  top: -12px;
  z-index: 3;
  margin: -12px -12px 0;
  padding: 12px 12px 8px;
  background: linear-gradient(to bottom, var(--bg-primary) 75%, transparent);

  &__meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
}

.history-count {
  font-size: 11px;
  color: var(--text-dim);
  line-height: 1.45;
}

.history-stat {
  &--ok {
    color: var(--success);
  }

  &--err {
    color: var(--error);
  }
}

.history-limit {
  font-size: 10px;
  color: var(--warning, #d97706);
  font-weight: 600;
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
    max-width: 260px;
    font-size: 11px;
    line-height: 1.55;
    color: var(--text-dim);
  }
}

.history-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.history-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 10px 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--border-30);
  background: var(--bg-secondary);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;

  &:hover {
    border-color: var(--accent-20);
    background: var(--accent-08);
  }

  &__main {
    flex: 1;
    min-width: 0;
  }

  &__top {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }

  &__body {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
  }

  &__actions {
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s;
  }

  &:hover &__actions {
    opacity: 1;
  }
}

.history-time {
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 600;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.history-elapsed {
  flex-shrink: 0;
  font-size: 10px;
  color: var(--text-dim);
  font-family: var(--font-terminal);
  opacity: 0.85;
}

.history-url {
  flex: 1;
  min-width: 0;
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

.history-load-more {
  display: flex;
  justify-content: center;
  padding: 4px 0 8px;

  &__btn {
    width: 100%;
    max-width: 320px;
    height: 32px;
  }
}

.method-tag {
  flex-shrink: 0;
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
}

.status-tag {
  flex-shrink: 0;
  min-width: 36px;
  padding: 2px 7px;
  font-size: 9px;
  font-weight: 700;
  text-align: center;
}
</style>
