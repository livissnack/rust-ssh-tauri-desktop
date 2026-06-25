<script setup lang="ts">
import {ref, computed, onMounted, onUnmounted, nextTick, watch, defineAsyncComponent} from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";
import {invoke} from "@tauri-apps/api/core";
import { homeDir } from '@tauri-apps/api/path';
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {toast} from './utils/toast.ts';
import {confirm} from './utils/confirm.ts';
import {throttle, formatSize} from "./utils/async";
import {applyTheme, defaultTheme} from "./utils/theme";
import { getTerminalTheme, applyTerminalTheme } from "./utils/terminalTheme";
import { LOCAL_SERVER_ID, isLocalSession, isSessionConnected, type OpenSession, type SessionStatus } from "./utils/session.ts";
import { beginPointerDrag } from "./utils/pointerDrag.ts";
import { detectAppPlatform } from "./utils/platform.ts";
import { useI18n, t } from "./utils/i18n.ts";

const { tr } = useI18n();

import Sidebar from "./components/Sidebar.vue";
import TerminalTabs from "./components/TerminalTabs.vue";
import WorkspaceHeader from "./components/WorkspaceHeader.vue";
import StatusBar from "./components/StatusBar.vue";
import TitleBar from "./components/TitleBar.vue";
import ServerModal from "./components/ServerModal.vue";
import SftpFileDialog, { type SftpFileDetail } from "./components/SftpFileDialog.vue";

const QuickCommandPanel = defineAsyncComponent(() => import("./components/QuickCommandPanel.vue"));
const AiAssistantPanel = defineAsyncComponent(() => import("./components/AiAssistantPanel.vue"));
const SyncSettings = defineAsyncComponent(() => import("./components/SyncSettings.vue"));
const ThemeSettings = defineAsyncComponent(() => import("./components/ThemeSettings.vue"));
const RedisManager = defineAsyncComponent(() => import("./components/RedisManager.vue"));
const ChatPanel = defineAsyncComponent(() => import("./components/ChatPanel.vue"));
const ApiDebuggerPanel = defineAsyncComponent({
  loader: () => import("./components/ApiDebuggerPanel.vue"),
  delay: 0,
  timeout: 30000,
  onError(error, retry, fail, attempts) {
    if (attempts <= 2) {
      retry();
      return;
    }
    console.error("Failed to load ApiDebuggerPanel:", error);
    fail();
  },
});

const panelMap: Record<string, any> = {
  'quick': QuickCommandPanel,
  'ai': AiAssistantPanel,
  'redis': RedisManager,
  'sync-settings': SyncSettings,
  'theme-settings': ThemeSettings,
  'chat': ChatPanel,
  'api': ApiDebuggerPanel,
};

const rightPanelComponent = computed(() => {
  return panelMap[rightPanelType.value] || null;
});

const servers = ref<any[]>(window.__INITIAL_SERVERS__ || []);
const activeId = ref<string | null>(null);
const openSessions = ref<OpenSession[]>([]);
const activeSessionId = ref<string | null>(null);
const showPassword = ref(false);
const sessionViewModes = ref<Record<string, 'terminal' | 'sftp'>>({});
type TerminalInstance = {
  term: Terminal;
  fitAddon: FitAddon;
  isLocal: boolean;
  backendReady: boolean;
  resizeObserver?: ResizeObserver;
};
const terminalMap = new Map<string, TerminalInstance>();

const markSessionBackendReady = (sessionId: string) => {
  const instance = terminalMap.get(sessionId);
  if (instance) instance.backendReady = true;
};
const onlineUserCount = ref(0);

const sessionStatuses = ref<Record<string, SessionStatus>>({});
const sessionErrors = ref<Record<string, string>>({});

const setSessionStatus = (sessionId: string, status: SessionStatus, error?: string) => {
  sessionStatuses.value = { ...sessionStatuses.value, [sessionId]: status };
  if (error !== undefined) {
    sessionErrors.value = { ...sessionErrors.value, [sessionId]: error };
  } else if (status === 'connected') {
    const next = { ...sessionErrors.value };
    delete next[sessionId];
    sessionErrors.value = next;
  }
};

const clearSessionState = (sessionId: string) => {
  const nextStatus = { ...sessionStatuses.value };
  const nextErrors = { ...sessionErrors.value };
  delete nextStatus[sessionId];
  delete nextErrors[sessionId];
  sessionStatuses.value = nextStatus;
  sessionErrors.value = nextErrors;
};

const activeSessionStatus = computed<SessionStatus>(() =>
  activeSessionId.value ? (sessionStatuses.value[activeSessionId.value] ?? 'idle') : 'idle',
);

const activeSessionError = computed(() =>
  activeSessionId.value ? sessionErrors.value[activeSessionId.value] : undefined,
);

const isActiveSessionConnected = computed(() =>
  isActiveLocalSession.value
    ? activeSessionStatus.value === 'connected'
    : isSessionConnected(activeSessionStatus.value),
);

const rightPanelVisible = ref(false);
const isModalOpen = ref(false);
const isEditing = ref(false);
const isSyncing = ref(false);

let unlisten: UnlistenFn | null = null;
let unlistenClosed: UnlistenFn | null = null;
let unlistenTransfer: UnlistenFn | null = null;
let unlistenSync: UnlistenFn | null = null;
let unlistenDragDrop: UnlistenFn | null = null;
/** Skip disconnect toast when user closes tab, reconnects, or app unmounts */
const suppressSshClosedToast = new Set<string>();
const transferTasks = ref<any[]>([]);
const isSftpInternalDragging = ref(false);
const sftpInternalDragKey = ref<string | null>(null);

const rightPanelType = ref<'quick' | 'ai' | 'redis' | 'history' | 'sync-settings' | 'theme-settings' | 'chat' | 'api'>('quick');

type RightPanelType = typeof rightPanelType.value;

const PANEL_WIDTH_STORAGE_KEY = 'right-panel-widths';
const LEGACY_PANEL_WIDTH_KEY = 'right-panel-width';

const DEFAULT_PANEL_WIDTHS: Record<RightPanelType, number> = {
  quick: 420,
  ai: 420,
  redis: 420,
  history: 420,
  'sync-settings': 420,
  'theme-settings': 400,
  chat: 420,
  api: 892,
};

const PANEL_MIN_WIDTHS: Partial<Record<RightPanelType, number>> = {
  api: 480,
  redis: 360,
};

const panelWidths = ref<Record<RightPanelType, number>>({ ...DEFAULT_PANEL_WIDTHS });

const panelWidth = computed(() =>
  panelWidths.value[rightPanelType.value] ?? DEFAULT_PANEL_WIDTHS[rightPanelType.value],
);

const getPanelMinWidth = (type: RightPanelType) =>
  PANEL_MIN_WIDTHS[type] ?? 300;

const localPath = ref("");
const remotePath = ref("");
const localFiles = ref<any[]>([]);
const remoteFiles = ref<any[]>([]);
const localFilesLoading = ref(false);
const remoteFilesLoading = ref(false);
const localFilesError = ref<string | null>(null);
const remoteFilesError = ref<string | null>(null);
const REMOTE_PATH_STORAGE_KEY = 'sftp-remote-paths';
const isDraggingOverLocal = ref(false);
const isDraggingOverRemote = ref(false);

const menuVisible = ref(false);
const menuPos = ref({x: 0, y: 0});
const contextFile = ref<any>(null);
const contextSource = ref<'local' | 'remote' | null>(null);
const contextTarget = ref<'file' | 'pane'>('file');

interface SftpClipboardItem {
  source: 'local' | 'remote';
  path: string;
  name: string;
  isDir: boolean;
  mode: 'copy' | 'cut';
}

const sftpClipboard = ref<SftpClipboardItem | null>(null);

type SftpMenuAction =
  | 'copy' | 'cut' | 'paste' | 'copyPath' | 'refresh' | 'newFile' | 'newFolder'
  | 'transfer' | 'info' | 'openExplorer' | 'rename' | 'chmod' | 'delete';

const sftpDialogVisible = ref(false);
const sftpDialogMode = ref<'info' | 'rename' | 'chmod' | 'createFile' | 'createFolder'>('info');
const sftpDialogLoading = ref(false);
const sftpDialogDetail = ref<SftpFileDetail | null>(null);
const sftpDialogInput = ref('');

const newHost = ref({
  id: "", name: "", host: "", username: "root", port: 22,
  auth_type: "password", password: "", private_key_path: "", jump_host_id: null
});

const currentViewMode = computed(() => {
  if (!activeSessionId.value) return 'terminal';
  return sessionViewModes.value[activeSessionId.value] || 'terminal';
});

const activeOpenSession = computed(() =>
  openSessions.value.find(s => s.id === activeSessionId.value) ?? null
);

