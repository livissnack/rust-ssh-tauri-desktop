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
import { LOCAL_SERVER_ID, isLocalSession, type OpenSession } from "./utils/session.ts";

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

const panelMap: Record<string, any> = {
  'quick': QuickCommandPanel,
  'ai': AiAssistantPanel,
  'redis': RedisManager,
  'sync-settings': SyncSettings,
  'theme-settings': ThemeSettings,
  'chat': ChatPanel,
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

const isConnecting = ref(false);
const isConnectError = ref(false);
const rightPanelVisible = ref(false);
const isModalOpen = ref(false);
const isEditing = ref(false);
const isSyncing = ref(false);

let unlisten: UnlistenFn | null = null;
let unlistenClosed: UnlistenFn | null = null;
let unlistenTransfer: UnlistenFn | null = null;
let unlistenSync: UnlistenFn | null = null;
let unlistenDragDrop: UnlistenFn | null = null;
let internalDragPayload: { source: 'local' | 'remote'; file: any } | null = null;
const transferTasks = ref<any[]>([]);

const rightPanelType = ref<'quick' | 'ai' | 'redis' | 'history' | 'sync-settings'>('quick');

const localPath = ref("");
const remotePath = ref("/root");
const localFiles = ref<any[]>([]);
const remoteFiles = ref<any[]>([]);
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

const hasActiveTasks = computed(() =>
    transferTasks.value.some(t => t.status === 'transferring')
);

const handleToggle = (type: any) => {
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
    toast.success('路径已复制到剪贴板');
  } catch {
    toast.error('复制到剪贴板失败');
  }
};

