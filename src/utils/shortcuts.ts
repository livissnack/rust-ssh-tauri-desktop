import { detectAppPlatform } from './platform.ts';

export type ShortcutAction =
  | 'closeTab'
  | 'newLocalShell'
  | 'newHost'
  | 'nextTab'
  | 'prevTab'
  | 'tab1'
  | 'tab2'
  | 'tab3'
  | 'tab4'
  | 'tab5'
  | 'tab6'
  | 'tab7'
  | 'tab8'
  | 'tab9'
  | 'terminalSearch'
  | 'toggleSftp'
  | 'clearTerminal'
  | 'copyTerminalSelection'
  | 'commandPalette'
  | 'toggleQuickPanel'
  | 'toggleAiPanel'
  | 'toggleRedisPanel'
  | 'toggleApiPanel'
  | 'toggleChatPanel'
  | 'toggleSyncPanel'
  | 'themeSettings'
  | 'exportTerminal'
  | 'portForwardPanel'
  | 'reconnect'
  | 'shortcutHelp';

export type ShortcutGroupId = 'session' | 'terminal' | 'panels' | 'dock';

export interface ShortcutItemDef {
  action: ShortcutAction;
  /** Empty = palette-only, shown in help as palette hint */
  keys: string;
  labelKey: string;
  group: ShortcutGroupId;
  inHelp?: boolean;
  inPalette?: boolean;
}

const TAB_SHORTCUTS: ShortcutItemDef[] = Array.from({ length: 9 }, (_, i) => ({
  action: `tab${i + 1}` as ShortcutAction,
  keys: `mod+${i + 1}`,
  labelKey: 'shortcuts.switchTabN',
  group: 'session' as const,
}));

/** Single source of truth for shortcuts, help, and command palette */
export const SHORTCUT_ITEMS: ShortcutItemDef[] = [
  { action: 'newLocalShell', keys: 'mod+T', labelKey: 'shortcuts.newLocalShell', group: 'session' },
  { action: 'newHost', keys: 'mod+N', labelKey: 'shortcuts.newHost', group: 'session' },
  { action: 'closeTab', keys: 'mod+W', labelKey: 'shortcuts.closeTab', group: 'session' },
  { action: 'nextTab', keys: 'mod+Tab', labelKey: 'shortcuts.nextTab', group: 'session' },
  { action: 'prevTab', keys: 'mod+shift+Tab', labelKey: 'shortcuts.prevTab', group: 'session' },
  ...TAB_SHORTCUTS,
  { action: 'reconnect', keys: 'mod+shift+R', labelKey: 'shortcuts.reconnect', group: 'session' },
  { action: 'terminalSearch', keys: 'mod+F', labelKey: 'shortcuts.terminalSearch', group: 'terminal' },
  { action: 'toggleSftp', keys: 'mod+shift+F', labelKey: 'shortcuts.toggleSftp', group: 'terminal' },
  { action: 'clearTerminal', keys: 'mod+L', labelKey: 'shortcuts.clearTerminal', group: 'terminal' },
  { action: 'copyTerminalSelection', keys: 'mod+shift+C', labelKey: 'shortcuts.copySelection', group: 'terminal' },
  { action: 'exportTerminal', keys: '', labelKey: 'shortcuts.exportTerminal', group: 'terminal', inHelp: true, inPalette: true },
  { action: 'commandPalette', keys: 'mod+shift+P', labelKey: 'shortcuts.commandPalette', group: 'panels' },
  { action: 'toggleQuickPanel', keys: 'mod+K', labelKey: 'shortcuts.toggleQuickPanel', group: 'panels' },
  { action: 'toggleAiPanel', keys: 'mod+shift+A', labelKey: 'shortcuts.toggleAiPanel', group: 'panels' },
  { action: 'themeSettings', keys: 'mod+,', labelKey: 'shortcuts.themeSettings', group: 'panels' },
  { action: 'shortcutHelp', keys: 'F1', labelKey: 'shortcuts.shortcutHelp', group: 'panels' },
  { action: 'toggleRedisPanel', keys: '', labelKey: 'shortcuts.toggleRedisPanel', group: 'dock', inHelp: true, inPalette: true },
  { action: 'toggleApiPanel', keys: '', labelKey: 'shortcuts.toggleApiPanel', group: 'dock', inHelp: true, inPalette: true },
  { action: 'toggleChatPanel', keys: '', labelKey: 'shortcuts.toggleChatPanel', group: 'dock', inHelp: true, inPalette: true },
  { action: 'toggleSyncPanel', keys: '', labelKey: 'shortcuts.toggleSyncPanel', group: 'dock', inHelp: true, inPalette: true },
  { action: 'portForwardPanel', keys: '', labelKey: 'shortcuts.portForwardPanel', group: 'dock', inHelp: true, inPalette: true },
];

