<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import "xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

const appWindow = getCurrentWindow();

// --- 数据定义 ---
const servers = ref<any[]>([]);
const activeId = ref<string | null>(null);
const openSessions = ref<{id: string, serverId: string, name: string}[]>([]);
const activeSessionId = ref<string | null>(null);
const showPassword = ref(false);

// --- 终端池管理 ---
const terminalMap = new Map<string, { term: Terminal; fitAddon: FitAddon }>();
let unlisten: UnlistenFn | null = null;
let unlistenClosed: UnlistenFn | null = null;
let unlistenTransfer: UnlistenFn | null = null; // 新增：传输进度监听

/**
 * 初始化指定 Session 的终端
 */
const initTerminal = async (sessionId: string) => {
  if (terminalMap.has(sessionId)) {
    await nextTick();
    terminalMap.get(sessionId)?.fitAddon.fit();
    return;
  }

  const term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    theme: {
      background: "#1a1b26",
      foreground: "#a9b1d6",
      cursor: "#7aa2f7",
      selectionBackground: "rgba(122, 162, 247, 0.3)",
    },
    allowProposedApi: true,
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

  await nextTick();
  const container = document.getElementById(`terminal-${sessionId}`);
  if (container) {
    term.open(container);
    fitAddon.fit();

    term.onData((data) => {
      invoke("write_to_ssh", { sessionId, data });
    });

    terminalMap.set(sessionId, { term, fitAddon });
  }
};

const isConnecting = ref(false);

/**
 * 基础连接逻辑
 */
const connectToServer = async () => {
  const server = servers.value.find(s => s.id === activeId.value);
  if (!server) return;

  isConnecting.value = true;
  const sessionId = server.id;

  if (!openSessions.value.find(s => s.id === sessionId)) {
    openSessions.value.push({ id: sessionId, serverId: server.id, name: server.name });
  }

  activeSessionId.value = sessionId;
  await initTerminal(sessionId);

  const instance = terminalMap.get(sessionId);
  instance?.term.writeln(`\r\n\x1b[33mConnecting to ${server.name}...\x1b[0m`);

  try {
    await invoke("connect_ssh", { serverId: server.id, sessionId });
    instance?.term.writeln("\x1b[32mSSH Connection Established.\x1b[0m\r\n");
    focusTerminal(sessionId);
  } catch (err) {
    instance?.term.writeln(`\r\n\x1b[31mConnection Error: ${err}\x1b[0m`);
  } finally {
    isConnecting.value = false;
  }
};

/**
 * 新增：克隆当前会话
 */
const cloneSession = async () => {
  const currentTab = openSessions.value.find(s => s.id === activeSessionId.value);
  if (!currentTab) return;

  const server = servers.value.find(s => s.id === currentTab.serverId);
  if (!server) return;

  const newSessionId = `${server.id}-clone-${Date.now()}`;

  openSessions.value.push({
    id: newSessionId,
    serverId: server.id,
    name: `${server.name} (Clone)`
  });

  activeSessionId.value = newSessionId;
  await initTerminal(newSessionId);

  const instance = terminalMap.get(newSessionId);
  instance?.term.writeln(`\r\n\x1b[33mCloning session for ${server.name}...\x1b[0m`);

  try {
    await invoke("connect_ssh", { serverId: server.id, sessionId: newSessionId });
    instance?.term.writeln("\x1b[32mCloned Session Established.\x1b[0m\r\n");
    focusTerminal(newSessionId);
  } catch (err) {
    instance?.term.writeln(`\r\n\x1b[31mClone Error: ${err}\x1b[0m`);
  }
};

const internalUiCleanup = (id: string) => {
  const instance = terminalMap.get(id);
  if (instance) {
    instance.term.dispose();
    terminalMap.delete(id);
  }
  openSessions.value = openSessions.value.filter(s => s.id !== id);
  if (activeSessionId.value === id) {
    activeSessionId.value = openSessions.value.length > 0 ? openSessions.value[openSessions.value.length - 1].id : null;
  }
};

/**
 * 关闭标签页
 */
const closeTab = async (e: Event | null, id: string) => {
  e?.stopPropagation();
  await invoke("disconnect_ssh", { sessionId: id }).catch(console.error);
  internalUiCleanup(id);
};

// --- SFTP 逻辑相关变量 ---
const localPath = ref("C:/");
const remotePath = ref("/");
const localFiles = ref<any[]>([]);
const remoteFiles = ref<any[]>([]);
const transferTasks = ref<any[]>([]);
const currentViewMode = ref<'terminal' | 'sftp'>('terminal');

/**
 * 切换模式
 */
const toggleViewMode = async () => {
  if (!activeSessionId.value) return;
  currentViewMode.value = currentViewMode.value === 'terminal' ? 'sftp' : 'terminal';
  if (currentViewMode.value === 'sftp') {
    await refreshRemoteFiles();
    await refreshLocalFiles();
  }
};