const isActiveLocalSession = computed(() => isLocalSession(activeOpenSession.value));

const currentServer = computed(() => servers.value.find(s => s.id === activeId.value));

const activeTabServer = computed(() => {
  const session = activeOpenSession.value;
  if (!session || isLocalSession(session)) return null;
  return servers.value.find((s) => s.id === session.serverId) ?? null;
});

const hasActiveTasks = computed(() =>
    transferTasks.value.some(t => t.status === 'transferring')
);

const handleToggle = (type: RightPanelType) => {
  if (rightPanelVisible.value && rightPanelType.value === type) {
    rightPanelVisible.value = false;
  } else {
    rightPanelType.value = type;
    rightPanelVisible.value = true;
  }
}

const toggleRightPanel = throttle(handleToggle, 300);

const canPaste = computed(() => {
  if (!contextSource.value || !sftpClipboard.value) return false;
  return sftpClipboard.value.source === contextSource.value;
});

const openContextMenu = (e: MouseEvent, source: 'local' | 'remote', target: 'file' | 'pane') => {
  contextTarget.value = target;
  contextSource.value = source;
  menuVisible.value = true;
  const menuWidth = 210;
  const menuHeight = target === 'pane' ? 180 : 420;
  let x = e.clientX;
  let y = e.clientY;
  if (x + menuWidth > window.innerWidth) x -= menuWidth;
  if (y + menuHeight > window.innerHeight) y -= menuHeight;
  menuPos.value = {x, y};

  const closeMenu = () => {
    menuVisible.value = false;
    window.removeEventListener('click', closeMenu);
  };
  setTimeout(() => {
    window.addEventListener('click', closeMenu);
  }, 10);
};

const handleContextMenu = (e: MouseEvent, file: any, source: 'local' | 'remote') => {
  e.preventDefault();
  e.stopPropagation();
  if (file.name === '..') return;
  contextFile.value = file;
  openContextMenu(e, source, 'file');
};

const handlePaneContextMenu = (e: MouseEvent, source: 'local' | 'remote') => {
  e.preventDefault();
  contextFile.value = null;
  openContextMenu(e, source, 'pane');
};

const joinLocalPath = (base: string, name: string) => {
  const normalized = base.replace(/[/\\]$/, '');
  const sep = normalized.includes('\\') || /^[A-Za-z]:/.test(normalized) ? '\\' : '/';
  if (normalized.endsWith(':')) return `${normalized}\\${name}`;
  return `${normalized}${sep}${name}`;
};

const joinRemotePath = (base: string, name: string) => {
  const normalized = base.replace(/\/$/, '');
  return normalized ? `${normalized}/${name}` : `/${name}`;
};

const getContextFilePath = (source: 'local' | 'remote', file: any) => {
  if (source === 'local') return joinLocalPath(localPath.value, file.name);
  return joinRemotePath(remotePath.value, file.name);
};

const getPaneBasePath = (source: 'local' | 'remote') =>
  source === 'local' ? localPath.value : remotePath.value.replace(/\/$/, '') || '/';

const buildPathInPane = (source: 'local' | 'remote', name: string) =>
  source === 'local' ? joinLocalPath(localPath.value, name) : joinRemotePath(remotePath.value, name);

const copyTextToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(t('toast.pathCopied'));
  } catch {
    toast.error(t('toast.copyFailed'));
  }
};

const pasteClipboard = async (source: 'local' | 'remote') => {
  const item = sftpClipboard.value;
  if (!item || item.source !== source) {
    toast.warning(t('toast.nothingToPaste'));
    return;
  }

  const destPath = buildPathInPane(source, item.name);
  if (destPath === item.path) {
    if (item.mode === 'cut') {
      sftpClipboard.value = null;
    } else {
      toast.warning(t('toast.samePath'));
    }
    return;
  }

  try {
    if (item.mode === 'cut') {
      if (source === 'local') {
        await invoke('move_local_path', {src: item.path, dest: destPath, isDir: item.isDir});
        await refreshLocalFiles();
      } else {
        await invoke('move_remote_path', {
          sessionId: activeSessionId.value,
          src: item.path,
          dest: destPath,
        });
        await refreshRemoteFiles();
      }
      sftpClipboard.value = null;
      toast.success(t('toast.moveSuccess'));
    } else {
      if (source === 'local') {
        await invoke('copy_local_path', {src: item.path, dest: destPath});
        await refreshLocalFiles();
      } else {
        await invoke('copy_remote_path', {
          sessionId: activeSessionId.value,
          src: item.path,
          dest: destPath,
        });
        await refreshRemoteFiles();
      }
      toast.success(t('toast.pasteSuccess'));
    }
  } catch (err) {
    toast.error(t('toast.pasteFailed', { err: String(err) }));
  }
};

const loadFileDetail = async (source: 'local' | 'remote', file: any): Promise<SftpFileDetail> => {
  const path = getContextFilePath(source, file);
  if (source === 'local') {
    return await invoke('get_local_file_info', {path});
  }
  return await invoke('get_remote_file_info', {sessionId: activeSessionId.value, path});
};

const openSftpDialog = (
  mode: 'info' | 'rename' | 'chmod' | 'createFile' | 'createFolder',
  source: 'local' | 'remote',
  file: any | null,
  inputValue = ''
) => {
  sftpDialogMode.value = mode;
  sftpDialogInput.value = inputValue;
  sftpDialogDetail.value = file
    ? {
        path: getContextFilePath(source, file),
        name: file.name,
        isDir: file.is_dir,
        size: file.size ?? 0,
      }
    : {
        path: getPaneBasePath(source),
        name: '',
        isDir: mode === 'createFolder',
        size: 0,
      };
  sftpDialogVisible.value = true;
};

const closeSftpDialog = () => {
  sftpDialogVisible.value = false;
  sftpDialogLoading.value = false;
};