const pasteClipboard = async (source: 'local' | 'remote') => {
  const item = sftpClipboard.value;
  if (!item || item.source !== source) {
    toast.warning('当前面板没有可粘贴的内容');
    return;
  }

  const destPath = buildPathInPane(source, item.name);
  if (destPath === item.path) {
    if (item.mode === 'cut') {
      sftpClipboard.value = null;
    } else {
      toast.warning('不能复制到相同路径');
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
      toast.success('移动成功');
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
      toast.success('粘贴成功');
    }
  } catch (err) {
    toast.error(`粘贴失败: ${err}`);
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
      toast.error('名称包含非法字符');
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
      toast.success(isDir ? '文件夹已创建' : '文件已创建');
      closeSftpDialog();
    } catch (err) {
      toast.error(`创建失败: ${err}`);
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
      toast.error('文件名包含非法字符');
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
      toast.success('重命名成功');
      closeSftpDialog();
    } catch (err) {
      toast.error(`重命名失败: ${err}`);
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
      toast.success('权限已更新');
      closeSftpDialog();
    } catch (err) {
      toast.error(`修改权限失败: ${err}`);
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
    openSftpDialog('createFile', source, null, '新建文件.txt');
    return;
  }

  if (action === 'newFolder') {
    openSftpDialog('createFolder', source, null, '新建文件夹');
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
    toast.info('已复制');
  } else if (action === 'cut') {
    sftpClipboard.value = {
      source,
      path: getContextFilePath(source, file),
      name: file.name,
      isDir: file.is_dir,
      mode: 'cut',
    };
    toast.info('已剪切');
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
      toast.error(`获取文件信息失败: ${err}`);
    } finally {
      sftpDialogLoading.value = false;
    }
  } else if (action === 'openExplorer') {
    try {
      await invoke('reveal_in_file_manager', {path: getContextFilePath('local', file)});
    } catch (err) {
      toast.error(`打开资源管理器失败: ${err}`);
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
      toast.error(`获取权限失败: ${err}`);
    } finally {
      sftpDialogLoading.value = false;
    }
  } else if (action === 'delete') {
    const ok = await confirm.error(
        `确定要永久删除${source === 'local' ? '本地' : '远程'}文件 "${file.name}" 吗？`,
        '确认删除'
    );

    if (ok) {
      try {
        if (source === 'remote') {
          const path = joinRemotePath(remotePath.value, file.name);
          await invoke("delete_remote_file", {sessionId: activeSessionId.value, path, isDir: file.is_dir});
          await refreshRemoteFiles();
        } else {
          const path = getContextFilePath('local', file);
          await invoke("delete_local_file", {path, isDir: file.is_dir});
          await refreshLocalFiles();
          toast.success("删除成功");
        }
      } catch (err) {
        toast.error(`删除失败: ${err}`);
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

const onDragStart = (e: DragEvent, file: any, source: 'local' | 'remote') => {
  if (file.name === '..') {
    e.preventDefault();
    return;
  }
  internalDragPayload = {source, file};
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "copy";
    const payload = JSON.stringify({source, file});
    e.dataTransfer.setData("text/plain", payload);
  }
};

const handleDragOver = (e: DragEvent) => {
  e.preventDefault();
  if (e.dataTransfer) {
    e.dataTransfer.dropEffect = "copy";
  }
};

const handleDragEnter = (e: DragEvent, type: 'local' | 'remote') => {
  e.preventDefault();
  if (type === 'local') isDraggingOverLocal.value = true;
  else isDraggingOverRemote.value = true;
};

const handleDragLeave = (e: DragEvent, type: 'local' | 'remote') => {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  if (
      e.clientX <= rect.left || e.clientX >= rect.right ||
      e.clientY <= rect.top || e.clientY >= rect.bottom
  ) {
    if (type === 'local') isDraggingOverLocal.value = false;
    else isDraggingOverRemote.value = false;
  }
};

const handleDrop = async (e: DragEvent, targetType: 'local' | 'remote') => {
  e.preventDefault();
  clearDragOverHighlight();

  const rawData = e.dataTransfer?.getData("text/plain");
  if (!rawData) return;

  try {
    const data = JSON.parse(rawData);
    if (data.source === 'local' && targetType === 'remote') {
      await startTransfer('upload', data.file);
    } else if (data.source === 'remote' && targetType === 'local') {
      await startTransfer('download', data.file);
    }
  } catch (err) {
    console.error("Drop Error:", err);
  }
};

const finishInternalDrag = async (e: DragEvent) => {
  const payload = internalDragPayload;
  internalDragPayload = null;
  if (!payload || currentViewMode.value !== 'sftp' || !activeSessionId.value) {
    clearDragOverHighlight();
    return;
  }

  const targetPane = getSftpDropPaneAtLogicalPoint(e.clientX, e.clientY);
  try {
    if (payload.source === 'local' && targetPane === 'remote') {
      await startTransfer('upload', payload.file);
    } else if (payload.source === 'remote' && targetPane === 'local') {
      await startTransfer('download', payload.file);
    }
  } finally {
    clearDragOverHighlight();
  }
};

const handleOsFileDrop = async (paths: string[], pane: 'local' | 'remote') => {
  if (!activeSessionId.value || currentViewMode.value !== 'sftp') return;

  if (pane !== 'remote') {
    toast.error('请拖放到右侧远程面板以上传文件');
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
    toast.error(`切换目录失败: ${err}`);
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
    toast.error(`传输失败: ${err}`);
  }
};

const startTransfer = async (type: 'upload' | 'download', file: any) => {
  if (file.is_dir || file.name === '..') {
    toast.error(`暂不支持${type === 'upload' ? '上传' : '下载'}文件夹，请先压缩后再操作`);
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

const connectToServer = async () => {
  const server = servers.value.find(s => s.id === activeId.value);
  if (!server) return;
  isConnectError.value = false;
  isConnecting.value = true;
  const sessionId = server.id;
  if (!openSessions.value.find(s => s.id === sessionId)) {
    openSessions.value.push({id: sessionId, serverId: server.id, name: server.name});
    sessionViewModes.value[sessionId] = 'terminal';
  }
  activeSessionId.value = sessionId;
  await initTerminal(sessionId);
  try {
    await invoke("connect_ssh", {serverId: server.id, sessionId});
    markSessionBackendReady(sessionId);
    await syncTerminalSize(sessionId);
    focusTerminal(sessionId);
    isConnectError.value = false;
  } catch (err) {
    toast.error(`连接失败: ${err}`);
    isConnectError.value = true
  } finally {
    isConnecting.value = false;
  }
};

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
    if (!isLocal) {
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

      // drain 成一个 chunk，减少 invoke 次数；但不等定时器，尽量保持低延迟
      const chunk = sendQueue.join("");
      sendQueue = [];

      await invoke("write_to_ssh", { sessionId, data: chunk }).catch((e) =>
        console.error("write_to_ssh failed:", e)
      );

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
      // 本地 ConPTY（PowerShell / macOS Terminal）必须原样发送；SSH 远端才需要 \r\n
      const payload = isLocal ? data : data.replace(/\r/g, "\r\n");
      sendQueue.push(payload);
      scheduleFlush();
    });
    terminalMap.set(sessionId, { term, fitAddon, isLocal, backendReady: false, resizeObserver });
  }
};

const closeTab = async (id: string) => {
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
  openSessions.value = openSessions.value.filter(s => s.id !== id);
  if (activeSessionId.value === id) {
    activeSessionId.value = openSessions.value.length > 0 ? openSessions.value[openSessions.value.length - 1].id : null;
  }
};

const toggleViewMode = async () => {
  if (!activeSessionId.value || isActiveLocalSession.value) return;
  const currentMode = sessionViewModes.value[activeSessionId.value] || 'terminal';
  const newMode = currentMode === 'terminal' ? 'sftp' : 'terminal';
  sessionViewModes.value[activeSessionId.value] = newMode;
  if (newMode === 'sftp') {
    refreshRemoteFiles();
    refreshLocalFiles();
  }
};

const refreshLocalFiles = async () => {
  try {
    localFiles.value = await invoke("list_local_dir", {path: localPath.value});
  } catch (e) {
    console.error(e);
  }
};
const refreshRemoteFiles = async () => {
  try {
    remoteFiles.value = await invoke("list_remote_dir", {sessionId: activeSessionId.value, path: remotePath.value});
  } catch (e) {
    console.error(e);
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
  const name = siblingCount >= 1
    ? `${server.name} (${siblingCount + 1})`
    : `${server.name} (Copy)`;
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
  sessionViewModes.value[newSessionId] = 'terminal';
  await initTerminal(newSessionId, false);

  isConnecting.value = true;
  try {
    await invoke("connect_ssh", { serverId: server.id, sessionId: newSessionId });
    markSessionBackendReady(newSessionId);
    await syncTerminalSize(newSessionId);
    focusTerminal(newSessionId);
    toast.success('已在新标签打开会话');
  } catch (err) {
    toast.error(`克隆失败: ${err}`);
    internalUiCleanup(newSessionId);
  } finally {
    isConnecting.value = false;
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
    toast.success('已在新窗口打开会话');
  } catch (err) {
    toast.error(`打开新窗口失败: ${err}`);
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
    } catch (err) {
      toast.error(`打开本地终端失败: ${err}`);
      internalUiCleanup(bootstrap.session_id);
    }
    return;
  }

  const server = servers.value.find(s => s.id === bootstrap.server_id);
  if (!server) {
    toast.error('无法找到主机配置');
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
  isConnecting.value = true;

  await initTerminal(bootstrap.session_id);
  try {
    await invoke('connect_ssh', {
      serverId: bootstrap.server_id,
      sessionId: bootstrap.session_id,
    });
    markSessionBackendReady(bootstrap.session_id);
    await syncTerminalSize(bootstrap.session_id);
    focusTerminal(bootstrap.session_id);
    isConnectError.value = false;
  } catch (err) {
    toast.error(`连接失败: ${err}`);
    isConnectError.value = true;
  } finally {
    isConnecting.value = false;
  }
};

const focusTerminal = async (sessionId: string | null) => {
  if (!sessionId) return;
  await nextTick();
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
    await initTerminal(sessionId, true);
    const { rows, cols } = await measureTerminalSize(sessionId);
    await invoke('spawn_local_shell', { sessionId, rows, cols });
    markSessionBackendReady(sessionId);
    await scheduleLocalTerminalSizeSync(sessionId);
    focusTerminal(sessionId);
  } catch (err) {
    toast.error(`打开本地终端失败: ${err}`);
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
    jump_host_id: ""
  };
  isModalOpen.value = true;
};

const openEditModal = (s: any) => {
  isEditing.value = true;
  newHost.value = {...s, jump_host_id: s.jump_host_id || ""};
  isModalOpen.value = true;
};

const closeModal = () => {
  isModalOpen.value = false;
  showPassword.value = false;
};

const saveHost = async (e: any) => {
  if (e.name && e.host) {
    const serverToSave = {...e, port: Number(e.port), jump_host_id: e.jump_host_id || null};
    try {
      await invoke("save_server", {server: serverToSave});
      await loadServers();
      closeModal();
    } catch (error) {
      toast.error('保存失败：' + error);
    }
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
    if (isLocalSession(session)) {
      sessionViewModes.value[newId] = 'terminal';
    }
    await focusTerminal(newId);
  }
});

