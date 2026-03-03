<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast.ts';

const props = defineProps<{
  activeSessionId: string | null
}>();

const commands = ref<any[]>([]);
const searchQuery = ref('');
const isAdding = ref(false);

const newCmd = ref({ name: '', content: '' });

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

const saveCommand = async () => {
  if (!newCmd.value.name || !newCmd.value.content) {
    toast.warning("请填写完整信息", "输入校验");
    return;
  }
  try {
    await invoke('save_quick_command', { cmd: { ...newCmd.value, id: '' } });
    newCmd.value = { name: '', content: '' };
    isAdding.value = false;
    await loadCommands();
    toast.success("快捷指令保存成功");
  } catch (err) {
    toast.error(`保存失败: ${err}`);
  }
};

const executeCommand = async (content: string) => {
  if (!props.activeSessionId) {
    toast.warning("请先连接到一个 SSH 会话", "未就绪");
    return;
  }
  const data = content.endsWith('\n') ? content : content + '\n';
  await invoke('write_to_ssh', { sessionId: props.activeSessionId, data });
  toast.success("指令已发送", "终端响应");
};

const deleteCommand = async (id: string) => {
  try {
    await invoke('delete_quick_command', { id });
    await loadCommands();
    toast.success("指令已移除");
  } catch (err) {
    toast.error("删除失败");
  }
};

onMounted(loadCommands);
</script>

<template>
  <div class="quick-command-panel">
    <div class="panel-header">
      <div class="title">
        <i class="fas fa-bolt"></i>
        <span>快捷指令</span>
      </div>
      <button class="icon-btn" @click="isAdding = !isAdding" :class="{ 'is-active': isAdding }">
        <i class="fas fa-plus"></i>
      </button>
    </div>

    <div class="expand-container" :class="{ 'is-expanded': isAdding }">
      <div class="expand-content">
        <div class="add-form">
          <input v-model="newCmd.name" placeholder="指令名称 (如: 查看日志)" class="form-input" />
          <textarea v-model="newCmd.content" placeholder="输入命令内容..." class="form-textarea"></textarea>
          <div class="form-actions">
            <button @click="isAdding = false" class="btn-cancel">取消</button>
            <button @click="saveCommand" class="btn-save">保存</button>
          </div>
        </div>
      </div>
    </div>

    <div class="search-section">
      <div class="search-wrapper">
        <i class="fas fa-search"></i>
        <input v-model="searchQuery" placeholder="搜索已存指令..." @keyup.esc="searchQuery = ''" />
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
          <button class="delete-btn" @click.stop="deleteCommand(cmd.id)" title="删除指令">
            <i class="fas fa-trash-alt"></i>
          </button>
          <div class="execute-icon">
            <i class="fas fa-terminal"></i>
          </div>
        </div>
      </div>

      <div v-if="filteredCommands.length === 0" class="empty-state">
        <i class="fas fa-inbox"></i>
        <p>{{ searchQuery ? '未找到匹配项' : '暂无快捷指令' }}</p>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
// --- 变量定义 ---
$bg-primary: #1a1b26;
$bg-secondary: #16161e;
$accent: #7aa2f7;
$border: #292e42;
$text-dim: #565f89;
$error: #f7768e;

.quick-command-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: $bg-primary;
  color: #a9b1d6;
}

.panel-header {
  padding: 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid $border;
  z-index: 10;

  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: $accent;
  }
}

.icon-btn {
  background: transparent;
  border: none;
  color: $text-dim;
  cursor: pointer;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);

  &:hover { color: $accent; }

  &.is-active {
    color: $error;
    transform: rotate(135deg); // 旋转成叉号
  }
}

/* --- 核心动画容器 --- */
.expand-container {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s;
  overflow: hidden;
  background: $bg-secondary;
  border-bottom: 0 solid $border;
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

  .form-input, .form-textarea {
    background: $bg-primary;
    border: 1px solid $border;
    border-radius: 6px;
    padding: 10px;
    color: #a9b1d6;
    font-size: 12px;
    transition: all 0.2s;
    &:focus {
      outline: none;
      border-color: $accent;
      background: rgba($accent, 0.05);
    }
  }

  .form-textarea {
    height: 80px;
    resize: none;
    font-family: 'JetBrains Mono', monospace;
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
    .btn-cancel { background: #292e42; color: #a9b1d6; }
    .btn-save { background: $accent; color: #1a1b26; font-weight: bold; }
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
      color: $text-dim;
      font-size: 12px;
      z-index: 1;
    }

    input {
      width: 100%;
      box-sizing: border-box;
      background: $bg-secondary;
      border: 1px solid $border;
      border-radius: 20px;
      padding: 8px 12px 8px 32px;
      color: #a9b1d6;
      font-size: 11px;
      transition: border-color 0.2s;
      &:focus { outline: none; border-color: $accent; }
    }
  }
}

.command-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 12px 12px;

  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb { background: $border; border-radius: 4px; }

  .command-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid $border;
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);

    &:hover {
      background: rgba($accent, 0.08);
      border-color: rgba($accent, 0.5);
      transform: translateY(-1px);

      .execute-icon { color: $accent; }
      .delete-btn { opacity: 0.6; transform: translateX(0); }
    }

    .card-content {
      flex: 1;
      min-width: 0;
      .cmd-name { font-size: 12px; font-weight: 600; color: #c0caf5; margin-bottom: 4px; }
      .cmd-code {
        font-size: 10px;
        color: $text-dim;
        font-family: 'JetBrains Mono', monospace;
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

      .delete-btn {
        opacity: 0;
        transform: translateX(5px);
        background: transparent;
        border: none;
        color: $text-dim;
        cursor: pointer;
        font-size: 13px;
        padding: 4px;
        transition: all 0.2s ease;

        &:hover {
          opacity: 1 !important;
          color: $error;
          transform: scale(1.15);
        }
      }

      .execute-icon {
        color: #414868;
        font-size: 12px;
        transition: color 0.2s;
        pointer-events: none;
      }
    }
  }
}

.empty-state {
  text-align: center;
  padding-top: 50px;
  color: $text-dim;
  i { font-size: 28px; margin-bottom: 12px; opacity: 0.5; }
  p { font-size: 12px; }
}
</style>