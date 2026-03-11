<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { toast } from '../utils/toast.ts';

// --- 引入 Markdown 相关依赖 (Deno 支持 npm: 语法) ---
import { marked } from 'marked';
import hljs from 'highlight.js';
// 引入一个酷炫的深色主题
import 'highlight.js/styles/tokyo-night-dark.css';

const props = defineProps<{
  activeSessionId: string | null
}>();

// --- 状态管理 ---
const isConfigMode = ref(false);
const messages = ref([
  { role: 'assistant', content: '你好！我是你的 SSH 助手。请点击右上方设置配置 API Key 以启用 AI 功能。' }
]);
const userInput = ref('');
const isLoading = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);
let unlistenChunk: UnlistenFn | null = null;

// --- AI 配置状态 ---
const aiConfig = ref({
  currentProvider: 'deepseek',
  apiKey: '',
  model: 'deepseek-chat',
  temperature: 0.7
});

const providers = [
  { id: 'deepseek', name: 'DeepSeek', models: ['deepseek-chat', 'deepseek-coder'] },
  { id: 'qwen', name: '通义千问', models: ['qwen-max', 'qwen-plus', 'qwen-turbo'] },
  { id: 'doubao', name: '豆包', models: ['doubao-pro-4k', 'doubao-lite-4k'] },
  { id: 'gemini', name: 'Gemini', models: ['gemini-1.5-pro', 'gemini-1.5-flash'] }
];

// --- Markdown 配置 ---
const renderer = new marked.Renderer();
marked.setOptions({
  renderer,
  highlight: function(code: string, lang: string) {
    const language = hljs.getLanguage(lang) ? lang : 'plaintext';
    return hljs.highlight(code, { language }).value;
  },
  langPrefix: 'hljs language-',
  breaks: true,
  gfm: true
});

const renderMarkdown = (content: string) => {
  try {
    return marked.parse(content);
  } catch (e) {
    return content;
  }
};

// --- 逻辑方法 ---

const loadSettings = async () => {
  try {
    const saved = await invoke<any>('get_ai_config');
    if (saved) aiConfig.value = saved;
  } catch (err) {
    console.error("加载 AI 配置失败:", err);
  }
};

const saveSettings = async () => {
  try {
    await invoke('save_ai_config', { config: aiConfig.value });
    toast.success("配置已保存到数据库");
    isConfigMode.value = false;
  } catch (err) {
    toast.error("保存失败: " + err);
  }
};

const scrollToBottom = async () => {
  await nextTick();
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
  }
};

const sendMessage = async () => {
  if (!userInput.value.trim() || isLoading.value) return;
  if (!aiConfig.value.apiKey) {
    toast.warning("请先配置 API Key");
    isConfigMode.value = true;
    return;
  }

  const content = userInput.value;
  const taskId = Math.random().toString(36).substring(7);
  const originalInput = content;

  messages.value.push({ role: 'user', content });
  messages.value.push({ role: 'assistant', content: '' });

  userInput.value = '';
  isLoading.value = true;
  await scrollToBottom();

  try {
    await invoke('ask_ai', {
      prompt: content,
      config: aiConfig.value,
      taskId: taskId
    });
  } catch (err) {
    toast.error("AI 响应失败: " + err);
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'assistant' && !lastMsg.content) {
      lastMsg.content = `❌ 呼叫 AI 失败: ${err}`;
    }
    userInput.value = originalInput;
  } finally {
    isLoading.value = false;
    await scrollToBottom();
  }
};

const sendToTerminal = async (fullContent: string) => {
  if (!props.activeSessionId) {
    toast.warning("请先连接到一个 SSH 会话");
    return;
  }
  // 提取代码块逻辑
  let code = fullContent;
  const match = fullContent.match(/```(?:bash|sh|linux|json|yaml)?\n([\s\S]*?)```/);
  if (match) code = match[1];

  const data = code.endsWith('\n') ? code : code + '\n';
  await invoke('write_to_ssh', { sessionId: props.activeSessionId, data });
  toast.success("指令已发送至终端");
};

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendMessage();
  }
};

watch(() => aiConfig.value.currentProvider, (newProvider) => {
  const p = providers.find(item => item.id === newProvider);
  if (p) aiConfig.value.model = p.models[0];
});