const panelWidth = ref(420);
const isResizing = ref(false);

const startResizing = (e: MouseEvent) => {
  isResizing.value = true;
  const startX = e.clientX;
  const startWidth = panelWidth.value;

  const doResize = (moveEvent: MouseEvent) => {
    if (!isResizing.value) return;
    const delta = moveEvent.clientX - startX;
    const newWidth = startWidth - delta;

    const maxWidth = window.innerWidth - 300;
    const minWidth = 300;

    if (newWidth >= minWidth && newWidth <= maxWidth) {
      panelWidth.value = newWidth;
    }
  };

  const stopResizing = () => {
    isResizing.value = false;
    document.removeEventListener('mousemove', doResize);
    document.removeEventListener('mouseup', stopResizing);
    document.body.style.cursor = 'default';
    localStorage.setItem('right-panel-width', String(panelWidth.value));
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
    toast.error("保存排序失败: " + err);
  }
};

watch(defaultTheme, async () => {
  await nextTick();
  terminalMap.forEach(({ term }) => applyTerminalTheme(term));
}, { immediate: false });

onMounted(async () => {
  const themeId = localStorage.getItem('app-theme-id') || defaultTheme.value;
  applyTheme(themeId);
  initLocalRootPath()
  window.addEventListener("resize", handleResize);
  window.addEventListener("dragend", finishInternalDrag);
  await setupNativeDragDrop();
  const saved = localStorage.getItem('right-panel-width');
  if (saved) panelWidth.value = parseInt(saved);
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
    if (payload.session_id) {
      internalUiCleanup(payload.session_id);
    }
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
      : payload.message || "同步失败";
    toast.error(message, typeof payload === "object" ? payload.phase : "同步");
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
    await invoke("disconnect_ssh", { sessionId: session.id }).catch(console.error);
  }
  terminalMap.forEach(instance => {
    instance.term.dispose();
  });
  terminalMap.clear();
  window.removeEventListener("resize", handleResize);
  window.removeEventListener("dragend", finishInternalDrag);
  if (unlisten) unlisten();
  if (unlistenClosed) unlistenClosed();
  if (unlistenTransfer) unlistenTransfer();
  if (unlistenSync) unlistenSync();
  if (unlistenDragDrop) unlistenDragDrop();
});
</script>

