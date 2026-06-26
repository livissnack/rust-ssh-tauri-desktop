<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';
import { useI18n } from '../utils/i18n.ts';
import { formatShortcutKeys } from '../utils/shortcuts.ts';
import type { ShortcutAction } from '../utils/shortcuts.ts';

export interface CommandPaletteItem {
  id: string;
  label: string;
  hint?: string;
  shortcut?: string;
  action: ShortcutAction | string;
  disabled?: boolean;
}

const props = defineProps<{
  visible: boolean;
  items: CommandPaletteItem[];
  recentActionIds?: string[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'run', action: string): void;
}>();

const { t } = useI18n();
const query = ref('');
const activeIndex = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);
const listRef = ref<HTMLElement | null>(null);

const filteredItems = computed(() => {
  const q = query.value.trim().toLowerCase();
  const base = props.items;
  if (!q) return base;
  return base.filter((i) => i.label.toLowerCase().includes(q) || i.hint?.toLowerCase().includes(q));
});

const recentItems = computed(() => {
  if (query.value.trim()) return [] as CommandPaletteItem[];
  const ids = props.recentActionIds ?? [];
  if (!ids.length) return [] as CommandPaletteItem[];
  const map = new Map(props.items.map((i) => [String(i.action), i]));
  return ids.map((id) => map.get(id)).filter(Boolean) as CommandPaletteItem[];
});

const otherItems = computed(() => {
  if (query.value.trim()) return filteredItems.value;
  const recentSet = new Set((props.recentActionIds ?? []).map(String));
  return props.items.filter((i) => !recentSet.has(String(i.action)));
});

const displaySections = computed(() => {
  if (query.value.trim()) {
    return [{ key: 'search', title: '', items: filteredItems.value }];
  }
  const sections: { key: string; title: string; items: CommandPaletteItem[] }[] = [];
  if (recentItems.value.length) {
    sections.push({ key: 'recent', title: t('shortcuts.paletteRecent'), items: recentItems.value });
  }
  sections.push({
    key: 'all',
    title: recentItems.value.length ? t('shortcuts.paletteAll') : '',
    items: otherItems.value,
  });
  return sections;
});

const flatDisplayItems = computed(() => displaySections.value.flatMap((s) => s.items));

watch(
  () => props.visible,
  async (open) => {
    if (!open) {
      query.value = '';
      activeIndex.value = 0;
      return;
    }
    await nextTick();
    inputRef.value?.focus();
  },
);

watch(flatDisplayItems, () => {
  activeIndex.value = 0;
});

const runItem = (item: CommandPaletteItem) => {
  if (item.disabled) return;
  emit('run', item.action);
  emit('close');
};

const onKeydown = (e: KeyboardEvent) => {
  const list = flatDisplayItems.value;
  if (e.key === 'Escape') {
    e.preventDefault();
    emit('close');
    return;
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    if (!list.length) return;
    activeIndex.value = (activeIndex.value + 1) % list.length;
    scrollActiveIntoView();
    return;
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault();
    if (!list.length) return;
    activeIndex.value = (activeIndex.value - 1 + list.length) % list.length;
    scrollActiveIntoView();
    return;
  }
  if (e.key === 'Enter') {
    e.preventDefault();
    const item = list[activeIndex.value];
    if (item) runItem(item);
  }
};

const scrollActiveIntoView = () => {
  nextTick(() => {
    listRef.value
      ?.querySelector('.command-palette__item.is-active')
      ?.scrollIntoView({ block: 'nearest' });
  });
};
</script>

<template>
  <Transition name="cmd-palette-fade">
    <div v-if="visible" class="command-palette-overlay" @click.self="emit('close')">
      <div class="command-palette" role="dialog" @keydown.stop="onKeydown">
        <div class="command-palette__header">
          <div class="command-palette__search">
            <i class="fas fa-search command-palette__search-icon"></i>
            <input
                ref="inputRef"
                v-model="query"
                class="command-palette__input"
                type="text"
                :placeholder="t('shortcuts.palettePlaceholder')"
                spellcheck="false"
            />
          </div>
          <span class="command-palette__esc">Esc</span>
        </div>
        <ul ref="listRef" class="command-palette__list custom-scrollbar">
          <template v-for="section in displaySections" :key="section.key">
            <li v-if="section.title" class="command-palette__section">{{ section.title }}</li>
            <li
                v-for="item in section.items"
                :key="`${section.key}-${item.id}`"
                class="command-palette__item"
                :class="{
                  'is-active': flatDisplayItems[activeIndex]?.id === item.id &&
                    flatDisplayItems[activeIndex]?.action === item.action,
                  'is-disabled': item.disabled,
                }"
                @click="runItem(item)"
                @mouseenter="activeIndex = flatDisplayItems.findIndex((x) => x.id === item.id && x.action === item.action)"
            >
              <span class="command-palette__label">{{ item.label }}</span>
              <span v-if="item.shortcut" class="command-palette__keys">{{ formatShortcutKeys(item.shortcut) }}</span>
            </li>
          </template>
          <li v-if="!flatDisplayItems.length" class="command-palette__empty">{{ t('shortcuts.paletteEmpty') }}</li>
        </ul>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.command-palette-overlay {
  position: fixed;
  inset: 0;
  z-index: 5000;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 12vh;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(6px);
}

.command-palette {
  width: min(520px, calc(100vw - 32px));
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: 0 24px 48px var(--shadow);
  overflow: hidden;

  &__header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-30);
  }

  &__search {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 10px;
    border-radius: 7px;
    border: 1px solid var(--border-30);
    background: var(--bg-input);
    transition: border-color 0.2s ease;

    &:focus-within {
      border-color: var(--border);
    }
  }

  &__search-icon {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-dim);
    opacity: 0.75;
    pointer-events: none;
  }

  &__input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--text-main);
    font-size: 13px;
    outline: none;
    box-shadow: none;

    &:focus,
    &:focus-visible {
      border: none !important;
      outline: none !important;
      box-shadow: none !important;
    }

    &::placeholder {
      color: var(--text-dim);
      opacity: 0.55;
    }
  }

  &__esc {
    flex-shrink: 0;
    font-size: 10px;
    color: var(--text-dim);
    padding: 3px 7px;
    border-radius: 5px;
    border: 1px solid var(--border-30);
    background: var(--bg-input);
  }

  &__list {
    list-style: none;
    margin: 0;
    padding: 6px;
    max-height: 320px;
    overflow-y: auto;
  }

  &__section {
    padding: 8px 12px 4px;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: default;
  }

  &__item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.12s ease;

    &:hover,
    &.is-active {
      background: var(--accent-10);
    }

    &.is-disabled {
      opacity: 0.45;
      cursor: not-allowed;
    }
  }

  &__label {
    font-size: 13px;
    color: var(--text-main);
  }

  &__keys {
    flex-shrink: 0;
    font-size: 10px;
    color: var(--text-dim);
    padding: 2px 7px;
    border-radius: 5px;
    border: 1px solid var(--border-30);
    background: var(--bg-input);
    font-family: var(--font-ui);
  }

  &__empty {
    padding: 24px 12px;
    text-align: center;
    font-size: 12px;
    color: var(--text-dim);
  }
}

.cmd-palette-fade-enter-active,
.cmd-palette-fade-leave-active {
  transition: opacity 0.18s ease;

  .command-palette {
    transition: transform 0.2s ease, opacity 0.18s ease;
  }
}

.cmd-palette-fade-enter-from,
.cmd-palette-fade-leave-to {
  opacity: 0;

  .command-palette {
    opacity: 0;
    transform: scale(0.97) translateY(-8px);
  }
}
</style>
