<script setup lang="ts">
import {ref, nextTick, onMounted, onUnmounted, computed, watch} from 'vue';
import {invoke} from '@tauri-apps/api/core';
import {listen, type UnlistenFn} from '@tauri-apps/api/event';
import {toast} from '../utils/toast.ts';
import {confirm} from '../utils/confirm.ts';
import {throttle, debounce} from '../utils/async.ts';
import {
  AI_CHAT_RETENTION_DAYS,
  buildSessionTitle,
  clearActiveSessionId,
  clearAiChatSessions,
  createAiChatSessionId,
  deleteAiChatSession,
  formatAiChatTime,
  getActiveSessionId,
  getAiChatSession,
  listAiChatSessions,
  pruneAiChatSessions,
  saveAiChatSession,
  setActiveSessionId,
  type AiChatMessage,
  type AiChatSessionSummary,
} from '../utils/aiChatStorage.ts';
import { useI18n } from '../utils/i18n.ts';
import { useDelegatedTooltip } from '../composables/useDelegatedTooltip.ts';
import {marked} from 'marked';
import hljs from 'highlight.js';
import 'highlight.js/styles/tokyo-night-dark.css';

type ChatMessage = {
  role: 'user' | 'assistant';
  content: string;
  timestamp?: number;
  isWelcome?: boolean;
};

const { t, tr, locale } = useI18n();
const {
  visible: codeTipVisible,
  text: codeTipText,
  portalStyle: codeTipStyle,
  tooltipRef: codeTipRef,
  onPointerOver: onCodeTipOver,
  onPointerOut: onCodeTipOut,
} = useDelegatedTooltip({ placement: 'top', delay: 350 });

const createWelcomeMessage = (): ChatMessage => ({
  role: 'assistant',
  content: t('ai.welcome'),
  isWelcome: true,
});

const props = defineProps<{
  activeSessionId: string | null;
  sessionConnected?: boolean;
}>();

const isConfigMode = ref(false);
const showHistoryPanel = ref(false);
const showApiKey = ref(false);
const messages = ref<ChatMessage[]>([createWelcomeMessage()]);
const userInput = ref('');
const isLoading = ref(false);
const isHistoryLoading = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);
const currentChatSessionId = ref(createAiChatSessionId());
const sessionCreatedAt = ref(Date.now());
const historySessions = ref<AiChatSessionSummary[]>([]);
let unlistenChunk: UnlistenFn | null = null;

const aiConfig = ref({
  currentProvider: 'deepseek',
  apiKey: '',
  model: 'deepseek-chat',
  temperature: 0.7,
  id: 'default',
  updated_at: 0,
  deleted: false
});

const providers = [
  {id: 'deepseek', label: 'DeepSeek', models: ['deepseek-chat', 'deepseek-coder']},
  {id: 'qwen', labelKey: 'qwen' as const, models: ['qwen-max', 'qwen-plus', 'qwen-turbo']},
  {id: 'doubao', labelKey: 'doubao' as const, models: ['doubao-pro-4k', 'doubao-lite-4k']},
  {id: 'gemini', label: 'Gemini', models: ['gemini-1.5-pro', 'gemini-1.5-flash']}
];

const TERMINAL_LANGS = new Set([
  '', 'bash', 'sh', 'shell', 'zsh', 'fish', 'linux', 'console', 'terminal',
  'cmd', 'powershell', 'pwsh', 'plaintext', 'text',
]);

const escapeHtml = (text: string) =>
  text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');

const encodeCommandAttr = (text: string) => encodeURIComponent(text);

const decodeCommandAttr = (text: string) => decodeURIComponent(text);

const isShellLang = (lang?: string) => {
  const key = (lang || '').trim().toLowerCase();
  return TERMINAL_LANGS.has(key);
};

const renderer = new marked.Renderer();

