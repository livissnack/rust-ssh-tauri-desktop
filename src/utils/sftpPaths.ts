/** Pure SFTP path helpers (no Vue reactivity). */

export function joinLocalPath(base: string, name: string): string {
  const normalized = base.replace(/[/\\]$/, '');
  const sep = normalized.includes('\\') || /^[A-Za-z]:/.test(normalized) ? '\\' : '/';
  if (normalized.endsWith(':')) return `${normalized}\\${name}`;
  return `${normalized}${sep}${name}`;
}

export function joinRemotePath(base: string, name: string): string {
  const normalized = base.replace(/\/$/, '');
  return normalized ? `${normalized}/${name}` : `/${name}`;
}

export function getPaneBasePath(source: 'local' | 'remote', localPath: string, remotePath: string): string {
  return source === 'local' ? localPath : remotePath.replace(/\/$/, '') || '/';
}

export function buildPathInPane(
  source: 'local' | 'remote',
  name: string,
  localPath: string,
  remotePath: string,
): string {
  return source === 'local'
    ? joinLocalPath(localPath, name)
    : joinRemotePath(remotePath, name);
}

export function getContextFilePath(
  source: 'local' | 'remote',
  file: { name: string },
  localPath: string,
  remotePath: string,
): string {
  if (source === 'local') return joinLocalPath(localPath, file.name);
  return joinRemotePath(remotePath, file.name);
}
