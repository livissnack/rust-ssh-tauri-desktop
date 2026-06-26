export const SFTP_EDIT_MAX_BYTES = 2 * 1024 * 1024;

export type SftpEditorEncoding = 'utf-8' | 'gbk' | 'latin1';

export interface SftpFileRevision {
  size: number;
  modifiedAt?: number;
}

export function revisionsEqual(a: SftpFileRevision, b: SftpFileRevision): boolean {
  return a.size === b.size && a.modifiedAt === b.modifiedAt;
}