renderer.code = ({ text, lang }: { text: string; lang?: string }) => {
  const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
  const highlighted = hljs.highlight(text, { language }).value;
  const langLabel = (lang || 'text').toLowerCase();
  const encoded = encodeCommandAttr(text);
  const showShellActions = isShellLang(lang);
  const toolbar = showShellActions
    ? `<div class="ai-code-block__toolbar">
        <button type="button" class="ai-code-btn ai-code-copy" data-command="${encoded}" data-tip="${escapeHtml(t('ai.copyCommand'))}" aria-label="${escapeHtml(t('ai.copy'))}">
          <i class="fas fa-copy"></i>
        </button>
        <button type="button" class="ai-code-btn ai-code-btn--run ai-code-run" data-command="${encoded}" data-tip="${escapeHtml(t('ai.runInTerminal'))}" aria-label="${escapeHtml(t('ai.run'))}">
          <i class="fas fa-play"></i>
        </button>
      </div>`
    : '';

  return `<div class="ai-code-block${showShellActions ? ' ai-code-block--shell' : ''}">
    <div class="ai-code-block__pre">
      <pre><code class="hljs language-${escapeHtml(language)}">${highlighted}</code></pre>
      <span class="ai-code-block__lang">${escapeHtml(langLabel)}</span>
      ${toolbar}
    </div>
  </div>`;
};

renderer.codespan = ({ text }: { text: string }) => `<code>${escapeHtml(text)}</code>`;

marked.setOptions({
  renderer,
  breaks: true,
  gfm: true,
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
      aiConfig.value = {
        ...aiConfig.value,
        ...saved
      };
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
    await invoke('save_ai_config', { config: { ...aiConfig.value } });
    toast.success(t('ai.configSaved'));
    isConfigMode.value = false;
    showApiKey.value = false;
    await loadSettings();
  } catch (err) {
    toast.error(t('ai.configSaveFailed', { err: String(err) }));
  }
};

const scrollToBottom = async () => {
  await nextTick();
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
  }
};

const getPersistableMessages = (): AiChatMessage[] =>
  messages.value
    .filter((m) => m.content.trim())
    .filter((m) => !m.isWelcome)
    .map((m) => ({
      role: m.role,
      content: m.content,
      timestamp: m.timestamp ?? Date.now(),
    }));

const refreshHistorySessions = async () => {
  historySessions.value = await listAiChatSessions();
};

const persistCurrentSession = async () => {
  const persistable = getPersistableMessages();
  if (!persistable.some((m) => m.role === 'user')) return;

  const summary = await saveAiChatSession({
    id: currentChatSessionId.value,
    title: buildSessionTitle(persistable),
    messages: persistable,
    createdAt: sessionCreatedAt.value,
    updatedAt: Date.now(),
  });
  setActiveSessionId(currentChatSessionId.value);
  const idx = historySessions.value.findIndex((s) => s.id === summary.id);
  if (idx >= 0) historySessions.value[idx] = summary;
  else historySessions.value.unshift(summary);
  historySessions.value.sort((a, b) => b.updatedAt - a.updatedAt);
};

const schedulePersist = debounce(() => {
  void persistCurrentSession().catch((err) => console.error('保存 AI 会话失败:', err));
}, 600);

const applySessionToView = (sessionMessages: AiChatMessage[]) => {
  messages.value = [
    createWelcomeMessage(),
    ...sessionMessages.map((m) => ({
      role: m.role,
      content: m.content,
      timestamp: m.timestamp,
    })),
  ];
};

const startNewChat = (notify = true) => {
  currentChatSessionId.value = createAiChatSessionId();
  sessionCreatedAt.value = Date.now();
  messages.value = [createWelcomeMessage()];
  userInput.value = '';
  setActiveSessionId(currentChatSessionId.value);
  showHistoryPanel.value = false;
  if (notify) toast.info(t('ai.newChatStarted'));
  void scrollToBottom();
};

const loadHistorySession = async (sessionId: string) => {
  isHistoryLoading.value = true;
  try {
    const session = await getAiChatSession(sessionId);
    if (!session) {
      toast.warning(t('ai.sessionExpired'));
      await refreshHistorySessions();
      if (getActiveSessionId() === sessionId) clearActiveSessionId();
      return;
    }
    currentChatSessionId.value = session.id;
    sessionCreatedAt.value = session.createdAt;
    applySessionToView(session.messages);
    setActiveSessionId(session.id);
    showHistoryPanel.value = false;
    await scrollToBottom();
  } catch (err) {
    toast.error(t('ai.loadSessionFailed', { err: String(err) }));
  } finally {
    isHistoryLoading.value = false;
  }
};

