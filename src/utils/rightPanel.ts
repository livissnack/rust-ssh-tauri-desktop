export type RightPanelType =
  | 'quick'
  | 'ai'
  | 'redis'
  | 'history'
  | 'sync-settings'
  | 'theme-settings'
  | 'chat'
  | 'api';

export const PANEL_WIDTH_STORAGE_KEY = 'right-panel-widths';
export const LEGACY_PANEL_WIDTH_KEY = 'right-panel-width';

export const DEFAULT_PANEL_WIDTHS: Record<RightPanelType, number> = {
  quick: 420,
  ai: 420,
  redis: 420,
  history: 420,
  'sync-settings': 420,
  'theme-settings': 400,
  chat: 420,
  api: 892,
};

export const PANEL_MIN_WIDTHS: Partial<Record<RightPanelType, number>> = {
  api: 480,
  redis: 360,
};

export function loadPanelWidths(): Record<RightPanelType, number> {
  const widths = { ...DEFAULT_PANEL_WIDTHS };
  try {
    const raw = localStorage.getItem(PANEL_WIDTH_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Record<RightPanelType, number>>;
      for (const key of Object.keys(DEFAULT_PANEL_WIDTHS) as RightPanelType[]) {
        const value = parsed[key];
        if (typeof value === 'number' && value >= getPanelMinWidth(key)) {
          widths[key] = value;
        }
      }
      return widths;
    }
    const legacy = localStorage.getItem(LEGACY_PANEL_WIDTH_KEY);
    if (legacy) {
      const n = Number(legacy);
      if (!Number.isNaN(n) && n >= 300) {
        for (const key of Object.keys(DEFAULT_PANEL_WIDTHS) as RightPanelType[]) {
          widths[key] = n;
        }
      }
    }
  } catch {
    /* ignore */
  }
  return widths;
}

export function savePanelWidths(widths: Record<RightPanelType, number>): void {
  try {
    localStorage.setItem(PANEL_WIDTH_STORAGE_KEY, JSON.stringify(widths));
  } catch {
    /* ignore */
  }
}

export function getPanelMinWidth(type: RightPanelType): number {
  return PANEL_MIN_WIDTHS[type] ?? 300;
}
