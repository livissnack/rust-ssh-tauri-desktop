export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right';

export function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

export function pickVerticalPlacement(
  preferred: 'top' | 'bottom',
  rect: DOMRect,
  tipH: number,
  gap: number,
): 'top' | 'bottom' {
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
}

export function applyTooltipCoords(
  placement: TooltipPlacement,
  rect: DOMRect,
  tipRect: DOMRect | null,
  gap: number,
): { top: number; left: number } {
  const tipW = tipRect?.width ?? 0;
  const tipH = tipRect?.height ?? 0;
  const pad = 8;

  switch (placement) {
    case 'top':
      return {
        top: rect.top - gap,
        left: clamp(rect.left + rect.width / 2, pad + tipW / 2, window.innerWidth - pad - tipW / 2),
      };
    case 'bottom':
      return {
        top: rect.bottom + gap,
        left: clamp(rect.left + rect.width / 2, pad + tipW / 2, window.innerWidth - pad - tipW / 2),
      };
    case 'left':
      return {
        top: clamp(rect.top + rect.height / 2, pad + tipH / 2, window.innerHeight - pad - tipH / 2),
        left: rect.left - gap,
      };
    case 'right':
      return {
        top: clamp(rect.top + rect.height / 2, pad + tipH / 2, window.innerHeight - pad - tipH / 2),
        left: rect.right + gap,
      };
    default:
      return { top: rect.top, left: rect.left };
  }
}

export function portalStyleFromPlacement(
  coords: { top: number; left: number },
  placement: TooltipPlacement,
): Record<string, string> {
  const base = {
    top: `${coords.top}px`,
    left: `${coords.left}px`,
  };

  switch (placement) {
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
}

export async function resolveTooltipPlacement(
  preferred: TooltipPlacement,
  triggerEl: HTMLElement,
  tipEl: HTMLElement | null,
  gap = 8,
): Promise<{ coords: { top: number; left: number }; placement: TooltipPlacement }> {
  const rect = triggerEl.getBoundingClientRect();
  let tipRect = tipEl?.getBoundingClientRect() ?? null;
  let placement = preferred;

  if (placement === 'top' || placement === 'bottom') {
    placement = pickVerticalPlacement(placement, rect, tipRect?.height ?? 32, gap);
  }

  let coords = applyTooltipCoords(placement, rect, tipRect, gap);

  if (!tipRect && tipEl) {
    await new Promise<void>((r) => requestAnimationFrame(() => r()));
    tipRect = tipEl.getBoundingClientRect();
    if (tipRect && (preferred === 'top' || preferred === 'bottom')) {
      placement = pickVerticalPlacement(preferred, rect, tipRect.height, gap);
      coords = applyTooltipCoords(placement, rect, tipRect, gap);
    }
  }

  return { coords, placement };
}
