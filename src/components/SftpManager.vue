<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

// 将所有 props 设为可选并提供默认值
const props = withDefaults(defineProps<{
  sessionId?: string;
  transferTasks?: any[];
  localPath?: string;
  remotePath?: string;
  localFiles?: any[];
  remoteFiles?: any[];
  isDraggingOverLocal?: boolean;
  isDraggingOverRemote?: boolean;
}>(), {
  sessionId: '',
  transferTasks: () => [],
  localPath: 'C:/',
  remotePath: '/',
  localFiles: () => [],
  remoteFiles: () => [],
  isDraggingOverLocal: false,
  isDraggingOverRemote: false
});

const emit = defineEmits<{
  (e: 'update:transferTasks', tasks: any[]): void;
  (e: 'update:localPath', path: string): void;
  (e: 'update:remotePath', path: string): void;
  (e: 'refreshLocalFiles'): void;
  (e: 'refreshRemoteFiles'): void;
  (e: 'handleFileDblClick', file: any, type: 'local' | 'remote'): void;
  (e: 'onDragStart', event: DragEvent, file: any, source: 'local' | 'remote'): void;
  (e: 'handleRemoteDrop', event: DragEvent): void;
  (e: 'handleLocalDrop', event: DragEvent): void;
  (e: 'handleDragEnter', event: DragEvent, type: 'local' | 'remote'): void;
  (e: 'handleDragLeave', event: DragEvent, type: 'local' | 'remote'): void;
}>();

let unlistenTransfer: UnlistenFn | null = null;

const getFileIcon = (file: any) => {
  if (file.name === '..') return 'fa-level-up-alt';
  return file.is_dir ? 'fa-folder' : 'fa-file-alt';
};

const refreshLocalFiles = () => {
  emit('refreshLocalFiles');
};

const refreshRemoteFiles = () => {
  emit('refreshRemoteFiles');
};

onMounted(async () => {
  // 只有当 sessionId 存在时才监听传输进度
  if (props.sessionId) {
    unlistenTransfer = await listen("transfer-progress", (event) => {
      const { taskId, progress } = event.payload as { taskId: string, progress: number };
      const task = props.transferTasks?.find(t => t.id === taskId);
      if (task) {
        task.progress = progress;
        if (progress >= 100) {
          setTimeout(() => {
            const updatedTasks = props.transferTasks?.filter(t => t.id !== taskId) || [];
            emit('update:transferTasks', updatedTasks);
            refreshLocalFiles();
            refreshRemoteFiles();
          }, 2000);
        }
      }
    });
  }
});

onUnmounted(() => {
  if (unlistenTransfer) unlistenTransfer();
});
</script>

<template>
  <div class="sftp-manager">
    <div class="file-pane local-pane">
      <div class="pane-header">
        <i class="fas fa-laptop" style="margin-right: 8px; color: #565f89;"></i>
        <input
            :value="localPath"
            @input="emit('update:localPath', ($event.target as HTMLInputElement).value)"
            class="path-input"
            @keyup.enter="refreshLocalFiles"
        />
      </div>
      <div
          class="file-list"
          :class="{ 'drag-over': isDraggingOverLocal }"
          @drop="emit('handleLocalDrop', $event);"
          @dragover.stop.prevent
          @dragenter.prevent="emit('handleDragEnter', $event, 'local')"
          @dragleave.prevent="emit('handleDragLeave', $event, 'local')"
      >
        <div
            v-for="file in localFiles"
            :key="file.name"
            :class="['file-item', { 'is-dir': file.is_dir }]"
            draggable="true"
            @dragstart="emit('onDragStart', $event, file, 'local')"
            @dblclick="emit('handleFileDblClick', file, 'local')"
        >
          <span class="file-icon">
            <i class="fas" :class="getFileIcon(file)"></i>
          </span>
          <span class="file-name">{{ file.name }}</span>
          <span class="file-size" v-if="!file.is_dir">{{ (file.size / 1024).toFixed(1) }} KB</span>
        </div>
      </div>
    </div>

    <div class="file-pane remote-pane">
      <div class="pane-header">
        <i class="fas fa-server" style="margin-right: 8px; color: #565f89;"></i>
        <input
            :value="remotePath"
            @input="emit('update:remotePath', ($event.target as HTMLInputElement).value)"
            class="path-input"
            @keyup.enter="refreshRemoteFiles"
        />
      </div>
      <div
          class="file-list"
          :class="{ 'drag-over': isDraggingOverRemote }"
          @drop="emit('handleRemoteDrop', $event);"
          @dragover.prevent
          @dragenter.prevent="emit('handleDragEnter', $event, 'remote')"
          @dragleave.prevent="emit('handleDragLeave', $event, 'remote')"
      >
        <div
            v-for="file in remoteFiles"
            :key="file.name"
            :class="['file-item', { 'is-dir': file.is_dir }]"
            draggable="true"
            @dragstart="emit('onDragStart', $event, file, 'remote')"
            @dblclick="emit('handleFileDblClick', file, 'remote')"
        >
          <span class="file-icon">
            <i class="fas" :class="getFileIcon(file)"></i>
          </span>
          <span class="file-name">{{ file.name }}</span>
          <span class="file-size" v-if="!file.is_dir">{{ (file.size / 1024).toFixed(1) }} KB</span>
        </div>
      </div>
    </div>

    <div class="transfer-status" v-if="transferTasks?.length > 0">
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
</template>

<style lang="scss" scoped>
$bg-dark: #0f111a;
$accent: #7aa2f7;
$border-color: #292e42;
$text-dim: #565f89;

.sftp-manager {
  display: flex;
  height: 100%;
  background: #1a1b26;
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

    &:hover {
      background: #2f334d;
    }

    .file-icon {
      width: 20px;
      margin-right: 10px;
      text-align: center;
      color: #565f89;
    }

    .file-name {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    &.is-dir {
      .file-icon {
        color: #e0af68;
      }

      .file-name {
        color: #7aa2f7;
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

    &:focus {
      outline: none;
      background: #1a1b26;
    }
  }
}
</style>
