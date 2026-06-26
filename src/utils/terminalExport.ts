import type { Terminal } from '@xterm/xterm';
import { save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

export function serializeTerminalBuffer(term: Terminal): string {
  const buffer = term.buffer.active;
  const lines: string[] = [];
  for (let i = 0; i < buffer.length; i++) {
    const line = buffer.getLine(i);
    lines.push(line?.translateToString(true) ?? '');
  }
  while (lines.length && lines[lines.length - 1] === '') {
    lines.pop();
  }
  return lines.join('\n');
}

export async function exportTerminalOutput(
  term: Terminal,
  defaultName: string,
): Promise<boolean> {
  const content = serializeTerminalBuffer(term);
  if (!content.trim()) return false;

  const path = await save({
    title: 'Export terminal output',
    filters: [{ name: 'Text', extensions: ['txt', 'log'] }],
    defaultPath: defaultName,
  });
  if (!path) return false;

  await invoke('write_text_file', { path, content });
  return true;
}
