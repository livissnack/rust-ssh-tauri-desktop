/**
 * Pointer-based drag (SortableJS / @dnd-kit style) — does NOT use HTML5 DnD.
 * Safe to use alongside Tauri native file drag-drop (dragDropEnabled: true).
 */

export type PointerDragCallbacks = {
  threshold?: number;
  onActivate?: () => void;
  onMove?: (x: number, y: number) => void;
  onFinish: (x: number, y: number, activated: boolean) => void;
  onCancel?: () => void;
};

type Session = PointerDragCallbacks & {
  pointerId: number;
  startX: number;
  startY: number;
  activated: boolean;
};

let session: Session | null = null;

function cleanup() {
  document.removeEventListener("pointermove", onMove);
  document.removeEventListener("pointerup", onUp);
  document.removeEventListener("pointercancel", onCancel);
  session = null;
}

function onMove(e: PointerEvent) {
  if (!session || session.pointerId !== e.pointerId) return;

  if (!session.activated) {
    const dx = e.clientX - session.startX;
    const dy = e.clientY - session.startY;
    if (Math.hypot(dx, dy) < (session.threshold ?? 6)) return;
    session.activated = true;
    session.onActivate?.();
  }

  session.onMove?.(e.clientX, e.clientY);
}

function onUp(e: PointerEvent) {
  if (!session || session.pointerId !== e.pointerId) return;
  const activated = session.activated;
  const { onFinish } = session;
  cleanup();
  onFinish(e.clientX, e.clientY, activated);
}

function onCancel(e: PointerEvent) {
  if (!session || session.pointerId !== e.pointerId) return;
  session.onCancel?.();
  cleanup();
}

export function beginPointerDrag(e: PointerEvent, callbacks: PointerDragCallbacks) {
  if (e.button !== 0) return;
  e.preventDefault();
  e.stopPropagation();
  if (session) cleanup();

  session = {
    ...callbacks,
    threshold: callbacks.threshold ?? 6,
    pointerId: e.pointerId,
    startX: e.clientX,
    startY: e.clientY,
    activated: false,
  };

  document.addEventListener("pointermove", onMove);
  document.addEventListener("pointerup", onUp);
  document.addEventListener("pointercancel", onCancel);
}

export function cancelPointerDrag() {
  if (session) {
    session.onCancel?.();
    cleanup();
  }
}

export function isPointerDragActive() {
  return session?.activated ?? false;
}

/** Find attribute value from element under point (skips `exclude`). */
export function findAttrFromPoint(
  x: number,
  y: number,
  attr: string,
  exclude?: string,
): string | null {
  const el = document.elementFromPoint(x, y);
  if (!el) return null;
  const match = el.closest(`[${attr}]`) as HTMLElement | null;
  const value = match?.getAttribute(attr);
  if (!value || value === exclude) return null;
  return value;
}
