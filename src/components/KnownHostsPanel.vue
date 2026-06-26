<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '../utils/i18n.ts';
import { confirm } from '../utils/confirm.ts';
import { toast } from '../utils/toast.ts';

export type KnownHostRecord = {
  host: string;
  port: number;
  fingerprint: string;
  keyType: string;
  publicKey: string;
  trustedAt: number;
};

const { t } = useI18n();
const hosts = ref<KnownHostRecord[]>([]);
const loading = ref(false);

const formatTrustedAt = (ts: number) => {
  if (!ts) return '—';
  return new Date(ts * 1000).toLocaleString();
};

const loadHosts = async () => {
  loading.value = true;
  try {
    hosts.value = await invoke<KnownHostRecord[]>('list_known_hosts');
  } catch (err) {
    toast.error(t('knownHosts.loadFailed', { err: String(err) }));
  } finally {
    loading.value = false;
  }
};

const removeHost = async (record: KnownHostRecord) => {
  const ok = await confirm(
    t('knownHosts.removeConfirm', { host: record.host, port: record.port }),
    'warning',
    t('knownHosts.removeTitle'),
  );
  if (!ok) return;

  try {
    await invoke('remove_known_host', { host: record.host, port: record.port });
    hosts.value = hosts.value.filter(
      (h) => !(h.host === record.host && h.port === record.port),
    );
    toast.success(t('knownHosts.removed'));
  } catch (err) {
    toast.error(t('knownHosts.removeFailed', { err: String(err) }));
  }
};

onMounted(loadHosts);
</script>

<template>
  <section class="known-hosts">
    <h3 class="section-title">
      <i class="fas fa-shield-halved"></i>
      {{ t('knownHosts.title') }}
    </h3>
    <p class="section-hint">{{ t('knownHosts.hint') }}</p>

    <div v-if="loading" class="known-hosts__loading">{{ t('common.loading') }}</div>
    <div v-else-if="!hosts.length" class="known-hosts__empty">{{ t('knownHosts.empty') }}</div>
    <ul v-else class="known-hosts__list custom-scrollbar">
      <li v-for="item in hosts" :key="`${item.host}:${item.port}`" class="known-hosts__item">
        <div class="known-hosts__main">
          <div class="known-hosts__head">
            <code>{{ item.host }}:{{ item.port }}</code>
            <span class="known-hosts__type">{{ item.keyType }}</span>
          </div>
          <code class="known-hosts__fp">{{ item.fingerprint }}</code>
          <span class="known-hosts__time">{{ t('knownHosts.trustedAt', { time: formatTrustedAt(item.trustedAt) }) }}</span>
        </div>
        <button type="button" class="known-hosts__remove" @click="removeHost(item)">
          {{ t('knownHosts.remove') }}
        </button>
      </li>
    </ul>
  </section>
</template>

<style lang="scss" scoped>
.known-hosts {
  .section-title {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-main);

    i {
      color: var(--accent);
    }
  }

  .section-hint {
    margin: 0 0 12px;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  &__loading,
  &__empty {
    padding: 12px;
    font-size: 13px;
    color: var(--text-muted);
    border: 1px dashed var(--border-30);
    border-radius: 8px;
    text-align: center;
  }

  &__list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 280px;
    overflow-y: auto;
  }

  &__item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border-30);
    border-radius: 8px;
    background: var(--bg-card);
  }

  &__main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  &__head {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;

    code {
      font-size: 0.82rem;
    }
  }

  &__type {
    font-size: 0.72rem;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--text-muted);
    border: 1px solid var(--border-30);
  }

  &__fp {
    display: block;
    font-size: 0.75rem;
    word-break: break-all;
    color: var(--text-muted);
  }

  &__time {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  &__remove {
    flex-shrink: 0;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;

    &:hover {
      color: #ef4444;
      border-color: color-mix(in srgb, #ef4444 40%, var(--border));
    }
  }
}
</style>
