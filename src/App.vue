<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import "xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

// 组件导入
import Sidebar from "./components/Sidebar.vue";
import TerminalTabs from "./components/TerminalTabs.vue";
import WorkspaceHeader from "./components/WorkspaceHeader.vue";
import StatusBar from "./components/StatusBar.vue";
import ServerModal from "./components/ServerModal.vue";
import TitleBar from "./components/TitleBar.vue";
import QuickCommandPanel from "./components/QuickCommandPanel.vue";
import AiAssistantPanel from "./components/AiAssistantPanel.vue";

const appWindow = getCurrentWindow();

// --- 基础状态 ---
const servers = ref<any[]>([]);
const activeId = ref<string | null>(null);
const openSessions = ref<{id: string, serverId: string, name: string}[]>([]);
const activeSessionId = ref<string | null>(null);
const showPassword = ref(false);
const sessionViewModes = ref<Record<string, 'terminal' | 'sftp'>>({});
const terminalMap = new Map<string, { term: Terminal; fitAddon: FitAddon }>();

// --- UI 状态 ---
const isConnecting = ref(false);
const rightPanelVisible = ref(false); // 控制快捷命令面板显示
const isModalOpen = ref(false);
const isEditing = ref(false);

// --- 传输与事件监听 ---
let unlisten: UnlistenFn | null = null;
let unlistenClosed: UnlistenFn | null = null;
let unlistenTransfer: UnlistenFn | null = null;
const transferTasks = ref<any[]>([]);

const rightPanelType = ref<'quick' | 'ai'>('quick');

// --- SFTP 状态 ---
const localPath = ref("C:/");
const remotePath = ref("/");
const localFiles = ref<any[]>([]);
const remoteFiles = ref<any[]>([]);
const isDraggingOverLocal = ref(false);
const isDraggingOverRemote = ref(false);

// --- 右键菜单 ---
const menuVisible = ref(false);
const menuPos = ref({ x: 0, y: 0 });
const contextFile = ref<any>(null);
const contextSource = ref<'local' | 'remote' | null>(null);

