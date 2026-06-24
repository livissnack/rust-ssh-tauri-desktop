<script setup lang="ts">
import { ref, computed, nextTick, onUnmounted } from 'vue';

const props = withDefaults(defineProps<{
  text?: string;
  placement?: 'top' | 'bottom' | 'left' | 'right';
  delay?: number;
  disabled?: boolean;
  inline?: boolean;
  block?: boolean;
  wrap?: boolean;
}>(), {
  placement: 'top',
  delay: 350,
  disabled: false,
  inline: false,
  block: false,
  wrap: false,
});

const triggerRef = ref<HTMLElement | null>(null);
const visible = ref(false);
const coords = ref({ top: 0, left: 0 });

let showTimer: ReturnType<typeof setTimeout> | null = null;
let hideTimer: ReturnType<typeof setTimeout> | null = null;

const clearTimers = () => {
  if (showTimer) clearTimeout(showTimer);
  if (hideTimer) clearTimeout(hideTimer);
  showTimer = null;
  hideTimer = null;
};

const updatePosition = async () => {
  await nextTick();
  const el = triggerRef.value;
  if (!el) return;

  const rect = el.getBoundingClientRect();
  const gap = 8;

  switch (props.placement) {
    case 'top':
      coords.value = { top: rect.top - gap, left: rect.left + rect.width / 2 };
      break;
    case 'bottom':
      coords.value = { top: rect.bottom + gap, left: rect.left + rect.width / 2 };
      break;
    case 'left':
      coords.value = { top: rect.top + rect.height / 2, left: rect.left - gap };
      break;
    case 'right':
      coords.value = { top: rect.top + rect.height / 2, left: rect.right + gap };
      break;
  }
};

const show = () => {
  if (props.disabled || !props.text?.trim()) return;
  clearTimers();
  showTimer = setTimeout(async () => {
    await updatePosition();
    visible.value = true;
  }, props.delay);
};

const hide = () => {
  clearTimers();
  hideTimer = setTimeout(() => {
    visible.value = false;
  }, 60);
};

const portalStyle = computed(() => {
  const { top, left } = coords.value;
  const base = {
    top: `${top}px`,
    left: `${left}px`,
  };

  switch (props.placement) {
    case 'top':
      return { ...base, transform: 'translate(-50%, -100%)' };
    case 'bottom':
      return { ...base, transform: 'translate(-50%, 0)' };
    case 'left':
      return { ...base, transform: 'translate(-100%, -50%)' };
    case 'right':
      return { ...base, transform: 'translate(0, -50%)' };
    default:
      return base;
  }
});

onUnmounted(clearTimers);
</script>

<template>
  <span
      ref="triggerRef"
      class="tooltip-trigger"
      :class="{
        'tooltip-trigger--inline': inline,
        'tooltip-trigger--block': block,
      }"
      @mouseenter="show"
      @mouseleave="hide"
      @focusin="show"
      @focusout="hide"
  >
    <slot />
    <Teleport to="body">
      <Transition name="tooltip-fade">
        <div
            v-if="visible && text"
            class="app-tooltip"
            :class="{ 'app-tooltip--wrap': wrap }"
            :style="portalStyle"
            role="tooltip"
        >
          {{ text }}
        </div>
      </Transition>
    </Teleport>
  </span>
</template>

<style lang="scss" scoped>
.tooltip-trigger {
  display: inline-flex;
  flex-shrink: 0;
  vertical-align: middle;

  &--inline {
    display: inline;
  }

  &--block {
    display: flex;
    width: 100%;
    min-width: 0;
  }
}

.app-tooltip {
  position: fixed;
  z-index: 12000;
  box-sizing: border-box;
  width: max-content;
  max-width: 280px;
  padding: 7px 12px;
  border-radius: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: 0 8px 24px var(--shadow);
  color: var(--text-main);
  font-size: 12px;
  font-weight: 500;
  font-family: inherit;
  line-height: 1.5;
  letter-spacing: 0.02em;
  white-space: nowrap;
  pointer-events: none;
  user-select: none;

  &--wrap {
    white-space: normal;
    word-break: break-word;
  }
}

.tooltip-fade-enter-active,
.tooltip-fade-leave-active {
  transition: opacity 0.15s ease;
}

.tooltip-fade-enter-from,
.tooltip-fade-leave-to {
  opacity: 0;
}
</style>