const deleteHistorySession = async (sessionId: string) => {
  const target = historySessions.value.find((s) => s.id === sessionId);
  const ok = await confirm.warning(
    t('ai.deleteSessionConfirm', { title: target?.title || t('ai.thisChat') }),
    t('ai.deleteSessionTitle'),
  );
  if (!ok) return;
  try {
    await deleteAiChatSession(sessionId);
    historySessions.value = historySessions.value.filter((s) => s.id !== sessionId);
    if (currentChatSessionId.value === sessionId) {
      if (historySessions.value.length > 0) {
        await loadHistorySession(historySessions.value[0].id);
      } else {
        startNewChat(false);
      }
    }
    toast.success(t('ai.deleted'));
  } catch (err) {
    toast.error(t('ai.deleteFailed', { err: String(err) }));
  }
};

const clearAllHistory = async () => {
  if (!historySessions.value.length) return;
  const ok = await confirm.warning(
    t('ai.clearAllConfirm', { days: AI_CHAT_RETENTION_DAYS }),
    t('ai.clearAllTitle'),
  );
  if (!ok) return;
  try {
    await clearAiChatSessions();
    historySessions.value = [];
    startNewChat(false);
    toast.success(t('ai.historyCleared'));
  } catch (err) {
    toast.error(t('ai.clearFailed', { err: String(err) }));
  }
};

const bootstrapChatHistory = async () => {
  await pruneAiChatSessions();
  await refreshHistorySessions();

  const activeId = getActiveSessionId();
  if (activeId) {
    const session = await getAiChatSession(activeId);
    if (session) {
      currentChatSessionId.value = session.id;
      sessionCreatedAt.value = session.createdAt;
      applySessionToView(session.messages);
      return;
    }
    clearActiveSessionId();
  }

  if (historySessions.value.length > 0) {
    await loadHistorySession(historySessions.value[0].id);
    return;
  }

  startNewChat(false);
};

const toggleHistoryPanel = () => {
  if (isConfigMode.value) return;
  showHistoryPanel.value = !showHistoryPanel.value;
  if (showHistoryPanel.value) void refreshHistorySessions();
};

const handleSendMessage = throttle(() => {
  if (!userInput.value.trim() || isLoading.value) return;
  sendMessage();
}, 1000);

const sendMessage = async () => {
  if (!userInput.value.trim() || isLoading.value) return;
  if (!aiConfig.value.apiKey) {
    toast.warning(t('ai.configureApiKeyFirst'));
    isConfigMode.value = true;
    return;
  }

  const content = userInput.value;
  const taskId = Math.random().toString(36).substring(7);
  const originalInput = content;

  messages.value.push({role: 'user', content, timestamp: Date.now()});
  messages.value.push({role: 'assistant', content: '', timestamp: Date.now()});

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
    toast.error(t('ai.aiResponseFailed', { err: String(err) }));
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'assistant' && !lastMsg.content) {
      lastMsg.content = t('ai.aiCallFailed', { err: String(err) });
    }
    userInput.value = originalInput;
  } finally {
    isLoading.value = false;
    await scrollToBottom();
    schedulePersist();
  }
};

const runCommandInTerminal = async (command: string) => {
  if (!props.activeSessionId) {
    toast.warning(t('ai.connectSshFirst'));
    return;
  }
  if (!props.sessionConnected) {
    toast.warning(t('ai.sessionNotConnected'));
    return;
  }
  const trimmed = command.trim();
  if (!trimmed) return;
  const data = trimmed.endsWith('\n') ? trimmed : `${trimmed}\n`;
  await invoke('write_to_ssh', { sessionId: props.activeSessionId, data });
  toast.success(t('ai.commandSent'));
};

const copyCommand = async (command: string) => {
  try {
    await navigator.clipboard.writeText(command);
    toast.success(t('ai.copied'));
  } catch {
    toast.error(t('ai.copyFailed'));
  }
};