const newHost = ref({
  id: "", name: "", host: "", username: "root", port: 22,
  auth_type: "password", password: "", private_key_path: "", jump_host_id: ""
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

const toggleRightPanel = (type: 'quick' | 'ai') => {
  if (rightPanelVisible.value && rightPanelType.value === type) {
    rightPanelVisible.value = false;
  } else {
    rightPanelType.value = type;
    rightPanelVisible.value = true;
  }
};

// --- 方法定义 (保持原有逻辑) ---
const handleContextMenu = (e: MouseEvent, file: any, source: 'local' | 'remote') => {
  e.preventDefault();
  if (file.name === '..') return;
  contextFile.value = file;
  contextSource.value = source;
  menuPos.value = { x: e.clientX, y: e.clientY };
  menuVisible.value = true;
  const closeMenu = () => { menuVisible.value = false; window.removeEventListener('click', closeMenu); };
  window.addEventListener('click', closeMenu);
};

const handleMenuAction = async (action: 'transfer' | 'delete') => {
  menuVisible.value = false;
  if (!contextFile.value || !contextSource.value) return;
  if (action === 'transfer') {
    const type = contextSource.value === 'local' ? 'upload' : 'download';
    await startTransfer(type, contextFile.value);
  } else if (action === 'delete') {
    if (contextSource.value === 'remote') {
      if (confirm(`确定删除远程文件 "${contextFile.value.name}"？`)) {
        try {
          const path = `${remotePath.value.replace(/\/$/, '')}/${contextFile.value.name}`;
          await invoke("delete_remote_file", { sessionId: activeSessionId.value, path, isDir: contextFile.value.is_dir });
          await refreshRemoteFiles();
        } catch (err) { alert(`删除失败: ${err}`); }
      }
    }
  }
};

// 拖拽逻辑
const onDragStart = (e: DragEvent, file: any, source: 'local' | 'remote') => {
  if (file.name === '..') { e.preventDefault(); return; }
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "copy";
    const payload = JSON.stringify({ source, file });
    e.dataTransfer.setData("file-data", payload);
  }
};

const handleDragOver = (e: DragEvent) => {
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
};

const handleRemoteDrop = async (e: DragEvent) => {
  e.preventDefault(); isDraggingOverRemote.value = false;
  const rawData = e.dataTransfer?.getData("file-data");
  if (!rawData) return;
  const data = JSON.parse(rawData);
  if (data.source === 'local') await startTransfer('upload', data.file);
};

const handleLocalDrop = async (e: DragEvent) => {
  e.preventDefault(); isDraggingOverLocal.value = false;
  const rawData = e.dataTransfer?.getData("file-data");
  if (!rawData) return;
  const data = JSON.parse(rawData);
  if (data.source === 'remote') await startTransfer('download', data.file);
};

const cancelTask = async (taskId: string) => {
  const task = transferTasks.value.find(t => t.id === taskId);
  if (!task) return;
  try {
    await invoke("abort_transfer", { taskId });
    task.status = 'error';
    setTimeout(() => { transferTasks.value = transferTasks.value.filter(t => t.id !== taskId); }, 3000);
  } catch (err) { console.error(err); }
};

const getTaskIcon = (task: any) => {
  if (task.status === 'error') return 'fas fa-exclamation-circle';
  if (task.status === 'success') return 'fas fa-check-circle';
  return task.type === 'upload' ? 'fas fa-cloud-upload-alt' : 'fas fa-cloud-download-alt';
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
    theme: { background: "#1a1b26", foreground: "#a9b1d6", cursor: "#7aa2f7" },
    allowProposedApi: true,
  });
  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  await nextTick();
  const container = document.getElementById(`terminal-${sessionId}`);
  if (container) {
    term.open(container);
    fitAddon.fit();
    term.onData((data) => invoke("write_to_ssh", { sessionId, data }));
    terminalMap.set(sessionId, { term, fitAddon });
  }
};

const connectToServer = async () => {
  const server = servers.value.find(s => s.id === activeId.value);
  if (!server) return;
  isConnecting.value = true;
  const sessionId = server.id;
  if (!openSessions.value.find(s => s.id === sessionId)) {
    openSessions.value.push({ id: sessionId, serverId: server.id, name: server.name });
    sessionViewModes.value[sessionId] = 'terminal';
  }
  activeSessionId.value = sessionId;
  await initTerminal(sessionId);
  try {
    await invoke("connect_ssh", { serverId: server.id, sessionId });
    focusTerminal(sessionId);
  } catch (err) { console.error(err); }
  finally { isConnecting.value = false; }
};

const closeTab = async (id: string) => {
  await invoke("disconnect_ssh", { sessionId: id }).catch(console.error);
  internalUiCleanup(id);
};

const internalUiCleanup = (id: string) => {
  const instance = terminalMap.get(id);
  if (instance) { instance.term.dispose(); terminalMap.delete(id); }
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
  if (newMode === 'sftp') { await refreshRemoteFiles(); await refreshLocalFiles(); }
};

const refreshLocalFiles = async () => { try { localFiles.value = await invoke("list_local_dir", { path: localPath.value }); } catch (e) { console.error(e); } };
const refreshRemoteFiles = async () => { try { remoteFiles.value = await invoke("list_remote_dir", { sessionId: activeSessionId.value, path: remotePath.value }); } catch (e) { console.error(e); } };

