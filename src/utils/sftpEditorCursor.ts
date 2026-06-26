const STORAGE_KEY = 'sftp-editor-cursors';

interface CursorPos {
  line: number;
  col: number;
}

type CursorMap = Record<string, CursorPos>;

function readMap(): CursorMap {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as CursorMap) : {};
  } catch {
    return {};
  }
}

function writeMap(map: CursorMap) {
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(map));
  } catch {
    /* ignore quota errors */
  }
}

export function loadSftpEditorCursor(path: string): CursorPos | null {
  if (!path) return null;
  return readMap()[path] ?? null;
}

export function saveSftpEditorCursor(path: string, line: number, col: number) {
  if (!path) return;
  const map = readMap();
  map[path] = { line, col };
  writeMap(map);
}
