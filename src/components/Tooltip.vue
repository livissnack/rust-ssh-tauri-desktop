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
const tooltipRef = ref<HTMLElement | null>(null);
const visible = ref(false);
const coords = ref({ top: 0, left: 0 });
const resolvedPlacement = ref<'top' | 'bottom' | 'left' | 'right'>(props.placement);

let showTimer: ReturnType<typeof setTimeout> | null = null;
let hideTimer: ReturnType<typeof setTimeout> | null = null;

const clearTimers = () => {
  if (showTimer) clearTimeout(showTimer);
  if (hideTimer) clearTimeout(hideTimer);
  showTimer = null;
  hideTimer = null;
};

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

const applyCoords = (
  placement: 'top' | 'bottom' | 'left' | 'right',
  rect: DOMRect,
  tipRect: DOMRect | null,
  gap: number,
) => {
  resolvedPlacement.value = placement;
  const tipW = tipRect?.width ?? 0;
  const tipH = tipRect?.height ?? 0;
  const pad = 8;

  switch (placement) {
    case 'top':
      coords.value = {
        top: rect.top - gap,
        left: clamp(rect.left + rect.width / 2, pad + tipW / 2, window.innerWidth - pad - tipW / 2),
      };
      break;
    case 'bottom':
      coords.value = {
        top: rect.bottom + gap,
        left: clamp(rect.left + rect.width / 2, pad + tipW / 2, window.innerWidth - pad - tipW / 2),
      };
      break;
    case 'left':
      coords.value = {
        top: clamp(rect.top + rect.height / 2, pad + tipH / 2, window.innerHeight - pad - tipH / 2),
        left: rect.left - gap,
      };
      break;
    case 'right':
      coords.value = {
        top: clamp(rect.top + rect.height / 2, pad + tipH / 2, window.innerHeight - pad - tipH / 2),
        left: rect.right + gap,
      };
      break;
  }
};

const pickVerticalPlacement = (
  preferred: 'top' | 'bottom',
  rect: DOMRect,
  tipH: number,
  gap: number,
) => {
  const pad = 8;
  const spaceAbove = rect.top - gap - pad;
  const spaceBelow = window.innerHeight - rect.bottom - gap - pad;
  const needs = tipH || 32;

  if (preferred === 'bottom') {
    if (spaceBelow >= needs) return 'bottom';
    if (spaceAbove >= needs) return 'top';
    return spaceBelow >= spaceAbove ? 'bottom' : 'top';
  }

  if (spaceAbove >= needs) return 'top';
  if (spaceBelow >= needs) return 'bottom';
  return spaceAbove >= spaceBelow ? 'top' : 'bottom';
};

const updatePosition = async () => {
  await nextTick();
  const el = triggerRef.value;
  if (!el) return;

  const rect = el.getBoundingClientRect();
  const gap = 8;
  let tipRect = tooltipRef.value?.getBoundingClientRect() ?? null;

  let placement = props.placement;
  if (placement === 'top' || placement === 'bottom') {
    placement = pickVerticalPlacement(placement, rect, tipRect?.height ?? 32, gap);
  }

  applyCoords(placement, rect, tipRect, gap);

  if (!tipRect) {
    await nextTick();
    tipRect = tooltipRef.value?.getBoundingClientRect() ?? null;
    if (tipRect && (props.placement === 'top' || props.placement === 'bottom')) {
      placement = pickVerticalPlacement(props.placement, rect, tipRect.height, gap);
      applyCoords(placement, rect, tipRect, gap);
    }
  }
};

const show = () => {
  if (props.disabled || !props.text?.trim()) return;
  clearTimers();
  showTimer = setTimeout(async () => {
    resolvedPlacement.value = props.placement;
    visible.value = true;
    await updatePosition();
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

  switch (resolvedPlacement.value) {
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
            ref="tooltipRef"
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