<template>
  <div class="termius-container">
    <TitleBar :active-session-id="activeSessionId"/>
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
            v-model:active-session-id="activeSessionId"
            @close="closeTab"
            @clone-tab="cloneSessionToTab"
            @clone-window="cloneSessionToWindow"
            @new-local-shell="openLocalShell"
        />
        <WorkspaceHeader
            :current-server="currentServer"
            :active-id="activeId"
            :active-session-id="activeSessionId"
            :is-connecting="isConnecting"
            :is-error="isConnectError"
            :current-view-mode="currentViewMode"
            :open-sessions="openSessions"
            :servers="servers"
            :is-local-session="isActiveLocalSession"
            @toggle-view-mode="toggleViewMode"
            @connect="connectToServer"
        />

        <div class="terminal-shell">
          <div v-show="currentViewMode === 'terminal'" class="terminal-wrapper">
            <div v-if="openSessions.length > 0" class="terminal-multi-wrapper">
              <div
                  v-for="session in openSessions"
                  :key="session.id"
                  :id="`terminal-${session.id}`"
                  v-show="activeSessionId === session.id"
                  class="xterm-container"
                  @click="focusTerminal(session.id)"
              ></div>
            </div>
            <div v-else class="empty-state">
              <div class="empty-state-content">
                <div class="icon-stack">
                  <i class="fas fa-terminal main-icon"></i>
                  <div class="glow-ring"></div>
                </div>
                <h3 class="empty-title">Ready to Connect</h3>
                <p class="empty-description">Select a server from the sidebar or create a new connection to start your
                  session.</p>
                <button class="create-btn" @click="openAddModal">
                  <i class="fas fa-plus"></i>
                  New Connection
                </button>
              </div>
            </div>
          </div>

          <div v-show="currentViewMode === 'sftp'" class="sftp-wrapper">
            <div v-if="activeSessionId" class="sftp-manager">
              <div class="file-pane local-pane">
                <div class="pane-header">
                  <i class="fas fa-laptop" style="margin-right: 8px; color: #565f89;"></i>
                  <input v-model="localPath" class="path-input" @keyup.enter="refreshLocalFiles"/>
                </div>
                <div class="file-list"
                     :class="{ 'drag-over': isDraggingOverLocal }"
                     @contextmenu="handlePaneContextMenu($event, 'local')"
                     @dragover="handleDragOver"
                     @dragenter="handleDragEnter($event, 'local')"
                     @dragleave="handleDragLeave($event, 'local')"
                     @drop="handleDrop($event, 'local')">
                  <div v-for="file in localFiles" :key="file.name" class="file-item"
                       :class="{ 'is-dir': file.is_dir }"
                       :draggable="file.name !== '..'"
                       @dragstart="onDragStart($event, file, 'local')"
                       @dblclick.stop="handleFileDblClick(file, 'local')"
                       @contextmenu="handleContextMenu($event, file, 'local')">
                    <span class="file-icon">
                      <Tooltip text="双击" inline>
                        <i class="fas"
                           :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i>
                      </Tooltip>
                    </span>
                    <span class="file-name">{{ file.name }}</span>
                    <span class="file-size" v-if="!file.is_dir">
                      {{ formatSize(file.size) }}
                    </span>
                  </div>
                </div>
              </div>

              <div class="file-pane remote-pane">
                <div class="pane-header">
                  <i class="fas fa-server" style="margin-right: 8px; color: #565f89;"></i>
                  <input v-model="remotePath" class="path-input" @keyup.enter="refreshRemoteFiles"/>
                </div>
                <div class="file-list"
                     :class="{ 'drag-over': isDraggingOverRemote }"
                     @contextmenu="handlePaneContextMenu($event, 'remote')"
                     @dragover="handleDragOver"
                     @dragenter="handleDragEnter($event, 'remote')"
                     @dragleave="handleDragLeave($event, 'remote')"
                     @drop="handleDrop($event, 'remote')">
                  <div v-for="file in remoteFiles" :key="file.name" class="file-item"
                       :class="{ 'is-dir': file.is_dir }"
                       :draggable="file.name !== '..'"
                       @dragstart="onDragStart($event, file, 'remote')"
                       @dblclick.stop="handleFileDblClick(file, 'remote')"
                       @contextmenu="handleContextMenu($event, file, 'remote')">
                    <span class="file-icon">
                      <Tooltip text="双击" inline>
                        <i class="fas"
                           :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i>
                      </Tooltip>
                    </span>
                    <span class="file-name">{{ file.name }}</span>
                    <span class="file-size" v-if="!file.is_dir">{{ (file.size / 1024).toFixed(1) }} KB</span>
                  </div>
                </div>
              </div>

              <div class="transfer-status" v-if="transferTasks.length > 0">
                <div class="status-header">
                  <div class="header-left"><i class="fas fa-layer-group"></i><span>传输队列 ({{
                      transferTasks.length
                    }})</span></div>
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
        <StatusBar :open-sessions="openSessions" :current-server="currentServer"/>
      </main>

      <div class="right-dock">
        <div class="icon-bar">
          <div class="top-group">
            <Tooltip text="快捷命令" placement="left">
              <div class="icon-item" :class="{ active: rightPanelVisible && rightPanelType === 'quick' }"
                   @click="toggleRightPanel('quick')">
                <i class="fas fa-bolt"></i>
              </div>
            </Tooltip>
            <Tooltip text="AI 助手" placement="left">
              <div class="icon-item" :class="{ active: rightPanelVisible && rightPanelType === 'ai' }"
                   @click="toggleRightPanel('ai')">
                <i class="fas fa-robot"></i>
              </div>
            </Tooltip>
            <Tooltip text="Redis 数据库" placement="left">
              <div class="icon-item"
                   :class="{ active: rightPanelVisible && rightPanelType === 'redis' }"
                   @click="toggleRightPanel('redis')">
                <svg class="redis-icon" viewBox="0 0 24 24" width="18" height="18">
                  <path fill="currentColor" d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
                </svg>
              </div>
            </Tooltip>
          </div>
          <div class="bottom-group">
            <Tooltip text="局域网聊天" placement="left">
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
            <Tooltip text="同步设置" placement="left">
              <div class="icon-item"
                   :class="{
                     active: rightPanelVisible && rightPanelType === 'sync-settings',
                     'is-syncing': isSyncing
                   }"
                   @click="toggleRightPanel('sync-settings')">
                <i class="fas fa-sync-alt" :class="{ 'fa-spin': isSyncing }"></i>
              </div>
            </Tooltip>
            <Tooltip text="主题设置" placement="left">
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
              :class="{ 'is-redis': rightPanelType === 'redis' }"
              :style="{ width: panelWidth + 'px' }"
          >
            <div class="panel-resizer" @mousedown="startResizing"></div>

            <div class="panel-content-wrapper">
              <KeepAlive :max="5">
                <component :is="rightPanelComponent" :activeSessionId="activeSessionId" @update-online-count="handleOnlineCountUpdate"/>
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
            <span class="menu-text">新建文件</span>
          </div>
          <div class="menu-item" @click="handleMenuAction('newFolder')">
            <i class="fas fa-folder-plus"></i>
            <span class="menu-text">新建文件夹</span>
          </div>
          <div class="menu-divider"></div>
          <div class="menu-item" :class="{ disabled: !canPaste }" @click="handleMenuAction('paste')">
            <i class="fas fa-paste"></i>
            <span class="menu-text">粘贴</span>
          </div>
          <div class="menu-divider"></div>
          <div class="menu-item" @click="handleMenuAction('refresh')">
            <i class="fas fa-rotate-right"></i>
            <span class="menu-text">刷新</span>
          </div>
        </template>

        <template v-else>
          <div class="menu-item" @click="handleMenuAction('copy')">
            <i class="fas fa-copy"></i>
            <span class="menu-text">复制</span>
          </div>
          <div class="menu-item" @click="handleMenuAction('cut')">
            <i class="fas fa-scissors"></i>
            <span class="menu-text">剪切</span>
          </div>
          <div class="menu-item" :class="{ disabled: !canPaste }" @click="handleMenuAction('paste')">
            <i class="fas fa-paste"></i>
            <span class="menu-text">粘贴</span>
          </div>
          <div class="menu-item" @click="handleMenuAction('copyPath')">
            <i class="fas fa-link"></i>
            <span class="menu-text">复制路径</span>
          </div>

          <div class="menu-divider"></div>

          <div class="menu-item" @click="handleMenuAction('transfer')">
            <i class="fas" :class="contextSource === 'local' ? 'fa-cloud-upload-alt' : 'fa-cloud-download-alt'"></i>
            <span class="menu-text">
              {{ contextSource === 'local' ? '上传到远程' : '下载到本地' }}
            </span>
          </div>

          <div class="menu-divider"></div>

          <div class="menu-item" @click="handleMenuAction('info')">
            <i class="fas fa-circle-info"></i>
            <span class="menu-text">文件信息</span>
          </div>

          <div v-if="contextSource === 'local'" class="menu-item" @click="handleMenuAction('openExplorer')">
            <i class="fas fa-folder-open"></i>
            <span class="menu-text">在资源管理器中打开</span>
          </div>

          <div class="menu-item" @click="handleMenuAction('rename')">
            <i class="fas fa-pen"></i>
            <span class="menu-text">重命名</span>
          </div>

          <div v-if="contextSource === 'remote'" class="menu-item" @click="handleMenuAction('chmod')">
            <i class="fas fa-key"></i>
            <span class="menu-text">修改权限</span>
          </div>

          <div class="menu-divider"></div>

          <div class="menu-item" @click="handleMenuAction('refresh')">
            <i class="fas fa-rotate-right"></i>
            <span class="menu-text">刷新</span>
          </div>

          <div class="menu-divider"></div>

          <div class="menu-item danger" @click="handleMenuAction('delete')">
            <i class="fas fa-trash-alt"></i>
            <span class="menu-text">删除</span>
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