<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';

export type SelectOption = {
  value: string | number;
  label: string;
};

const props = withDefaults(defineProps<{
  modelValue?: string | number;
  options: SelectOption[];
  placeholder?: string;
  disabled?: boolean;
  icon?: string;
}>(), {
  placeholder: '请选择',
  disabled: false,
});

const emit = defineEmits<{
  'update:modelValue': [value: string | number];
}>();

const rootRef = ref<HTMLElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const isOpen = ref(false);
const menuPosition = ref({ top: 0, left: 0, width: 0 });

const selectedLabel = computed(() => {
  const match = props.options.find((option) => option.value === props.modelValue);
  return match?.label ?? props.placeholder;
});

const menuStyle = computed(() => ({
  top: `${menuPosition.value.top}px`,
  left: `${menuPosition.value.left}px`,
  width: `${menuPosition.value.width}px`,
}));

const updateMenuPosition = () => {
  const trigger = rootRef.value?.querySelector('.app-select__trigger') as HTMLElement | null;
  if (!trigger) return;
  const rect = trigger.getBoundingClientRect();
  menuPosition.value = {
    top: rect.bottom + 6,
    left: rect.left,
    width: rect.width,
  };
};

const close = () => {
  isOpen.value = false;
};

const toggle = (e: Event) => {
  e.stopPropagation();
  if (props.disabled) return;
  if (isOpen.value) {
    close();
  } else {
    isOpen.value = true;
    nextTick(updateMenuPosition);
  }
};

const selectOption = (value: string | number) => {
  emit('update:modelValue', value);
  close();
};

const handlePointerDownOutside = (e: PointerEvent) => {
  if (!isOpen.value) return;
  const target = e.target as Node;
  if (rootRef.value?.contains(target)) return;
  if (menuRef.value?.contains(target)) return;
  close();
};

watch(isOpen, (open) => {
  if (open) nextTick(updateMenuPosition);
});

watch(() => props.options, () => {
  if (isOpen.value) nextTick(updateMenuPosition);
});

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDownOutside);
  window.addEventListener('resize', updateMenuPosition);
  window.addEventListener('scroll', updateMenuPosition, true);
});

onUnmounted(() => {
  document.removeEventListener('pointerdown', handlePointerDownOutside);
  window.removeEventListener('resize', updateMenuPosition);
  window.removeEventListener('scroll', updateMenuPosition, true);
});
</script>

<template>
  <div
      ref="rootRef"
      class="app-select"
      :class="{ open: isOpen, 'is-disabled': disabled }"
  >
    <button type="button" class="app-select__trigger" :disabled="disabled" @click="toggle">
      <i v-if="icon" :class="[icon, 'app-select__icon']"></i>
      <span class="app-select__label">{{ selectedLabel }}</span>
      <i class="fas fa-chevron-down app-select__arrow"></i>
    </button>

    <Teleport to="body">
      <ul
          v-if="isOpen"
          ref="menuRef"
          class="app-select__menu app-select__menu--portal"
          :style="menuStyle"
      >
        <li
            v-for="option in options"
            :key="String(option.value)"
            :class="['app-select__option', { active: option.value === modelValue }]"
            @pointerdown.prevent.stop="selectOption(option.value)"
        >
          <i class="fas fa-check app-select__check"></i>
          <span class="app-select__option-label">{{ option.label }}</span>
        </li>
        <li v-if="options.length === 0" class="app-select__empty">暂无选项</li>
      </ul>
    </Teleport>
  </div>
</template>

<style scoped lang="scss">
.app-select {
  position: relative;
  width: 100%;

  &.is-disabled {
    opacity: 0.55;
    pointer-events: none;
  }

  &__trigger {
    position: relative;
    width: 100%;
    height: 38px;
    padding: 0 36px 0 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    box-sizing: border-box;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-main);
    font-size: 13px;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;

    &:hover:not(:disabled) {
      border-color: var(--border-50);
    }

    &:disabled {
      cursor: not-allowed;
    }
  }

  &__icon {
    flex-shrink: 0;
    font-size: 12px;
    color: var(--text-dim);
    opacity: 0.65;
  }

  &__label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__arrow {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 10px;
    color: var(--text-dim);
    opacity: 0.7;
    transition: transform 0.2s ease, color 0.2s ease;
    pointer-events: none;
  }

  &.open {
    .app-select__trigger {
      border-color: var(--accent);
      background: var(--accent-05);
      box-shadow: 0 0 0 3px var(--accent-15);
    }

    .app-select__arrow {
      transform: translateY(-50%) rotate(180deg);
      color: var(--accent);
    }
  }

  &__menu {
    margin: 0;
    padding: 6px;
    list-style: none;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 12px 28px var(--shadow);
    max-height: 220px;
    overflow-y: auto;

    &::-webkit-scrollbar { width: 4px; }
    &::-webkit-scrollbar-thumb {
      background: var(--border);
      border-radius: 4px;
    }

    &--portal {
      position: fixed;
      z-index: 3000;
    }
  }

  &__option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 7px;
    font-size: 13px;
    color: var(--text-main);
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;

    &:hover {
      background: var(--accent-08);
    }

    &.active {
      background: var(--accent-12);
      color: var(--accent);
      font-weight: 500;

      .app-select__check {
        opacity: 1;
      }
    }
  }

  &__check {
    width: 12px;
    font-size: 10px;
    color: var(--accent);
    opacity: 0;
    flex-shrink: 0;
  }

  &__option-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__empty {
    padding: 10px 12px;
    font-size: 12px;
    color: var(--text-dim);
    text-align: center;
  }
}
</style>
