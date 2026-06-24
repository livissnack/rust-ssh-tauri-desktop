<script setup lang="ts">
import { onUnmounted } from 'vue';

const props = withDefaults(defineProps<{
  modelValue?: number;
  min?: number;
  max?: number;
  step?: number;
  placeholder?: string;
  disabled?: boolean;
}>(), {
  step: 1,
  disabled: false,
});

const emit = defineEmits<{
  'update:modelValue': [value: number];
}>();

let repeatTimer: ReturnType<typeof setInterval> | null = null;

const clamp = (value: number) => {
  let next = value;
  if (props.min !== undefined) next = Math.max(props.min, next);
  if (props.max !== undefined) next = Math.min(props.max, next);
  return next;
};

const canIncrement = () =>
  !props.disabled && (props.max === undefined || (props.modelValue ?? 0) < props.max);

const canDecrement = () =>
  !props.disabled && (props.min === undefined || (props.modelValue ?? 0) > props.min);

const increment = () => {
  if (!canIncrement()) return;
  const current = Number(props.modelValue);
  const base = Number.isFinite(current) ? current : (props.min ?? 0);
  emit('update:modelValue', clamp(base + props.step));
};

const decrement = () => {
  if (!canDecrement()) return;
  const current = Number(props.modelValue);
  const base = Number.isFinite(current) ? current : (props.min ?? 0);
  emit('update:modelValue', clamp(base - props.step));
};

const stopRepeat = () => {
  if (repeatTimer) clearInterval(repeatTimer);
  repeatTimer = null;
};

const startRepeat = (action: () => void) => {
  stopRepeat();
  action();
  repeatTimer = setInterval(action, 120);
};

const onInput = (e: Event) => {
  const raw = (e.target as HTMLInputElement).value.trim();
  if (raw === '' || raw === '-') return;
  const parsed = Number(raw);
  if (Number.isFinite(parsed)) {
    emit('update:modelValue', clamp(parsed));
  }
};

const onBlur = (e: Event) => {
  const raw = (e.target as HTMLInputElement).value.trim();
  if (raw === '' || !Number.isFinite(Number(raw))) {
    emit('update:modelValue', clamp(props.min ?? 0));
  }
};

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'ArrowUp') {
    e.preventDefault();
    increment();
  } else if (e.key === 'ArrowDown') {
    e.preventDefault();
    decrement();
  }
};

const onWheel = (e: WheelEvent) => {
  if (props.disabled) return;
  e.preventDefault();
  if (e.deltaY < 0) increment();
  else decrement();
};

onUnmounted(stopRepeat);
</script>

<template>
  <div class="number-input" :class="{ 'is-disabled': disabled }">
    <input
      class="number-input__field"
      type="text"
      inputmode="numeric"
      :value="modelValue ?? ''"
      :placeholder="placeholder"
      :disabled="disabled"
      @input="onInput"
      @blur="onBlur"
      @keydown="onKeydown"
      @wheel="onWheel"
    />
    <div class="number-input__spin">
      <button
        type="button"
        class="number-input__spin-btn number-input__spin-btn--up"
        :disabled="!canIncrement()"
        tabindex="-1"
        @mousedown.prevent="startRepeat(increment)"
        @mouseup="stopRepeat"
        @mouseleave="stopRepeat"
      />
      <button
        type="button"
        class="number-input__spin-btn number-input__spin-btn--down"
        :disabled="!canDecrement()"
        tabindex="-1"
        @mousedown.prevent="startRepeat(decrement)"
        @mouseup="stopRepeat"
        @mouseleave="stopRepeat"
      />
    </div>
  </div>
</template>

<style scoped lang="scss">
.number-input {
  --number-input-height: 36px;
  --number-input-spin-width: 22px;

  position: relative;
  display: flex;
  align-items: stretch;
  height: var(--number-input-height);
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  transition: border-color 0.2s, box-shadow 0.2s;

  &:focus-within:not(.is-disabled) {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-15);
  }

  &.is-disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  &.number-input--lg {
    --number-input-height: 38px;
    border-radius: 9px;

    .number-input__spin {
      border-radius: 0 8px 8px 0;
    }
  }

  &__field {
    flex: 1;
    min-width: 0;
    width: 0;
    height: 100%;
    padding: 0 calc(var(--number-input-spin-width) + 8px) 0 10px;
    border: none;
    background: transparent;
    color: var(--text-main);
    font-size: 13px;
    font-family: var(--font-terminal);
    outline: none;

    &::placeholder {
      color: var(--text-dim);
      opacity: 0.65;
    }

    &:disabled {
      cursor: not-allowed;
    }
  }

  &__spin {
    position: absolute;
    top: 1px;
    right: 1px;
    bottom: 1px;
    width: var(--number-input-spin-width);
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    border-radius: 0 7px 7px 0;
    overflow: hidden;
    background: var(--bg-secondary);
  }

  &__spin-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;

    &::before {
      content: '';
      display: block;
      width: 0;
      height: 0;
      border-left: 4px solid transparent;
      border-right: 4px solid transparent;
    }

    &--up::before {
      border-bottom: 5px solid currentColor;
      margin-top: 1px;
    }

    &--down::before {
      border-top: 5px solid currentColor;
      margin-bottom: 1px;
    }

    &:hover:not(:disabled) {
      background: var(--accent-10);
      color: var(--accent);
    }

    &:active:not(:disabled) {
      background: var(--accent-15);
    }

    &:disabled {
      opacity: 0.35;
      cursor: not-allowed;
    }

    & + & {
      border-top: 1px solid var(--border);
    }
  }
}
</style>
