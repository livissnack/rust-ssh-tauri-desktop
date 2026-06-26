<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from '../utils/i18n.ts';
import { type HostKeyPrompt, respondToHostKeyPrompt } from '../utils/hostKey.ts';

const props = defineProps<{
  prompt: HostKeyPrompt | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const { t } = useI18n();

const visible = computed(() => props.prompt !== null);
const isChanged = computed(() => props.prompt?.kind === 'changed');
const isJump = computed(() => props.prompt?.hostRole === 'jump');
const isTarget = computed(() => props.prompt?.hostRole === 'target');

const roleLabel = computed(() => {
  if (isJump.value) return t('hostKey.roleJump');
  if (isTarget.value) return t('hostKey.roleTarget');
  return t('hostKey.roleDirect');
});

const titleText = computed(() => {
  if (!props.prompt) return '';
  if (isChanged.value) {
    if (isJump.value) return t('hostKey.jumpChangedTitle');
    if (isTarget.value) return t('hostKey.targetChangedTitle');
    return t('hostKey.changedTitle');
  }
  if (isJump.value) return t('hostKey.jumpNewTitle');
  if (isTarget.value) return t('hostKey.targetNewTitle');
  return t('hostKey.newTitle');
});

const hintText = computed(() => {
  if (!props.prompt) return '';
  if (isChanged.value) {
    if (isJump.value) return t('hostKey.jumpChangedHint');
    if (isTarget.value) return t('hostKey.targetChangedHint');
    return t('hostKey.changedHint');
  }
  if (isJump.value) return t('hostKey.jumpNewHint');
  if (isTarget.value) return t('hostKey.targetNewHint');
  return t('hostKey.newHint');
});

const respond = async (trust: boolean) => {
  if (!props.prompt) return;
  const requestId = props.prompt.requestId;
  try {
    await respondToHostKeyPrompt(requestId, trust);
  } catch (err) {
    console.error('Host key prompt response failed:', err);
  } finally {
    emit('close');
  }
};
</script>

<template>
  <Transition name="host-key-fade">
    <div v-if="visible && prompt" class="host-key-overlay" role="presentation">
      <div class="host-key-dialog" role="alertdialog" :aria-label="t('hostKey.title')">
        <header class="host-key-dialog__header">
          <div class="host-key-dialog__icon" :class="{ 'host-key-dialog__icon--warn': isChanged }">
            <i :class="isChanged ? 'fas fa-triangle-exclamation' : 'fas fa-shield-halved'"></i>
          </div>
          <div>
            <div class="host-key-dialog__title-row">
              <h3>{{ titleText }}</h3>
              <span class="host-key-dialog__role-badge">{{ roleLabel }}</span>
            </div>
            <p>{{ hintText }}</p>
          </div>
        </header>

        <section class="host-key-dialog__body">
          <dl class="host-key-dialog__meta">
            <div>
              <dt>{{ t('hostKey.server') }}</dt>
              <dd>{{ prompt.serverName || prompt.host }}</dd>
            </div>
            <div>
              <dt>{{ t('hostKey.address') }}</dt>
              <dd><code>{{ prompt.host }}:{{ prompt.port }}</code></dd>
            </div>
            <div>
              <dt>{{ t('hostKey.keyType') }}</dt>
              <dd><code>{{ prompt.keyType }}</code></dd>
            </div>
          </dl>

          <div v-if="isChanged && prompt.kind === 'changed'" class="host-key-dialog__fp host-key-dialog__fp--old">
            <span class="host-key-dialog__fp-label">{{ t('hostKey.oldFingerprint') }}</span>
            <code>{{ prompt.oldFingerprint }}</code>
          </div>

          <div class="host-key-dialog__fp" :class="{ 'host-key-dialog__fp--warn': isChanged }">
            <span class="host-key-dialog__fp-label">
              {{ isChanged ? t('hostKey.newFingerprint') : t('hostKey.fingerprint') }}
            </span>
            <code>{{ prompt.fingerprint }}</code>
          </div>
        </section>

        <footer class="host-key-dialog__actions">
          <button type="button" class="host-key-dialog__reject" @click="respond(false)">
            {{ t('hostKey.reject') }}
          </button>
          <button
              type="button"
              class="host-key-dialog__trust"
              :class="{ 'host-key-dialog__trust--warn': isChanged }"
              @click="respond(true)"
          >
            {{ isChanged ? t('hostKey.trustChanged') : t('hostKey.trust') }}
          </button>
        </footer>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.host-key-overlay {
  position: fixed;
  inset: 0;
  z-index: 5000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(8px);
}

.host-key-dialog {
  width: min(560px, 100%);
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: 0 24px 48px var(--shadow);
  overflow: hidden;

  &__header {
    display: flex;
    gap: 14px;
    padding: 18px 20px 12px;
    border-bottom: 1px solid var(--border-30);

    h3 {
      margin: 0;
      font-size: 1.05rem;
    }

    p {
      margin: 6px 0 0;
      font-size: 0.875rem;
      color: var(--text-muted);
      line-height: 1.5;
    }
  }

  &__title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  &__role-badge {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent);
  }

  &__icon {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);

    &--warn {
      background: color-mix(in srgb, #f59e0b 20%, transparent);
      color: #f59e0b;
    }
  }

  &__body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  &__meta {
    margin: 0;
    display: grid;
    gap: 8px;

    div {
      display: grid;
      grid-template-columns: 88px 1fr;
      gap: 8px;
      align-items: baseline;
    }

    dt {
      margin: 0;
      font-size: 0.8rem;
      color: var(--text-muted);
    }

    dd {
      margin: 0;
      font-size: 0.875rem;
      word-break: break-word;
    }

    code {
      font-family: var(--font-mono, ui-monospace, monospace);
      font-size: 0.82rem;
    }
  }

  &__fp {
    padding: 12px;
    border-radius: 8px;
    border: 1px solid var(--border-30);
    background: var(--bg-subtle, rgba(255, 255, 255, 0.03));

    &--old {
      opacity: 0.85;
    }

    &--warn {
      border-color: color-mix(in srgb, #f59e0b 40%, var(--border));
      background: color-mix(in srgb, #f59e0b 8%, transparent);
    }

    &-label {
      display: block;
      margin-bottom: 6px;
      font-size: 0.75rem;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }

    code {
      display: block;
      font-family: var(--font-mono, ui-monospace, monospace);
      font-size: 0.82rem;
      line-height: 1.45;
      word-break: break-all;
    }
  }

  &__actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px 18px;
    border-top: 1px solid var(--border-30);
  }

  &__reject,
  &__trust {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.875rem;
    cursor: pointer;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);

    &:hover {
      background: var(--bg-hover);
    }
  }

  &__trust {
    border-color: var(--accent);
    background: var(--accent);
    color: #fff;

    &:hover {
      filter: brightness(1.05);
    }

    &--warn {
      border-color: #f59e0b;
      background: #f59e0b;
    }
  }
}

.host-key-fade-enter-active,
.host-key-fade-leave-active {
  transition: opacity 0.15s ease;
}

.host-key-fade-enter-from,
.host-key-fade-leave-to {
  opacity: 0;
}
</style>