const handleSftpDialogConfirm = async (value: string) => {
  if (!contextSource.value) return;
  const source = contextSource.value;
  const file = contextFile.value;

  if (sftpDialogMode.value === 'createFile' || sftpDialogMode.value === 'createFolder') {
    const name = value.trim();
    if (!name) return;
    if (/[\\/:*?"<>|]/.test(name)) {
      toast.error(t('toast.invalidName'));
      return;
    }
    const path = buildPathInPane(source, name);
    const isDir = sftpDialogMode.value === 'createFolder';
    try {
      if (source === 'local') {
        await invoke('create_local_path', {path, isDir});
        await refreshLocalFiles();
      } else {
        await invoke('create_remote_path', {
          sessionId: activeSessionId.value,
          path,
          isDir,
        });
        await refreshRemoteFiles();
      }
      toast.success(isDir ? t('toast.folderCreated') : t('toast.fileCreated'));
      closeSftpDialog();
    } catch (err) {
      toast.error(t('toast.createFailed', { err: String(err) }));
    }
    return;
  }

  if (!file) return;
  const oldPath = getContextFilePath(source, file);

  if (sftpDialogMode.value === 'rename') {
    const newName = value.trim();
    if (!newName || newName === file.name) {
      closeSftpDialog();
      return;
    }
    if (/[\\/:*?"<>|]/.test(newName)) {
      toast.error(t('toast.invalidFileName'));
      return;
    }
    try {
      if (source === 'local') {
        await invoke('rename_local_file', {
          oldPath,
          newPath: joinLocalPath(localPath.value, newName),
        });
        await refreshLocalFiles();
      } else {
        await invoke('rename_remote_file', {
          sessionId: activeSessionId.value,
          oldPath,
          newPath: joinRemotePath(remotePath.value, newName),
        });
        await refreshRemoteFiles();
      }
      toast.success(t('toast.renameSuccess'));
      closeSftpDialog();
    } catch (err) {
      toast.error(t('toast.renameFailed', { err: String(err) }));
    }
    return;
  }

  if (sftpDialogMode.value === 'chmod' && source === 'remote') {
    try {
      await invoke('set_remote_file_permissions', {
        sessionId: activeSessionId.value,
        path: oldPath,
        mode: value.trim(),
      });
      await refreshRemoteFiles();
      toast.success(t('toast.chmodSuccess'));
      closeSftpDialog();
    } catch (err) {
      toast.error(t('toast.chmodFailed', { err: String(err) }));
    }
  }
};

const handleMenuAction = async (action: SftpMenuAction) => {
  if (!contextSource.value) return;
  const source = contextSource.value;
  const file = contextFile.value;
  menuVisible.value = false;

  if (action === 'refresh') {
    if (source === 'local') await refreshLocalFiles();
    else await refreshRemoteFiles();
    return;
  }

  if (action === 'paste') {
    await pasteClipboard(source);
    return;
  }

  if (action === 'newFile') {
    openSftpDialog('createFile', source, null, t('sftp.defaultNewFile'));
    return;
  }

  if (action === 'newFolder') {
    openSftpDialog('createFolder', source, null, t('sftp.defaultNewFolder'));
    return;
  }

  if (contextTarget.value === 'pane') return;
  if (!file) return;

  if (action === 'copy') {
    sftpClipboard.value = {
      source,
      path: getContextFilePath(source, file),
      name: file.name,
      isDir: file.is_dir,
      mode: 'copy',
    };
    toast.info(t('toast.copied'));
  } else if (action === 'cut') {
    sftpClipboard.value = {
      source,
      path: getContextFilePath(source, file),
      name: file.name,
      isDir: file.is_dir,
      mode: 'cut',
    };
    toast.info(t('toast.cut'));
  } else if (action === 'copyPath') {
    await copyTextToClipboard(getContextFilePath(source, file));
  } else if (action === 'transfer') {
    const type = source === 'local' ? 'upload' : 'download';
    await startTransfer(type, file);
  } else if (action === 'info') {
    openSftpDialog('info', source, file);
    sftpDialogLoading.value = true;
    try {
      sftpDialogDetail.value = await loadFileDetail(source, file);
    } catch (err) {
      closeSftpDialog();
      toast.error(t('toast.fileInfoFailed', { err: String(err) }));
    } finally {
      sftpDialogLoading.value = false;
    }
  } else if (action === 'openExplorer') {
    try {
      await invoke('reveal_in_file_manager', {path: getContextFilePath('local', file)});
    } catch (err) {
      toast.error(t('toast.explorerFailed', { err: String(err) }));
    }
  } else if (action === 'rename') {
    openSftpDialog('rename', source, file, file.name);
  } else if (action === 'chmod') {
    openSftpDialog('chmod', source, file);
    sftpDialogLoading.value = true;
    try {
      const detail = await loadFileDetail('remote', file);
      sftpDialogDetail.value = detail;
      sftpDialogInput.value = detail.permissions || '644';
    } catch (err) {
      closeSftpDialog();
      toast.error(t('toast.chmodFetchFailed', { err: String(err) }));
    } finally {
      sftpDialogLoading.value = false;
    }
  } else if (action === 'delete') {
    const ok = await confirm.error(
        t('toast.deleteConfirm', {
          side: source === 'local' ? t('common.local') : t('common.remote'),
          name: file.name,
        }),
        t('toast.deleteTitle')
    );

    if (ok) {
      try {
        if (source === 'remote') {
          const path = joinRemotePath(remotePath.value, file.name);
          await invoke("delete_remote_file", {sessionId: activeSessionId.value, path, isDir: file.is_dir});
          await refreshRemoteFiles();
          toast.success(t('toast.deleteSuccess'));
        } else {
          const path = getContextFilePath('local', file);
          await invoke("delete_local_file", {path, isDir: file.is_dir});
          await refreshLocalFiles();
          toast.success(t('toast.deleteSuccess'));
        }
      } catch (err) {
        toast.error(t('toast.deleteFailed', { err: String(err) }));
      }
    }
  }
};

const getSftpDropPaneAtLogicalPoint = (x: number, y: number): 'local' | 'remote' | null => {
  const remoteEl = document.querySelector('.sftp-manager .remote-pane .file-list');
  const localEl = document.querySelector('.sftp-manager .local-pane .file-list');
  if (remoteEl) {
    const r = remoteEl.getBoundingClientRect();
    if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return 'remote';
  }
  if (localEl) {
    const r = localEl.getBoundingClientRect();
    if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return 'local';
  }
  return null;
};

const getSftpDropPaneAtPhysicalPoint = (physicalX: number, physicalY: number): 'local' | 'remote' | null => {
  const scale = window.devicePixelRatio || 1;
  return getSftpDropPaneAtLogicalPoint(physicalX / scale, physicalY / scale);
};

const clearDragOverHighlight = () => {
  isDraggingOverLocal.value = false;
  isDraggingOverRemote.value = false;
};

const updateDragOverHighlightFromPhysicalPoint = (physicalX: number, physicalY: number) => {
  const pane = getSftpDropPaneAtPhysicalPoint(physicalX, physicalY);
  isDraggingOverLocal.value = pane === 'local';
  isDraggingOverRemote.value = pane === 'remote';
};

const onSftpFilePointerDown = (e: PointerEvent, file: any, source: 'local' | 'remote') => {
  if (file.name === '..') return;

  beginPointerDrag(e, {
    onActivate: () => {
      isSftpInternalDragging.value = true;
      sftpInternalDragKey.value = `${source}:${file.name}`;
    },
    onMove: (x, y) => {
      const pane = getSftpDropPaneAtLogicalPoint(x, y);
      isDraggingOverLocal.value = pane === 'local';
      isDraggingOverRemote.value = pane === 'remote';
    },
    onFinish: async (x, y, activated) => {
      isSftpInternalDragging.value = false;
      sftpInternalDragKey.value = null;
      const pane = getSftpDropPaneAtLogicalPoint(x, y);
      clearDragOverHighlight();
      if (!activated) return;
      if (source === 'local' && pane === 'remote') {
        await startTransfer('upload', file);
      } else if (source === 'remote' && pane === 'local') {
        await startTransfer('download', file);
      }
    },
    onCancel: () => {
      isSftpInternalDragging.value = false;
      sftpInternalDragKey.value = null;
      clearDragOverHighlight();
    },
  });
};

const handleOsFileDrop = async (paths: string[], pane: 'local' | 'remote') => {
  if (!activeSessionId.value || currentViewMode.value !== 'sftp') return;

  if (pane !== 'remote') {
    toast.error(t('toast.dropToRemote'));
    return;
  }

  const remoteBase = remotePath.value.replace(/\/$/, '');
  for (const filePath of paths) {
    const name = filePath.split(/[/\\]/).pop();
    if (!name) continue;
    await startTransferFromPath('upload', {
      localPath: filePath,
      remotePath: `${remoteBase}/${name}`,
      name,
    });
  }
};

const setupNativeDragDrop = async () => {
  unlistenDragDrop = await getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload;
    if (currentViewMode.value !== 'sftp') return;

    if (payload.type === 'enter' || payload.type === 'over') {
      updateDragOverHighlightFromPhysicalPoint(payload.position.x, payload.position.y);
    } else if (payload.type === 'leave') {
      clearDragOverHighlight();
    } else if (payload.type === 'drop') {
      clearDragOverHighlight();
      if (!activeSessionId.value || !payload.paths?.length) return;
      const pane = getSftpDropPaneAtPhysicalPoint(payload.position.x, payload.position.y);
      if (pane) {
        void handleOsFileDrop(payload.paths, pane);
      }
    }
  });
};

const handleFileDblClick = async (file: any, type: 'local' | 'remote') => {
  if (!file.is_dir && file.name !== '..') return;

  const isRemote = type === 'remote';
  let currentPath = isRemote ? remotePath.value : localPath.value;

  currentPath = currentPath.replace(/[/\\]$/, '');

  if (file.name === '..') {
    let parts = currentPath.split(/[/\\]/).filter(p => p !== "");
    if (isRemote) {
      parts.pop();
      currentPath = '/' + parts.join('/');
    } else {
      parts.pop();
      currentPath = parts.join('\\');
      if (currentPath.length === 2 && currentPath.endsWith(':')) currentPath += '\\';
    }
  } else {
    const separator = isRemote ? '/' : '\\';
    if (!isRemote && currentPath.endsWith(':')) currentPath += '\\';

    const base = currentPath.endsWith(separator) ? currentPath : currentPath + separator;
    currentPath = base + file.name;
  }

  if (!currentPath) currentPath = isRemote ? "/" : "C:\\";

  try {
    if (isRemote) {
      remotePath.value = currentPath;
      await refreshRemoteFiles();
    } else {
      localPath.value = currentPath;
      await refreshLocalFiles();
    }
  } catch (err) {
    toast.error(t('toast.cdFailed', { err: String(err) }));
  }
};

const startTransferFromPath = async (
  type: 'upload' | 'download',
  opts: { localPath: string; remotePath: string; name: string }
) => {
  const {localPath: localFilePath, remotePath: remoteFilePath, name} = opts;
  const taskId = Math.random().toString(36).substring(7);
  transferTasks.value.push({id: taskId, name, progress: 0, type, status: 'transferring'});
  try {
    await invoke(type === 'upload' ? "sftp_upload" : "sftp_download", {
      sessionId: activeSessionId.value,
      localPath: localFilePath,
      remotePath: remoteFilePath,
      taskId
    });
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) {
      task.status = 'success';
      task.progress = 100;
      setTimeout(() => {
        transferTasks.value = transferTasks.value.filter(t => t.id !== taskId);
      }, 2000);
    }
    refreshLocalFiles();
    refreshRemoteFiles();
  } catch (err) {
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) task.status = 'error';
    toast.error(t('toast.transferFailed', { err: String(err) }));
  }
};

