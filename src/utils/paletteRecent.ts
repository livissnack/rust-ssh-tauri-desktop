import type { ShortcutAction } from './shortcuts.ts';

const STORAGE_KEY = 'command-palette-recent';
const MAX_RECENT = 8;

function readRecent(): ShortcutAction[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((x) => typeof x === 'string') : [];
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
