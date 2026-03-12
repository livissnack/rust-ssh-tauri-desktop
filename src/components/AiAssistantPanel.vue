<script setup lang="ts">
import {ref, nextTick, onMounted, onUnmounted, computed, watch} from 'vue';
import {invoke} from '@tauri-apps/api/core';
import {listen, type UnlistenFn} from '@tauri-apps/api/event';
import {toast} from '../utils/toast.ts';
import {marked} from 'marked';
import hljs from 'highlight.js';
import 'highlight.js/styles/tokyo-night-dark.css';

const props = defineProps<{
  activeSessionId: string | null
}>();

const isConfigMode = ref(false);
const messages = ref([
  {role: 'assistant', content: '你好！我是你的 SSH 助手。请点击右上方设置配置 API Key 以启用 AI 功能。'}
]);
const userInput = ref('');
const isLoading = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);
let unlistenChunk: UnlistenFn | null = null;

const aiConfig = ref({
  currentProvider: 'deepseek',
  apiKey: '',
  model: 'deepseek-chat',
  temperature: 0.7
});

const providers = [
  {id: 'deepseek', name: 'DeepSeek', models: ['deepseek-chat', 'deepseek-coder']},
  {id: 'qwen', name: '通义千问', models: ['qwen-max', 'qwen-plus', 'qwen-turbo']},
  {id: 'doubao', name: '豆包', models: ['doubao-pro-4k', 'doubao-lite-4k']},
  {id: 'gemini', name: 'Gemini', models: ['gemini-1.5-pro', 'gemini-1.5-flash']}
];

