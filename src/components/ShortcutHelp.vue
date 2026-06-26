<script setup lang="ts">
import { useI18n } from '../utils/i18n.ts';
import { SHORTCUT_GROUPS, formatShortcutKeys, shortcutLabelParams, platformModKeyName } from '../utils/shortcuts.ts';

defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const { t } = useI18n();
</script>

<template>
  <Transition name="shortcut-help-fade">
    <div v-if="visible" class="shortcut-help-overlay" @click.self="emit('close')">
      <div class="shortcut-help" role="dialog" :aria-label="t('shortcuts.title')">
        <header class="shortcut-help__header">
          <div>
            <h3>{{ t('shortcuts.title') }}</h3>
              <p>{{ t('shortcuts.subtitle', { mod: platformModKeyName() }) }}</p>
          </div>
          <button type="button" class="shortcut-help__close" @click="emit('close')">
            <i class="fas fa-xmark"></i>
          </button>
        </header>
        <div class="shortcut-help__body custom-scrollbar">
          <section v-for="group in SHORTCUT_GROUPS" :key="group.titleKey" class="shortcut-help__group">
            <h4>{{ t(group.titleKey) }}</h4>
            <ul>
              <li v-for="item in group.items" :key="item.action">
                <span class="shortcut-help__label">
                  {{ t(item.labelKey, shortcutLabelParams(item.action) ?? {}) }}
                </span>
                <kbd v-if="item.keys">{{ formatShortcutKeys(item.keys) }}</kbd>
                <span v-else class="shortcut-help__palette-only">{{ t('shortcuts.paletteOnly') }}</span>
              </li>
            </ul>
          </section>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.shortcut-help-overlay {
  position: fixed;
  inset: 0;
  z-index: 5000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
}

.shortcut-help {
  width: min(560px, 100%);
  max-height: min(80vh, 640px);
  display: flex;
  flex-direction: column;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: 0 24px 48px var(--shadow);
  overflow: hidden;

  &__header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px 14px;
    border-bottom: 1px solid var(--border-30);

    h3 {
      margin: 0 0 4px;
      font-size: 16px;
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
    width: 32px;
    height: 32px;
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

  &__body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 20px 20px;
  }

  &__group {
    margin-top: 16px;

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
    }

    li {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 8px 0;
      border-bottom: 1px solid var(--border-30);

      &:last-child {
        border-bottom: none;
      }
    }

    kbd {
      flex-shrink: 0;
      font-size: 10px;
      color: var(--text-dim);
      padding: 4px 8px;
      border-radius: 6px;
      border: 1px solid var(--border-30);
      background: var(--bg-input);
      font-family: var(--font-ui);
    }
  }

  &__label {
    font-size: 13px;
    color: var(--text-main);
  }

  &__palette-only {
    flex-shrink: 0;
    font-size: 10px;
    color: var(--text-dim);
    padding: 2px 7px;
    border-radius: 5px;
    border: 1px solid var(--border-30);
    background: var(--bg-input);
  }
}

.shortcut-help-fade-enter-active,
.shortcut-help-fade-leave-active {
  transition: opacity 0.18s ease;
}

.shortcut-help-fade-enter-from,
.shortcut-help-fade-leave-to {
  opacity: 0;
}
</style>