onMounted(async () => {
  await loadSettings();
  unlistenChunk = await listen<{taskId: string, content: string}>('ai-res-chunk', (event) => {
    const { content } = event.payload;
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'assistant') {
      lastMsg.content += content;
      scrollToBottom();
    }
  });
});

onUnmounted(() => {
  if (unlistenChunk) unlistenChunk();
});
</script>

<template>
  <div class="ai-panel">
    <div class="panel-header">
      <div class="title">
        <i class="fas fa-robot"></i>
        <span>AI 终端助手</span>
      </div>
      <button class="icon-btn" @click="isConfigMode = !isConfigMode" :class="{ active: isConfigMode }">
        <i class="fas fa-cog"></i>
      </button>
    </div>

    <div v-if="isConfigMode" class="config-container custom-scrollbar">
      <div class="config-group">
        <label>模型供应商</label>
        <select v-model="aiConfig.currentProvider">
          <option v-for="p in providers" :key="p.id" :value="p.id">{{ p.name }}</option>
        </select>
      </div>

      <div class="config-group">
        <label>API Key</label>
        <div class="input-with-icon">
          <input type="password" v-model="aiConfig.apiKey" placeholder="在此输入 API 令牌..." />
          <i class="fas fa-key"></i>
        </div>
      </div>

      <div class="config-group">
        <label>指定模型</label>
        <select v-model="aiConfig.model">
          <option v-for="m in providers.find(p => p.id === aiConfig.currentProvider)?.models" :key="m" :value="m">
            {{ m }}
          </option>
        </select>
      </div>

      <div class="config-group">
        <label>Temperature: {{ aiConfig.temperature }}</label>
        <input type="range" min="0" max="1.5" step="0.1" v-model.number="aiConfig.temperature" />
      </div>

      <button class="btn-save-config" @click="saveSettings">
        <i class="fas fa-save"></i> 保存并返回
      </button>
    </div>

    <template v-else>
      <div class="chat-viewport custom-scrollbar" ref="scrollContainer">
        <div v-for="(msg, index) in messages" :key="index" :class="['msg-row', msg.role]">
          <div class="msg-bubble">
            <div class="msg-text markdown-body" v-html="renderMarkdown(msg.content || (msg.role === 'assistant' && isLoading && index === messages.length - 1 ? '...' : ''))"></div>

            <button v-if="msg.role === 'assistant' && msg.content.includes('```')"
                    class="btn-execute" @click="sendToTerminal(msg.content)">
              <i class="fas fa-terminal"></i> 运行建议
            </button>
          </div>
        </div>
        <div v-if="isLoading" class="ai-typing">
          <span class="dot"></span><span class="dot"></span><span class="dot"></span>
        </div>
      </div>

      <div class="input-bar">
        <textarea
            v-model="userInput"
            placeholder="询问 Linux 相关问题..."
            @keydown="handleKeydown"
        ></textarea>
        <button class="btn-send" @click="sendMessage" :disabled="isLoading || !userInput.trim()">
          <i class="fas fa-paper-plane"></i>
        </button>
      </div>
    </template>
  </div>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

/* 覆盖样式以穿透 v-html (深度选择器) */
:deep(.markdown-body) {
  font-size: 13px;
  line-height: 1.6;
  color: base.$text-main; // 修改：跟随主题主文字颜色

  p { margin: 0 0 8px 0; }

  code {
    background: rgba(base.$accent, 0.15);
    color: base.$accent;
    padding: 2px 4px;
    border-radius: 4px;
    font-family: 'Fira Code', monospace;
  }

  pre {
    background: base.$bg-dark !important; // 修改：使用主题最深背景
    padding: 12px;
    border-radius: 8px;
    border: 1px solid base.$border;
    margin: 10px 0;
    overflow-x: auto;

    code {
      background: transparent;
      padding: 0;
      color: base.$text-main; // 修改：代码文字跟随主题
    }
  }

  ul, ol { padding-left: 20px; margin-bottom: 8px; }
  a {
    color: base.$accent;
    text-decoration: none;
    &:hover { text-decoration: underline; }
  }
}

.ai-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: base.$bg-primary; // 修改：通常面板使用主背景
  color: base.$text-main;
}

