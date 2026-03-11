<script setup lang="ts">
import {ref, computed, onMounted, onUnmounted, nextTick, watch} from "vue";
import {getCurrentWindow} from "@tauri-apps/api/window";
import {Terminal} from "xterm";
import {FitAddon} from "xterm-addon-fit";
import "xterm/css/xterm.css";
import {invoke} from "@tauri-apps/api/core";
import {listen, UnlistenFn} from "@tauri-apps/api/event";
import { ask } from '@tauri-apps/plugin-dialog';
import { toast } from './utils/toast.ts';

// 组件导入
import Sidebar from "./components/Sidebar.vue";
import TerminalTabs from "./components/TerminalTabs.vue";
import WorkspaceHeader from "./components/WorkspaceHeader.vue";
import StatusBar from "./components/StatusBar.vue";
import ServerModal from "./components/ServerModal.vue";
import TitleBar from "./components/TitleBar.vue";
import QuickCommandPanel from "./components/QuickCommandPanel.vue";
import AiAssistantPanel from "./components/AiAssistantPanel.vue";
import SyncSettings from "./components/SyncSettings.vue";
import ThemeSettings from "./components/ThemeSettings.vue";
import RedisManager from "./components/RedisManager.vue";

const appWindow = getCurrentWindow();

// --- 基础状态 ---
const servers = ref<any[]>([]);
const activeId = ref<string | null>(null);
const openSessions = ref<{ id: string, serverId: string, name: string }[]>([]);
const activeSessionId = ref<string | null>(null);
const showPassword = ref(false);
const sessionViewModes = ref<Record<string, 'terminal' | 'sftp'>>({});
const terminalMap = new Map<string, { term: Terminal; fitAddon: FitAddon }>();

// --- UI 状态 ---
const isConnecting = ref(false);
const rightPanelVisible = ref(false);
const isModalOpen = ref(false);
const isEditing = ref(false);

// --- 传输与事件监听 ---
let unlisten: UnlistenFn | null = null;
let unlistenClosed: UnlistenFn | null = null;
let unlistenTransfer: UnlistenFn | null = null;
const transferTasks = ref<any[]>([]);

const rightPanelType = ref<'quick' | 'ai' | 'redis' | 'history' | 'sync-settings'>('quick');

// --- SFTP 状态 ---
const localPath = ref("C:/");
const remotePath = ref("/");
const localFiles = ref<any[]>([]);
const remoteFiles = ref<any[]>([]);
const isDraggingOverLocal = ref(false);
const isDraggingOverRemote = ref(false);

// --- 右键菜单 ---
const menuVisible = ref(false);
const menuPos = ref({x: 0, y: 0});
const contextFile = ref<any>(null);
const contextSource = ref<'local' | 'remote' | null>(null);

const newHost = ref({
  id: "", name: "", host: "", username: "root", port: 22,
  auth_type: "password", password: "", private_key_path: "", jump_host_id: null
});

// --- 计算属性 ---
const currentViewMode = computed(() => {
  if (!activeSessionId.value) return 'terminal';
  return sessionViewModes.value[activeSessionId.value] || 'terminal';
});

const currentServer = computed(() => servers.value.find(s => s.id === activeId.value));

const hasActiveTasks = computed(() =>
    transferTasks.value.some(t => t.status === 'transferring')
);

// --- 交互逻辑 ---

const toggleRightPanel = (type: any) => {
  if (rightPanelVisible.value && rightPanelType.value === type) {
    rightPanelVisible.value = false;
  } else {
    rightPanelType.value = type;
    rightPanelVisible.value = true;
  }
};

const handleContextMenu = (e: MouseEvent, file: any, source: 'local' | 'remote') => {
  e.preventDefault();
  if (file.name === '..') return;
  contextFile.value = file;
  contextSource.value = source;
  menuPos.value = {x: e.clientX, y: e.clientY};
  menuVisible.value = true;

  const closeMenu = () => {
    menuVisible.value = false;
    window.removeEventListener('click', closeMenu);
  };
  window.addEventListener('click', closeMenu);
};

const handleMenuAction = async (action: 'transfer' | 'delete') => {
  if (!contextFile.value || !contextSource.value) return;
  const file = contextFile.value;
  const source = contextSource.value;
  menuVisible.value = false;

  if (action === 'transfer') {
    const type = source === 'local' ? 'upload' : 'download';
    await startTransfer(type, file);
  } else if (action === 'delete') {
    const confirmed = await ask(`确定删除${source === 'local' ? '本地' : '远程'}文件 "${file.name}"？`, {
      title: '确认删除',
      kind: 'warning',
    });

    if (confirmed) {
      try {
        if (source === 'remote') {
          const path = `${remotePath.value.replace(/\/$/, '')}/${file.name}`;
          await invoke("delete_remote_file", {sessionId: activeSessionId.value, path, isDir: file.is_dir});
          await refreshRemoteFiles();
        } else {
          // 这里可以添加删除本地文件的 invoke
          toast.info("本地删除功能待对接");
        }
      } catch (err) {
        toast.error(`删除失败: ${err}`);
      }
    }
  }
};