const handleChatActionClick = (event: MouseEvent) => {
  const target = event.target as HTMLElement;
  const runBtn = target.closest('.ai-code-run') as HTMLElement | null;
  if (runBtn?.dataset.command) {
    event.preventDefault();
    event.stopPropagation();
    void runCommandInTerminal(decodeCommandAttr(runBtn.dataset.command));
    return;
  }
  const copyBtn = target.closest('.ai-code-copy') as HTMLElement | null;
  if (copyBtn?.dataset.command) {
    event.preventDefault();
    event.stopPropagation();
    void copyCommand(decodeCommandAttr(copyBtn.dataset.command));
  }
};

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSendMessage();
  }
};

const currentModels = computed(() => {
  const p = providers.find(item => item.id === aiConfig.value.currentProvider);
  return p ? p.models : [];
});

const providerOptions = computed(() =>
  providers.map((provider) => ({
    value: provider.id,
    label: 'labelKey' in provider && provider.labelKey
      ? t(`ai.providers.${provider.labelKey}`)
      : provider.label,
  }))
);

const modelOptions = computed(() =>
  currentModels.value.map((model) => ({ value: model, label: model }))
);

const isAiReady = computed(() => !!aiConfig.value.apiKey?.trim());

const terminalStatusTag = computed(() => {
  if (!props.activeSessionId) return null;
  if (props.sessionConnected) {
    return { kind: 'linked' as const, text: t('ai.terminalLinked') };
  }
  return { kind: 'idle' as const, text: t('ai.terminalIdle') };
});

const handleToggleConfig = throttle(() => {
  isConfigMode.value = !isConfigMode.value;
  if (isConfigMode.value) {
    showHistoryPanel.value = false;
    showApiKey.value = false;
  }
}, 300);