const handleFileDblClick = async (file: any, type: 'local' | 'remote') => {
  if (!file.is_dir && file.name !== '..') return;
  const isRemote = type === 'remote';
  let currentPath = isRemote ? remotePath.value : localPath.value;
  currentPath = currentPath.replace(/[/\\]$/, '');
  if (file.name === '..') {
    let parts = currentPath.split(/[/\\]/).filter(p => p !== "");
    if (isRemote) { parts.pop(); currentPath = '/' + parts.join('/'); }
    else { parts.pop(); currentPath = parts.join('\\'); if (currentPath.length === 2 && currentPath.endsWith(':')) currentPath += '\\'; }
  } else {
    const separator = isRemote ? '/' : (currentPath.includes('\\') ? '\\' : '/');
    currentPath = `${currentPath}${separator}${file.name}`;
  }
  if (!currentPath) currentPath = isRemote ? "/" : "C:\\";
  try {
    if (isRemote) { remotePath.value = currentPath; await refreshRemoteFiles(); }
    else { localPath.value = currentPath; await refreshLocalFiles(); }
  } catch (err) { console.error(err); }
};

const startTransfer = async (type: 'upload' | 'download', file: any) => {
  const taskId = Math.random().toString(36).substring(7);
  const sourcePath = type === 'upload' ? `${localPath.value.replace(/[/\\]$/, '')}/${file.name}` : `${remotePath.value.replace(/\/$/, '')}/${file.name}`;
  const targetPath = type === 'upload' ? `${remotePath.value.replace(/\/$/, '')}/${file.name}` : `${localPath.value.replace(/[/\\]$/, '')}/${file.name}`;

  transferTasks.value.push({ id: taskId, name: file.name, progress: 0, type, status: 'transferring' });
  try {
    await invoke(type === 'upload' ? "sftp_upload" : "sftp_download", {
      sessionId: activeSessionId.value,
      localPath: type === 'upload' ? sourcePath : targetPath,
      remotePath: type === 'upload' ? targetPath : sourcePath,
      taskId
    });
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) { task.status = 'success'; task.progress = 100; setTimeout(() => { transferTasks.value = transferTasks.value.filter(t => t.id !== taskId); }, 2000); }
    if (currentViewMode.value === 'sftp') { refreshLocalFiles(); refreshRemoteFiles(); }
  } catch (err) {
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) task.status = 'error';
  }
};

// --- 逻辑方法 ---
const cloneSession = async () => {
  if (!activeSessionId.value) return;

  // 查找当前活跃的服务器信息
  const sessionToClone = openSessions.value.find(s => s.id === activeSessionId.value);
  const server = servers.value.find(s => s.id === sessionToClone?.serverId);

  if (server) {
    // 生成一个新的临时会话 ID（或者由后端生成）
    // 这里简单调用连接逻辑，开启一个同服务器的新会话
    console.log("正在克隆会话:", server.name);

    // 如果你的后端 connect_ssh 支持多个 session_id 对应同一个 server_id：
    const newSessionId = `${server.id}-${Math.random().toString(36).substring(7)}`;

    openSessions.value.push({
      id: newSessionId,
      serverId: server.id,
      name: `${server.name} (Copy)`
    });

    activeSessionId.value = newSessionId;
    sessionViewModes.value[newSessionId] = 'terminal';

    await initTerminal(newSessionId);
    try {
      await invoke("connect_ssh", { serverId: server.id, sessionId: newSessionId });
    } catch (err) {
      console.error("克隆会话失败:", err);
    }
  }
};

const focusTerminal = async (sessionId: string | null) => {
  if (!sessionId) return;
  await nextTick();
  const instance = terminalMap.get(sessionId);
  if (instance) instance.term.focus();
};

const handleResize = () => { terminalMap.forEach(instance => instance.fitAddon.fit()); };
watch(activeSessionId, (newId) => { if (newId) focusTerminal(newId); });

const openAddModal = () => { isEditing.value = false; newHost.value = { id: "", name: "", host: "", username: "root", port: 22, auth_type: "password", password: "", private_key_path: "", jump_host_id: "" }; isModalOpen.value = true; };
const openEditModal = (s: any) => { isEditing.value = true; newHost.value = { ...s }; isModalOpen.value = true; };
const closeModal = () => { isModalOpen.value = false; showPassword.value = false; };
const saveHost = async () => { if (newHost.value.name && newHost.value.host) { await invoke("save_server", { server: { ...newHost.value, port: Number(newHost.value.port) } }); await loadServers(); closeModal(); } };
const loadServers = async () => { servers.value = await invoke("get_servers"); if (servers.value.length > 0 && !activeId.value) activeId.value = servers.value[0].id; };

