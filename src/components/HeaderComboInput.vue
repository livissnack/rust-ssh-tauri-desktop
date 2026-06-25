<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue';

const props = withDefaults(defineProps<{
  modelValue: string;
  options?: string[];
  placeholder?: string;
  disabled?: boolean;
}>(), {
  options: () => [],
  placeholder: '',
  disabled: false,
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
  select: [value: string];
}>();

const rootRef = ref<HTMLElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const isOpen = ref(false);
const menuPosition = ref({ top: 0, left: 0, width: 0 });

const filteredOptions = computed(() => {
  const query = props.modelValue.trim().toLowerCase();
  const list = props.options.filter(Boolean);
  if (!query) return list;
  return list.filter((option) => option.toLowerCase().includes(query));
});

const menuStyle = computed(() => ({
  top: `${menuPosition.value.top}px`,
  left: `${menuPosition.value.left}px`,
  width: `${menuPosition.value.width}px`,
}));

const updateMenuPosition = () => {
  const trigger = rootRef.value?.querySelector('.header-combo__input') as HTMLElement | null;
  if (!trigger) return;
  const rect = trigger.getBoundingClientRect();
  menuPosition.value = {
    top: rect.bottom + 4,
    left: rect.left,
    width: rect.width,
  };
};

const close = () => {
  isOpen.value = false;
};

const open = () => {
  if (props.disabled || props.options.length === 0) return;
  isOpen.value = true;
  nextTick(updateMenuPosition);
};

const toggleMenu = () => {
  if (isOpen.value) close();
  else open();
};

const onInput = (event: Event) => {
  emit('update:modelValue', (event.target as HTMLInputElement).value);
  if (!props.disabled && props.options.length > 0) {
    if (!isOpen.value) open();
    else nextTick(updateMenuPosition);
  }
};

const selectOption = (value: string) => {
  emit('update:modelValue', value);
  emit('select', value);
  close();
};

const handlePointerDownOutside = (event: PointerEvent) => {
  if (!isOpen.value) return;
  const target = event.target as Node;
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
    class="header-combo"
    :class="{ open: isOpen, 'has-options': options.length > 0, 'is-disabled': disabled }"
  >
    <input
      class="header-combo__input"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      @input="onInput"
      @focus="open"
    />
    <button
      v-if="options.length > 0"
      type="button"
      class="header-combo__toggle"
      :disabled="disabled"
      tabindex="-1"
      @mousedown.prevent
      @click="toggleMenu"
    >
      <i class="fas fa-chevron-down"></i>
    </button>

    <Teleport to="body">
      <ul
        v-if="isOpen && filteredOptions.length > 0"
        ref="menuRef"
        class="header-combo__menu"
        :style="menuStyle"
      >
        <li
          v-for="option in filteredOptions"
          :key="option"
          class="header-combo__option"
          :class="{ active: option === modelValue }"
          @pointerdown.prevent.stop="selectOption(option)"
        >
          {{ option }}
        </li>
      </ul>
      <div
        v-else-if="isOpen && options.length > 0"
        ref="menuRef"
        class="header-combo__menu header-combo__menu--empty"
        :style="menuStyle"
      >
        无匹配项
      </div>
    </Teleport>
  </div>
</template>

<style scoped lang="scss">
.header-combo {
  position: relative;
  flex: 1;
  min-width: 0;

  &.is-disabled {
    opacity: 0.45;
  }

  &__input {
    width: 100%;
    height: 30px;
    padding: 0 28px 0 8px;
    box-sizing: border-box;
    border-radius: 6px;
    border: 1px solid var(--border-30);
    background: var(--bg-input);
    color: var(--text-main);
    font-size: 11px;
    user-select: text;
    -webkit-user-select: text;

    &:focus {
      outline: none;
      border-color: var(--accent-30);
      box-shadow: 0 0 0 2px var(--accent-10);
    }

    &:disabled {
      cursor: not-allowed;
    }
  }

  &__toggle {
    position: absolute;
    top: 50%;
    right: 4px;
    transform: translateY(-50%);
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: color 0.15s, background 0.15s;

    i {
      font-size: 9px;
      transition: transform 0.2s;
    }

    &:hover:not(:disabled) {
      color: var(--accent);
      background: var(--accent-08);
    }

    &:disabled {
      cursor: not-allowed;
    }
  }

  &.open &__toggle i {
    transform: rotate(180deg);
    color: var(--accent);
  }

  &__menu {
    margin: 0;
    padding: 4px;
    list-style: none;
    position: fixed;
    z-index: 3000;
    max-height: 180px;
    overflow-y: auto;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 10px 24px var(--shadow);

    &--empty {
      padding: 8px 10px;
      font-size: 11px;
      color: var(--text-dim);
    }
  }

  &__option {
    padding: 7px 10px;
    border-radius: 6px;
    font-size: 11px;
    color: var(--text-main);
    cursor: pointer;
    word-break: break-all;
    transition: background 0.15s, color 0.15s;

    &:hover {
      background: var(--accent-08);
    }

    &.active {
      background: var(--accent-12);
      color: var(--accent);
      font-weight: 600;
    }
  }
}
</style>