watch(locale, () => {
  const welcome = messages.value.find((m) => m.isWelcome);
  if (welcome) welcome.content = t('ai.welcome');
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
  await bootstrapChatHistory();
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
        <span>{{ t('ai.title') }}</span>
      </div>
      <div class="header-actions">
        <button
            v-if="!isConfigMode"
            class="icon-btn"
            :class="{ active: showHistoryPanel }"
            :title="t('ai.history')"
            @click="toggleHistoryPanel"
        >
          <i class="fas fa-clock-rotate-left"></i>
        </button>
        <button
            v-if="!isConfigMode"
            class="icon-btn"
            :title="t('ai.newChat')"
            @click="startNewChat()"
        >
          <i class="fas fa-plus"></i>
        </button>
        <button class="icon-btn" @click="handleToggleConfig" :class="{ active: isConfigMode }" :title="t('ai.settings')">
          <i class="fas fa-cog"></i>
        </button>
      </div>
    </div>

    <div v-if="isConfigMode" class="config-container custom-scrollbar">
      <div class="config-group">
        <label>{{ t('ai.provider') }}</label>
        <AppSelect
            v-model="aiConfig.currentProvider"
            :options="providerOptions"
            icon="fas fa-cloud"
        />
      </div>

      <div class="config-group">
        <label>{{ t('ai.apiKey') }}</label>
        <div class="input-with-icon">
          <input
              v-model="aiConfig.apiKey"
              :type="showApiKey ? 'text' : 'password'"
              :placeholder="t('ai.apiKeyPlaceholder')"
          />
          <button
              type="button"
              class="key-btn"
              :class="{ 'is-active': showApiKey }"
              @click="showApiKey = !showApiKey"
          >
            <i class="fas fa-key"></i>
          </button>
        </div>
      </div>

      <div class="config-group">
        <label>{{ t('ai.model') }}</label>
        <AppSelect
            v-model="aiConfig.model"
            :options="modelOptions"
            icon="fas fa-microchip"
        />
      </div>

      <div class="config-group">
        <label>{{ t('ai.temperature') }}: {{ aiConfig.temperature }}</label>
        <input type="range" min="0" max="1.5" step="0.1" v-model.number="aiConfig.temperature"/>
      </div>

      <button class="btn-save-config" @click="saveSettings">
        <i class="fas fa-save"></i> {{ t('ai.saveAndBack') }}
      </button>
    </div>

    <template v-else>
      <div v-if="showHistoryPanel" class="history-panel custom-scrollbar">
        <div class="history-panel__toolbar">
          <div class="history-panel__meta">
            <span class="history-panel__title">{{ t('ai.historyTitle') }}</span>
            <span class="history-panel__hint">{{ t('ai.historyHint', { days: AI_CHAT_RETENTION_DAYS }) }}</span>
          </div>
          <button type="button" class="btn-history-clear" :disabled="!historySessions.length" @click="clearAllHistory">
            <i class="fas fa-trash-alt"></i>
            {{ t('ai.clearAll') }}
          </button>
        </div>

        <div v-if="isHistoryLoading" class="history-empty">
          <i class="fas fa-spinner fa-spin"></i>
          <span>{{ tr.common.loading }}</span>
        </div>
        <div v-else-if="!historySessions.length" class="history-empty">
          <i class="fas fa-inbox"></i>
          <span>{{ t('ai.noHistory') }}</span>
        </div>
        <ul v-else class="history-list">
          <li
              v-for="item in historySessions"
              :key="item.id"
              class="history-item"
              :class="{ active: item.id === currentChatSessionId }"
          >
            <button type="button" class="history-item__body" @click="loadHistorySession(item.id)">
              <span class="history-item__title">{{ item.title }}</span>
              <span class="history-item__preview">{{ item.preview || t('ai.noContent') }}</span>
              <span class="history-item__meta">
                {{ formatAiChatTime(item.updatedAt) }} · {{ t('ai.messageCount', { count: item.messageCount }) }}
              </span>
            </button>
            <button
                type="button"
                class="history-item__delete"
                :title="tr.common.delete"
                @click.stop="deleteHistorySession(item.id)"
            >
              <i class="fas fa-trash-can"></i>
            </button>
          </li>
        </ul>
      </div>

      <div
          v-else
          class="chat-viewport custom-scrollbar"
          ref="scrollContainer"
          @click="handleChatActionClick"
          @mouseover="onCodeTipOver"
          @mouseout="onCodeTipOut"
      >
        <div v-for="(msg, index) in messages" :key="index" :class="['msg-row', msg.role]">
          <div class="msg-bubble">
            <div class="msg-text markdown-body"
                 v-html="renderMarkdown(msg.content || (msg.role === 'assistant' && isLoading && index === messages.length - 1 ? '...' : ''))"></div>
          </div>
        </div>
        <div v-if="isLoading" class="ai-typing">
          <span class="dot"></span><span class="dot"></span><span class="dot"></span>
        </div>
      </div>

      <div v-if="!showHistoryPanel" class="input-bar">
        <div class="input-inner-wrapper">
          <textarea
              class="input-textarea"
              v-model="userInput"
              :placeholder="t('ai.inputPlaceholder')"
              @keydown="handleKeydown"
              :rows="userInput.split('\n').length > 3 ? 3 : 2"
              spellcheck="false"
          ></textarea>

          <div class="input-actions">
            <div class="input-info">
              <span v-if="isAiReady" class="status-tag success">
                <i class="fas fa-robot"></i> {{ t('ai.aiReady') }}
              </span>
              <span v-else class="status-tag warning">
                <i class="fas fa-key"></i> {{ t('ai.apiKeyMissing') }}
              </span>
              <span
                  v-if="terminalStatusTag?.kind === 'linked'"
                  class="status-tag linked"
              >
                <i class="fas fa-link"></i> {{ terminalStatusTag.text }}
              </span>
              <span
                  v-else-if="terminalStatusTag?.kind === 'idle'"
                  class="status-tag muted"
              >
                <i class="fas fa-terminal"></i> {{ terminalStatusTag.text }}
              </span>
            </div>

            <div class="action-right">
              <span class="kb-hint">{{ t('ai.enterToSend') }}</span>
              <button
                  class="btn-send"
                  @click="handleSendMessage"
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

    <Teleport to="body">
      <Transition name="ai-code-tip-fade">
        <div
            v-if="codeTipVisible && codeTipText"
            ref="codeTipRef"
            class="app-tooltip"
            :style="codeTipStyle"
            role="tooltip"
        >
          {{ codeTipText }}
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

:deep(.markdown-body) {
  font-size: 12px;
  line-height: 1.65;
  color: var(--text-main);
  word-break: break-word;
  user-select: text;
  -webkit-user-select: text;

  > :first-child {
    margin-top: 0 !important;
  }

  > :last-child {
    margin-bottom: 0 !important;
  }

  p {
    margin: 0 0 10px 0;
  }

  h1, h2, h3, h4 {
    margin: 16px 0 8px;
    font-weight: 600;
    line-height: 1.35;
    color: var(--text-main);

    &:first-child {
      margin-top: 0;
    }
  }

  h1 { font-size: 1.25em; }
  h2 { font-size: 1.12em; }
  h3 { font-size: 1.05em; }
  h4 { font-size: 1em; color: var(--text-dim); }

  strong {
    font-weight: 600;
    color: var(--text-main);
  }

  em {
    font-style: italic;
    color: var(--text-dim);
  }

  hr {
    margin: 14px 0;
    border: none;
    border-top: 1px solid var(--border-30);
  }

  blockquote {
    margin: 10px 0;
    padding: 8px 12px;
    border-left: 3px solid var(--accent-30);
    border-radius: 0 8px 8px 0;
    background: var(--accent-05);
    color: var(--text-dim);

    p:last-child {
      margin-bottom: 0;
    }
  }

  table {
    width: 100%;
    margin: 10px 0;
    border-collapse: collapse;
    font-size: 11px;
    overflow: hidden;
    border-radius: 8px;
    border: 1px solid var(--border-30);
  }

  th, td {
    padding: 8px 10px;
    border: 1px solid var(--border-30);
    text-align: left;
  }

  th {
    background: var(--bg-secondary);
    font-weight: 600;
    color: var(--text-main);
  }

  td {
    background: var(--bg-input);
  }

  code:not(.hljs) {
    background: var(--accent-08);
    color: var(--accent);
    padding: 2px 7px;
    border-radius: 5px;
    border: 1px solid var(--accent-15);
    font-family: var(--font-terminal);
    font-size: 0.92em;
    user-select: text;
    -webkit-user-select: text;
  }

  :deep(.ai-code-block) {
    margin: 12px 0;
    border: 1px solid var(--border-30);
    border-radius: 10px;
    overflow: hidden;
    background: var(--bg-input);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  }

  pre {
    background: var(--bg-input) !important;
    padding: 14px 16px;
    border-radius: 10px;
    border: 1px solid var(--border-30);
    margin: 12px 0;
    overflow-x: auto;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);

    code {
      background: transparent;
      padding: 0;
      border: none;
      color: var(--text-main);
      font-size: 11px;
      line-height: 1.55;
    }
  }

  ul, ol {
    margin: 0 0 10px 0;
    padding-left: 1.35em;
  }

  li {
    margin: 4px 0;

    &::marker {
      color: var(--accent);
    }
  }

  li > p {
    margin-bottom: 4px;
  }

  a {
    color: var(--accent);
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: border-color 0.15s ease;

    &:hover {
      border-bottom-color: var(--accent-30);
    }
  }
}

.ai-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-main);
  font-size: 12px;
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
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
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
      font-size: 10px;
      color: var(--text-dim);
      font-weight: bold;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      -webkit-font-smoothing: antialiased; /* macOS 文字抗锯齿 */
    }

    input[type="password"],
    input[type="text"] {
      background: var(--bg-input);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 10px 12px;
      color: var(--text-main);
      font-size: 12px;
      width: 100%;
      box-sizing: border-box;
      transition: border-color 0.2s, box-shadow 0.2s;

      &:focus {
        border-color: var(--accent);
        outline: none;
        box-shadow: 0 0 0 3px var(--accent-15);
      }
    }

    /* 滑动条：跨平台重灾区，需要彻底重写 */
    input[type="range"] {
      -webkit-appearance: none;
      background: transparent;
      height: 20px;
      cursor: pointer;

      /* 轨道样式 */
      &::-webkit-slider-runnable-track {
        width: 100%;
        height: 4px;
        background: var(--border);
        border-radius: 2px;
      }
      /* 滑块样式 (Webkit: Mac/Win/Linux-Chrome) */
      &::-webkit-slider-thumb {
        -webkit-appearance: none;
        height: 16px;
        width: 16px;
        border-radius: 50%;
        background: var(--accent);
        margin-top: -6px; /* (4-16)/2 */
        box-shadow: 0 1px 3px rgba(0,0,0,0.3);
        border: 2px solid var(--bg-secondary);
        transition: transform 0.1s;
      }
      &:active::-webkit-slider-thumb {
        transform: scale(1.2);
      }
    }

    .input-with-icon {
      position: relative;

      input {
        padding-right: 40px;
      }

      .key-btn {
        position: absolute;
        right: 4px;
        top: 50%;
        transform: translateY(-50%);
        width: 28px;
        height: 28px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: none;
        border-radius: 6px;
        background: transparent;
        color: var(--text-dim);
        font-size: 11px;
        cursor: pointer;
        transition: background 0.15s ease, color 0.15s ease;

        &:hover,
        &.is-active {
          background: var(--accent-10);
          color: var(--accent);
        }
      }
    }
  }

  /* 保存按钮：增加触感反馈 */
  .btn-save-config {
    margin-top: 10px;
    padding: 10px;
    background: var(--accent);
    color: #ffffff; /* 强行指定白色，防止亮色主题下看不清 */
    border: none;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: all 0.2s;

    &:hover {
      filter: brightness(1.1);
      transform: translateY(-1px);
    }
    &:active {
      transform: translateY(0);
    }
  }
}