const startTransfer = async (type: 'upload' | 'download', file: any) => {
  if (file.is_dir || file.name === '..') {
    toast.error(t('toast.folderTransferUnsupported', {
      action: type === 'upload' ? t('toast.upload') : t('toast.download'),
    }));
    return;
  }
  const localBase = localPath.value.replace(/[/\\]$/, '');
  const remoteBase = remotePath.value.replace(/\/$/, '');
  const localFilePath = `${localBase}/${file.name}`;
  const remoteFilePath = `${remoteBase}/${file.name}`;
  await startTransferFromPath(type, {
    localPath: localFilePath,
    remotePath: remoteFilePath,
    name: file.name,
  });
};

const getDefaultRemotePath = (server: { id: string; username: string }) => {
  const username = server.username || 'root';
  try {
    const raw = localStorage.getItem(REMOTE_PATH_STORAGE_KEY);
    if (raw) {
      const map = JSON.parse(raw) as Record<string, string>;
      const stored = map[server.id];
      if (stored) {
        if (username === 'root' && stored === '/home/root') return '/root';
        return stored;
      }
    }
  } catch { /* ignore */ }
  return username === 'root' ? '/root' : `/home/${username}`;
};

const persistRemotePath = (serverId: string, path: string) => {
  try {
    const raw = localStorage.getItem(REMOTE_PATH_STORAGE_KEY);
    const map = raw ? (JSON.parse(raw) as Record<string, string>) : {};
    map[serverId] = path;
    localStorage.setItem(REMOTE_PATH_STORAGE_KEY, JSON.stringify(map));
  } catch { /* ignore */ }
};

const syncRemotePathForSession = (sessionId: string | null) => {
  if (!sessionId) return;
  const session = openSessions.value.find((s) => s.id === sessionId);
  if (!session || isLocalSession(session)) return;
  const server = servers.value.find((s) => s.id === session.serverId);
  if (!server) return;
  remotePath.value = getDefaultRemotePath(server);
};

const performConnect = async (sessionId: string, server: { id: string; name: string }) => {
  setSessionStatus(sessionId, 'connecting');
  await initTerminal(sessionId, false);
  const instance = terminalMap.get(sessionId);
  if (instance) instance.backendReady = false;
  suppressSshClosedToast.add(sessionId);
  await invoke('disconnect_ssh', { sessionId }).catch(() => {});

  try {
    await invoke('connect_ssh', { serverId: server.id, sessionId });
    markSessionBackendReady(sessionId);
    await syncTerminalSize(sessionId);
    focusTerminal(sessionId);
    setSessionStatus(sessionId, 'connected');
    syncRemotePathForSession(sessionId);
  } catch (err) {
    const msg = String(err);
    setSessionStatus(sessionId, 'failed', msg);
    toast.error(t('toast.connectFailed', { msg }));
  }
};

const connectToServer = async (serverId?: string) => {
  let targetServerId = serverId;
  let sessionId: string | undefined;

  if (!targetServerId) {
    const session = activeOpenSession.value;
    if (session && !isLocalSession(session)) {
      targetServerId = session.serverId;
      sessionId = session.id;
    } else {
      targetServerId = activeId.value ?? undefined;
    }
  }

  if (!targetServerId) return;
  const server = servers.value.find((s) => s.id === targetServerId);
  if (!server) return;

  activeId.value = server.id;

  if (!sessionId) {
    const existing = openSessions.value.find((s) => s.id === server.id);
    sessionId = existing?.id ?? server.id;
  }

  const status = sessionStatuses.value[sessionId];
  if (status === 'connected') {
    activeSessionId.value = sessionId;
    await focusTerminal(sessionId);
    return;
  }

  if (!openSessions.value.find((s) => s.id === sessionId)) {
    openSessions.value.push({ id: sessionId, serverId: server.id, name: server.name });
    sessionViewModes.value[sessionId] = 'terminal';
  }
  activeSessionId.value = sessionId;
  await performConnect(sessionId, server);
};

const reconnectSession = async (sessionId?: string) => {
  const id = sessionId ?? activeSessionId.value;
  if (!id) return;
  const session = openSessions.value.find((s) => s.id === id);
  if (!session || isLocalSession(session)) return;
  const server = servers.value.find((s) => s.id === session.serverId);
  if (!server) return;
  activeSessionId.value = id;
  activeId.value = server.id;
  await performConnect(id, server);
  if (sessionStatuses.value[id] === 'connected') {
    toast.success(t('toast.reconnected'));
  }
};

const showSessionOverlay = (sessionId: string) => {
  const status = sessionStatuses.value[sessionId];
  return status === 'failed' || status === 'disconnected';
};

const getSessionOverlayMessage = (sessionId: string) =>
  sessionErrors.value[sessionId] ?? (
    sessionStatuses.value[sessionId] === 'failed'
      ? t('session.defaultFailed')
      : t('session.defaultDisconnected')
  );

const isTerminalContainerVisible = (sessionId: string) => {
  const container = document.getElementById(`terminal-${sessionId}`);
  return !!container && container.offsetWidth > 0 && container.offsetHeight > 0;
};

const measureTerminalSize = async (sessionId: string) => {
  await nextTick();
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

  const instance = terminalMap.get(sessionId);
  if (!instance || !isTerminalContainerVisible(sessionId)) {
    return { rows: 24, cols: 120 };
  }

  instance.fitAddon.fit();
  return {
    rows: Math.max(instance.term.rows, 8),
    cols: Math.max(instance.term.cols, 40),
  };
};

const syncTerminalSize = async (sessionId: string) => {
  const instance = terminalMap.get(sessionId);
  if (!instance || !isTerminalContainerVisible(sessionId)) return;

  instance.fitAddon.fit();
  if (!instance.backendReady) return;

  const rows = Math.max(instance.term.rows, 8);
  const cols = Math.max(instance.term.cols, 40);
  if (rows === 0 || cols === 0) return;

  await invoke("resize_ssh", { sessionId, rows, cols }).catch(console.error);
};

const scheduleLocalTerminalSizeSync = async (sessionId: string) => {
  await syncTerminalSize(sessionId);
  for (const delay of [50, 200]) {
    await new Promise<void>((resolve) => setTimeout(resolve, delay));
    await syncTerminalSize(sessionId);
  }
};

const attachTerminalResizeObserver = (sessionId: string, container: HTMLElement) => {
  const observer = new ResizeObserver(() => {
    void syncTerminalSize(sessionId);
  });
  observer.observe(container);
  return observer;
};

const initTerminal = async (sessionId: string, isLocal = false) => {
  if (terminalMap.has(sessionId)) {
    await nextTick();
    terminalMap.get(sessionId)?.fitAddon.fit();
    return;
  }
  const term = new Terminal({
    cursorBlink: true,
    cursorStyle: 'bar',
    fontSize: 14,
    lineHeight: isLocal ? 1 : 1.3,
    letterSpacing: 0,
    fontWeight: '400',
    fontWeightBold: '700',
    fontFamily: isLocal
      ? "'Cascadia Mono', 'Cascadia Code', 'Microsoft YaHei UI', Consolas, monospace"
      : "'Cascadia Mono', 'Cascadia Code', Consolas, 'SF Mono', Menlo, monospace",
    theme: getTerminalTheme(),
    allowProposedApi: true,
    drawBoldTextInBrightColors: true,
    minimumContrastRatio: 1,
    scrollback: 8000,
  });
  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  if (isLocal) {
    const unicode11Addon = new Unicode11Addon();
    term.loadAddon(unicode11Addon);
    term.unicode.activeVersion = "11";
  }
  await nextTick();
  const container = document.getElementById(`terminal-${sessionId}`);
  if (container) {
    term.open(container);
    applyTerminalTheme(term);
    if (!isLocal && detectAppPlatform() !== "windows") {
      try {
        const webglAddon = new WebglAddon();
        term.loadAddon(webglAddon);
        webglAddon.onContextLoss(() => {
          webglAddon.dispose();
        });
        console.log(`Terminal ${sessionId} 启用 WebGL 加速`);
      } catch (e) {
        console.warn("WebGL 加速启动失败，自动降级为 Canvas 渲染", e);
      }
    }
    fitAddon.fit();
    const resizeObserver = attachTerminalResizeObserver(sessionId, container);

    // 性能优先：不要引入定时器延迟。只做串行化发送（避免并发触发后端 handle 锁争抢）。
    let sendQueue: string[] = [];
    let isSending = false;
    let flushScheduled = false;

    const flush = async () => {
      if (isSending) return;
      if (sendQueue.length === 0) return;

      isSending = true;
      flushScheduled = false;

      while (sendQueue.length > 0) {
        const chunk = sendQueue.join("");
        sendQueue = [];

        try {
          await invoke("write_to_ssh", { sessionId, data: chunk });
        } catch (e) {
          console.error("write_to_ssh failed:", e);
          toast.error(t('toast.terminalWriteFailed'));
          sendQueue.unshift(chunk);
          break;
        }
      }

      isSending = false;
      if (sendQueue.length > 0) scheduleFlush();
    };

    const scheduleFlush = () => {
      if (flushScheduled || isSending) return;
      flushScheduled = true;
      // 使用微任务，尽量降低输入到达服务器的延迟
      Promise.resolve().then(() => flush());
    };

    term.onData((data) => {
      sendQueue.push(data);
      scheduleFlush();
    });
    terminalMap.set(sessionId, { term, fitAddon, isLocal, backendReady: false, resizeObserver });
  }
};