export interface ShortcutGroupDef {
  titleKey: string;
  group: ShortcutGroupId;
  items: ShortcutItemDef[];
}

const GROUP_META: { group: ShortcutGroupId; titleKey: string }[] = [
  { group: 'session', titleKey: 'shortcuts.groups.session' },
  { group: 'terminal', titleKey: 'shortcuts.groups.terminal' },
  { group: 'panels', titleKey: 'shortcuts.groups.panels' },
  { group: 'dock', titleKey: 'shortcuts.groups.dock' },
];

export const SHORTCUT_GROUPS: ShortcutGroupDef[] = GROUP_META.map(({ group, titleKey }) => ({
  titleKey,
  group,
  items: SHORTCUT_ITEMS.filter(
    (item) => item.group === group && item.inHelp !== false && (item.keys || item.group === 'dock'),
  ),
}));

export const PALETTE_SHORTCUT_ITEMS = SHORTCUT_ITEMS.filter((item) => item.inPalette !== false);

export function shortcutLabelParams(action: ShortcutAction): Record<string, number> | undefined {
  const match = /^tab(\d)$/.exec(action);
  if (match) return { n: Number(match[1]) };
  return undefined;
}

export function modLabel(): string {
  const platform = detectAppPlatform();
  if (platform === 'macos') return '⌘';
  return 'Ctrl';
}

export function platformModKeyName(): string {
  const platform = detectAppPlatform();
  if (platform === 'macos') return '⌘ Command';
  return 'Ctrl';
}

export function formatShortcutKeys(keys: string): string {
  if (!keys.trim()) return '';
  const platform = detectAppPlatform();
  const mod = modLabel();
  return keys
    .split('+')
    .map((part) => {
      if (part === 'mod') return mod;
      if (part === 'shift') return platform === 'macos' ? '⇧' : 'Shift';
      if (part === 'alt') return platform === 'macos' ? '⌥' : 'Alt';
      if (part === 'Tab') return 'Tab';
      if (/^F\d+$/.test(part)) return part;
      return part.length === 1 ? part.toUpperCase() : part;
    })
    .join(' + ');
}

export function isModKey(e: KeyboardEvent): boolean {
  return e.ctrlKey || e.metaKey;
}

/** True when user is typing in a normal form field (not xterm). */
export function isTypingContext(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  if (target.closest('.xterm-helper-textarea')) return false;
  if (target.closest('.command-palette')) return false;
  if (target.closest('.port-forward-dialog')) return false;
  if (target.closest('.terminal-search-bar')) return true;
  const tag = target.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA';
}

function keyToken(e: KeyboardEvent): string {
  if (e.key === 'Tab') return 'Tab';
  if (e.key === ',') return ',';
  if (/^F\d+$/.test(e.key)) return e.key;
  if (e.key.length === 1) return e.key.toLowerCase();
  return e.key;
}

export function matchShortcut(e: KeyboardEvent, spec: string): boolean {
  if (!spec.trim()) return false;
  const parts = spec.toLowerCase().split('+');
  const needMod = parts.includes('mod');
  const needShift = parts.includes('shift');
  const needAlt = parts.includes('alt');
  const keyPart = parts.filter((p) => !['mod', 'shift', 'alt'].includes(p))[0];
  if (!keyPart) return false;

  if (needMod && !isModKey(e)) return false;
  if (!needMod && isModKey(e) && keyPart !== 'tab') return false;
  if (needShift !== e.shiftKey) return false;
  if (needAlt !== e.altKey) return false;

  return keyToken(e).toLowerCase() === keyPart.toLowerCase();
}

export function resolveShortcutAction(e: KeyboardEvent): ShortcutAction | null {
  for (const item of SHORTCUT_ITEMS) {
    if (item.keys && matchShortcut(e, item.keys)) return item.action;
  }
  return null;
}

export function isTabSwitchAction(action: ShortcutAction): boolean {
  return /^tab[1-9]$/.test(action);
}

export function tabIndexFromAction(action: ShortcutAction): number | null {
  const match = /^tab([1-9])$/.exec(action);
  return match ? Number(match[1]) - 1 : null;
}