.history-panel {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 12px 14px;
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-30);

  &__toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  &__meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  &__title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-main);
  }

  &__hint {
    font-size: 9px;
    color: var(--text-dim);
    line-height: 1.4;
  }
}

.btn-history-clear {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--border-30);
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  font-size: 10px;
  cursor: pointer;
  flex-shrink: 0;

  &:hover:not(:disabled) {
    color: var(--error);
    border-color: var(--error-30);
    background: var(--error-10);
  }

  &:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
}

.history-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 48px 16px;
  color: var(--text-dim);
  font-size: 11px;

  i { font-size: 20px; opacity: 0.5; }
}

.history-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.history-item {
  display: flex;
  align-items: stretch;
  gap: 6px;
  border: 1px solid var(--border-30);
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg-card);
  transition: border-color 0.15s ease, box-shadow 0.15s ease;

  &.active {
    border-color: var(--accent-30);
    box-shadow: 0 0 0 1px var(--accent-10);
  }

  &:hover {
    border-color: var(--accent-20);
  }

  &__body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: inherit;
  }

  &__title {
    width: 100%;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-main);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  &__preview {
    width: 100%;
    font-size: 10px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  &__meta {
    font-size: 9px;
    color: var(--text-dim);
    opacity: 0.85;
  }

  &__delete {
    width: 36px;
    flex-shrink: 0;
    border: none;
    border-left: 1px solid var(--border-30);
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.15s ease;

    &:hover {
      color: var(--error);
      background: var(--error-10);
    }
  }
}