const closeTab = async (id: string) => {
  suppressSshClosedToast.add(id);
  await invoke("disconnect_ssh", {sessionId: id}).catch(console.error);
  internalUiCleanup(id);
};

const internalUiCleanup = (id: string) => {
  const instance = terminalMap.get(id);
  if (instance) {
    instance.resizeObserver?.disconnect();
    instance.term.dispose();
    terminalMap.delete(id);
  }
  delete sessionViewModes.value[id];
  clearSessionState(id);
  openSessions.value = openSessions.value.filter(s => s.id !== id);
  if (activeSessionId.value === id) {
    activeSessionId.value = openSessions.value.length > 0 ? openSessions.value[openSessions.value.length - 1].id : null;
  }
};

const toggleViewMode = async () => {
  if (!activeSessionId.value) return;
  const currentMode = sessionViewModes.value[activeSessionId.value] || 'terminal';
  const newMode = currentMode === 'terminal' ? 'sftp' : 'terminal';
  sessionViewModes.value[activeSessionId.value] = newMode;
  if (newMode === 'sftp') {
    await refreshLocalFiles();
    if (!isActiveLocalSession.value) {
      await refreshRemoteFiles();
    }
  } else {
    await focusTerminal(activeSessionId.value);
  }
};

const refreshLocalFiles = async () => {
  localFilesLoading.value = true;
  localFilesError.value = null;
  try {
    localFiles.value = await invoke("list_local_dir", {path: localPath.value});
  } catch (e) {
    localFilesError.value = String(e);
    localFiles.value = [];
    toast.error(t('toast.readLocalFailed', { err: String(e) }));
  } finally {
    localFilesLoading.value = false;
  }
};

const refreshRemoteFiles = async () => {
  if (!activeSessionId.value || isActiveLocalSession.value) return;
  remoteFilesLoading.value = true;
  remoteFilesError.value = null;
  try {
    remoteFiles.value = await invoke("list_remote_dir", {
      sessionId: activeSessionId.value,
      path: remotePath.value,
    });
    const session = activeOpenSession.value;
    if (session && !isLocalSession(session)) {
      persistRemotePath(session.serverId, remotePath.value);
    }
  } catch (e) {
    remoteFilesError.value = String(e);
    remoteFiles.value = [];
    toast.error(t('toast.readRemoteFailed', { err: String(e) }));
  } finally {
    remoteFilesLoading.value = false;
  }
};

const getSessionContext = (sourceSessionId?: string | null) => {
  const sessionId = sourceSessionId ?? activeSessionId.value;
  if (!sessionId) return null;
  const session = openSessions.value.find(s => s.id === sessionId);
  if (!session) return null;
  if (isLocalSession(session)) {
    return {
      session,
      server: { id: LOCAL_SERVER_ID, name: session.name },
    };
  }
  const server = servers.value.find(s => s.id === session.serverId);
  if (!server) return null;
  return { session, server };
};

const buildClonedSessionMeta = (server: { id: string; name: string }) => {
  const siblingCount = openSessions.value.filter(s => s.serverId === server.id).length;
  const newSessionId = `${server.id}-${Math.random().toString(36).substring(2, 9)}`;
  const name = `${server.name} (${siblingCount + 1})`;
  return { newSessionId, name };
};

const cloneSessionToTab = async (sourceSessionId?: string) => {
  const ctx = getSessionContext(sourceSessionId);
  if (!ctx) return;

  if (isLocalSession(ctx.session)) {
    await openLocalShell();
    return;
  }

  const { server } = ctx;
  const { newSessionId, name } = buildClonedSessionMeta(server);

  openSessions.value.push({ id: newSessionId, serverId: server.id, name });
  activeSessionId.value = newSessionId;
  activeId.value = server.id;
  sessionViewModes.value[newSessionId] = 'terminal';
  await performConnect(newSessionId, server);
  if (sessionStatuses.value[newSessionId] === 'connected') {
    toast.success(t('toast.newTabOpened'));
  }
};

const cloneSessionToWindow = async (sourceSessionId?: string) => {
  const ctx = getSessionContext(sourceSessionId);
  if (!ctx) return;

  const { session, server } = ctx;
  try {
    await invoke('open_session_window', {
      serverId: isLocalSession(session) ? LOCAL_SERVER_ID : server.id,
      baseName: session.name || server.name,
    });
    toast.success(t('toast.newWindowOpened'));
    } catch (err) {
    toast.error(t('toast.newWindowFailed', { err: String(err) }));
  }
};

const bootstrapSessionWindow = async () => {
  const bootstrap = window.__SESSION_BOOTSTRAP__;
  if (!bootstrap) return;

  delete window.__SESSION_BOOTSTRAP__;

  if (bootstrap.server_id === LOCAL_SERVER_ID || bootstrap.is_local) {
    const name = bootstrap.session_name || await invoke<string>('get_local_shell_label');
    openSessions.value.push({
      id: bootstrap.session_id,
      serverId: LOCAL_SERVER_ID,
      name,
      kind: 'local',
    });
    activeSessionId.value = bootstrap.session_id;
    sessionViewModes.value[bootstrap.session_id] = 'terminal';
    await initTerminal(bootstrap.session_id, true);
    try {
      const { rows, cols } = await measureTerminalSize(bootstrap.session_id);
      await invoke('spawn_local_shell', {
        sessionId: bootstrap.session_id,
        rows,
        cols,
      });
      markSessionBackendReady(bootstrap.session_id);
      await scheduleLocalTerminalSizeSync(bootstrap.session_id);
      focusTerminal(bootstrap.session_id);
      setSessionStatus(bootstrap.session_id, 'connected');
    } catch (err) {
      toast.error(t('toast.localTerminalFailed', { err: String(err) }));
      setSessionStatus(bootstrap.session_id, 'failed', String(err));
    }
    return;
  }

  const server = servers.value.find(s => s.id === bootstrap.server_id);
  if (!server) {
    toast.error(t('toast.hostNotFound'));
    return;
  }

  activeId.value = server.id;
  openSessions.value.push({
    id: bootstrap.session_id,
    serverId: bootstrap.server_id,
    name: bootstrap.session_name,
  });
  activeSessionId.value = bootstrap.session_id;
  sessionViewModes.value[bootstrap.session_id] = 'terminal';
  await performConnect(bootstrap.session_id, server);
};

const focusTerminal = async (sessionId: string | null) => {
  if (!sessionId) return;
  await nextTick();
  for (const [id, { term }] of terminalMap) {
    if (id !== sessionId) term.blur();
  }
  const instance = terminalMap.get(sessionId);
  if (!instance) return;
  if (isTerminalContainerVisible(sessionId)) {
    await syncTerminalSize(sessionId);
  }
  instance.term.focus();
};

const handleResize = throttle(async () => {
  await nextTick();
  for (const [sessionId] of terminalMap) {
    if (!isTerminalContainerVisible(sessionId)) continue;
    await syncTerminalSize(sessionId);
  }
}, 100);

const openLocalShell = async () => {
  try {
    const label = await invoke<string>('get_local_shell_label');
    const sessionId = `local-${Math.random().toString(36).substring(2, 9)}`;
    const localCount = openSessions.value.filter(s => isLocalSession(s)).length;
    const name = localCount > 0 ? `${label} (${localCount + 1})` : label;

    openSessions.value.push({
      id: sessionId,
      serverId: LOCAL_SERVER_ID,
      name,
      kind: 'local',
    });
    activeSessionId.value = sessionId;
    sessionViewModes.value[sessionId] = 'terminal';
    setSessionStatus(sessionId, 'connecting');
    await initTerminal(sessionId, true);
    const { rows, cols } = await measureTerminalSize(sessionId);
    await invoke('spawn_local_shell', { sessionId, rows, cols });
    markSessionBackendReady(sessionId);
    await scheduleLocalTerminalSizeSync(sessionId);
    focusTerminal(sessionId);
    setSessionStatus(sessionId, 'connected');
  } catch (err) {
    toast.error(t('toast.localTerminalFailed', { err: String(err) }));
    setSessionStatus(sessionId, 'failed', String(err));
  }
};

