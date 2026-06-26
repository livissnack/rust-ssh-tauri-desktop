<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from '../utils/i18n.ts';

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'find', query: string, direction: 'next' | 'prev'): void;
}>();

const { t } = useI18n();
const query = ref('');
const inputRef = ref<HTMLInputElement | null>(null);

watch(
  () => props.visible,
  async (open) => {
    if (!open) {
      query.value = '';
      return;
    }
    await nextTick();
    inputRef.value?.focus();
    inputRef.value?.select();
  },
);

const find = (direction: 'next' | 'prev') => {
  const q = query.value.trim();
  if (!q) return;
  emit('find', q, direction);
};

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    e.preventDefault();
    emit('close');
    return;
  }
  if (e.key === 'Enter') {
    e.preventDefault();
    find(e.shiftKey ? 'prev' : 'next');
  }
};
</script>

<template>
  <Transition name="term-search-fade">
    <div v-if="visible" class="terminal-search-bar" @mousedown.stop>
      <i class="fas fa-search terminal-search-bar__icon"></i>
      <input
          ref="inputRef"
          v-model="query"
          class="terminal-search-bar__input"
          type="text"
          :placeholder="t('shortcuts.searchPlaceholder')"
          spellcheck="false"
          @keydown="onKeydown"
          @input="query.trim() && find('next')"
      />
      <button type="button" class="terminal-search-bar__btn" :title="t('shortcuts.findPrev')" @click="find('prev')">
        <i class="fas fa-chevron-up"></i>
      </button>
      <button type="button" class="terminal-search-bar__btn" :title="t('shortcuts.findNext')" @click="find('next')">
        <i class="fas fa-chevron-down"></i>
      </button>
      <button type="button" class="terminal-search-bar__btn terminal-search-bar__btn--close" :title="t('shortcuts.closeSearch')" @click="emit('close')">
        <i class="fas fa-xmark"></i>
      </button>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.terminal-search-bar {
  position: absolute;
  top: 10px;
  right: 12px;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 6px 4px 10px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: 0 8px 24px var(--shadow);

  &__icon {
    font-size: 11px;
    color: var(--text-dim);
    flex-shrink: 0;
  }

  &__input {
    width: 200px;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--border-30);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--text-main);
    font-size: 12px;
    outline: none;

    &:focus {
      border-color: var(--accent);
      box-shadow: 0 0 0 2px var(--accent-15);
    }
  }

  &__btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;

    i { font-size: 11px; }

    &:hover {
      background: var(--accent-10);
      color: var(--accent);
    }

    &--close:hover {
      background: var(--error-10);
      color: var(--error);
    }
  }
}

.term-search-fade-enter-active,
.term-search-fade-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.term-search-fade-enter-from,
.term-search-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