.chat-viewport {
  flex: 1;
  overflow-y: auto;
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  background: var(--bg-primary);
  min-height: 0;
  user-select: text;
  -webkit-user-select: text;

  .msg-row {
    display: flex;
    flex-direction: column;
    max-width: 88%;
    animation: ai-msg-in 0.22s ease-out;

    &.user {
      align-self: flex-end;
      max-width: 82%;

      .msg-bubble {
        background: linear-gradient(135deg, var(--accent-15) 0%, var(--accent-08) 100%);
        border: 1px solid var(--accent-25, var(--accent-30));
        color: var(--text-main);
        border-radius: 14px 14px 4px 14px;
        box-shadow: 0 2px 10px var(--accent-08);
      }
    }

    &.assistant {
      align-self: flex-start;
      max-width: 94%;

      .msg-bubble {
        background: var(--bg-card);
        border: 1px solid var(--border-30);
        color: var(--text-main);
        border-radius: 14px 14px 14px 4px;
        box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
      }
    }
  }

  .msg-bubble {
    padding: 10px 14px;
    font-size: 12px;
    line-height: 1.6;
    overflow: hidden;
    user-select: text;
    -webkit-user-select: text;
  }

  .msg-text {
    min-width: 0;
    user-select: text;
    -webkit-user-select: text;
  }
}