// --- 重写的拖拽逻辑 ---

const onDragStart = (e: DragEvent, file: any, source: 'local' | 'remote') => {
  if (file.name === '..') {
    e.preventDefault();
    return;
  }
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "copy";
    // 使用标准 text 类型提高跨浏览器稳定性
    const payload = JSON.stringify({source, file});
    e.dataTransfer.setData("text/plain", payload);
  }
};

const handleDragOver = (e: DragEvent) => {
  e.preventDefault(); // 必须调用，否则 drop 不触发
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
  // 只有当真正离开容器时才取消高亮（防止子元素干扰）
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
  isDraggingOverLocal.value = false;
  isDraggingOverRemote.value = false;

  const rawData = e.dataTransfer?.getData("text/plain");
  if (!rawData) return;

  try {
    const data = JSON.parse(rawData);
    // 逻辑：跨端拖拽才触发传输
    if (data.source === 'local' && targetType === 'remote') {
      await startTransfer('upload', data.file);
    } else if (data.source === 'remote' && targetType === 'local') {
      await startTransfer('download', data.file);
    }
  } catch (err) {
    console.error("Drop Error:", err);
  }
};

const handleFileDblClick = async (file: any, type: 'local' | 'remote') => {
  if (!file.is_dir && file.name !== '..') return;

  const isRemote = type === 'remote';
  let currentPath = isRemote ? remotePath.value : localPath.value;

  // 标准化路径处理
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
    // 针对 Windows 根目录的特殊处理
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

// --- 其他功能逻辑 (保持原有) ---

const startTransfer = async (type: 'upload' | 'download', file: any) => {
  if (file.is_dir || file.name === '..') {
    toast.error(`暂不支持${type === 'upload' ? '上传' : '下载'}文件夹，请先压缩后再操作`);
    return;
  }
  const taskId = Math.random().toString(36).substring(7);
  const sourcePath = type === 'upload' ? `${localPath.value.replace(/[/\\]$/, '')}/${file.name}` : `${remotePath.value.replace(/\/$/, '')}/${file.name}`;
  const targetPath = type === 'upload' ? `${remotePath.value.replace(/\/$/, '')}/${file.name}` : `${localPath.value.replace(/[/\\]$/, '')}/${file.name}`;

  transferTasks.value.push({id: taskId, name: file.name, progress: 0, type, status: 'transferring'});
  try {
    await invoke(type === 'upload' ? "sftp_upload" : "sftp_download", {
      sessionId: activeSessionId.value,
      localPath: type === 'upload' ? sourcePath : targetPath,
      remotePath: type === 'upload' ? targetPath : sourcePath,
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
    console.log(err, 'kkk---')
    toast.error(`传输失败: ${err}`);
  }
};

const connectToServer = async () => {
  const server = servers.value.find(s => s.id === activeId.value);
  if (!server) return;
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
    focusTerminal(sessionId);
  } catch (err) {
    toast.error(`连接失败: ${err}`);
  } finally {
    isConnecting.value = false;
  }
};

const getTerminalTheme = () => {
  const style = getComputedStyle(document.documentElement);
  return {
    background: style.getPropertyValue('--bg-primary').trim(),
    foreground: style.getPropertyValue('--text-main').trim(),
    cursor: style.getPropertyValue('--accent').trim(),
    selectionBackground: style.getPropertyValue('--accent-glow').trim(),
  };
};

const initTerminal = async (sessionId: string) => {
  if (terminalMap.has(sessionId)) {
    await nextTick();
    terminalMap.get(sessionId)?.fitAddon.fit();
    return;
  }
  const term = new Terminal({
    cursorBlink: true, fontSize: 14,
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    theme: getTerminalTheme(),
    allowProposedApi: true,
  });
  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  await nextTick();
  const container = document.getElementById(`terminal-${sessionId}`);
  if (container) {
    term.open(container);
    fitAddon.fit();
    term.onData((data) => invoke("write_to_ssh", {sessionId, data}));
    terminalMap.set(sessionId, {term, fitAddon});
  }
};

const closeTab = async (id: string) => {
  await invoke("disconnect_ssh", {sessionId: id}).catch(console.error);
  internalUiCleanup(id);
};

const internalUiCleanup = (id: string) => {
  const instance = terminalMap.get(id);
  if (instance) {
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
  if (!activeSessionId.value) return;
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
  } catch (e) { console.error(e); }
};
const refreshRemoteFiles = async () => {
  try {
    remoteFiles.value = await invoke("list_remote_dir", {sessionId: activeSessionId.value, path: remotePath.value});
  } catch (e) { console.error(e); }
};

const cloneSession = async () => {
  if (!activeSessionId.value) return;
  const sessionToClone = openSessions.value.find(s => s.id === activeSessionId.value);
  const server = servers.value.find(s => s.id === sessionToClone?.serverId);
  if (server) {
    const newSessionId = `${server.id}-${Math.random().toString(36).substring(7)}`;
    openSessions.value.push({id: newSessionId, serverId: server.id, name: `${server.name} (Copy)`});
    activeSessionId.value = newSessionId;
    sessionViewModes.value[newSessionId] = 'terminal';
    await initTerminal(newSessionId);
    try {
      await invoke("connect_ssh", {serverId: server.id, sessionId: newSessionId});
    } catch (err) { console.error(err); }
  }
};

const focusTerminal = async (sessionId: string | null) => {
  if (!sessionId) return;
  await nextTick();
  const instance = terminalMap.get(sessionId);
  if (instance) instance.term.focus();
};

const handleResize = () => {
  terminalMap.forEach(instance => instance.fitAddon.fit());
};

const openAddModal = () => {
  isEditing.value = false;
  newHost.value = {id: "", name: "", host: "", username: "root", port: 22, auth_type: "password", password: "", private_key_path: "", jump_host_id: ""};
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
  } catch (err) { console.error(err); }
};

watch(activeSessionId, (newId) => {
  if (newId) focusTerminal(newId);
});

const panelWidth = ref(400); // 默认宽度
const isResizing = ref(false);

const startResizing = (e: MouseEvent) => {
  isResizing.value = true;
  const startX = e.clientX;
  const startWidth = panelWidth.value;

  const doResize = (moveEvent: MouseEvent) => {
    if (!isResizing.value) return;
    // 因为面板在右侧，向左拖动（delta为负）应该是增加宽度
    const delta = moveEvent.clientX - startX;
    const newWidth = startWidth - delta;

    // 限制最小和最大宽度
    if (newWidth > 300 && newWidth < 800) {
      panelWidth.value = newWidth;
    }
  };

  const stopResizing = () => {
    isResizing.value = false;
    document.removeEventListener('mousemove', doResize);
    document.removeEventListener('mouseup', stopResizing);
    document.body.style.cursor = 'default';
    // 可以在这里把宽度存入 localStorage，下次打开自动恢复
    localStorage.setItem('right-panel-width', String(panelWidth.value));
  };

  document.addEventListener('mousemove', doResize);
  document.addEventListener('mouseup', stopResizing);
  document.body.style.cursor = 'col-resize';
};

onMounted(async () => {
  window.addEventListener("resize", handleResize);
  const saved = localStorage.getItem('right-panel-width');
  if (saved) panelWidth.value = parseInt(saved);
  loadServers();
  unlisten = await listen("ssh-output", (event) => {
    const payload = event.payload as { session_id: string, data: string };
    const instance = terminalMap.get(payload.session_id);
    if (instance && currentViewMode.value === 'terminal') instance.term.write(payload.data);
  });
  await listen('database-changed', () => loadServers());
  unlistenClosed = await listen("ssh-closed", (event) => {
    internalUiCleanup((event.payload as any).session_id);
  });
  unlistenTransfer = await listen("transfer-progress", (event) => {
    const {taskId, progress} = event.payload as { taskId: string, progress: number };
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) task.progress = progress;
  });
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  if (unlisten) unlisten();
  if (unlistenClosed) unlistenClosed();
  if (unlistenTransfer) unlistenTransfer();
});
</script>

