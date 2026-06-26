<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';
import { throttle } from '../utils/async.ts';
import { confirm } from '../utils/confirm.ts';
import { t } from '../utils/i18n.ts';

const props = defineProps<{
  activeSessionId: string | null;
  sessionConnected?: boolean;
}>();

const commands = ref<any[]>([]);
const searchQuery = ref('');
const isAdding = ref(false);
const editingId = ref<string | null>(null);

const emptyCmd = () => ({
  id: '',
  name: '',
  content: '',
  updated_at: 0,
  deleted: false,
});

const newCmd = ref(emptyCmd());

const isEditing = computed(() => editingId.value !== null);

const filteredCommands = computed(() => {
  const query = searchQuery.value.toLowerCase();
  return commands.value.filter(c =>
      c.name.toLowerCase().includes(query) ||
      c.content.toLowerCase().includes(query)
  );
});

const loadCommands = async () => {
  try {
    commands.value = await invoke('get_quick_commands');
  } catch (err) {
    console.error("加载命令失败:", err);
  }
};

const resetForm = () => {
  newCmd.value = emptyCmd();
  editingId.value = null;
  isAdding.value = false;
};

const saveCommand = async () => {
  if (!newCmd.value.name || !newCmd.value.content) {
    toast.warning(t('quickCommand.validationMessage'), t('quickCommand.validationTitle'));
    return;
  }
  try {
    await invoke('save_quick_command', {
      cmd: {
        ...newCmd.value,
        id: editingId.value || newCmd.value.id || '',
      },
    });
    const wasEditing = !!editingId.value;
    resetForm();
    await loadCommands();
    toast.success(wasEditing ? t('quickCommand.updated') : t('quickCommand.saved'));
  } catch (err) {
    toast.error(t('quickCommand.saveFailed', { err: String(err) }));
  }
};

const executeCommand = async (content: string) => {
  if (!props.activeSessionId) {
    toast.warning(t('quickCommand.connectFirst'), t('quickCommand.notReadyTitle'));
    return;
  }
  if (!props.sessionConnected) {
    toast.warning(t('quickCommand.sessionNotConnected'), t('quickCommand.notReadyTitle'));
    return;
  }
  const data = content.endsWith('\n') ? content : content + '\n';
  await invoke('write_to_ssh', { sessionId: props.activeSessionId, data });
  toast.success(t('quickCommand.sent'), t('quickCommand.sentTitle'));
};

const deleteCommand = async (id: string, name: string) => {
  const ok = await confirm.warning(
    t('quickCommand.deleteConfirm', { name }),
    t('quickCommand.deleteTitle'),
  );
  if (!ok) return;
  try {
    await invoke('delete_quick_command', { id });
    if (editingId.value === id) resetForm();
    await loadCommands();
    toast.success(t('quickCommand.removed'));
  } catch (err) {
    toast.error(t('quickCommand.deleteFailed'));
  }
};

const startEdit = (cmd: { id: string; name: string; content: string; updated_at: number; deleted?: boolean }) => {
  newCmd.value = {
    id: cmd.id,
    name: cmd.name,
    content: cmd.content,
    updated_at: cmd.updated_at,
    deleted: cmd.deleted ?? false,
  };
  editingId.value = cmd.id;
  isAdding.value = true;
};

const handleToggleAddCommand = throttle(() => {
  if (isAdding.value) {
    resetForm();
    return;
  }
  newCmd.value = emptyCmd();
  editingId.value = null;
  isAdding.value = true;
}, 300);

onMounted(loadCommands);
</script>

<template>
  <div class="quick-command-panel">
    <div class="panel-header">
      <div class="title">
        <i class="fas fa-bolt"></i>
        <span>{{ t('quickCommand.title') }}</span>
      </div>
      <button class="icon-btn" @click="handleToggleAddCommand" :class="{ 'is-active': isAdding }">
        <i class="fas fa-plus"></i>
      </button>
    </div>

    <div class="expand-container" :class="{ 'is-expanded': isAdding }">
      <div class="expand-content">
        <div class="add-form">
          <div class="form-title">{{ isEditing ? t('quickCommand.editCommand') : t('quickCommand.newCommand') }}</div>
          <input v-model="newCmd.name" :placeholder="t('quickCommand.namePlaceholder')" class="form-input" />
          <textarea v-model="newCmd.content" :placeholder="t('quickCommand.contentPlaceholder')" class="form-textarea"></textarea>
          <div class="form-actions">
            <button type="button" @click="resetForm" class="btn-cancel">{{ t('common.cancel') }}</button>
            <button type="button" @click="saveCommand" class="btn-save">{{ isEditing ? t('quickCommand.update') : t('common.save') }}</button>
          </div>
        </div>
      </div>
    </div>

    <div class="search-section">
      <div class="search-wrapper">
        <i class="fas fa-search"></i>
        <input v-model="searchQuery" :placeholder="t('quickCommand.searchPlaceholder')" @keyup.esc="searchQuery = ''" />
      </div>
    </div>

    <div class="command-list custom-scrollbar">
      <div v-for="cmd in filteredCommands"
           :key="cmd.id"
           class="command-card"
           @click="executeCommand(cmd.content)">
        <div class="card-content">
          <div class="cmd-name">{{ cmd.name }}</div>
          <div class="cmd-code">{{ cmd.content }}</div>
        </div>

        <div class="card-actions">
          <Tooltip :text="t('quickCommand.editTooltip')">
            <button type="button" class="edit-btn" @click.stop="startEdit(cmd)">
              <i class="fas fa-pen"></i>
            </button>
          </Tooltip>
          <Tooltip :text="t('quickCommand.deleteTooltip')">
            <button type="button" class="delete-btn" @click.stop="deleteCommand(cmd.id, cmd.name)">
              <i class="fas fa-trash-alt"></i>
            </button>
          </Tooltip>
          <div class="execute-icon">
            <i class="fas fa-terminal"></i>
          </div>
        </div>
      </div>

      <div v-if="filteredCommands.length === 0" class="empty-state">
        <i class="fas fa-inbox"></i>
        <p>{{ searchQuery ? t('quickCommand.emptySearch') : t('quickCommand.emptyList') }}</p>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

