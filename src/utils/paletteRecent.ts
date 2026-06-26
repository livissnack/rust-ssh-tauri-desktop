import type { ShortcutAction } from './shortcuts.ts';
import { SHORTCUT_ITEMS } from './shortcuts.ts';

const STORAGE_KEY = 'command-palette-recent';
const MAX_RECENT = 8;

const VALID_ACTIONS = new Set<string>(SHORTCUT_ITEMS.map((item) => item.action));

function isShortcutAction(value: string): value is ShortcutAction {
  return VALID_ACTIONS.has(value);
}

function readRecent(): ShortcutAction[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((x): x is ShortcutAction => typeof x === 'string' && isShortcutAction(x));
  } catch {
    return [];
  }
}

export function getRecentPaletteActions(): ShortcutAction[] {
  return readRecent().slice(0, MAX_RECENT);
}

export function recordPaletteUse(action: ShortcutAction): void {
  const prev = readRecent().filter((a) => a !== action);
  const next = [action, ...prev].slice(0, MAX_RECENT);
  localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
}