onMounted(async () => {
  window.addEventListener("resize", handleResize);
  loadServers();
  unlisten = await listen("ssh-output", (event) => {
    const payload = event.payload as { session_id: string, data: string };
    const instance = terminalMap.get(payload.session_id);
    if (instance && currentViewMode.value === 'terminal') instance.term.write(payload.data);
  });
  unlistenClosed = await listen("ssh-closed", (event) => { internalUiCleanup((event.payload as any).session_id); });
  unlistenTransfer = await listen("transfer-progress", (event) => {
    const { taskId, progress } = event.payload as { taskId: string, progress: number };
    const task = transferTasks.value.find(t => t.id === taskId);
    if (task) task.progress = progress;
  });
});

onUnmounted(async () => {
  window.removeEventListener("resize", handleResize);
  if (unlisten) unlisten();
  if (unlistenClosed) unlistenClosed();
  if (unlistenTransfer) unlistenTransfer();
});
</script>

<template>
  <div class="termius-container">
    <TitleBar />
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
              <div class="empty-icon">🛰️</div>
              <p>No active sessions.</p>
            </div>
          </div>

          <div v-show="currentViewMode === 'sftp'" class="sftp-wrapper">
            <div v-if="activeSessionId" class="sftp-manager">
              <div class="file-pane local-pane">
                <div class="pane-header">
                  <i class="fas fa-laptop" style="margin-right: 8px; color: #565f89;"></i>
                  <input v-model="localPath" class="path-input" @keyup.enter="refreshLocalFiles" />
                </div>
                <div class="file-list" :class="{ 'drag-over': isDraggingOverLocal }"
                     @dragenter.stop.prevent="isDraggingOverLocal = true" @dragover.stop.prevent="handleDragOver"
                     @dragleave.stop.prevent="isDraggingOverLocal = false" @drop.stop.prevent="handleLocalDrop">
                  <div v-for="file in localFiles" :key="file.name" class="file-item"
                       :class="{ 'is-dir': file.is_dir }" draggable="true"
                       @contextmenu="handleContextMenu($event, file, 'local')"
                       @dragstart="onDragStart($event, file, 'local')" @dblclick="handleFileDblClick(file, 'local')">
                    <span class="file-icon"><i class="fas" :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i></span>
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
                <div class="file-list" :class="{ 'drag-over': isDraggingOverRemote }"
                     @dragenter.stop.prevent="isDraggingOverRemote = true" @dragover.stop.prevent="handleDragOver"
                     @dragleave.stop.prevent="isDraggingOverRemote = false" @drop.stop.prevent="handleRemoteDrop">
                  <div v-for="file in remoteFiles" :key="file.name" class="file-item"
                       :class="{ 'is-dir': file.is_dir }" draggable="true"
                       @contextmenu="handleContextMenu($event, file, 'remote')"
                       @dragstart="onDragStart($event, file, 'remote')" @dblclick="handleFileDblClick(file, 'remote')">
                    <span class="file-icon"><i class="fas" :class="file.name === '..' ? 'fa-level-up-alt' : (file.is_dir ? 'fa-folder' : 'fa-file-alt')"></i></span>
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
                      <div class="progress-container"><div class="progress-bar" :style="{ width: task.progress + '%' }"></div></div>
                    </div>
                  </TransitionGroup>
                </div>
              </div>
            </div>
          </div>
        </div>
        <StatusBar :open-sessions="openSessions" :current-server="currentServer" />
      </main>

      <div class="right-dock">
        <div class="icon-bar">
          <div class="icon-item" title="快捷命令" :class="{ active: rightPanelVisible && rightPanelType === 'quick' }" @click="toggleRightPanel('quick')">
            <i class="fas fa-bolt"></i>
          </div>

          <div class="icon-item" title="AI 助手" :class="{ active: rightPanelVisible && rightPanelType === 'ai' }" @click="toggleRightPanel('ai')">
            <i class="fas fa-robot"></i>
          </div>
        </div>

        <Transition name="panel-slide">
          <div v-if="rightPanelVisible" class="floating-panel">
            <QuickCommandPanel
                v-if="rightPanelType === 'quick'"
                :activeSessionId="activeSessionId"
            />
            <AiAssistantPanel
                v-else-if="rightPanelType === 'ai'"
                :activeSessionId="activeSessionId"
            />
          </div>
        </Transition>
      </div>
    </div>

    <ServerModal :is-open="isModalOpen" :is-editing="isEditing" :server="newHost" :servers="servers" @close="closeModal" @save="saveHost" />
    <Transition name="fade">
      <div v-if="menuVisible" class="context-menu" :style="{ top: menuPos.y + 'px', left: menuPos.x + 'px' }">
        <div class="menu-item" @click="handleMenuAction('transfer')"><span>{{ contextSource === 'local' ? '📤' : '📥' }}</span>{{ contextSource === 'local' ? '上传到服务器' : '下载到本地' }}</div>
        <div class="menu-item divider"></div>
        <div class="menu-item danger" @click="handleMenuAction('delete')"><span>🗑️</span> 删除</div>
      </div>
    </Transition>
  </div>