const openAddModal = () => {
  isEditing.value = false;
  newHost.value = {
    id: "",
    name: "",
    host: "",
    username: "root",
    port: 22,
    auth_type: "password",
    password: "",
    private_key_path: "",
    jump_host_id: "",
    group: ""
  };
  isModalOpen.value = true;
};

const openEditModal = (s: any) => {
  isEditing.value = true;
  newHost.value = {...s, jump_host_id: s.jump_host_id || "", group: s.group || ""};
  isModalOpen.value = true;
};

const openEditModalForSession = (session: OpenSession) => {
  const server = servers.value.find((s) => s.id === session.serverId);
  if (server) openEditModal(server);
};

const closeModal = () => {
  isModalOpen.value = false;
  showPassword.value = false;
};

const saveHost = async (e: any) => {
  if (!e.name?.trim() || !e.host?.trim()) {
    toast.warning(t('toast.fillRequired'));
    return;
  }
  const serverToSave = {
    ...e,
    port: Number(e.port),
    jump_host_id: e.jump_host_id || null,
    group: e.group?.trim() || null,
  };
  try {
    await invoke("save_server", {server: serverToSave});
    await loadServers();
    closeModal();
    toast.success(t('toast.hostSaved'));
    } catch (error) {
    toast.error(t('toast.saveFailed', { err: String(error) }));
  }
};

const loadServers = async () => {
  servers.value = await invoke("get_servers");
  if (servers.value.length > 0 && !activeId.value) activeId.value = servers.value[0].id;
};

const getTaskIcon = (task: any) => {
  if (task.status === 'error') return 'fas fa-exclamation-circle';
  if (task.status === 'success') return 'fas fa-check-circle';
  return task.type === 'upload' ? 'fas fa-cloud-upload-alt' : 'fas fa-cloud-download-alt';
};

const cancelTask = async (taskId: string) => {
  const task = transferTasks.value.find(t => t.id === taskId);
  if (!task) return;
  try {
    await invoke("abort_transfer", {taskId});
    task.status = 'error';
    setTimeout(() => {
      transferTasks.value = transferTasks.value.filter(t => t.id !== taskId);
    }, 3000);
  } catch (err) {
    console.error(err);
  }
};

watch(activeSessionId, async (newId) => {
  if (newId) {
    const session = openSessions.value.find(s => s.id === newId);
    if (session && !isLocalSession(session)) {
      activeId.value = session.serverId;
      syncRemotePathForSession(newId);
    }
    if (isLocalSession(session)) {
      sessionViewModes.value[newId] = 'terminal';
    }
    await focusTerminal(newId);
  }
});

watch(currentViewMode, async (mode) => {
  if (mode === 'terminal' && activeSessionId.value) {
    await focusTerminal(activeSessionId.value);
  }
});

const isResizing = ref(false);

const savePanelWidths = () => {
  localStorage.setItem(PANEL_WIDTH_STORAGE_KEY, JSON.stringify(panelWidths.value));
};

const loadPanelWidths = () => {
  try {
    const raw = localStorage.getItem(PANEL_WIDTH_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Record<RightPanelType, number>>;
      panelWidths.value = { ...DEFAULT_PANEL_WIDTHS, ...parsed };
      return;
    }
    const legacy = localStorage.getItem(LEGACY_PANEL_WIDTH_KEY);
    if (legacy) {
      const width = parseInt(legacy, 10);
      if (!Number.isNaN(width) && width >= 300) {
        panelWidths.value = {
          ...DEFAULT_PANEL_WIDTHS,
          quick: width,
          ai: width,
          redis: width,
          history: width,
          'sync-settings': width,
          'theme-settings': width,
          chat: width,
        };
        savePanelWidths();
        localStorage.removeItem(LEGACY_PANEL_WIDTH_KEY);
      }
    }
  } catch {
    panelWidths.value = { ...DEFAULT_PANEL_WIDTHS };
  }
};

const startResizing = (e: MouseEvent) => {
  isResizing.value = true;
  const startX = e.clientX;
  const panelType = rightPanelType.value;
  const startWidth = panelWidths.value[panelType];

  const doResize = (moveEvent: MouseEvent) => {
    if (!isResizing.value) return;
    const delta = moveEvent.clientX - startX;
    const newWidth = startWidth - delta;

    const maxWidth = window.innerWidth - 300;
    const minWidth = getPanelMinWidth(panelType);

    if (newWidth >= minWidth && newWidth <= maxWidth) {
      panelWidths.value = { ...panelWidths.value, [panelType]: newWidth };
    }
  };

  const stopResizing = () => {
    isResizing.value = false;
    document.removeEventListener('mousemove', doResize);
    document.removeEventListener('mouseup', stopResizing);
    document.body.style.cursor = 'default';
    savePanelWidths();
  };

  document.addEventListener('mousemove', doResize);
  document.addEventListener('mouseup', stopResizing);
  document.body.style.cursor = 'col-resize';
};

const initLocalRootPath = async () => {
  try {
    localPath.value = await homeDir();
    refreshLocalFiles();
  } catch (err) {
    const isWin = navigator.userAgent.includes('Windows');
    localPath.value = isWin ? "C:/" : "/";
    refreshLocalFiles();
  }
}

const handleOnlineCountUpdate = (count: number) => {
  onlineUserCount.value = count;
};

const updateOnlineCount = async () => {
  try {
    const peers = await invoke<string[]>("get_online_peers");
    onlineUserCount.value = peers.length;
  } catch (err) {
    console.error("无法获取在线人数:", err);
  }
};

const handleOrderChange = async (newList) => {
  servers.value = newList;

  const ids = newList.map(s => s.id);
  try {
    await invoke("update_server_order", { ids });
    console.log("后端排序更新成功");
  } catch (err) {
    toast.error(t('toast.saveSortFailed', { err: String(err) }));
  }
};

watch(defaultTheme, async () => {
  await nextTick();
  terminalMap.forEach(({ term }) => applyTerminalTheme(term));
}, { immediate: false });

const handleGlobalKeydown = (e: KeyboardEvent) => {
  const target = e.target as HTMLElement;
  const tag = target.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || target.isContentEditable) return;

  if (e.ctrlKey && e.key === 'w') {
    e.preventDefault();
    if (activeSessionId.value) void closeTab(activeSessionId.value);
  }
};

onMounted(async () => {
  const themeId = localStorage.getItem('app-theme-id') || defaultTheme.value;
  applyTheme(themeId);
  initLocalRootPath()
  window.addEventListener("resize", handleResize);
  window.addEventListener("keydown", handleGlobalKeydown);
  await setupNativeDragDrop();
  loadPanelWidths();
  loadServers();
  updateOnlineCount();
  unlisten = await listen("ssh-output", (event) => {
    const payload = event.payload as { session_id: string, data: string };
    const instance = terminalMap.get(payload.session_id);
    if (instance) instance.term.write(payload.data);
  });
  await listen('database-changed', () => loadServers());
  await bootstrapSessionWindow();
  unlistenClosed = await listen("ssh-closed", (event) => {
    const payload = event.payload as { session_id?: string; server_id?: string };
    if (!payload.session_id) return;

    if (suppressSshClosedToast.has(payload.session_id)) {
      suppressSshClosedToast.delete(payload.session_id);
      return;
    }

    if (!openSessions.value.some((s) => s.id === payload.session_id)) return;

    const instance = terminalMap.get(payload.session_id);
    if (instance) instance.backendReady = false;
    setSessionStatus(payload.session_id, 'disconnected', t('session.remoteDisconnected'));
    toast.warning(t('session.sshDisconnectedTitle'));
  });
  unlistenTransfer = await listen("transfer-progress", (event) => {
    const {taskId, progress} = event.payload as { taskId: string, progress: number };
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) task.progress = progress;
  });

  await listen("sync-error", (event) => {
    const payload = event.payload as { phase?: string; message?: string } | string;
    const message = typeof payload === "string"
      ? payload
      : payload.message || t('toast.syncFailed');
    toast.error(message, typeof payload === "object" ? payload.phase : t('toast.syncPhase'));
  });

  unlistenSync = await listen("sync-status", (event) => {
    isSyncing.value = event.payload as boolean;
  });

  await listen("sync-finished", (event) => {
    toast.success(event.payload as string);
  });
});

