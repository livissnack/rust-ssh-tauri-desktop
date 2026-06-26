<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '../utils/i18n.ts';
import { toast } from '../utils/toast.ts';
import NumberInput from './NumberInput.vue';

export interface PortForwardInfo {
  id: string;
  localHost: string;
  localPort: number;
  remoteHost: string;
  remotePort: number;
}

const props = defineProps<{
  visible: boolean;
  sessionId: string | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const { t } = useI18n();
const forwards = ref<PortForwardInfo[]>([]);
const loading = ref(false);
const localPort = ref(8080);
const remoteHost = ref('127.0.0.1');
const remotePort = ref(3306);

const canSubmit = computed(() =>
  !!props.sessionId && localPort.value >= 1 && remotePort.value >= 1 && remoteHost.value.trim(),
);

const loadForwards = async () => {
  if (!props.sessionId) {
    forwards.value = [];
    return;
  }
  forwards.value = await invoke<PortForwardInfo[]>('list_port_forwards', {
    sessionId: props.sessionId,
  });
};

watch(
  () => [props.visible, props.sessionId] as const,
  async ([visible]) => {
    if (!visible) return;
    await loadForwards();
  },
  { immediate: true },
);

const startForward = async () => {
  if (!props.sessionId || !canSubmit.value) return;
  loading.value = true;
  try {
    await invoke('start_port_forward', {
      sessionId: props.sessionId,
      localHost: '127.0.0.1',
      localPort: localPort.value,
      remoteHost: remoteHost.value.trim(),
      remotePort: remotePort.value,
    });
    toast.success(t('portForward.started'));
    await loadForwards();
  } catch (err) {
    toast.error(t('portForward.startFailed', { err: String(err) }));
  } finally {
    loading.value = false;
  }
};

const stopForward = async (id: string) => {
  if (!props.sessionId) return;
  try {
    await invoke('stop_port_forward', { sessionId: props.sessionId, forwardId: id });
    toast.success(t('portForward.stopped'));
    await loadForwards();
  } catch (err) {
    toast.error(t('portForward.stopFailed', { err: String(err) }));
  }
};
</script>

<template>
  <Transition name="port-forward-fade">
    <div v-if="visible" class="port-forward-overlay" @click.self="emit('close')">
      <div class="port-forward-dialog" role="dialog" :aria-label="t('portForward.title')">
        <header class="port-forward-dialog__header">
          <div>
            <h3>{{ t('portForward.title') }}</h3>
            <p>{{ t('portForward.subtitle') }}</p>
          </div>
          <button type="button" class="port-forward-dialog__close" @click="emit('close')">
            <i class="fas fa-xmark"></i>
          </button>
        </header>

        <section class="port-forward-dialog__form">
          <div class="port-forward-dialog__row">
            <label>{{ t('portForward.localPort') }}</label>
            <NumberInput v-model="localPort" :min="1" :max="65535" />
          </div>
          <div class="port-forward-dialog__row">
            <label>{{ t('portForward.remoteHost') }}</label>
            <input v-model="remoteHost" class="port-forward-dialog__input" type="text" spellcheck="false" />
          </div>
          <div class="port-forward-dialog__row">
            <label>{{ t('portForward.remotePort') }}</label>
            <NumberInput v-model="remotePort" :min="1" :max="65535" />
          </div>
          <button
              type="button"
              class="port-forward-dialog__submit"
              :disabled="!canSubmit || loading"
              @click="startForward"
          >
            <i class="fas fa-play"></i>
            {{ t('portForward.start') }}
          </button>
        </section>

        <section class="port-forward-dialog__list custom-scrollbar">
          <h4>{{ t('portForward.active') }}</h4>
          <ul v-if="forwards.length">
            <li v-for="item in forwards" :key="item.id">
              <div class="port-forward-dialog__item-main">
                <code>127.0.0.1:{{ item.localPort }}</code>
                <i class="fas fa-arrow-right"></i>
                <code>{{ item.remoteHost }}:{{ item.remotePort }}</code>
              </div>
              <button type="button" class="port-forward-dialog__stop" @click="stopForward(item.id)">
                {{ t('portForward.stop') }}
              </button>
            </li>
          </ul>
          <p v-else class="port-forward-dialog__empty">{{ t('portForward.empty') }}</p>
        </section>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.port-forward-overlay {
  position: fixed;
  inset: 0;
  z-index: 4800;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
}

.port-forward-dialog {
  width: min(520px, 100%);
  max-height: min(80vh, 560px);
  display: flex;
  flex-direction: column;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: 0 24px 48px var(--shadow);
  overflow: hidden;

  &__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border-30);

    h3 {
      margin: 0 0 4px;
      font-size: 15px;
      font-weight: 700;
      color: var(--text-main);
    }

    p {
      margin: 0;
      font-size: 12px;
      color: var(--text-dim);
    }
  }

  &__close {
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;

    &:hover {
      background: var(--error-10);
      color: var(--error);
    }
  }

  &__form {
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-30);
    display: grid;
    gap: 10px;
  }

  &__row {
    display: grid;
    grid-template-columns: 110px 1fr;
    align-items: center;
    gap: 10px;

    label {
      font-size: 12px;
      color: var(--text-dim);
    }
  }

  &__input {
    height: 32px;
    padding: 0 10px;
    border-radius: 6px;
    border: 1px solid var(--border-30);
    background: var(--bg-input);
    color: var(--text-main);
    font-size: 13px;
    outline: none;

    &:focus {
      border-color: var(--accent) !important;
      box-shadow: none !important;
    }
  }

  &__submit {
    justify-self: start;
    margin-top: 4px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 14px;
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    cursor: pointer;

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  &__list {
    flex: 1;
    overflow-y: auto;
    padding: 12px 18px 16px;

    h4 {
      margin: 0 0 10px;
      font-size: 11px;
      font-weight: 600;
      color: var(--accent);
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }

    ul {
      list-style: none;
      margin: 0;
      padding: 0;
      display: grid;
      gap: 8px;
    }

    li {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 10px 12px;
      border-radius: 6px;
      border: 1px solid var(--border-30);
      background: var(--bg-input);
    }
  }

  &__item-main {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 12px;
    color: var(--text-main);

    code {
      font-family: var(--font-ui);
      word-break: break-all;
    }

    i {
      font-size: 10px;
      color: var(--text-dim);
      flex-shrink: 0;
    }
  }

  &__stop {
    flex-shrink: 0;
    height: 28px;
    padding: 0 10px;
    border-radius: 5px;
    border: 1px solid var(--border-30);
    background: var(--bg-card);
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;

    &:hover {
      border-color: var(--error-30);
      color: var(--error);
    }
  }

  &__empty {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }
}

.port-forward-fade-enter-active,
.port-forward-fade-leave-active {
  transition: opacity 0.18s ease;
}

.port-forward-fade-enter-from,
.port-forward-fade-leave-to {
  opacity: 0;
}
</style>