<template>
  <div class="termius-container">
    <TitleBar :active-session-id="activeSessionId" />
    <div class="main-layout">
      <Sidebar
          v-model:active-id="activeId"
          :servers="servers"
          @connect="connectToServer"
          @edit="openEditModal"
          @delete="loadServers"
          @open-add-modal="openAddModal"
      />

      <main class="workspace">
        <TerminalTabs
            :open-sessions="openSessions"
            v-model:active-session-id="activeSessionId"
            @close="closeTab"
            @open-add-modal="openAddModal"
        />
        <WorkspaceHeader
            :current-server="currentServer"
            :active-id="activeId"
            :active-session-id="activeSessionId"
            :is-connecting="isConnecting"
            :current-view-mode="currentViewMode"
            :open-sessions="openSessions"
            :servers="servers"
            @toggle-view-mode="toggleViewMode"
            @clone-session="cloneSession"
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
                <p class="empty-description">Select a server from the sidebar or create a new connection to start your session.</p>
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
                      <i class="fas" :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i>
                    </span>
                    <span class="file-name">{{ file.name }}</span>
                    <span class="file-size" v-if="!file.is_dir">{{ (file.size / 1024).toFixed(1) }} KB</span>
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
                      <i class="fas" :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i>
                    </span>
                    <span class="file-name">{{ file.name }}</span>
                    <span class="file-size" v-if="!file.is_dir">{{ (file.size / 1024).toFixed(1) }} KB</span>
                  </div>
                </div>
              </div>

              <div class="transfer-status" v-if="transferTasks.length > 0">
                <div class="status-header">
                  <div class="header-left"><i class="fas fa-layer-group"></i><span>传输队列 ({{ transferTasks.length }})</span></div>
                  <div class="header-status-dot" :class="{ 'is-syncing': hasActiveTasks }"></div>
                </div>
                <div class="task-list-wrapper">
                  <TransitionGroup name="task-list">
                    <div v-for="task in transferTasks" :key="task.id" class="task-row" :class="[`status-${task.status}`]">
                      <div class="task-info">
                        <div class="name-box" :title="task.name"><i :class="getTaskIcon(task)" class="type-icon"></i><span class="task-name">{{ task.name }}</span></div>
                        <div class="task-actions">
                          <button v-if="task.status === 'transferring'" class="cancel-btn" @click.stop="cancelTask(task.id)"><i class="fas fa-times"></i></button>
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
            <div class="icon-item" title="快捷命令" :class="{ active: rightPanelVisible && rightPanelType === 'quick' }" @click="toggleRightPanel('quick')">
              <i class="fas fa-bolt"></i>
            </div>
            <div class="icon-item" title="AI 助手" :class="{ active: rightPanelVisible && rightPanelType === 'ai' }" @click="toggleRightPanel('ai')">
              <i class="fas fa-robot"></i>
            </div>
            <div class="icon-item" title="Redis 数据库" :class="{ active: rightPanelVisible && rightPanelType === 'redis' }" @click="toggleRightPanel('redis')">
              <svg class="redis-icon" viewBox="0 0 24 24" width="18" height="18"><path fill="currentColor" d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>
            </div>
          </div>
          <div class="bottom-group">
            <div class="icon-item" title="操作审计" :class="{ active: rightPanelVisible && rightPanelType === 'history' }" @click="toggleRightPanel('history')">
              <i class="fas fa-list-check"></i>
            </div>
            <div class="icon-item" title="同步设置" :class="{ active: rightPanelVisible && rightPanelType === 'sync-settings' }" @click="toggleRightPanel('sync-settings')">
              <i class="fas fa-cog"></i>
            </div>
            <div class="icon-item" title="主题设置" :class="{ active: rightPanelVisible && rightPanelType === 'theme-settings' }" @click="toggleRightPanel('theme-settings')">
              <i class="fas fa-palette"></i>
            </div>
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
              <QuickCommandPanel v-if="rightPanelType === 'quick'" :activeSessionId="activeSessionId" />
              <AiAssistantPanel v-else-if="rightPanelType === 'ai'" :activeSessionId="activeSessionId" />
              <RedisManager v-else-if="rightPanelType === 'redis'" :activeSessionId="activeSessionId" />
              <SyncSettings v-else-if="rightPanelType === 'sync-settings'" :activeSessionId="activeSessionId" />
              <ThemeSettings v-else-if="rightPanelType === 'theme-settings'" :activeSessionId="activeSessionId" />
            </div>
          </div>
        </Transition>
      </div>
    </div>

    <ServerModal :is-open="isModalOpen" :is-editing="isEditing" :server="newHost" :servers="servers" @close="closeModal" @save="saveHost"/>

    <Transition name="fade">
      <div v-if="menuVisible" class="context-menu" :style="{ top: menuPos.y + 'px', left: menuPos.x + 'px' }">
        <div class="menu-item" @click="handleMenuAction('transfer')">
          <span>{{ contextSource === 'local' ? '📤' : '📥' }}</span>
          {{ contextSource === 'local' ? '上传到服务器' : '下载到本地' }}
        </div>
        <div class="menu-item divider"></div>
        <div class="menu-item danger" @click="handleMenuAction('delete')">
          <span>🗑️</span> 删除
        </div>
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