</template>

<style lang="scss" scoped>
// ============================================
// 🎨 变量定义
// ============================================
$bg-dark: #0f111a;
$bg-pane: #1a1b26;
$bg-header: #16161e;
$border-color: #292e42;
$text-main: #a9b1d6;
$text-dim: #565f89;
$accent: #7aa2f7;
$success: #73daca;
$error: #f7768e;

// ============================================
// 🏗️ 基础布局
// ============================================
.termius-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background-color: $bg-dark;
  color: $text-main;
}

.main-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
  position: relative;
}

.workspace {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: $bg-dark;
  overflow: hidden;
}

// ============================================
// 💻 终端容器
// ============================================
.terminal-shell {
  flex: 1;
  margin: 0 16px 16px;
  background: $bg-pane;
  border-radius: 12px;
  border: 1px solid $border-color;
  position: relative;
  overflow: hidden;

  .terminal-wrapper, .sftp-wrapper, .terminal-multi-wrapper {
    width: 100%;
    height: 100%;
  }

  .xterm-container {
    width: 100%;
    height: 100%;
    padding: 10px;
    box-sizing: border-box;

    :deep(.xterm) {
      padding: 4px;
      .xterm-viewport {
        background-color: transparent !important;
      }
    }
  }

  .empty-state {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: $text-dim;

    .empty-icon {
      font-size: 48px;
      opacity: 0.3;
    }
  }
}

// ============================================
// 📂 SFTP 管理面板
// ============================================
.sftp-manager {
  display: flex;
  height: 100%;
  background: $bg-pane;
  overflow: hidden;

  .file-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    border-right: 1px solid #24283b;

    .pane-header {
      flex-shrink: 0;
      padding: 10px 12px;
      background: $bg-header;
      display: flex;
      align-items: center;
      gap: 8px;

      .path-input {
        background: transparent;
        border: none;
        color: $accent;
        font-family: 'JetBrains Mono', monospace;
        font-size: 11px;
        width: 100%;

        &:focus {
          outline: none;
          background: $bg-pane;
        }
      }
    }

    .file-list {
      flex: 1;
      overflow-y: auto;
      padding: 4px 0;
      scrollbar-gutter: stable;
      transition: background-color 0.2s;

      &.drag-over {
        background-color: rgba($accent, 0.1);
        outline: 2px dashed $accent;
        outline-offset: -4px;
      }

      .file-item {
        display: flex;
        align-items: center;
        padding: 6px 12px;
        font-size: 13px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.02);

        &:hover {
          background: #2f334d;
        }

        .file-icon {
          width: 20px;
          margin-right: 10px;
          text-align: center;

          i.fa-folder { color: #e0af68; }
          i.fa-file-alt { color: $accent; }
          i.fa-level-up-alt { color: #9ece6a; }
        }

        .file-name {
          flex: 1;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;

          &.is-dir {
            color: $accent;
            font-weight: 500;
          }
        }

        .file-size {
          color: #444b6a;
          font-size: 11px;
          margin-left: 8px;
        }
      }
    }
  }
}