@keyframes ai-msg-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
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
      font-size: 12px;
      line-height: 1.55;
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

      .input-info {
        display: flex;
        align-items: center;
        gap: 6px;
        flex-wrap: wrap;
      }

      .status-tag {
        font-size: 9px;
        color: var(--text-dim);
        background: var(--accent-05);
        padding: 2px 8px;
        border-radius: 4px;
        display: flex;
        align-items: center;
        gap: 5px;
        opacity: 0.8;

        i {
          font-size: 8px;
        }

        &.success {
          color: var(--success);
          background: var(--success-10, rgba(16, 185, 129, 0.1));
          opacity: 1;
        }

        &.linked {
          color: var(--accent);
          background: var(--accent-08);
          opacity: 1;
        }

        &.muted {
          color: var(--text-dim);
          background: var(--bg-input);
          opacity: 0.75;
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
          font-size: 9px;
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
            font-size: 12px;
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
  align-self: flex-start;
  gap: 5px;
  padding: 10px 14px;
  margin-left: 2px;
  background: var(--bg-card);
  border: 1px solid var(--border-30);
  border-radius: 14px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);

  .dot {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    animation: blink 1.4s infinite;
    opacity: 0.5;
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

<style lang="scss">
/* v-html 注入内容无法命中 scoped，代码块工具栏单独写全局样式 */
.ai-panel .markdown-body {
  .ai-code-block__pre {
    position: relative;
    background: var(--bg-input);

    pre {
      margin: 0 !important;
      padding: 12px 16px 14px !important;
      border: none !important;
      border-radius: 0 !important;
      background: transparent !important;
      overflow-x: auto;
      user-select: text;
      -webkit-user-select: text;

      code {
        background: transparent !important;
        padding: 0 !important;
        border: none !important;
        color: var(--text-main);
        font-size: 11px;
        line-height: 1.55;
        font-family: var(--font-terminal);
        user-select: text;
        -webkit-user-select: text;
      }
    }
  }

  /* shell 块顶部留空：lang/工具栏悬浮，代码内容与图标区留出间距 */
  .ai-code-block--shell .ai-code-block__pre pre {
    padding-top: 42px !important;
  }

  .ai-code-block__lang {
    position: absolute;
    top: 10px;
    left: 12px;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 6px;
    border-radius: 4px;
    background: var(--accent-08);
    border: 1px solid var(--accent-15);
    color: var(--accent);
    font-size: 8px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    font-family: var(--font-terminal);
    pointer-events: none;
    user-select: none;
  }

  .ai-code-block__toolbar {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 1px;
    padding: 1px;
    border-radius: 6px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-30);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.22);
    opacity: 0;
    transition: opacity 0.18s ease;
    pointer-events: auto;
  }

  .ai-code-block__pre:hover .ai-code-block__toolbar,
  .ai-code-block__toolbar:focus-within {
    opacity: 1;
  }

  @media (hover: none) {
    .ai-code-block__toolbar {
      opacity: 1;
    }
  }

  .ai-code-btn {
    appearance: none;
    -webkit-appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    margin: 0;
    padding: 0;
    border: none !important;
    border-radius: 5px;
    background: transparent !important;
    background-color: transparent !important;
    color: var(--text-dim) !important;
    font-family: inherit;
    cursor: pointer;
    user-select: none;
    box-shadow: none;
    transition: background 0.15s ease, color 0.15s ease, transform 0.1s ease;

    i {
      font-size: 9px;
      pointer-events: none;
      color: inherit;
    }

    &:hover {
      background: var(--accent-10) !important;
      color: var(--accent) !important;
    }

    &:active {
      transform: scale(0.94);
    }

    &.ai-code-btn--run:hover {
      background: var(--success-10, rgba(16, 185, 129, 0.12)) !important;
      color: var(--success) !important;
    }
  }
}

.app-tooltip {
  position: fixed;
  z-index: 12000;
  box-sizing: border-box;
  width: max-content;
  max-width: 280px;
  padding: 7px 12px;
  border-radius: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: 0 8px 24px var(--shadow);
  color: var(--text-main);
  font-size: 12px;
  font-weight: 500;
  font-family: inherit;
  line-height: 1.5;
  letter-spacing: 0.02em;
  white-space: nowrap;
  pointer-events: none;
  user-select: none;
}

.ai-code-tip-fade-enter-active,
.ai-code-tip-fade-leave-active {
  transition: opacity 0.15s ease;
}

.ai-code-tip-fade-enter-from,
.ai-code-tip-fade-leave-to {
  opacity: 0;
}
</style>