const refreshLocalFiles = async () => {
  try {
    localFiles.value = await invoke("list_local_dir", { path: localPath.value });
  } catch (e) { console.error("Local list failed", e); }
};

const refreshRemoteFiles = async () => {
  try {
    remoteFiles.value = await invoke("list_remote_dir", {
      sessionId: activeSessionId.value,
      path: remotePath.value
    });
  } catch (e) { console.error("Remote list failed", e); }
};

/**
 * 文件夹双击跳转
 */
/**
 * 格式化路径，确保没有多余的分隔符
 */
const normalizePath = (parts: string[], isRemote: boolean) => {
  const separator = isRemote ? '/' : (localPath.value.includes('\\') ? '\\' : '/');
  // 过滤空字符串，但在 Linux 远程路径下保留开头的空（代表根目录 /）
  let filtered = parts.filter(p => p !== "");
  if (isRemote) return '/' + filtered.join('/');
  return filtered.join(separator);
};

const isDraggingOverRemote = ref(false);
const isDraggingOverLocal = ref(false);

// 处理拖入高亮
const handleDragEnter = (e: DragEvent, type: 'local' | 'remote') => {
  e.preventDefault();
  if (type === 'remote') isDraggingOverRemote.value = true;
  else isDraggingOverLocal.value = true;
};

const handleDragLeave = (e: DragEvent, type: 'local' | 'remote') => {
  e.preventDefault();
  if (type === 'remote') isDraggingOverRemote.value = false;
  else isDraggingOverLocal.value = false;
};

const handleFileDblClick = async (file: any, type: 'local' | 'remote') => {
  // 1. 拦截非目录点击（除了 '..' 之外）
  if (!file.is_dir && file.name !== '..') return;

  const isRemote = type === 'remote';
  let currentPath = isRemote ? remotePath.value : localPath.value;

  // 标准化路径：去掉结尾的斜杠，方便分割
  currentPath = currentPath.replace(/[/\\]$/, '');

  if (file.name === '..') {
    // --- 返回上级逻辑 ---

    // 使用正则分割路径
    let parts = currentPath.split(/[/\\]/).filter(p => p !== "");

    if (isRemote) {
      // 远程 Linux 处理
      parts.pop();
      currentPath = '/' + parts.join('/');
    } else {
      // 本地 Windows 处理
      if (parts.length > 1) {
        parts.pop();
        currentPath = parts.join('\\');
        // 如果弹出后只剩盘符 (如 C:)，补全为 C:\
        if (currentPath.length === 2 && currentPath.endsWith(':')) {
          currentPath += '\\';
        }
      } else {
        // 已经在盘符根目录了，或者是 Unix 根目录
        currentPath = currentPath.endsWith(':') ? currentPath + '\\' : currentPath;
      }
    }
  } else {
    // --- 进入下级逻辑 ---
    const separator = isRemote ? '/' : (currentPath.includes('\\') ? '\\' : '/');
    currentPath = `${currentPath}${separator}${file.name}`;
  }

  // 最后兜底：确保路径不为空
  if (!currentPath || currentPath === "") {
    currentPath = isRemote ? "/" : "C:\\";
  }

  console.log(`[${type}] 最终跳转路径:`, currentPath);

  try {
    if (isRemote) {
      remotePath.value = currentPath;
      await refreshRemoteFiles();
    } else {
      localPath.value = currentPath;
      await refreshLocalFiles();
    }
  } catch (err) {
    console.error("切换目录失败:", err);
  }
};

/**
 * 拖拽逻辑
 */
const onDragStart = (e: DragEvent, file: any, source: 'local' | 'remote') => {
  if (file.name === '..') return;
  e.dataTransfer?.setData("file-data", JSON.stringify({ source, file }));
};

const handleRemoteDrop = async (e: DragEvent) => {
  console.log(e, 'lll-----')
  const data = JSON.parse(e.dataTransfer?.getData("file-data") || "{}");
  if (data.source === 'local') {
    await startTransfer('upload', data.file);
  }
};

const handleLocalDrop = async (e: DragEvent) => {
  console.log(e, 'kkk-----')
  const data = JSON.parse(e.dataTransfer?.getData("file-data") || "{}");
  if (data.source === 'remote') {
    await startTransfer('download', data.file);
  }
};