// ============================================
// 📦 传输状态显示
// ============================================
.transfer-status {
  position: fixed;
  bottom: 60px;
  right: 25px;
  width: 320px;
  background: rgba(22, 22, 30, 0.95);
  backdrop-filter: blur(12px);
  border: 1px solid $border-color;
  border-radius: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  z-index: 2000;
  overflow: hidden;

  .status-header {
    padding: 12px 16px;
    background: rgba(41, 46, 66, 0.4);
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(65, 72, 104, 0.3);

    .header-left {
      display: flex;
      align-items: center;
      gap: 10px;
      color: $accent;
      font-size: 13px;
      font-weight: bold;
    }

    .header-status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: #414868;

      &.is-syncing {
        background: $accent;
        box-shadow: 0 0 8px $accent;
        animation: pulse 1.5s infinite;
      }
    }
  }

  .task-list-wrapper {
    max-height: 280px;
    overflow-y: auto;
    padding: 8px;

    &::-webkit-scrollbar {
      width: 4px;
    }

    &::-webkit-scrollbar-thumb {
      background: #414868;
      border-radius: 10px;
    }
  }

  .task-row {
    background: rgba(255, 255, 255, 0.03);
    padding: 10px;
    border-radius: 8px;
    margin-bottom: 8px;
    transition: all 0.3s ease;

    &.status-success {
      border-left: 3px solid $success;
      .task-percent { color: $success; }
    }

    &.status-error {
      border-left: 3px solid $error;
      .task-percent { color: $error; }
    }

    .task-info {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 6px;

      .name-box {
        display: flex;
        align-items: center;
        gap: 8px;
        flex: 1;
        min-width: 0;

        .task-name {
          font-size: 12px;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
      }

      .task-actions {
        display: flex;
        align-items: center;
        gap: 8px;

        .cancel-btn {
          background: rgba($error, 0.1);
          border: none;
          color: $text-dim;
          cursor: pointer;
          padding: 4px;
          border-radius: 4px;
          transition: all 0.2s ease;

          &:hover {
            color: $error;
            background: rgba($error, 0.2);
            transform: rotate(90deg);
          }
        }

        .task-percent {
          font-size: 11px;
          font-family: 'JetBrains Mono', monospace;
          min-width: 35px;
          text-align: right;
        }
      }
    }

    .progress-container {
      height: 4px;
      background: rgba(0, 0, 0, 0.2);
      border-radius: 2px;
      overflow: hidden;

      .progress-bar {
        height: 100%;
        background: $accent;
        transition: width 0.4s ease;
        position: relative;

        &.transferring::after {
          content: "";
          position: absolute;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background-image: linear-gradient(-45deg, rgba(255,255,255,0.2) 25%, transparent 25%, transparent 50%, rgba(255,255,255,0.2) 50%, rgba(255,255,255,0.2) 75%, transparent 75%);
          background-size: 20px 20px;
          animation: move-stripes 2s linear infinite;
        }
      }
    }
  }
}