.quick-command-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-main);
}

.panel-header {
  padding: 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
  z-index: 10;

  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent);
  }
}

.icon-btn {
  background: transparent;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);

  &:hover { color: var(--accent); }

  &.is-active {
    color: var(--error);
    transform: rotate(135deg);
  }
}

/* --- 核心动画容器 --- */
.expand-container {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s;
  overflow: hidden;
  background: var(--bg-secondary);
  border-bottom: 0 solid var(--border);
  opacity: 0;

  &.is-expanded {
    grid-template-rows: 1fr;
    border-bottom-width: 1px;
    opacity: 1;
  }

  .expand-content {
    min-height: 0;
  }
}

.add-form {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;

  .form-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .form-input, .form-textarea {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    color: var(--text-main);
    font-size: 12px;
    transition: all 0.2s;

    &:focus {
      outline: none;
      border-color: var(--accent);
      background: var(--accent-05); // 修复点：使用预计算透明变量
    }
  }

  .form-textarea {
    height: 80px;
    resize: none;
    font-family: var(--font-terminal);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;

    button {
      padding: 6px 14px;
      border-radius: 4px;
      font-size: 11px;
      cursor: pointer;
      border: none;
      transition: opacity 0.2s;
      &:hover { opacity: 0.8; }
    }

    .btn-cancel {
      background: var(--bg-card);
      color: var(--text-dim); // 使用 text-dim 替代 text-muted
    }

    .btn-save {
      background: var(--accent);
      color: var(--bg-primary);
      font-weight: bold;
    }
  }
}

.search-section {
  padding: 12px;
  box-sizing: border-box;

  .search-wrapper {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;

    i {
      position: absolute;
      left: 12px;
      color: var(--text-dim);
      font-size: 12px;
      z-index: 1;
    }

    input {
      width: 100%;
      box-sizing: border-box;
      background: var(--bg-secondary);
      border: 1px solid var(--border);
      border-radius: 20px;
      padding: 8px 12px 8px 32px;
      color: var(--text-main);
      font-size: 11px;
      transition: all 0.2s;

      &:focus {
        outline: none;
        border-color: var(--accent);
        background: var(--bg-input);
      }
    }
  }
}

.command-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px 12px 12px 12px;

  /* 滚动条跟随主题 */
  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 4px;
    &:hover { background: var(--text-dim); }
  }

  .command-card {
    // 修复点：使用预计算的 bg-card 透明变量，或者直接用变量配合主题 shadow
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);

    &:hover {
      background: var(--accent-08); // 修复点
      border-color: var(--accent);
      transform: translateY(-1px);

      .execute-icon { color: var(--accent); opacity: 1; }
      .edit-btn,
      .delete-btn { opacity: 0.6; transform: translateX(0); }
    }

    .card-content {
      flex: 1;
      min-width: 0;
      .cmd-name {
        font-size: 12px;
        font-weight: 600;
        color: var(--text-main);
        margin-bottom: 4px;
      }
      .cmd-code {
        font-size: 10px;
        color: var(--text-dim);
        font-family: var(--font-terminal);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
    }

    .card-actions {
      display: flex;
      align-items: center;
      gap: 12px;
      padding-left: 8px;

      .edit-btn,
      .delete-btn {
        opacity: 0;
        transform: translateX(5px);
        background: transparent;
        border: none;
        color: var(--text-dim);
        cursor: pointer;
        font-size: 13px;
        padding: 4px;
        transition: all 0.2s ease;

        &:hover {
          opacity: 1 !important;
          transform: scale(1.15);
        }
      }

      .edit-btn:hover {
        color: var(--accent);
      }

      .delete-btn:hover {
        color: var(--error);
      }

      .execute-icon {
        color: var(--text-dim);
        opacity: 0.5;
        font-size: 12px;
        transition: all 0.2s;
        pointer-events: none;
      }
    }
  }
}

.empty-state {
  text-align: center;
  padding-top: 50px;
  color: var(--text-dim);
  i {
    font-size: 28px;
    margin-bottom: 12px;
    color: var(--accent);
    opacity: 0.3;
  }
  p { font-size: 12px; }
}
</style>