const renderer = new marked.Renderer();
marked.setOptions({
  renderer,
  highlight: function (code: string, lang: string) {
    const language = hljs.getLanguage(lang) ? lang : 'plaintext';
    return hljs.highlight(code, {language}).value;
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

const loadSettings = async () => {
  try {
    const saved = await invoke<any>('get_ai_config');
    if (saved) {
      aiConfig.value.currentProvider = saved.currentProvider;
      await nextTick();
      aiConfig.value.model = saved.model;
      aiConfig.value.apiKey = saved.apiKey;
      aiConfig.value.temperature = saved.temperature;
    }
  } catch (err) {
    console.error("加载 AI 配置失败:", err);
  }
};

const saveSettings = async () => {
  try {
    await invoke('save_ai_config', {config: aiConfig.value});
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

  messages.value.push({role: 'user', content});
  messages.value.push({role: 'assistant', content: ''});

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
  let code = fullContent;
  const match = fullContent.match(/```(?:bash|sh|linux|json|yaml)?\n([\s\S]*?)```/);
  if (match) code = match[1];

  const data = code.endsWith('\n') ? code : code + '\n';
  await invoke('write_to_ssh', {sessionId: props.activeSessionId, data});
  toast.success("指令已发送至终端");
};

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendMessage();
  }
};

const currentModels = computed(() => {
  const p = providers.find(item => item.id === aiConfig.value.currentProvider);
  return p ? p.models : [];
});

watch(() => aiConfig.value.currentProvider, (newProvider, oldProvider) => {
  if (!oldProvider) return;

  const p = providers.find(item => item.id === newProvider);
  if (p && p.models.length > 0) {
    aiConfig.value.model = p.models[0];
  }
}, {immediate: false});

onMounted(async () => {
  await loadSettings();
  unlistenChunk = await listen<{ taskId: string, content: string }>('ai-res-chunk', (event) => {
    const {content} = event.payload;
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
          <input type="password" v-model="aiConfig.apiKey" placeholder="在此输入 API 令牌..."/>
          <i class="fas fa-key"></i>
        </div>
      </div>

      <div class="config-group">
        <label>指定模型</label>
        <select v-model="aiConfig.model">
          <option v-for="m in currentModels" :key="m" :value="m">
            {{ m }}
          </option>
        </select>
      </div>

      <div class="config-group">
        <label>Temperature: {{ aiConfig.temperature }}</label>
        <input type="range" min="0" max="1.5" step="0.1" v-model.number="aiConfig.temperature"/>
      </div>

      <button class="btn-save-config" @click="saveSettings">
        <i class="fas fa-save"></i> 保存并返回
      </button>
    </div>

    <template v-else>
      <div class="chat-viewport custom-scrollbar" ref="scrollContainer">
        <div v-for="(msg, index) in messages" :key="index" :class="['msg-row', msg.role]">
          <div class="msg-bubble">
            <div class="msg-text markdown-body"
                 v-html="renderMarkdown(msg.content || (msg.role === 'assistant' && isLoading && index === messages.length - 1 ? '...' : ''))"></div>

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
        <div class="input-inner-wrapper">
          <textarea
              class="input-textarea"
              v-model="userInput"
              placeholder="输入问题，Shift + Enter 换行..."
              @keydown="handleKeydown"
              :rows="userInput.split('\n').length > 3 ? 3 : 2"
              spellcheck="false"
          ></textarea>

          <div class="input-actions">
            <div class="input-info">
              <span v-if="activeSessionId" class="status-tag">
                <i class="fas fa-link"></i> 已挂载终端
              </span>
              <span v-else class="status-tag warning">
                <i class="fas fa-unlink"></i> 未连接会话
              </span>
            </div>

            <div class="action-right">
              <span class="kb-hint">Enter 发送</span>
              <button
                  class="btn-send"
                  @click="sendMessage"
                  :disabled="isLoading || !userInput.trim()"
                  :class="{ 'is-loading': isLoading }"
              >
                <i class="fas" :class="isLoading ? 'fa-circle-notch fa-spin' : 'fa-paper-plane'"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

:deep(.markdown-body) {
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-main);

  p {
    margin: 0 0 8px 0;
  }

  code {
    background: var(--accent-15);
    color: var(--accent);
    padding: 2px 4px;
    border-radius: 4px;
    font-family: 'Fira Code', monospace;
  }

  pre {
    background: var(--bg-secondary) !important;
    padding: 12px;
    border-radius: 8px;
    border: 1px solid var(--border);
    margin: 10px 0;
    overflow-x: auto;

    code {
      background: transparent;
      padding: 0;
      color: var(--text-main);
    }
  }

  ul, ol {
    padding-left: 20px;
    margin-bottom: 8px;
  }

  a {
    color: var(--accent);
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }
}

.ai-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-main);
}

.panel-header {
  padding: 14px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);

  .title {
    display: flex;
    align-items: center;
    gap: 10px;
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
  transition: color 0.2s;

  &:hover, &.active {
    color: var(--accent);
  }
}

.config-container {
  flex: 1;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  background: var(--bg-secondary);

  .config-group {
    display: flex;
    flex-direction: column;
    gap: 8px;

    label {
      font-size: 11px;
      color: var(--text-dim);
      font-weight: bold;
      text-transform: uppercase;
    }

    input, select {
      background: var(--bg-input);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 10px;
      color: var(--text-main);
      font-size: 12px;

      &:focus {
        border-color: var(--accent);
        outline: none;
        box-shadow: 0 0 0 2px var(--accent-20);
      }
    }

    .input-with-icon {
      position: relative;

      input {
        width: 100%;
        padding-right: 35px;
        box-sizing: border-box;
      }

      i {
        position: absolute;
        right: 12px;
        top: 12px;
        color: var(--text-dim);
        font-size: 12px;
      }
    }
  }

  .btn-save-config {
    margin-top: 10px;
    padding: 12px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 6px;
    font-weight: bold;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: opacity 0.2s;

    &:hover {
      opacity: 0.9;
    }
  }
}

.chat-viewport {
  flex: 1;
  overflow-y: auto;
  padding: 15px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  background: var(--bg-primary);

  .msg-row {
    display: flex;
    flex-direction: column;
    max-width: 85%;

    &.user {
      align-self: flex-end;

      .msg-bubble {
        background: var(--accent-20);
        border: 1px solid var(--accent-30);
        color: var(--text-main);
        border-radius: 12px 12px 2px 12px;
      }
    }

    &.assistant {
      align-self: flex-start;

      .msg-bubble {
        background: var(--bg-card);
        border: 1px solid var(--border);
        color: var(--text-main);
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
      background: var(--accent-10);
      border: 1px dashed var(--accent);
      color: var(--accent);
      border-radius: 4px;
      cursor: pointer;
      font-size: 11px;
      transition: all 0.2s;

      &:hover {
        background: var(--accent);
        color: var(--bg-primary);
      }
    }
  }
}

.input-bar {
  padding: 16px 20px;
  background: var(--bg-primary);
  border-top: 1px solid var(--border-50);

  .input-inner-wrapper {
    position: relative;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 10px 14px 8px 14px;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    display: flex;
    flex-direction: column;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);

    &:focus-within {
      border-color: var(--accent);
      background: var(--bg-primary);
      box-shadow: 0 4px 20px var(--accent-15);
      transform: translateY(-2px);
    }

    .input-textarea {
      width: 100%;
      min-height: 24px;
      max-height: 150px;
      background: transparent !important;

      border: none !important;
      outline: none !important;
      box-shadow: none !important;
      appearance: none !important;
      -webkit-appearance: none !important;

      padding: 4px 0;
      color: var(--text-main);
      font-size: 13.5px;
      line-height: 1.6;
      resize: none;
      font-family: inherit;

      &:focus {
        border: none !important;
        outline: none !important;
        box-shadow: none !important;
      }

      &::placeholder {
        color: var(--text-dim);
        opacity: 0.4;
      }
    }

    .input-actions {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-top: 8px;
      padding-top: 8px;
      border-top: 1px solid var(--border-30);

      .status-tag {
        font-size: 10px;
        color: var(--text-dim);
        background: var(--accent-05);
        padding: 2px 8px;
        border-radius: 4px;
        display: flex;
        align-items: center;
        gap: 5px;
        opacity: 0.8;

        i {
          font-size: 9px;
        }

        &.warning {
          color: #e67e22;
          background: rgba(230, 126, 34, 0.1);
        }
      }

      .action-right {
        display: flex;
        align-items: center;
        gap: 12px;

        .kb-hint {
          font-size: 10px;
          color: var(--text-dim);
          opacity: 0.5;
          letter-spacing: 0.5px;
        }

        .btn-send {
          width: 30px;
          height: 30px;
          background: var(--accent);
          color: var(--bg-primary);
          border: none;
          border-radius: 8px;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          transition: all 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275);
          box-shadow: 0 2px 6px var(--accent-20);

          &:hover:not(:disabled) {
            transform: scale(1.1);
            filter: brightness(1.1);
            box-shadow: 0 4px 12px var(--accent-40);
          }

          &:active:not(:disabled) {
            transform: scale(0.9);
          }

          &:disabled {
            background: var(--border);
            color: var(--text-dim);
            cursor: not-allowed;
            box-shadow: none;
            opacity: 0.6;
          }

          i {
            font-size: 13px;
          }

          &.is-loading {
            background: var(--accent-20);
            color: var(--accent);
          }
        }
      }
    }
  }
}

[data-theme='rmb-red'], .rmb-red-theme {
  .input-inner-wrapper:focus-within {
    box-shadow: 0 4px 20px rgba(230, 0, 0, 0.2);
  }
}

.ai-typing {
  display: flex;
  gap: 4px;
  padding: 10px;

  .dot {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    animation: blink 1.4s infinite;
  }

  .dot:nth-child(2) {
    animation-delay: 0.2s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.4s;
  }
}

@keyframes blink {
  0%, 100% {
    opacity: 0.3;
  }
  50% {
    opacity: 1;
  }
}

.custom-scrollbar {
  &::-webkit-scrollbar {
    width: 4px;
  }

  &::-webkit-scrollbar-track {
    background: transparent;
  }

  &::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 4px;

    &:hover {
      background: var(--text-dim);
    }
  }
}
</style>