// ============================================
// 🖱️ 右键菜单
// ============================================
.context-menu {
  position: fixed;
  z-index: 9999;
  background: #24283b;
  border: 1px solid #414868;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  padding: 4px;
  min-width: 160px;

  .menu-item {
    padding: 8px 12px;
    display: flex;
    align-items: center;
    cursor: pointer;
    font-size: 13px;
    border-radius: 4px;

    &:hover {
      background: #3b4261;
      color: $accent;
    }

    &.danger:hover {
      background: #44232f;
      color: $error;
    }

    &.divider {
      height: 1px;
      background: #414868;
      margin: 6px 4px;
      padding: 0;
      cursor: default;
      pointer-events: none;

      &:hover {
        background: #414868;
      }
    }
  }
}

// ============================================
// 🎯 右侧工具栏
// ============================================
.right-dock {
  display: flex;
  position: relative;
  z-index: 100;

  .icon-bar {
    width: 40px;
    background: $bg-header;
    border-left: 1px solid $border-color;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 20px;

    .icon-item {
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: $text-dim;
      cursor: pointer;
      border-radius: 6px;
      margin-bottom: 12px;
      transition: all 0.2s;

      &:hover {
        background: rgba($accent, 0.1);
        color: $text-main;
      }

      &.active {
        color: $accent;
        background: rgba($accent, 0.15);
      }
    }
  }

  .floating-panel {
    position: absolute;
    top: 0;
    right: 40px;
    width: 320px;
    height: 100%;
    background: rgba($bg-pane, 0.95);
    backdrop-filter: blur(12px);
    border-left: 1px solid $border-color;
    box-shadow: -10px 0 30px rgba(0, 0, 0, 0.5);
  }
}

// ============================================
// 🎭 动画关键帧
// ============================================
@keyframes move-stripes {
  from { background-position: 0 0; }
  to { background-position: 40px 0; }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

// ============================================
// 🎬 Vue 过渡动画
// ============================================
.panel-slide-enter-active,
.panel-slide-leave-active {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s;
}

.panel-slide-enter-from,
.panel-slide-leave-to {
  transform: translateX(20px);
  opacity: 0;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.task-list-enter-from {
  opacity: 0;
  transform: translateX(30px);
}

.task-list-leave-to {
  opacity: 0;
  transform: scale(0.9);
}
</style>

<style lang="scss">
// ============================================
// 🌍 全局样式
// ============================================
:root {
  --bg-main: #0f111a;
  --scrollbar-track: transparent;
  --scrollbar-thumb: #292e42;
  --scrollbar-thumb-hover: #3b4261;
  --accent-color: #7aa2f7;
  --accent-glow: rgba(122, 162, 247, 0.2);

  /* 图标专属颜色 */
  --icon-folder: #e0af68;
  --icon-file: #7aa2f7;
  --icon-up: #9ece6a;
}

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  background-color: var(--bg-main);
  user-select: none;
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  -webkit-font-smoothing: antialiased;
}

// 滚动条美化
::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}

::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
}

::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 10px;
  transition: background 0.2s ease;

  &:hover {
    background: var(--scrollbar-thumb-hover);
  }
}

// 表单元素交互
input, select, textarea {
  transition: border-color 0.2s, box-shadow 0.2s;

  &:focus {
    border-color: var(--accent-color) !important;
    box-shadow: 0 0 0 2px var(--accent-glow);
    outline: none;
  }
}

// 文件图标系统
.file-icon {
  width: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;

  i.fas {
    font-family: "Font Awesome 6 Free", "Font Awesome 5 Free", sans-serif !important;
    font-weight: 900 !important;
    font-style: normal;
    font-variant: normal;
    text-rendering: auto;
    line-height: 1;
  }

  .fa-folder {
    color: var(--icon-folder) !important;
  }

  .fa-file-alt, .fa-file {
    color: var(--icon-file) !important;
  }

  .fa-level-up-alt, .fa-reply {
    color: var(--icon-up) !important;
  }
}

// 防止拖拽时的默认蓝色阴影
[draggable="true"] {
  user-select: none;
  -webkit-user-drag: element;
}
</style>
