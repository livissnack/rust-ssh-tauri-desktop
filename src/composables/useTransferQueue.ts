import { ref, computed, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import { t } from '../utils/i18n.ts';
import { type TransferTask, isTransferCancelled } from '../utils/transferTask.ts';

const MAX_CONCURRENT_TRANSFERS = 2;

export function useTransferQueue(options: {
  activeSessionId: Ref<string | null>;
  refreshLocalFiles: () => void | Promise<void>;
  refreshRemoteFiles: () => void | Promise<void>;
}) {
  const transferTasks = ref<TransferTask[]>([]);

  const hasActiveTasks = computed(() =>
    transferTasks.value.some((task) => task.status === 'transferring'),
  );

  const pumpTransferQueue = () => {
    const running = transferTasks.value.filter((task) => task.status === 'transferring').length;
    const slots = MAX_CONCURRENT_TRANSFERS - running;
    if (slots <= 0) return;

    const waiting = transferTasks.value.filter((task) => task.status === 'queued');
    for (let i = 0; i < Math.min(slots, waiting.length); i++) {
      void runTransferTask(waiting[i]);
    }
  };

  const runTransferTask = async (task: TransferTask) => {
    if (task.status !== 'queued' && task.status !== 'transferring' && task.status !== 'paused') {
      return;
    }
    if (task.status === 'queued') {
      const running = transferTasks.value.filter((item) => item.status === 'transferring').length;
      if (running >= MAX_CONCURRENT_TRANSFERS) return;
    }
    task.status = 'transferring';
    task.error = undefined;
    try {
      await invoke(task.type === 'upload' ? 'sftp_upload' : 'sftp_download', {
        sessionId: task.sessionId,
        localPath: task.localPath,
        remotePath: task.remotePath,
        taskId: task.id,
      });
      if (task.status === 'cancelled') return;
      task.status = 'success';
      task.progress = 100;
      setTimeout(() => {
        transferTasks.value = transferTasks.value.filter((item) => item.id !== task.id);
      }, 2000);
      await options.refreshLocalFiles();
      await options.refreshRemoteFiles();
    } catch (err) {
      if (task.status === 'cancelled' || isTransferCancelled(err)) {
        transferTasks.value = transferTasks.value.filter((item) => item.id !== task.id);
        return;
      }
      task.status = 'error';
      task.error = String(err);
      toast.error(t('toast.transferFailed', { err: String(err) }));
    } finally {
      pumpTransferQueue();
    }
  };

  const startTransferFromPath = async (
    type: 'upload' | 'download',
    opts: { localPath: string; remotePath: string; name: string },
  ) => {
    if (!options.activeSessionId.value) return;
    const { localPath: localFilePath, remotePath: remoteFilePath, name } = opts;
    const task: TransferTask = {
      id: crypto.randomUUID(),
      name,
      progress: 0,
      type,
      status: 'queued',
      localPath: localFilePath,
      remotePath: remoteFilePath,
      sessionId: options.activeSessionId.value,
    };
    transferTasks.value.push(task);
    pumpTransferQueue();
  };

  const getTaskIcon = (task: TransferTask) => {
    if (task.status === 'error') return 'fas fa-exclamation-circle';
    if (task.status === 'success') return 'fas fa-check-circle';
    if (task.status === 'paused') return 'fas fa-pause-circle';
    if (task.status === 'queued') return 'fas fa-clock';
    return task.type === 'upload' ? 'fas fa-cloud-upload-alt' : 'fas fa-cloud-download-alt';
  };

  const pauseTask = async (taskId: string) => {
    const task = transferTasks.value.find((item) => item.id === taskId);
    if (!task || task.status !== 'transferring') return;
    try {
      await invoke('pause_transfer', { taskId });
      task.status = 'paused';
    } catch (err) {
      toast.error(t('sftp.pauseFailed', { err: String(err) }));
    }
  };

  const resumeTask = async (taskId: string) => {
    const task = transferTasks.value.find((item) => item.id === taskId);
    if (!task || task.status !== 'paused') return;
    try {
      await invoke('resume_transfer', { taskId });
      task.status = 'queued';
      pumpTransferQueue();
    } catch (err) {
      toast.error(t('sftp.resumeFailed', { err: String(err) }));
    }
  };

  const retryTask = (taskId: string) => {
    const task = transferTasks.value.find((item) => item.id === taskId);
    if (!task || task.status !== 'error') return;
    task.progress = 0;
    task.status = 'queued';
    pumpTransferQueue();
  };

  const cancelTask = async (taskId: string) => {
    const task = transferTasks.value.find((item) => item.id === taskId);
    if (!task) return;
    const wasQueued = task.status === 'queued';
    task.status = 'cancelled';
    try {
      if (!wasQueued) {
        await invoke('abort_transfer', { taskId });
      }
      transferTasks.value = transferTasks.value.filter((item) => item.id !== taskId);
      pumpTransferQueue();
    } catch (err) {
      console.error(err);
    }
  };

  const updateTaskProgress = (taskId: string, progress: number) => {
    const task = transferTasks.value.find((item) => item.id === taskId);
    if (task) task.progress = progress;
  };

  return {
    transferTasks,
    hasActiveTasks,
    startTransferFromPath,
    getTaskIcon,
    pauseTask,
    resumeTask,
    retryTask,
    cancelTask,
    updateTaskProgress,
  };
}