const startTransfer = async (type: 'upload' | 'download', file: any) => {
  // 1. 创建任务 ID
  const taskId = Math.random().toString(36).substring(7);

  // 2. 预准备路径 (统一使用正斜杠，Rust 端通常能处理)
  const sourcePath = type === 'upload'
      ? `${localPath.value.replace(/[/\\]$/, '')}/${file.name}`
      : `${remotePath.value.replace(/\/$/, '')}/${file.name}`;

  const targetPath = type === 'upload'
      ? `${remotePath.value.replace(/\/$/, '')}/${file.name}`
      : `${localPath.value.replace(/[/\\]$/, '')}/${file.name}`;

  // 3. 添加到 UI 任务列表
  transferTasks.value.push({
    id: taskId,
    name: file.name,
    progress: 0,
    type,
    status: 'transferring'
  });

  try {
    // 4. 调用 Rust 后端
    await invoke(type === 'upload' ? "sftp_upload" : "sftp_download", {
      sessionId: activeSessionId.value,
      localPath: type === 'upload' ? sourcePath : targetPath,
      remotePath: type === 'upload' ? targetPath : sourcePath,
      taskId
    });
  } catch (err) {
    // 5. 错误处理：将任务标记为失败
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) task.status = 'error';
    alert(`传输失败: ${err}`);
  }
};

// --- 弹窗与基础逻辑 ---
const isModalOpen = ref(false);
const isEditing = ref(false);
const newHost = ref({ id: "", name: "", host: "", username: "root", port: 22, auth_type: "password", password: "", private_key_path: "", jump_host_id: "" });

const openAddModal = () => {
  isEditing.value = false;
  newHost.value = { id: "", name: "", host: "", username: "root", port: 22, auth_type: "password", password: "", private_key_path: "", jump_host_id: "" };
  isModalOpen.value = true;
};

const openEditModal = (s: any) => {
  isEditing.value = true;
  newHost.value = { ...s };
  isModalOpen.value = true;
};

const closeModal = () => {
  isModalOpen.value = false;
  showPassword.value = false;
};

const selectKeyFile = async () => {
  const selected = await open({ multiple: false, filters: [{ name: 'SSH Private Key', extensions: ['*', 'pem', 'key'] }] });
  if (selected) newHost.value.private_key_path = selected as string;
};

const loadServers = async () => {
  servers.value = await invoke("get_servers");
  if (servers.value.length > 0 && !activeId.value) activeId.value = servers.value[0].id;
};

const saveHost = async () => {
  if (newHost.value.name && newHost.value.host) {
    await invoke("save_server", { server: { ...newHost.value, port: Number(newHost.value.port) } });
    await loadServers();
    closeModal();
  }
};

const deleteServer = async (id: string) => {
  if (confirm("Delete this server configuration?")) {
    await invoke("delete_server", { id });
    await loadServers();
  }
};

const focusTerminal = async (sessionId: string | null) => {
  if (!sessionId) return;
  await nextTick();
  const instance = terminalMap.get(sessionId);
  if (instance) instance.term.focus();
};

const currentServer = computed(() => servers.value.find(s => s.id === activeId.value));

const handleResize = () => {
  terminalMap.forEach(instance => instance.fitAddon.fit());
};

watch(activeSessionId, (newId) => {
  if (newId) focusTerminal(newId);
});

onMounted(async () => {
  window.addEventListener("resize", handleResize);
  loadServers();

  unlisten = await listen("ssh-output", (event) => {
    const payload = event.payload as { session_id: string, data: string };
    const instance = terminalMap.get(payload.session_id);
    if (instance && currentViewMode.value === 'terminal') {
      instance.term.write(payload.data);
    }
  });

  unlistenClosed = await listen("ssh-closed", (event) => {
    const { session_id } = event.payload as { session_id: string };
    internalUiCleanup(session_id);
  });

  // 监听传输进度
  unlistenTransfer = await listen("transfer-progress", (event) => {
    const { taskId, progress } = event.payload as { taskId: string, progress: number };
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) {
      task.progress = progress;
      if (progress >= 100) {
        setTimeout(() => {
          transferTasks.value = transferTasks.value.filter(t => t.id !== taskId);
          if (currentViewMode.value === 'sftp') {
            refreshLocalFiles();
            refreshRemoteFiles();
          }
        }, 2000);
      }
    }
  });
});

onUnmounted(async () => {
  window.removeEventListener("resize", handleResize);
  if (unlisten) unlisten();
  if (unlistenClosed) unlistenClosed();
  if (unlistenTransfer) unlistenTransfer();

  for (const [sessionId, instance] of terminalMap) {
    await invoke("disconnect_ssh", { sessionId });
    instance.term.dispose();
  }
  terminalMap.clear();
});

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const closeApp = () => appWindow.close();
</script>

