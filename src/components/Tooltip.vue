<script setup lang="ts">
import { ref, computed, onUnmounted } from 'vue';
import {
  portalStyleFromPlacement,
  resolveTooltipPlacement,
  type TooltipPlacement,
} from '../utils/tooltipPosition.ts';

const props = withDefaults(defineProps<{
  text?: string;
  placement?: TooltipPlacement;
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
const resolvedPlacement = ref<TooltipPlacement>(props.placement);

let showTimer: ReturnType<typeof setTimeout> | null = null;
let hideTimer: ReturnType<typeof setTimeout> | null = null;

const clearTimers = () => {
  if (showTimer) clearTimeout(showTimer);
  if (hideTimer) clearTimeout(hideTimer);
  showTimer = null;
  hideTimer = null;
};

const updatePosition = async () => {
  const el = triggerRef.value;
  if (!el) return;
  const result = await resolveTooltipPlacement(
    props.placement,
    el,
    tooltipRef.value,
  );
  coords.value = result.coords;
  resolvedPlacement.value = result.placement;
};

const show = () => {
  if (props.disabled || !props.text?.trim()) return;
  clearTimers();
  showTimer = setTimeout(async () => {
    resolvedPlacement.value = props.placement;
    visible.value = true;
    await updatePosition();
    await updatePosition();
  }, props.delay);
};

const hide = () => {
  clearTimers();
  hideTimer = setTimeout(() => {
    visible.value = false;
  }, 60);
};

const portalStyle = computed(() =>
  portalStyleFromPlacement(coords.value, resolvedPlacement.value),
);

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