onUnmounted(async () => {
  for (const session of openSessions.value) {
    suppressSshClosedToast.add(session.id);
    await invoke("disconnect_ssh", { sessionId: session.id }).catch(console.error);
  }
  terminalMap.forEach(instance => {
    instance.term.dispose();
  });
  terminalMap.clear();
  window.removeEventListener("resize", handleResize);
  window.removeEventListener("keydown", handleGlobalKeydown);
  if (unlisten) unlisten();
  if (unlistenClosed) unlistenClosed();
  if (unlistenTransfer) unlistenTransfer();
  if (unlistenSync) unlistenSync();
  if (unlistenDragDrop) unlistenDragDrop();
});
</script>

<template>
  <div class="termius-container">
    <TitleBar :active-session-id="activeSessionId" :is-local-session="isActiveLocalSession"/>
    <div class="main-layout">
      <Sidebar
          v-model:active-id="activeId"
          :servers="servers"
          @connect="connectToServer"
          @edit="openEditModal"
          @delete="loadServers"
          @update:servers="handleOrderChange"
          @open-add-modal="openAddModal"
      />

      <main class="workspace">
        <TerminalTabs
            :open-sessions="openSessions"
            :session-statuses="sessionStatuses"
            v-model:active-session-id="activeSessionId"
            @close="closeTab"
            @clone-tab="cloneSessionToTab"
            @clone-window="cloneSessionToWindow"
            @new-local-shell="openLocalShell"
            @reconnect="reconnectSession"
        />
        <WorkspaceHeader
            :current-server="currentServer"
            :active-id="activeId"
            :active-session-id="activeSessionId"
            :session-status="activeSessionStatus"
            :session-error="activeSessionError"
            :current-view-mode="currentViewMode"
            :open-sessions="openSessions"
            :servers="servers"
            :is-local-session="isActiveLocalSession"
            @toggle-view-mode="toggleViewMode"
            @connect="connectToServer()"
            @reconnect="reconnectSession()"
        />

        <div class="terminal-shell">
          <div v-show="currentViewMode === 'terminal'" class="terminal-wrapper">
            <div v-if="openSessions.length > 0" class="terminal-multi-wrapper">
              <div
                  v-for="session in openSessions"
                  :key="session.id"
                  v-show="activeSessionId === session.id"
                  class="terminal-pane"
              >
                <div
                    :id="`terminal-${session.id}`"
                    class="xterm-container"
                    @mousedown="focusTerminal(session.id)"
                    @click="focusTerminal(session.id)"
                ></div>
                <div
                    v-if="showSessionOverlay(session.id)"
                    class="terminal-overlay"
                >
                  <div class="terminal-overlay__card">
                    <i class="fas fa-plug-circle-xmark"></i>
                    <h4>{{ sessionStatuses[session.id] === 'failed' ? tr.session.overlayFailed : tr.session.overlayDisconnected }}</h4>
                    <p>{{ getSessionOverlayMessage(session.id) }}</p>
                    <div class="terminal-overlay__actions">
                      <button type="button" class="btn-overlay" @click="reconnectSession(session.id)">
                        <i class="fas fa-rotate-right"></i> {{ tr.session.overlayReconnect }}
                      </button>
                      <button type="button" class="btn-overlay btn-overlay--ghost" @click="openEditModalForSession(session)">
                        <i class="fas fa-pen"></i> {{ tr.session.overlayEditHost }}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div v-else class="empty-state">
              <div class="empty-state-content">
                <div class="icon-stack">
                  <i class="fas fa-terminal main-icon"></i>
                  <div class="glow-ring"></div>
                </div>
                <h3 class="empty-title">{{ tr.empty.title }}</h3>
                <p class="empty-description">{{ tr.empty.description }}</p>
                <button class="create-btn" @click="openAddModal">
                  <i class="fas fa-plus"></i>
                  {{ tr.empty.newConnection }}
                </button>
              </div>
            </div>
          </div>

          <div v-show="currentViewMode === 'sftp'" class="sftp-wrapper">
            <div v-if="activeSessionId" class="sftp-manager" :class="{ 'sftp-manager--local-only': isActiveLocalSession, 'sftp-manager--internal-drag': isSftpInternalDragging }">
              <div class="file-pane local-pane">
                <div class="pane-header">
                  <i class="fas" :class="isActiveLocalSession ? 'fa-folder-open' : 'fa-laptop'" style="margin-right: 8px; color: #565f89;"></i>
                  <input v-model="localPath" class="path-input" @keyup.enter="refreshLocalFiles"/>
                </div>
                <div class="file-list"
                     :class="{ 'drag-over': isDraggingOverLocal }"
                     @contextmenu="handlePaneContextMenu($event, 'local')">
                  <div v-if="localFilesLoading" class="file-pane-state">
                    <i class="fas fa-spinner fa-spin"></i> {{ tr.common.loading }}
                  </div>
                  <div v-else-if="localFilesError" class="file-pane-state file-pane-state--error">
                    <i class="fas fa-circle-exclamation"></i>
                    <span>{{ localFilesError }}</span>
                    <button type="button" @click="refreshLocalFiles">{{ tr.common.retry }}</button>
                  </div>
                  <div v-else-if="localFiles.length === 0" class="file-pane-state">
                    <i class="fas fa-folder-open"></i> {{ tr.common.emptyFolder }}
                  </div>
                  <template v-else>
                  <div v-for="file in localFiles" :key="file.name" class="file-item"
                       :class="{ 'is-dir': file.is_dir, 'is-dragging': sftpInternalDragKey === `local:${file.name}` }"
                       @pointerdown="onSftpFilePointerDown($event, file, 'local')"
                       @dblclick.stop="handleFileDblClick(file, 'local')"
                       @contextmenu="handleContextMenu($event, file, 'local')">
                    <span class="file-icon">
                      <Tooltip :text="tr.common.doubleClick" inline>
                        <i class="fas"
                           :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i>
                      </Tooltip>
                    </span>
                    <span class="file-name">{{ file.name }}</span>
                    <span class="file-size" v-if="!file.is_dir">
                      {{ formatSize(file.size) }}
                    </span>
                  </div>
                  </template>
                </div>
              </div>

              <div v-if="!isActiveLocalSession" class="file-pane remote-pane">
                <div class="pane-header">
                  <i class="fas fa-server" style="margin-right: 8px; color: #565f89;"></i>
                  <input v-model="remotePath" class="path-input" @keyup.enter="refreshRemoteFiles"/>
                </div>
                <div class="file-list"
                     :class="{ 'drag-over': isDraggingOverRemote }"
                     @contextmenu="handlePaneContextMenu($event, 'remote')">
                  <div v-if="remoteFilesLoading" class="file-pane-state">
                    <i class="fas fa-spinner fa-spin"></i> {{ tr.common.loading }}
                  </div>
                  <div v-else-if="remoteFilesError" class="file-pane-state file-pane-state--error">
                    <i class="fas fa-circle-exclamation"></i>
                    <span>{{ remoteFilesError }}</span>
                    <button type="button" @click="refreshRemoteFiles">{{ tr.common.retry }}</button>
                  </div>
                  <div v-else-if="remoteFiles.length === 0" class="file-pane-state">
                    <i class="fas fa-folder-open"></i> {{ tr.common.emptyFolder }}
                  </div>
                  <template v-else>
                  <div v-for="file in remoteFiles" :key="file.name" class="file-item"
                       :class="{ 'is-dir': file.is_dir, 'is-dragging': sftpInternalDragKey === `remote:${file.name}` }"
                       @pointerdown="onSftpFilePointerDown($event, file, 'remote')"
                       @dblclick.stop="handleFileDblClick(file, 'remote')"
                       @contextmenu="handleContextMenu($event, file, 'remote')">
                    <span class="file-icon">
                      <Tooltip :text="tr.common.doubleClick" inline>
                        <i class="fas"
                           :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i>
                      </Tooltip>
                    </span>
                    <span class="file-name">{{ file.name }}</span>
                    <span class="file-size" v-if="!file.is_dir">{{ formatSize(file.size) }}</span>
                  </div>
                  </template>
                </div>
              </div>

              <div v-if="!isActiveLocalSession && transferTasks.length > 0" class="transfer-status">
                <div class="status-header">
                  <div class="header-left"><i class="fas fa-layer-group"></i><span>{{ t('sftp.transferQueue', { count: transferTasks.length }) }}</span></div>
                  <div class="header-status-dot" :class="{ 'is-syncing': hasActiveTasks }"></div>
                </div>
                <div class="task-list-wrapper">
                  <TransitionGroup name="task-list">
                    <div v-for="task in transferTasks" :key="task.id" class="task-row"
                         :class="[`status-${task.status}`]">
                      <div class="task-info">
                        <Tooltip :text="task.name" block wrap>
                          <div class="name-box">
                            <i :class="getTaskIcon(task)" class="type-icon"></i>
                            <span class="task-name">{{ task.name }}</span>
                          </div>
                        </Tooltip>
                        <div class="task-actions">
                          <button v-if="task.status === 'transferring'" class="cancel-btn"
                                  @click.stop="cancelTask(task.id)"><i class="fas fa-times"></i></button>
                          <span class="task-percent">{{ task.progress }}%</span>
                        </div>
                      </div>
                      <div class="progress-container">
                        <div class="progress-bar" :style="{ width: task.progress + '%' }"></div>
                      </div>
                    </div>
                  </TransitionGroup>
                </div>
              </div>
            </div>
          </div>
        </div>
        <StatusBar
            :open-sessions="openSessions"
            :latency-server="activeTabServer"
            :is-active-local-session="isActiveLocalSession"
            :session-status="activeSessionStatus"
            :servers="servers"
        />
      </main>

      <div class="right-dock">
        <div class="icon-bar">
          <div class="top-group">
            <Tooltip :text="tr.dock.quickCommands" placement="left">
              <div class="icon-item" :class="{ active: rightPanelVisible && rightPanelType === 'quick' }"
                   @click="toggleRightPanel('quick')">
                <i class="fas fa-bolt"></i>
              </div>
            </Tooltip>
            <Tooltip :text="tr.dock.aiAssistant" placement="left">
              <div class="icon-item" :class="{ active: rightPanelVisible && rightPanelType === 'ai' }"
                   @click="toggleRightPanel('ai')">
                <i class="fas fa-robot"></i>
              </div>
            </Tooltip>
            <Tooltip :text="tr.dock.redis" placement="left">
              <div class="icon-item"
                   :class="{ active: rightPanelVisible && rightPanelType === 'redis' }"
                   @click="toggleRightPanel('redis')">
                <svg class="redis-icon" viewBox="0 0 24 24" width="18" height="18">
                  <path fill="currentColor" d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
                </svg>
              </div>
            </Tooltip>
            <Tooltip :text="tr.dock.apiDebugger" placement="left">
              <div class="icon-item"
                   :class="{ active: rightPanelVisible && rightPanelType === 'api' }"
                   @click="toggleRightPanel('api')">
                <i class="fas fa-paper-plane"></i>
              </div>
            </Tooltip>
          </div>
          <div class="bottom-group">
            <Tooltip :text="tr.dock.chat" placement="left">
              <div class="icon-item"
                   :class="{ active: rightPanelVisible && rightPanelType === 'chat' }"
                   @click="toggleRightPanel('chat')">
                <i class="fas fa-comment-alt"></i>
                <Transition name="scale">
                  <span v-if="onlineUserCount > 0" class="online-badge">
                    {{ onlineUserCount }}
                  </span>
                </Transition>
              </div>
            </Tooltip>
            <Tooltip :text="tr.dock.syncSettings" placement="left">
              <div class="icon-item"
                   :class="{
                     active: rightPanelVisible && rightPanelType === 'sync-settings',
                     'is-syncing': isSyncing
                   }"
                   @click="toggleRightPanel('sync-settings')">
                <i class="fas fa-sync-alt" :class="{ 'fa-spin': isSyncing }"></i>
              </div>
            </Tooltip>
            <Tooltip :text="tr.dock.themeSettings" placement="left">
              <div class="icon-item"
                   :class="{ active: rightPanelVisible && rightPanelType === 'theme-settings' }"
                   @click="toggleRightPanel('theme-settings')">
                <i class="fas fa-palette"></i>
              </div>
            </Tooltip>
          </div>
        </div>

        <Transition name="panel-slide">
          <div
              v-if="rightPanelVisible"
              class="floating-panel"
              :class="{ 'is-redis': rightPanelType === 'redis', 'is-api': rightPanelType === 'api' }"
              :style="{ width: panelWidth + 'px' }"
          >
            <div class="panel-resizer" @mousedown="startResizing"></div>

            <div class="panel-content-wrapper">
              <KeepAlive :max="5">
                <component
                    :is="rightPanelComponent"
                    :activeSessionId="activeSessionId"
                    :session-connected="isActiveSessionConnected"
                    @update-online-count="handleOnlineCountUpdate"
                />
              </KeepAlive>
            </div>
          </div>
        </Transition>
      </div>
    </div>

    <ServerModal :is-open="isModalOpen" :is-editing="isEditing" :server="newHost" :servers="servers" @close="closeModal"
                 @save="saveHost"/>

    <SftpFileDialog
      :visible="sftpDialogVisible"
      :mode="sftpDialogMode"
      :source="contextSource || 'local'"
      :loading="sftpDialogLoading"
      :detail="sftpDialogDetail"
      :input-value="sftpDialogInput"
      @close="closeSftpDialog"
      @confirm="handleSftpDialogConfirm"
    />

    <Transition name="menu-scale">
      <div v-if="menuVisible" class="context-menu" :style="{ top: menuPos.y + 'px', left: menuPos.x + 'px' }"
           @click.stop>
        <template v-if="contextTarget === 'pane'">
          <div class="menu-item" @click="handleMenuAction('newFile')">
            <i class="fas fa-file-medical"></i>
            <span class="menu-text">{{ tr.sftp.newFile }}</span>
          </div>
          <div class="menu-item" @click="handleMenuAction('newFolder')">
            <i class="fas fa-folder-plus"></i>
            <span class="menu-text">{{ tr.sftp.newFolder }}</span>
          </div>
          <div class="menu-divider"></div>
          <div class="menu-item" :class="{ disabled: !canPaste }" @click="handleMenuAction('paste')">
            <i class="fas fa-paste"></i>
            <span class="menu-text">{{ tr.sftp.paste }}</span>
          </div>
          <div class="menu-divider"></div>
          <div class="menu-item" @click="handleMenuAction('refresh')">
            <i class="fas fa-rotate-right"></i>
            <span class="menu-text">{{ tr.sftp.refresh }}</span>
          </div>
        </template>

        <template v-else>
          <div class="menu-item" @click="handleMenuAction('copy')">
            <i class="fas fa-copy"></i>
            <span class="menu-text">{{ tr.sftp.copy }}</span>
          </div>
          <div class="menu-item" @click="handleMenuAction('cut')">
            <i class="fas fa-scissors"></i>
            <span class="menu-text">{{ tr.sftp.cut }}</span>
          </div>
          <div class="menu-item" :class="{ disabled: !canPaste }" @click="handleMenuAction('paste')">
            <i class="fas fa-paste"></i>
            <span class="menu-text">{{ tr.sftp.paste }}</span>
          </div>
          <div class="menu-item" @click="handleMenuAction('copyPath')">
            <i class="fas fa-link"></i>
            <span class="menu-text">{{ tr.sftp.copyPath }}</span>
          </div>

          <div class="menu-divider"></div>

          <div v-if="!isActiveLocalSession" class="menu-item" @click="handleMenuAction('transfer')">
            <i class="fas" :class="contextSource === 'local' ? 'fa-cloud-upload-alt' : 'fa-cloud-download-alt'"></i>
            <span class="menu-text">
              {{ contextSource === 'local' ? tr.sftp.uploadRemote : tr.sftp.downloadLocal }}
            </span>
          </div>

          <div v-if="!isActiveLocalSession" class="menu-divider"></div>

          <div class="menu-item" @click="handleMenuAction('info')">
            <i class="fas fa-circle-info"></i>
            <span class="menu-text">{{ tr.sftp.fileInfo }}</span>
          </div>

          <div v-if="contextSource === 'local'" class="menu-item" @click="handleMenuAction('openExplorer')">
            <i class="fas fa-folder-open"></i>
            <span class="menu-text">{{ tr.sftp.openExplorer }}</span>
          </div>

          <div class="menu-item" @click="handleMenuAction('rename')">
            <i class="fas fa-pen"></i>
            <span class="menu-text">{{ tr.sftp.rename }}</span>
          </div>

          <div v-if="contextSource === 'remote'" class="menu-item" @click="handleMenuAction('chmod')">
            <i class="fas fa-key"></i>
            <span class="menu-text">{{ tr.sftp.chmod }}</span>
          </div>

          <div class="menu-divider"></div>

          <div class="menu-item" @click="handleMenuAction('refresh')">
            <i class="fas fa-rotate-right"></i>
            <span class="menu-text">{{ tr.sftp.refresh }}</span>
          </div>

          <div class="menu-divider"></div>

          <div class="menu-item danger" @click="handleMenuAction('delete')">
            <i class="fas fa-trash-alt"></i>
            <span class="menu-text">{{ tr.common.delete }}</span>
          </div>
        </template>
      </div>
    </Transition>

  </div>
</template>

<style lang="scss">
@use './assets/css/base.scss';
</style>
<style lang="scss" scoped>
@use './assets/css/app.scss';
</style>