<template>
  <div class="termius-container">
    <header class="titlebar">
      <div class="titlebar-drag-handle" data-tauri-drag-region></div>
      <div class="titlebar-ui-layer">
        <div class="window-controls">
          <div class="dot close" @click="closeApp"></div>
          <div class="dot minimize" @click="minimize"></div>
          <div class="dot maximize" @click="toggleMaximize"></div>
        </div>
        <div class="title-text">Gemini SSH v2.0</div>
        <div class="titlebar-spacer"></div>
      </div>
    </header>

    <div class="main-layout">
      <aside class="sidebar">
        <div class="brand">
          <div class="logo-hex">G</div>
          <span class="brand-text">Gemini</span>
        </div>
        <div class="sidebar-scroll-area">
          <nav class="nav-groups">
            <div class="group-label">HOSTS</div>
            <div
                v-for="s in servers"
                :key="s.id"
                :class="['host-card', { active: activeId === s.id }]"
                @click="activeId = s.id"
                @dblclick="connectToServer"
            >
              <div class="host-icon-wrapper">
                <div v-if="activeId === s.id" class="pulse-ring"></div>
                <span class="icon">🖥️</span>
              </div>
              <div class="host-meta">
                <div class="name">{{ s.name }}</div>
                <div class="ip">{{ s.host }}</div>
              </div>
              <div class="host-actions">
                <span @click.stop="openEditModal(s)">⚙️</span>
                <span @click.stop="deleteServer(s.id)">×</span>
              </div>
            </div>
          </nav>
        </div>
        <div class="sidebar-footer">
          <button class="add-host-btn" @click="openAddModal">+ Add New Host</button>
        </div>
      </aside>

      <main class="workspace">
        <nav class="session-tabs">
          <div
              v-for="tab in openSessions"
              :key="tab.id"
              :class="['tab-item', { active: activeSessionId === tab.id }]"
              @click="activeSessionId = tab.id"
          >
            <span class="tab-icon">🐚</span>
            <span class="tab-name">{{ tab.name }}</span>
            <span class="tab-close" @click="closeTab($event, tab.id)">×</span>
          </div>
          <div class="tab-add" @click="openAddModal">+</div>
        </nav>

        <header class="workspace-header">
          <div class="breadcrumb">
            <span class="root">Hosts</span>
            <span class="sep">/</span>
            <span class="current">{{ currentServer?.name || 'Select a host' }}</span>
          </div>
          <div class="toolbar">
            <button
                class="action-btn mode-toggle"
                :class="{ 'is-sftp': currentViewMode === 'sftp' }"
                @click="toggleViewMode"
            >
              <i class="fas" :class="currentViewMode === 'sftp' ? 'fa-terminal' : 'fa-folder-open'"></i>
              <span>{{ currentViewMode === 'sftp' ? 'Terminal' : 'SFTP' }}</span>
            </button>

            <div class="separator"></div>

            <button
                class="action-btn clone-btn"
                @click="cloneSession"
                :disabled="!activeSessionId"
            >
              <i class="fas fa-copy"></i>
              <span>Clone</span>
            </button>

            <button
                class="connect-btn"
                @click="connectToServer"
                :disabled="!activeId || isConnecting"
                :class="{ 'loading': isConnecting }"
            >
              <i class="fas" :class="isConnecting ? 'fa-circle-notch fa-spin' : 'fa-plug'"></i>
              <span>{{ isConnecting ? 'Connecting' : 'Connect' }}</span>
            </button>
          </div>
        </header>

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
              <div class="empty-icon">🛰️</div>
              <p>No active sessions. Select a host to connect.</p>
            </div>
          </div>

          <div v-if="currentViewMode === 'sftp'" class="sftp-manager">
            <div class="file-pane local-pane">
              <div class="pane-header">
                <i class="fas fa-laptop" style="margin-right: 8px; color: #565f89;"></i>
                <input v-model="localPath" class="path-input" @keyup.enter="refreshLocalFiles" />
              </div>
              <div
                  class="file-list"
                  :class="{ 'drag-over': isDraggingOverLocal }"
                  @drop="handleLocalDrop($event); isDraggingOverLocal = false"
                  @dragover.prevent
                  @dragenter.prevent="isDraggingOverLocal = true"
                  @dragleave.prevent="isDraggingOverLocal = false"
              >
                <div
                    v-for="file in localFiles"
                    :key="file.name"
                    :class="['file-item', { 'is-dir': file.is_dir }]"
                    draggable="true"
                    @dragstart="onDragStart($event, file, 'local')"
                    @dblclick="handleFileDblClick(file, 'local')"
                >
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
                <input v-model="remotePath" class="path-input" @keyup.enter="refreshRemoteFiles" />
              </div>
              <div
                  class="file-list"
                  :class="{ 'drag-over': isDraggingOverRemote }"
                  @drop="handleRemoteDrop($event); isDraggingOverRemote = false"
                  @dragover.prevent
                  @dragenter.prevent="isDraggingOverRemote = true"
                  @dragleave.prevent="isDraggingOverRemote = false"
              >
                <div
                    v-for="file in remoteFiles"
                    :key="file.name"
                    :class="['file-item', { 'is-dir': file.is_dir }]"
                    draggable="true"
                    @dragstart="onDragStart($event, file, 'remote')"
                    @dblclick="handleFileDblClick(file, 'remote')"
                >
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
                <i class="fas fa-sync-alt fa-spin"></i> 正在传输 ({{ transferTasks.length }})
              </div>
              <div class="task-list-wrapper">
                <div v-for="task in transferTasks" :key="task.id" class="task-row">
                  <div class="task-info">
                    <span class="task-name">{{ task.name }}</span>
                    <span class="task-percent">{{ task.progress }}%</span>
                  </div>
                  <div class="progress-container">
                    <div class="progress-bar" :style="{ width: task.progress + '%' }"></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>



        <footer class="status-bar">
          <div class="status-left">
            <span class="status-item">
              <i :class="['dot', { online: openSessions.length > 0 }]"></i>
              {{ openSessions.length > 0 ? 'Connected' : 'Idle' }}
            </span>
          </div>
          <div class="status-right">
            <span v-if="currentServer?.jump_host_id" class="status-item">Jump: Active</span>
            <span class="status-item">UTF-8</span>
          </div>
        </footer>
      </main>
    </div>

    <Transition name="scale">
      <div v-if="isModalOpen" class="modal-overlay" @click.self="closeModal">
        <div class="modal-content">
          <div class="modal-header">
            <h3>{{ isEditing ? 'Edit Host' : 'Add New Host' }}</h3>
            <button class="close-btn" @click="closeModal">×</button>
          </div>
          <div class="modal-body scrollable">
            <div class="form-group">
              <label>Display Name</label>
              <input v-model="newHost.name" type="text" placeholder="Server Alpha" />
            </div>
            <div class="form-row">
              <div class="form-group flex-3">
                <label>Hostname / IP</label>
                <input v-model="newHost.host" type="text" placeholder="127.0.0.1" />
              </div>
              <div class="form-group flex-1">
                <label>Port</label>
                <div class="port-input-wrapper">
                  <button type="button" class="port-btn" @click="newHost.port > 1 && newHost.port--">-</button>
                  <input v-model.number="newHost.port" type="number" />
                  <button type="button" class="port-btn" @click="newHost.port < 65535 && newHost.port++">+</button>
                </div>
              </div>
            </div>
            <div class="form-group">
              <label>Username</label>
              <input v-model="newHost.username" type="text" />
            </div>
            <div class="form-group">
              <label>Auth Type</label>
              <select v-model="newHost.auth_type" class="styled-select">
                <option value="password">Password</option>
                <option value="key">SSH Key</option>
              </select>
            </div>
            <div v-if="newHost.auth_type === 'password'" class="form-group">
              <label>Password</label>
              <div class="password-wrapper">
                <input v-model="newHost.password" :type="showPassword ? 'text' : 'password'" />
                <button type="button" class="eye-btn" @click="showPassword = !showPassword">👁️</button>
              </div>
            </div>
            <div v-else class="form-group">
              <label>Private Key Path</label>
              <div class="file-picker">
                <input v-model="newHost.private_key_path" type="text" readonly />
                <button @click="selectKeyFile">Browse</button>
              </div>
            </div>
            <div class="form-group">
              <label>Jump Host (Optional)</label>
              <select v-model="newHost.jump_host_id" class="styled-select">
                <option value="">Direct Connection</option>
                <option v-for="s in servers.filter(x => x.id !== newHost.id)" :key="s.id" :value="s.id">{{ s.name }}</option>
              </select>
            </div>
          </div>
          <div class="modal-footer">
            <button class="cancel-btn" @click="closeModal">Cancel</button>
            <button class="save-btn" @click="saveHost">Save</button>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style lang="scss">
