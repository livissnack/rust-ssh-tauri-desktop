import { ref, computed, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import { confirm } from '../utils/confirm.ts';
import { t } from '../utils/i18n.ts';
import {
  SFTP_EDIT_MAX_BYTES,
  type SftpEditorEncoding,
  type SftpFileRevision,
  revisionsEqual,
} from '../utils/sftpEditor.ts';
import type { SftpFileDetail } from '../components/SftpFileDialog.vue';

interface SftpListFile {
  name: string;
  is_dir: boolean;
  size?: number;
}

export function useSftpEditor(options: {
  activeSessionId: Ref<string | null>;
  refreshLocalFiles: () => void | Promise<void>;
  refreshRemoteFiles: () => void | Promise<void>;
  getContextFilePath: (source: 'local' | 'remote', file: { name: string }) => string;
}) {
  const visible = ref(false);
  const loading = ref(false);
  const saving = ref(false);
  const readonly = ref(false);
  const source = ref<'local' | 'remote'>('local');
  const path = ref('');
  const fileName = ref('');
  const fileSize = ref(0);
  const content = ref('');
  const originalContent = ref('');
  const encoding = ref<SftpEditorEncoding>('utf-8');
  const savedAt = ref<number | null>(null);
  const baselineRevision = ref<SftpFileRevision | null>(null);

  const dirty = computed(() => !readonly.value && content.value !== originalContent.value);

  const fetchRevision = async (
    src: 'local' | 'remote',
    filePath: string,
  ): Promise<SftpFileRevision> => {
    if (src === 'local') {
      const detail = await invoke<SftpFileDetail>('get_local_file_info', { path: filePath });
      return { size: detail.size, modifiedAt: detail.modifiedAt };
    }
    const detail = await invoke<SftpFileDetail>('get_remote_file_info', {
      sessionId: options.activeSessionId.value,
      path: filePath,
    });
    return { size: detail.size, modifiedAt: detail.modifiedAt };
  };

  const readFile = async (src: 'local' | 'remote', filePath: string) => {
    const enc = encoding.value;
    if (src === 'local') {
      return invoke<string>('read_local_file', { path: filePath, encoding: enc });
    }
    return invoke<string>('read_remote_file', {
      sessionId: options.activeSessionId.value,
      path: filePath,
      encoding: enc,
    });
  };

  const writeFile = async (filePath: string, text: string) => {
    const enc = encoding.value;
    if (source.value === 'local') {
      await invoke('write_local_file', { path: filePath, content: text, encoding: enc });
      await options.refreshLocalFiles();
    } else {
      await invoke('write_remote_file', {
        sessionId: options.activeSessionId.value,
        path: filePath,
        content: text,
        encoding: enc,
      });
      await options.refreshRemoteFiles();
    }
  };

  const reloadContent = async () => {
    loading.value = true;
    try {
      content.value = await readFile(source.value, path.value);
      originalContent.value = content.value;
      baselineRevision.value = await fetchRevision(source.value, path.value);
      savedAt.value = null;
    } catch (err) {
      toast.error(t('sftp.readFileFailed', { err: String(err) }));
    } finally {
      loading.value = false;
    }
  };

  const openSftpEditor = async (
    src: 'local' | 'remote',
    file: SftpListFile,
    opts?: { readonly?: boolean },
  ) => {
    if (file.is_dir) return;

    const size = file.size ?? 0;
    if (size > SFTP_EDIT_MAX_BYTES) {
      toast.warning(t('sftp.fileTooLarge'));
      return;
    }

    const filePath = options.getContextFilePath(src, file);
    readonly.value = !!opts?.readonly;
    source.value = src;
    path.value = filePath;
    fileName.value = file.name;
    fileSize.value = size;
    content.value = '';
    originalContent.value = '';
    encoding.value = 'utf-8';
    savedAt.value = null;
    baselineRevision.value = null;
    visible.value = true;
    loading.value = true;

    try {
      content.value = await readFile(src, filePath);
      originalContent.value = content.value;
      baselineRevision.value = await fetchRevision(src, filePath);
    } catch (err) {
      visible.value = false;
      toast.error(t('sftp.readFileFailed', { err: String(err) }));
    } finally {
      loading.value = false;
    }
  };

  const closeSftpEditor = async () => {
    if (dirty.value) {
      const ok = await confirm.warning(t('sftp.discardConfirm'), t('sftp.discardTitle'));
      if (!ok) return;
    }
    visible.value = false;
    readonly.value = false;
    savedAt.value = null;
  };

  const ensureCanSave = async (): Promise<boolean> => {
    if (!baselineRevision.value) return true;
    try {
      const current = await fetchRevision(source.value, path.value);
      if (revisionsEqual(current, baselineRevision.value)) return true;

      const overwrite = await confirm.warning(
        t('sftp.fileChangedOverwrite'),
        t('sftp.fileChangedTitle'),
      );
      if (overwrite) return true;

      const reload = await confirm.warning(
        t('sftp.fileChangedReload'),
        t('sftp.fileChangedTitle'),
      );
      if (reload) {
        await reloadContent();
      }
      return false;
    } catch (err) {
      toast.error(t('sftp.revisionCheckFailed', { err: String(err) }));
      return false;
    }
  };

  const saveSftpEditor = async (closeAfter = false) => {
    if (readonly.value) return;
    if (saving.value || loading.value || !dirty.value) return;

    const allowed = await ensureCanSave();
    if (!allowed) return;

    saving.value = true;
    try {
      await writeFile(path.value, content.value);
      originalContent.value = content.value;
      baselineRevision.value = await fetchRevision(source.value, path.value);
      savedAt.value = Date.now();
      toast.success(t('sftp.fileSaved'));
      if (closeAfter) {
        visible.value = false;
      }
    } catch (err) {
      toast.error(t('sftp.saveFileFailed', { err: String(err) }));
    } finally {
      saving.value = false;
    }
  };

  const setEncoding = async (next: SftpEditorEncoding) => {
    if (next === encoding.value) return;
    if (dirty.value) {
      const ok = await confirm.warning(t('sftp.encodingChangeConfirm'), t('sftp.discardTitle'));
      if (!ok) return;
    }
    encoding.value = next;
    await reloadContent();
  };

  return {
    sftpEditorVisible: visible,
    sftpEditorLoading: loading,
    sftpEditorSaving: saving,
    sftpEditorReadonly: readonly,
    sftpEditorSource: source,
    sftpEditorPath: path,
    sftpEditorFileName: fileName,
    sftpEditorFileSize: fileSize,
    sftpEditorContent: content,
    sftpEditorEncoding: encoding,
    sftpEditorSavedAt: savedAt,
    sftpEditorDirty: dirty,
    openSftpEditor,
    closeSftpEditor,
    saveSftpEditor,
    setSftpEditorEncoding: setEncoding,
  };
}