.panel-header {
  padding: 14px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid base.$border;
  background: base.$bg-header; // 建议增加 header 背景映射

  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    font-weight: 600;
    color: base.$accent;
  }
}

.icon-btn {
  background: transparent;
  border: none;
  color: base.$text-dim; // 修改：使用更淡的文字色
  cursor: pointer;
  transition: color 0.2s;

  &:hover, &.active { color: base.$accent; }
}

.config-container {
  flex: 1;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  background: base.$bg-secondary; // 修改：配置区使用次要背景

  .config-group {
    display: flex;
    flex-direction: column;
    gap: 8px;

    label {
      font-size: 11px;
      color: base.$text-muted;
      font-weight: bold;
      text-transform: uppercase;
    }

    input, select {
      background: base.$bg-input; // 修改：使用专门的输入框背景
      border: 1px solid base.$border;
      border-radius: 6px;
      padding: 10px;
      color: base.$text-main;
      font-size: 12px;

      &:focus {
        border-color: base.$accent;
        outline: none;
        box-shadow: 0 0 0 2px rgba(base.$accent, 0.2); // 增加聚焦发光
      }
    }

    .input-with-icon {
      position: relative;
      input { width: 100%; padding-right: 35px; box-sizing: border-box; }
      i { position: absolute; right: 12px; top: 12px; color: base.$text-dim; font-size: 12px; }
    }
  }

  .btn-save-config {
    margin-top: 10px;
    padding: 12px;
    background: base.$accent;
    color: base.$bg-primary; // 修改：按钮文字使用背景色以形成反差
    border: none;
    border-radius: 6px;
    font-weight: bold;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: opacity 0.2s;

    &:hover { opacity: 0.9; }
  }
}

.chat-viewport {
  flex: 1;
  overflow-y: auto;
  padding: 15px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  background: base.$bg-primary;

  .msg-row {
    display: flex;
    flex-direction: column;
    max-width: 85%;

    &.user {
      align-self: flex-end;
      .msg-bubble {
        background: rgba(base.$accent, 0.2); // 修改：用户气泡使用强调色的透明色
        border: 1px solid rgba(base.$accent, 0.3);
        color: base.$text-main;
        border-radius: 12px 12px 2px 12px;
      }
    }

    &.assistant {
      align-self: flex-start;
      .msg-bubble {
        background: base.$bg-card; // 修改：助手使用卡片背景色
        border: 1px solid base.$border;
        color: base.$text-main;
        border-radius: 12px 12px 12px 2px;
      }
    }
  }

  .msg-bubble {
    padding: 10px 14px;
    font-size: 12.5px;

    .btn-execute {
      margin-top: 10px;
      width: 100%;
      padding: 6px;
      background: rgba(base.$accent, 0.1);
      border: 1px dashed base.$accent;
      color: base.$accent;
      border-radius: 4px;
      cursor: pointer;
      font-size: 11px;
      transition: all 0.2s;

      &:hover {
        background: base.$accent;
        color: base.$bg-primary;
      }
    }
  }
}

.input-bar {
  padding: 12px;
  border-top: 1px solid base.$border;
  display: flex;
  gap: 10px;
  align-items: flex-end;
  background: base.$bg-secondary;

  textarea {
    flex: 1;
    background: base.$bg-input;
    border: 1px solid base.$border;
    border-radius: 8px;
    padding: 10px;
    color: base.$text-main;
    font-size: 12px;
    height: 60px;
    resize: none;

    &:focus {
      outline: none;
      border-color: base.$accent;
    }
  }

  .btn-send {
    width: 40px;
    height: 40px;
    background: base.$accent;
    color: base.$bg-primary;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;

    &:disabled {
      background: base.$text-dim;
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}

.ai-typing {
  display: flex;
  gap: 4px;
  padding: 10px;

  .dot {
    width: 6px;
    height: 6px;
    background: base.$accent;
    border-radius: 50%;
    animation: blink 1.4s infinite;
  }
  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }
}

@keyframes blink { 0%, 100% { opacity: 0.3; } 50% { opacity: 1; } }

/* 使用 base.scss 中定义的滚动条变量或直接映射 */
.custom-scrollbar {
  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-track { background: transparent; }
  &::-webkit-scrollbar-thumb {
    background: base.$border;
    border-radius: 4px;
    &:hover { background: base.$text-dim; }
  }
}
</style>