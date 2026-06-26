import { computed, nextTick, onUnmounted, ref } from 'vue';
import {
  portalStyleFromPlacement,
  resolveTooltipPlacement,
  type TooltipPlacement,
} from '../utils/tooltipPosition.ts';

export function useDelegatedTooltip(options?: {
  placement?: TooltipPlacement;
  delay?: number;
}) {
  const preferredPlacement = options?.placement ?? 'top';
  const delay = options?.delay ?? 350;

  const visible = ref(false);
  const text = ref('');
  const coords = ref({ top: 0, left: 0 });
  const resolvedPlacement = ref<TooltipPlacement>(preferredPlacement);
  const tooltipRef = ref<HTMLElement | null>(null);

  let showTimer: ReturnType<typeof setTimeout> | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | null = null;
  let activeTarget: HTMLElement | null = null;

  const portalStyle = computed(() =>
    portalStyleFromPlacement(coords.value, resolvedPlacement.value),
  );

  const clearTimers = () => {
    if (showTimer) clearTimeout(showTimer);
    if (hideTimer) clearTimeout(hideTimer);
    showTimer = null;
    hideTimer = null;
  };

  const positionTo = async (el: HTMLElement) => {
    await nextTick();
    const result = await resolveTooltipPlacement(
      preferredPlacement,
      el,
      tooltipRef.value,
    );
    coords.value = result.coords;
    resolvedPlacement.value = result.placement;
  };

  const showFor = (el: HTMLElement) => {
    const tip = el.dataset.tip?.trim();
    if (!tip) return;
    clearTimers();
    activeTarget = el;
    showTimer = setTimeout(async () => {
      text.value = tip;
      visible.value = true;
      await positionTo(el);
      await positionTo(el);
    }, delay);
  };

  const hide = () => {
    clearTimers();
    hideTimer = setTimeout(() => {
      visible.value = false;
      activeTarget = null;
    }, 60);
  };

  const onPointerOver = (event: MouseEvent) => {
    const el = (event.target as HTMLElement).closest('[data-tip]') as HTMLElement | null;
    if (!el?.dataset.tip?.trim()) return;
    if (el === activeTarget && visible.value) return;
    showFor(el);
  };

  const onPointerOut = (event: MouseEvent) => {
    const el = (event.target as HTMLElement).closest('[data-tip]') as HTMLElement | null;
    if (!el) return;
    const related = event.relatedTarget as Node | null;
    if (related && el.contains(related)) return;
    if (related && (related as HTMLElement).closest?.('[data-tip]')) return;
    hide();
  };

  onUnmounted(clearTimers);

  return {
    visible,
    text,
    portalStyle,
    tooltipRef,
    onPointerOver,
    onPointerOut,
  };
}