body { margin: 0; padding: 0; overflow: hidden; background-color: #0f111a; user-select: none; font-family: 'Inter', sans-serif; }
/* 滚动条美化 */
::-webkit-scrollbar { width: 5px; height: 5px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb {
  background: #292e42;
  border-radius: 10px;
  &:hover { background: #3b4261; }
}
input, select {
  &:focus {
    border-color: #7aa2f7 !important;
    box-shadow: 0 0 0 2px rgba(122, 162, 247, 0.2);
    outline: none;
  }
}
/* 确保这段代码在全局 style 中，或者去掉 scoped */
.file-icon {
  width: 24px;
  display: inline-block;
  text-align: center;

  i.fas {
    /* 强制指定，防止被其他样式覆盖 */
    font-family: "Font Awesome 6 Free", "Font Awesome 5 Free" !important;
    font-weight: 900 !important;
    font-style: normal;
  }

  /* 使用具体的类名着色 */
  .fa-folder {
    color: #e0af68 !important;
  }
  .fa-file-alt {
    color: #7aa2f7 !important;
  }
  .fa-level-up-alt {
    color: #9ece6a !important;
  }
}
</style>

<style lang="scss" scoped>
/* 样式部分保持一致 */
$bg-dark: #0f111a; $bg-sidebar: #16161e; $accent: #7aa2f7; $border-color: #292e42; $text-dim: #565f89;

.clone-btn {
  background: #24283b;
  color: #a9b1d6;
  border: 1px solid #292e42;
  padding: 8px 15px;
  border-radius: 8px;
  cursor: pointer;
  font-weight: 600;
  margin-right: 10px;
  transition: all 0.2s;

  &:hover:not(:disabled) {
    background: #2d334a;
    border-color: #7aa2f7;
    color: #7aa2f7;
  }

  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  background: rgba(26, 27, 38, 0.5);
  border-radius: 10px;
  border: 1px solid #292e42;

  .separator {
    width: 1px;
    height: 20px;
    background: #292e42;
    margin: 0 4px;
  }

  /* 基础按钮样式 */
  .action-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid transparent;
    background: transparent;
    color: #a9b1d6;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

    i { font-size: 14px; }

    &:hover:not(:disabled) {
      background: rgba(122, 162, 247, 0.1);
      color: #7aa2f7;
      border-color: rgba(122, 162, 247, 0.2);
    }

    &:disabled {
      opacity: 0.3;
      cursor: not-allowed;
    }
  }

  /* 模式切换高亮 */
  .mode-toggle.is-sftp {
    background: rgba(224, 175, 104, 0.1);
    color: #e0af68;
    border-color: rgba(224, 175, 104, 0.3);

    &:hover {
      background: rgba(224, 175, 104, 0.2);
    }
  }

  /* 连接按钮 - 突出显示 */
  .connect-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    border-radius: 6px;
    border: none;
    background: #7aa2f7;
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: 0 2px 8px rgba(122, 162, 247, 0.3);

    &:hover:not(:disabled) {
      background: #89ddff;
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(122, 162, 247, 0.4);
    }

    &:active {
      transform: translateY(0);
    }

    &:disabled {
      background: #24283b;
      color: #565f89;
      box-shadow: none;
      cursor: not-allowed;
    }

    &.loading i {
      animation: spin 1s linear infinite;
    }
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.sftp-manager {
  display: flex;
  height: 100%;       /* 确保填满外层 terminal-shell */
  background: #1a1b26;
  overflow: hidden;   /* 防止外层溢出 */

  .file-pane {
    flex: 1;
    display: flex;
    flex-direction: column; /* 让 header 和 list 垂直排列 */
    height: 100%;           /* 关键：必须限制高度 */
    min-width: 0;           /* 防止内容撑开 Flex 项目 */
    border-right: 1px solid #24283b;

    .pane-header {
      flex-shrink: 0;
      padding: 10px 12px;
      background: #16161e;
    }

    .file-list {
      flex: 1;
      overflow-y: auto;
      overflow-x: hidden;
      padding: 4px 0;

      scrollbar-gutter: stable;
      transition: background-color 0.2s, outline 0.2s;
      &.drag-over {
        background-color: rgba(122, 162, 247, 0.1) !important;
        outline: 2px dashed #7aa2f7;
        outline-offset: -4px;
        * {
          pointer-events: none !important;
        }
      }
    }

    .transfer-status {
      position: absolute;
      bottom: 20px;
      right: 20px;
      width: 280px;
      background: #1a1b26;
      border: 1px solid #292e42;
      border-radius: 12px;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
      padding: 12px;
      z-index: 1000;

      .status-header {
        font-size: 11px;
        font-weight: bold;
        color: #7aa2f7;
        margin-bottom: 12px;
        display: flex;
        align-items: center;
        gap: 8px;
      }

      .task-list-wrapper {
        max-height: 200px;
        overflow-y: auto;
      }

      .task-row {
        margin-bottom: 10px;

        .task-info {
          display: flex;
          justify-content: space-between;
          font-size: 11px;
          color: #a9b1d6;
          margin-bottom: 4px;

          .task-name {
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            max-width: 180px;
          }
        }

        .progress-container {
          height: 4px;
          background: #24283b;
          border-radius: 2px;
          overflow: hidden;

          .progress-bar {
            height: 100%;
            background: linear-gradient(90deg, #7aa2f7, #bb9af7);
            transition: width 0.3s ease;
          }
        }
      }
    }
  }

  .file-item {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    font-size: 13px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.02);
    color: #a9b1d6;
    user-select: none;
    -webkit-user-drag: element;

    &:hover { background: #2f334d; }

    .file-icon {
      width: 20px;
      margin-right: 10px;
      text-align: center;
      color: #565f89; // 默认文件图标颜色
    }

    .file-name {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    /* 文件夹高亮 */
    &.is-dir {
      .file-icon { color: #e0af68; } // 橙黄色文件夹
      .file-name { color: #7aa2f7; font-weight: 500; } // 蓝色加粗文字
    }

    .file-size {
      color: #444b6a;
      font-size: 11px;
      margin-left: 8px;
    }
  }
}

// 路径导航栏优化
.pane-header {
  display: flex;
  align-items: center;
  gap: 8px;
  background: #16161e !important;
  border-bottom: 1px solid #292e42;

  .path-input {
    background: transparent;
    border: none;
    color: #7aa2f7;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    width: 100%;
    &:focus { outline: none; background: #1a1b26; }
  }
}

/* 彻底隐藏 Chrome, Safari, Edge, Opera 的加减箭头 */
input::-webkit-outer-spin-button,
input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

/* 彻底隐藏 Firefox 的加减箭头 */
input[type="number"] {
  -moz-appearance: textfield;
}

.port-input-wrapper {
  display: flex;
  align-items: center;
  background: #0f111a;
  border: 1px solid #292e42;
  border-radius: 8px;
  overflow: hidden;
  transition: border-color 0.2s;

  &:focus-within {
    border-color: #7aa2f7;
  }

  input {
    border: none !important;
    text-align: center;
    padding: 10px 0 !important;
    width: 100%;
    background: transparent !important;
  }

  .port-btn {
    background: transparent;
    color: #565f89;
    padding: 0 10px;
    font-size: 14px;
    height: 100%;
    cursor: pointer;
    border: none;

    &:hover {
      color: #7aa2f7;
      background: rgba(122, 162, 247, 0.1);
    }
  }
}

.termius-container { display: flex; flex-direction: column; height: 100vh; width: 100vw; background-color: $bg-dark; color: #a9b1d6; border: 1px solid $border-color; box-sizing: border-box; }
.titlebar { height: 38px; background-color: $bg-sidebar; position: relative; z-index: 1000; flex-shrink: 0;
  .titlebar-drag-handle { position: absolute; top: 0; left: 0; width: 100%; height: 100%; z-index: 10; }
  .titlebar-ui-layer { position: absolute; top: 0; left: 0; width: 100%; height: 100%; display: flex; align-items: center; justify-content: space-between; padding: 0 15px; z-index: 20; pointer-events: none;
    .window-controls { display: flex; gap: 8px; pointer-events: auto;
      .dot { width: 12px; height: 12px; border-radius: 50%; cursor: pointer; transition: filter 0.2s;
        &.close { background: #ff5f56; } &.minimize { background: #ffbd2e; } &.maximize { background: #27c93f; } &:hover { filter: brightness(1.2); } } }
    .title-text { font-size: 11px; color: $text-dim; font-weight: 600; text-transform: uppercase; letter-spacing: 1px; }
    .titlebar-spacer { width: 60px; } } }

.main-layout { display: flex; flex: 1; overflow: hidden; }
.sidebar { width: 260px; background-color: $bg-sidebar; border-right: 1px solid #1a1b26; display: flex; flex-direction: column; height: 100%;
  .brand { flex-shrink: 0; display: flex; align-items: center; gap: 12px; padding: 20px;
    .logo-hex { width: 30px; height: 30px; background: linear-gradient(45deg, $accent, #bb9af7); border-radius: 8px; display: flex; align-items: center; justify-content: center; color: #fff; font-weight: 800; }
    .brand-text { font-weight: 700; color: #fff; font-size: 18px; } }
  .sidebar-scroll-area { flex: 1; overflow-y: auto; padding: 0 12px; }
  .sidebar-footer { flex-shrink: 0; padding: 12px; border-top: 1px solid rgba(255, 255, 255, 0.03); } }

.host-card { display: flex; align-items: center; gap: 12px; padding: 12px; border-radius: 12px; margin-bottom: 6px; cursor: pointer; transition: all 0.2s; position: relative;
  &:hover { background: #1a1b26; .host-actions { opacity: 1; } }
  &.active { background: #24283b; box-shadow: 0 4px 15px rgba(0,0,0,0.2); }
  .host-icon-wrapper { position: relative; width: 36px; height: 36px; background: #1a1b26; border-radius: 10px; display: flex; align-items: center; justify-content: center; }
  &.active .host-icon-wrapper { background: $accent; color: #fff; }
  .host-meta { .name { font-size: 13px; font-weight: 600; color: #c0caf5; } .ip { font-size: 11px; color: $text-dim; } }
  .host-actions { position: absolute; right: 10px; display: flex; gap: 8px; opacity: 0; transition: opacity 0.2s; span { cursor: pointer; color: $text-dim; &:hover { color: $accent; } } } }

.host-card.active .pulse-ring {
  border-color: #e0af68;
  display: block;
}

.add-host-btn { width: 100%; background: #24283b; border: 1px dashed $text-dim; color: $accent; padding: 12px; border-radius: 12px; cursor: pointer; &:hover { border-color: $accent; background: #2d334a; } }

.workspace { flex: 1; display: flex; flex-direction: column; background: $bg-dark; overflow: hidden; }
.session-tabs { height: 44px; background: $bg-sidebar; display: flex; align-items: flex-end; padding-left: 10px; gap: 4px; flex-shrink: 0;
  .tab-item { height: 34px; padding: 0 15px; background: #1a1b26; border-radius: 10px 10px 0 0; display: flex; align-items: center; gap: 8px; font-size: 12px; color: $text-dim; cursor: pointer;
    &.active { background: $bg-dark; color: $accent; border-bottom: 2px solid $accent; }
    .tab-close { font-size: 16px; margin-left: 5px; &:hover { color: #ff5f56; } } }
  .tab-add { padding: 0 12px 10px; color: $text-dim; cursor: pointer; font-size: 20px; &:hover { color: $accent; } } }

.workspace-header { height: 56px; display: flex; justify-content: space-between; align-items: center; padding: 0 24px; flex-shrink: 0;
  .breadcrumb { font-size: 13px; .current { color: #fff; font-weight: 600; } }
  .connect-primary { background: $accent; color: #fff; border: none; padding: 8px 20px; border-radius: 8px; font-weight: 600; cursor: pointer; &:disabled { opacity: 0.5; cursor: not-allowed; } } }

.terminal-shell { flex: 1; margin: 0 16px 16px; background: #1a1b26; border-radius: 12px; border: 1px solid $border-color; position: relative; overflow: hidden;
  .terminal-multi-wrapper { width: 100%; height: 100%; }
  .xterm-container { width: 100%; height: 100%; padding: 10px; box-sizing: border-box;
    :deep(.xterm) { padding: 4px; .xterm-viewport { background-color: transparent !important; } } }
  .empty-state { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; color: $text-dim; .empty-icon { font-size: 48px; opacity: 0.3; } } }

.status-bar { height: 30px; background: $bg-sidebar; border-top: 1px solid $border-color; display: flex; justify-content: space-between; align-items: center; padding: 0 15px; font-size: 11px; color: $text-dim;
  .dot { width: 7px; height: 7px; border-radius: 50%; background: #414868; &.online { background: #9ece6a; box-shadow: 0 0 8px #9ece6a; } } }

.modal-overlay { position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0, 0, 0, 0.7); backdrop-filter: blur(8px); display: flex; align-items: center; justify-content: center; z-index: 2000; }
.modal-content { width: 440px; background: #1a1b26; border: 1px solid $border-color; border-radius: 20px; max-height: 90vh; display: flex; flex-direction: column;
  .modal-header { padding: 20px 24px; display: flex; justify-content: space-between; h3 { margin: 0; color: #fff; } .close-btn { background: none; border: none; color: #fff; font-size: 20px; cursor: pointer; } }
  .modal-body { padding: 24px; overflow-y: auto; flex: 1;
    .form-group { margin-bottom: 18px; label { display: block; font-size: 11px; color: $text-dim; margin-bottom: 8px; }
      input, .styled-select { width: 100%; background: $bg-dark; border: 1px solid $border-color; padding: 12px; border-radius: 10px; color: #c0caf5; outline: none; box-sizing: border-box; } }
    .form-row { display: flex; gap: 15px; .flex-3 { flex: 3; } .flex-1 { flex: 1; } }
    .file-picker { display: flex; gap: 8px; input { flex: 1; } button { background: #24283b; border: 1px solid $border-color; color: #c0caf5; padding: 0 15px; border-radius: 10px; cursor: pointer; } }
  }
  .modal-footer { padding: 20px 24px; background: #16161e; display: flex; justify-content: flex-end; gap: 12px; border-radius: 0 0 20px 20px;
    button { padding: 10px 20px; border-radius: 8px; cursor: pointer; }
    .cancel-btn { background: transparent; border: 1px solid $border-color; color: #a9b1d6; }
    .save-btn { background: $accent; border: none; color: #fff; font-weight: 600; } } }

.scale-enter-active, .scale-leave-active { transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1); }
.scale-enter-from, .scale-leave-to { opacity: 0; transform: scale(0.9); }
.pulse-ring { position: absolute; width: 100%; height: 100%; border: 2px solid $accent; border-radius: 10px; animation: pulse 2s infinite; }

.password-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  input { padding-right: 40px !important; width: 100%; }
  .eye-btn {
    position: absolute; right: 8px; background: transparent; border: none; padding: 4px; cursor: pointer; font-size: 16px; display: flex; align-items: center; justify-content: center; transition: transform 0.1s;
    &:hover { transform: scale(1.1); }
  }
}
@keyframes pulse { 0% { transform: scale(1); opacity: 0.5; } 100% { transform: scale(1.4); opacity: 0; } }
</style>