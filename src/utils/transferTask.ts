export type TransferTaskStatus =
  | 'queued'
  | 'transferring'
  | 'paused'
  | 'success'
  | 'error'
  | 'cancelled';

export interface TransferTask {
  id: string;
  name: string;
  progress: number;
  type: 'upload' | 'download';
  status: TransferTaskStatus;
  localPath: string;
  remotePath: string;
  sessionId: string;
  error?: string;
}

export function isTransferCancelled(err: unknown): boolean {
  return String(err).toLowerCase().includes('cancel');
}
