<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Socket } from 'socket.io-client';
import type { MqttClient } from 'mqtt';
import { toast } from '../utils/toast.ts';
import AppSelect from './AppSelect.vue';
import HeaderComboInput from './HeaderComboInput.vue';
import ApiDebuggerCollections from './ApiDebuggerCollections.vue';
import ApiDebuggerEnvironments from './ApiDebuggerEnvironments.vue';
import ApiDebuggerHistory from './ApiDebuggerHistory.vue';
import ApiDebuggerDataMenu from './ApiDebuggerDataMenu.vue';
import ApiDebuggerSaveRequestDialog, { type SaveRequestPayload } from './ApiDebuggerSaveRequestDialog.vue';
import {
  type ApiTab,
  type DebugLogEntry,
  type HeaderRow,
  COMMON_HEADER_KEYS,
  createHeaderRow,
  createLog,
  formatBodyPreview,
  getHeaderValueSuggestions,
  headersToText,
  tryFormatJson,
} from '../utils/apiDebugger.ts';
import {
  type ApiCollection,
  type ApiEnvironment,
  type ApiDebuggerStore,
  type HistoryEntry,
  type RequestSnapshot,
  type SavedRequest,
  applyEnvironmentVariables,
  buildStore,
  createId,
  fromSnapshot,
  loadStore,
  pushHistory,
  resolveHeaders,
  saveStore,
  toMqttSnapshot,
  toSnapshot,
  toSocketIoSnapshot,
  toSseSnapshot,
  toWsSnapshot,
} from '../utils/apiDebuggerStorage.ts';

type ManagerView = 'request' | 'collections' | 'environments' | 'history';

const managerTabs: { id: ManagerView; label: string; icon: string }[] = [
  { id: 'request', label: 'Request', icon: 'fa-paper-plane' },
  { id: 'collections', label: 'Collections', icon: 'fa-folder' },
  { id: 'environments', label: 'Environments', icon: 'fa-sliders-h' },
  { id: 'history', label: 'History', icon: 'fa-history' },
];

const managerView = ref<ManagerView>('request');
const collections = ref<ApiCollection[]>([]);
const environments = ref<ApiEnvironment[]>([]);
const history = ref<HistoryEntry[]>([]);
const activeEnvId = ref<string | null>(null);

const tabs: { id: ApiTab; label: string; icon: string }[] = [
  { id: 'http', label: 'HTTP', icon: 'fa-globe' },
  { id: 'ws', label: 'WebSocket', icon: 'fa-plug' },
  { id: 'sse', label: 'SSE', icon: 'fa-stream' },
  { id: 'socketio', label: 'Socket.IO', icon: 'fa-broadcast-tower' },
  { id: 'mqtt', label: 'MQTT', icon: 'fa-satellite-dish' },
];

const activeTab = ref<ApiTab>('http');
const logs = ref<DebugLogEntry[]>([]);
const logBox = ref<HTMLElement | null>(null);

const appendLog = (direction: DebugLogEntry['direction'], message: string) => {
  logs.value.push(createLog(direction, message));
  if (logs.value.length > 500) logs.value.shift();
  nextTick(() => {
    if (logBox.value) logBox.value.scrollTop = logBox.value.scrollHeight;
  });
};

type CloseIntent = 'manual' | 'remote' | 'error' | 'unmount';

const logConnectionEnd = (protocol: string, intent: CloseIntent, detail?: string) => {
  if (intent === 'unmount') return;
  if (intent === 'manual') {
    appendLog('system', `${protocol} 已断开`);
    return;
  }
  if (intent === 'error') {
    appendLog('system', `${protocol} 连接失败${detail ? `: ${detail}` : ''}`);
    return;
  }
  appendLog('system', `${protocol} 连接已关闭${detail ? ` (${detail})` : ''}`);
};

const createCloseSession = () => ({ intent: null as CloseIntent | null });

const clearLogs = () => {
  logs.value = [];
};

// --- HTTP ---
const httpMethod = ref('GET');
const httpUrl = ref('https://httpbin.org/get');
const httpHeaders = ref<HeaderRow[]>([createHeaderRow('Accept', 'application/json')]);
const httpBody = ref('');
const httpBodyType = ref<'none' | 'json' | 'text' | 'form'>('none');
const httpLoading = ref(false);
const httpResponse = ref<{
  status: number;
  statusText: string;
  headers: Record<string, string>;
  body: string;
  elapsedMs: number;
} | null>(null);
const httpFormatJson = ref(true);

const httpMethods = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];

const httpMethodOptions = httpMethods.map((method) => ({ value: method, label: method }));

const bodyTypeOptions: { value: typeof httpBodyType.value; label: string }[] = [
  { value: 'none', label: 'None' },
  { value: 'json', label: 'JSON' },
  { value: 'text', label: 'Text' },
  { value: 'form', label: 'Form' },
];

const httpResponseHeadersText = computed(() => {
  if (!httpResponse.value) return '';
  return headersToText(httpResponse.value.headers);
});

const copyText = async (text: string) => {
  if (!text) {
    toast.warning('没有可复制的内容');
    return;
  }
  try {
    await navigator.clipboard.writeText(text);
    toast.success('已复制到剪贴板');
  } catch {
    toast.error('复制失败');
  }
};

const httpBodyPlaceholder = computed(() => {
  if (httpBodyType.value === 'form') return 'key=value\nfoo=bar';
  if (httpBodyType.value === 'json') return '{\n  "key": "value"\n}';
  return 'raw body';
});

const httpResponseBody = computed(() => {
  if (!httpResponse.value) return '';
  const body = httpResponse.value.body;
  if (!httpFormatJson.value) return formatBodyPreview(body);
  return formatBodyPreview(tryFormatJson(body));
});

const activeVariables = computed(() => {
  const env = environments.value.find((item) => item.id === activeEnvId.value) ?? environments.value[0];
  return env?.variables ?? [];
});

const activeEnvironmentName = computed(() => {
  const env = environments.value.find((item) => item.id === activeEnvId.value) ?? environments.value[0];
  return env?.name ?? 'Default';
});

const persistStore = async () => {
  await saveStore(buildStore(
    collections.value,
    environments.value,
    history.value,
    activeEnvId.value,
  ));
};

const applyStore = (store: ApiDebuggerStore) => {
  collections.value = store.collections;
  environments.value = store.environments.length
    ? store.environments
    : [{ id: createId(), name: 'Default', variables: [] }];
  history.value = store.history;
  activeEnvId.value = store.activeEnvId ?? environments.value[0]?.id ?? null;
};

const updateCollections = async (next: ApiCollection[]) => {
  collections.value = next;
  await persistStore();
};

const updateEnvironments = async (next: ApiEnvironment[]) => {
  environments.value = next;
  await persistStore();
};

const updateHistory = async (next: HistoryEntry[]) => {
  history.value = next;
  await persistStore();
};

const updateActiveEnvId = async (id: string | null) => {
  activeEnvId.value = id;
  await persistStore();
};

const handleImportedStore = (store: ApiDebuggerStore) => {
  applyStore(store);
};

const showSaveDialog = ref(false);
const saveTargetCollectionId = ref<string | undefined>();
const editRequestTarget = ref<{
  id: string;
  collectionId: string;
  name: string;
  description?: string;
} | null>(null);

const openSaveRequestDialog = (collectionId?: string) => {
  if (collections.value.length === 0) {
    toast.warning('请先在 Collections 中创建集合');
    managerView.value = 'collections';
    return;
  }
  editRequestTarget.value = null;
  saveTargetCollectionId.value = collectionId;
  showSaveDialog.value = true;
};

const openEditRequestDialog = (payload: { collectionId: string; request: SavedRequest }) => {
  editRequestTarget.value = {
    id: payload.request.id,
    collectionId: payload.collectionId,
    name: payload.request.name,
    description: payload.request.description,
  };
  saveTargetCollectionId.value = payload.collectionId;
  showSaveDialog.value = true;
};

const handleSaveRequest = async (payload: SaveRequestPayload) => {
  if (payload.requestId) {
    collections.value = collections.value.map((collection) => {
      if (collection.id !== payload.collectionId) return collection;
      return {
        ...collection,
        updatedAt: Date.now(),
        requests: collection.requests.map((request) =>
          request.id === payload.requestId
            ? { ...request, name: payload.name, description: payload.description }
            : request,
        ),
      };
    });
    toast.success('已更新请求信息');
  } else {
    collections.value = collections.value.map((collection) => {
      if (collection.id !== payload.collectionId) return collection;
      return {
        ...collection,
        updatedAt: Date.now(),
        requests: [
          {
            id: createId(),
            name: payload.name,
            description: payload.description,
            snapshot: structuredClone(currentSnapshot.value),
          },
          ...collection.requests,
        ],
      };
    });
    toast.success('已保存到集合');
  }
  await persistStore();
  showSaveDialog.value = false;
  editRequestTarget.value = null;
};

const loadRequestSnapshot = (snapshot: RequestSnapshot) => {
  const protocol = snapshot.protocol ?? 'http';
  switch (protocol) {
    case 'ws':
      wsUrl.value = snapshot.url;
      wsMessage.value = snapshot.message ?? '';
      activeTab.value = 'ws';
      break;
    case 'sse':
      sseUrl.value = snapshot.url;
      activeTab.value = 'sse';
      break;
    case 'socketio':
      sioUrl.value = snapshot.url;
      sioPath.value = snapshot.path ?? '/socket.io/';
      sioEvent.value = snapshot.event ?? 'message';
      sioPayload.value = snapshot.payload ?? '';
      activeTab.value = 'socketio';
      break;
    case 'mqtt':
      mqttUrl.value = snapshot.url;
      mqttClientId.value = snapshot.clientId ?? '';
      mqttUsername.value = snapshot.username ?? '';
      mqttPassword.value = snapshot.password ?? '';
      mqttSubTopic.value = snapshot.subTopic ?? '';
      mqttPubTopic.value = snapshot.pubTopic ?? '';
      mqttPubMessage.value = snapshot.pubMessage ?? '';
      activeTab.value = 'mqtt';
      break;
    default: {
      const data = fromSnapshot(snapshot);
      httpMethod.value = data.method;
      httpUrl.value = data.url;
      httpHeaders.value = data.headers.length ? data.headers : [createHeaderRow('Accept', 'application/json')];
      httpBody.value = data.body;
      httpBodyType.value = data.bodyType;
      httpResponse.value = null;
      activeTab.value = 'http';
      break;
    }
  }
  managerView.value = 'request';
  toast.success('已加载请求');
};

const buildHttpBody = (method: string, bodyType: typeof httpBodyType.value, rawBody: string) => {
  if (bodyType === 'none' || ['GET', 'HEAD'].includes(method)) return undefined;
  if (bodyType === 'form') {
    return rawBody
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        const idx = line.indexOf('=');
        if (idx === -1) return `${encodeURIComponent(line)}=`;
        return `${encodeURIComponent(line.slice(0, idx))}=${encodeURIComponent(line.slice(idx + 1))}`;
      })
      .join('&');
  }
  return rawBody;
};

const sendHttp = async () => {
  const resolvedUrl = applyEnvironmentVariables(httpUrl.value.trim(), activeVariables.value);
  if (!resolvedUrl) {
    toast.warning('请输入 URL');
    return;
  }
  httpLoading.value = true;
  httpResponse.value = null;
  const snapshot = currentSnapshot.value;
  try {
    const resolvedBodyRaw = applyEnvironmentVariables(httpBody.value, activeVariables.value);
    let body = buildHttpBody(httpMethod.value, httpBodyType.value, resolvedBodyRaw);

    const headers = resolveHeaders(httpHeaders.value, activeVariables.value);
    if (httpBodyType.value === 'json' && body && !headers['Content-Type']) {
      headers['Content-Type'] = 'application/json';
    }
    if (httpBodyType.value === 'form' && body && !headers['Content-Type']) {
      headers['Content-Type'] = 'application/x-www-form-urlencoded';
    }

    appendLog('out', `${httpMethod.value} ${resolvedUrl}`);
    const res = await invoke<{
      status: number;
      statusText: string;
      headers: Record<string, string>;
      body: string;
      elapsedMs: number;
    }>('send_http_request', {
      payload: {
        method: httpMethod.value,
        url: resolvedUrl,
        headers,
        body,
        timeoutMs: 30000,
      },
    });
    httpResponse.value = res;
    appendLog('in', `${res.status} ${res.statusText} (${res.elapsedMs}ms)`);
    history.value = await pushHistory({
      timestamp: Date.now(),
      snapshot,
      status: res.status,
      elapsedMs: res.elapsedMs,
    }, buildStore(collections.value, environments.value, history.value, activeEnvId.value));
  } catch (err) {
    appendLog('error', String(err));
    toast.error(String(err));
    history.value = await pushHistory({
      timestamp: Date.now(),
      snapshot,
    }, buildStore(collections.value, environments.value, history.value, activeEnvId.value));
  } finally {
    httpLoading.value = false;
  }
};

// --- WebSocket ---
const wsUrl = ref('wss://echo.websocket.events');
const wsConnected = ref(false);
const wsActive = ref(false);
const wsMessage = ref('');
const wsSession = createCloseSession();
let wsClient: WebSocket | null = null;

const resetWsState = () => {
  wsClient = null;
  wsConnected.value = false;
  wsActive.value = false;
  wsSession.intent = null;
};

const finalizeWs = (intent: CloseIntent, detail?: string) => {
  if (!wsActive.value) return;
  logConnectionEnd('WebSocket', intent, detail);
  resetWsState();
};

const connectWs = () => {
  if (wsActive.value) return;
  if (!wsUrl.value.trim()) {
    toast.warning('请输入 WebSocket URL');
    return;
  }
  try {
    wsSession.intent = null;
    wsClient = new WebSocket(wsUrl.value.trim());
    wsActive.value = true;
    wsClient.onopen = () => {
      wsConnected.value = true;
      appendLog('system', `WebSocket connected: ${wsUrl.value}`);
    };
    wsClient.onmessage = (ev) => {
      appendLog('in', typeof ev.data === 'string' ? ev.data : '[binary data]');
    };
    wsClient.onerror = () => {
      if (!wsSession.intent && wsActive.value && !wsConnected.value) {
        appendLog('error', 'WebSocket 连接出错');
      }
    };
    wsClient.onclose = (ev) => {
      const intent = wsSession.intent
        ?? (wsConnected.value ? 'remote' : 'error');
      finalizeWs(intent, String(ev.code));
    };
  } catch (err) {
    finalizeWs('error', String(err));
  }
};

const disconnectWs = (intent: CloseIntent = 'manual') => {
  if (!wsActive.value) return;
  wsSession.intent = intent;
  const client = wsClient;
  client?.close();
  if (!client || client.readyState === WebSocket.CLOSED) {
    finalizeWs(intent);
  }
};

const sendWs = () => {
  if (!wsClient || wsClient.readyState !== WebSocket.OPEN) {
    toast.warning('WebSocket 未连接');
    return;
  }
  wsClient.send(wsMessage.value);
  appendLog('out', wsMessage.value);
  wsMessage.value = '';
};

// --- SSE ---
const sseUrl = ref('https://sse.dev/test');
const sseConnected = ref(false);
const sseActive = ref(false);
const sseSession = createCloseSession();
let sseSource: EventSource | null = null;

const resetSseState = () => {
  sseSource = null;
  sseConnected.value = false;
  sseActive.value = false;
  sseSession.intent = null;
};

const finalizeSse = (intent: CloseIntent, detail?: string) => {
  if (!sseActive.value) return;
  logConnectionEnd('SSE', intent, detail);
  resetSseState();
};

const connectSse = () => {
  if (sseActive.value) return;
  if (!sseUrl.value.trim()) {
    toast.warning('请输入 SSE URL');
    return;
  }
  try {
    sseSession.intent = null;
    sseSource = new EventSource(sseUrl.value.trim());
    sseActive.value = true;
    sseSource.onopen = () => {
      sseConnected.value = true;
      appendLog('system', `SSE connected: ${sseUrl.value}`);
    };
    sseSource.onmessage = (ev) => appendLog('in', ev.data);
    sseSource.onerror = () => {
      if (sseSession.intent) return;
      if (sseSource?.readyState === EventSource.CLOSED) {
        finalizeSse(sseConnected.value ? 'remote' : 'error');
        return;
      }
      if (sseActive.value && !sseConnected.value) {
        appendLog('error', 'SSE 连接出错');
      }
    };
    sseSource.addEventListener('ping', (ev: Event) => {
      const msg = ev as MessageEvent;
      appendLog('in', `[ping] ${msg.data ?? ''}`);
    });
  } catch (err) {
    finalizeSse('error', String(err));
  }
};

const disconnectSse = (intent: CloseIntent = 'manual') => {
  if (!sseActive.value) return;
  sseSession.intent = intent;
  sseSource?.close();
  finalizeSse(intent);
};

// --- Socket.IO ---
const sioUrl = ref('http://localhost:3000');
const sioPath = ref('/socket.io/');
const sioEvent = ref('message');
const sioPayload = ref('{"hello":"world"}');
const sioConnected = ref(false);
const sioActive = ref(false);
const sioSession = createCloseSession();
let sioClient: Socket | null = null;

const finalizeSio = (intent: CloseIntent, detail?: string) => {
  if (!sioActive.value) return;
  logConnectionEnd('Socket.IO', intent, detail);
  if (sioClient) {
    sioClient.removeAllListeners();
  }
  sioClient = null;
  sioConnected.value = false;
  sioActive.value = false;
  sioSession.intent = null;
};

const connectSio = async () => {
  if (sioActive.value) return;
  if (!sioUrl.value.trim()) {
    toast.warning('请输入 Socket.IO 服务地址');
    return;
  }
  try {
    const { io } = await import('socket.io-client');
    sioSession.intent = null;
    sioActive.value = true;
    sioClient = io(sioUrl.value.trim(), {
      path: sioPath.value.trim() || '/socket.io/',
      transports: ['websocket', 'polling'],
      reconnection: false,
    });
    sioClient.on('connect', () => {
      sioConnected.value = true;
      appendLog('system', `Socket.IO connected (${sioClient?.id})`);
    });
    sioClient.on('disconnect', (reason) => {
      const intent = sioSession.intent
        ?? (reason === 'io client disconnect' ? 'manual' : 'remote');
      finalizeSio(intent, reason);
    });
    sioClient.on('connect_error', (err) => {
      if (sioSession.intent) return;
      appendLog('error', err.message);
    });
    sioClient.onAny((event, ...args) => {
      appendLog('in', `${event}: ${JSON.stringify(args)}`);
    });
  } catch (err) {
    finalizeSio('error', String(err));
  }
};

const disconnectSio = (intent: CloseIntent = 'manual') => {
  if (!sioActive.value) return;
  sioSession.intent = intent;
  if (sioClient?.connected) {
    sioClient.disconnect();
    return;
  }
  if (sioClient) {
    sioClient.close();
  }
  finalizeSio(intent);
};

const emitSio = () => {
  if (!sioClient?.connected) {
    toast.warning('Socket.IO 未连接');
    return;
  }
  const event = sioEvent.value.trim() || 'message';
  let data: unknown = sioPayload.value;
  try {
    data = JSON.parse(sioPayload.value);
  } catch {
    // keep raw string
  }
  sioClient.emit(event, data);
  appendLog('out', `emit ${event}: ${typeof data === 'string' ? data : JSON.stringify(data)}`);
};

// --- MQTT ---
const mqttUrl = ref('ws://broker.emqx.io:8083/mqtt');
const mqttClientId = ref(`hiphup-${Math.random().toString(36).slice(2, 8)}`);
const mqttUsername = ref('');
const mqttPassword = ref('');
const mqttSubTopic = ref('test/topic');
const mqttPubTopic = ref('test/topic');
const mqttPubMessage = ref('hello mqtt');
const mqttConnected = ref(false);
const mqttActive = ref(false);
const mqttSession = createCloseSession();
let mqttClient: MqttClient | null = null;

const currentSnapshot = computed((): RequestSnapshot => {
  switch (activeTab.value) {
    case 'ws':
      return toWsSnapshot(wsUrl.value, wsMessage.value);
    case 'sse':
      return toSseSnapshot(sseUrl.value);
    case 'socketio':
      return toSocketIoSnapshot(sioUrl.value, sioPath.value, sioEvent.value, sioPayload.value);
    case 'mqtt':
      return toMqttSnapshot(
        mqttUrl.value,
        mqttClientId.value,
        mqttUsername.value,
        mqttPassword.value,
        mqttSubTopic.value,
        mqttPubTopic.value,
        mqttPubMessage.value,
      );
    default:
      return toSnapshot(httpMethod.value, httpUrl.value, httpHeaders.value, httpBody.value, httpBodyType.value);
  }
});

const resetMqttState = () => {
  mqttClient = null;
  mqttConnected.value = false;
  mqttActive.value = false;
  mqttSession.intent = null;
};

const finalizeMqtt = (intent: CloseIntent, detail?: string) => {
  if (!mqttActive.value) return;
  logConnectionEnd('MQTT', intent, detail);
  resetMqttState();
};

const connectMqtt = async () => {
  if (mqttActive.value) return;
  if (!mqttUrl.value.trim()) {
    toast.warning('请输入 MQTT Broker 地址');
    return;
  }
  try {
    const mqtt = (await import('mqtt')).default;
    mqttSession.intent = null;
    mqttActive.value = true;
    mqttClient = mqtt.connect(mqttUrl.value.trim(), {
      clientId: mqttClientId.value.trim() || undefined,
      username: mqttUsername.value.trim() || undefined,
      password: mqttPassword.value || undefined,
      clean: true,
      reconnectPeriod: 0,
    });
    mqttClient.on('connect', () => {
      mqttConnected.value = true;
      appendLog('system', 'MQTT connected');
      if (mqttSubTopic.value.trim()) {
        mqttClient?.subscribe(mqttSubTopic.value.trim(), (err) => {
          if (err) appendLog('error', `Subscribe failed: ${err.message}`);
          else appendLog('system', `Subscribed: ${mqttSubTopic.value.trim()}`);
        });
      }
    });
    mqttClient.on('message', (topic, payload) => {
      appendLog('in', `[${topic}] ${payload.toString()}`);
    });
    mqttClient.on('error', (err) => {
      if (mqttSession.intent) return;
      appendLog('error', err.message);
    });
    mqttClient.on('close', () => {
      const intent = mqttSession.intent
        ?? (mqttConnected.value ? 'remote' : 'error');
      finalizeMqtt(intent);
    });
  } catch (err) {
    finalizeMqtt('error', String(err));
  }
};

const disconnectMqtt = (intent: CloseIntent = 'manual') => {
  if (!mqttActive.value) return;
  mqttSession.intent = intent;
  mqttClient?.end(true);
  if (!mqttClient?.connected) {
    finalizeMqtt(intent);
  }
};

const publishMqtt = () => {
  if (!mqttClient?.connected) {
    toast.warning('MQTT 未连接');
    return;
  }
  const topic = mqttPubTopic.value.trim();
  if (!topic) {
    toast.warning('请输入发布 Topic');
    return;
  }
  mqttClient.publish(topic, mqttPubMessage.value, {}, (err) => {
    if (err) appendLog('error', err.message);
    else appendLog('out', `publish ${topic}: ${mqttPubMessage.value}`);
  });
};

const addHeaderRow = () => {
  httpHeaders.value.push(createHeaderRow());
};

const removeHeaderRow = (id: string) => {
  httpHeaders.value = httpHeaders.value.filter(r => r.id !== id);
};

const onHeaderKeySelect = (row: HeaderRow, key: string) => {
  row.key = key;
  if (!row.value.trim()) {
    const suggestions = getHeaderValueSuggestions(key);
    if (suggestions.length === 1) {
      row.value = suggestions[0];
    }
  }
};

onMounted(async () => {
  applyStore(await loadStore());
});

onUnmounted(() => {
  disconnectWs('unmount');
  disconnectSse('unmount');
  disconnectSio('unmount');
  disconnectMqtt('unmount');
});
</script>

<template>
  <div class="api-debugger">
    <header class="panel-header">
      <div class="title">
        <i class="fas fa-paper-plane"></i>
        <span>API 调试</span>
      </div>
    </header>

    <nav class="meta-bar">
      <button
        v-for="tab in managerTabs"
        :key="tab.id"
        type="button"
        class="meta-btn"
        :class="{ active: managerView === tab.id }"
        @click="managerView = tab.id"
      >
        <i class="fas" :class="tab.icon"></i>
        <span>{{ tab.label }}</span>
      </button>
      <ApiDebuggerDataMenu @imported="handleImportedStore" />
    </nav>

    <nav v-if="managerView === 'request'" class="tab-bar">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        type="button"
        class="tab-btn"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >
        <i class="fas" :class="tab.icon"></i>
        <span>{{ tab.label }}</span>
      </button>
    </nav>

    <div class="panel-body custom-scrollbar">
      <ApiDebuggerCollections
        v-if="managerView === 'collections'"
        :collections="collections"
        :current-snapshot="currentSnapshot"
        @update="updateCollections"
        @load="loadRequestSnapshot"
        @open-save="openSaveRequestDialog"
        @open-edit="openEditRequestDialog"
      />

      <ApiDebuggerEnvironments
        v-else-if="managerView === 'environments'"
        :environments="environments"
        :active-env-id="activeEnvId"
        @update="updateEnvironments"
        @update:active-env-id="updateActiveEnvId"
      />

      <ApiDebuggerHistory
        v-else-if="managerView === 'history'"
        :history="history"
        @update="updateHistory"
        @load="loadRequestSnapshot"
      />

      <template v-else>
      <!-- HTTP -->
      <section v-show="activeTab === 'http'" class="tab-panel">
        <div v-if="activeVariables.some(v => v.enabled && v.key)" class="env-banner">
          <i class="fas fa-sliders-h"></i>
          <span>环境: {{ activeEnvironmentName }}</span>
        </div>
        <div class="request-row">
          <div class="method-select-wrap">
            <AppSelect v-model="httpMethod" :options="httpMethodOptions" />
          </div>
          <input v-model="httpUrl" class="url-input" placeholder="https://api.example.com/path" />
          <button type="button" class="btn-primary" :disabled="httpLoading" @click="sendHttp">
            <i class="fas" :class="httpLoading ? 'fa-circle-notch fa-spin' : 'fa-paper-plane'"></i>
            Send
          </button>
          <Tooltip text="保存到集合" placement="bottom">
            <button type="button" class="btn-ghost" @click="openSaveRequestDialog()">
              <i class="fas fa-folder-plus"></i>
            </button>
          </Tooltip>
        </div>

        <label class="field-label">Headers</label>
        <div class="kv-list">
          <div class="kv-row kv-row--head">
            <span class="kv-head-cell kv-head-cell--check">启用</span>
            <span class="kv-head-cell">Key</span>
            <span class="kv-head-cell">Value</span>
            <span class="kv-head-cell kv-head-cell--action"></span>
          </div>
          <div v-for="row in httpHeaders" :key="row.id" class="kv-row" :class="{ 'is-disabled': !row.enabled }">
            <label class="header-toggle" :title="row.enabled ? '禁用此 Header' : '启用此 Header'">
              <input v-model="row.enabled" type="checkbox" class="header-toggle__input" />
              <span class="header-toggle__box">
                <i class="fas fa-check header-toggle__icon"></i>
              </span>
            </label>
            <HeaderComboInput
              v-model="row.key"
              :options="COMMON_HEADER_KEYS"
              placeholder="Key"
              :disabled="!row.enabled"
              @select="onHeaderKeySelect(row, $event)"
            />
            <HeaderComboInput
              v-model="row.value"
              :options="getHeaderValueSuggestions(row.key)"
              placeholder="Value"
              :disabled="!row.enabled"
            />
            <button type="button" class="icon-btn" @click="removeHeaderRow(row.id)">
              <i class="fas fa-times"></i>
            </button>
          </div>
          <button type="button" class="btn-ghost" @click="addHeaderRow">
            <i class="fas fa-plus"></i> Add header
          </button>
        </div>

        <label class="field-label">Body</label>
        <div class="segmented-control">
          <label v-for="option in bodyTypeOptions" :key="option.value" class="segment-item">
            <input v-model="httpBodyType" type="radio" :value="option.value" class="segment-item__input" />
            <span class="segment-item__label">{{ option.label }}</span>
          </label>
        </div>
        <textarea
          v-if="httpBodyType !== 'none'"
          v-model="httpBody"
          class="code-area"
          :placeholder="httpBodyPlaceholder"
        ></textarea>

        <div v-if="httpResponse" class="response-box">
          <div class="response-meta">
            <span class="status-chip" :class="{ ok: httpResponse.status < 400, err: httpResponse.status >= 400 }">
              {{ httpResponse.status }} {{ httpResponse.statusText }}
            </span>
            <span class="meta-text">{{ httpResponse.elapsedMs }} ms</span>
            <label class="option-toggle">
              <input v-model="httpFormatJson" type="checkbox" class="option-toggle__input" />
              <span class="option-toggle__box">
                <i class="fas fa-check option-toggle__icon"></i>
              </span>
              <span class="option-toggle__text">Format JSON</span>
            </label>
          </div>
          <div class="section-head">
            <span class="sub-label">Response Headers</span>
            <button type="button" class="copy-btn" title="复制 Headers" @click="copyText(httpResponseHeadersText)">
              <i class="fas fa-copy"></i>
            </button>
          </div>
          <pre class="code-preview selectable">{{ httpResponseHeadersText }}</pre>
          <div class="section-head">
            <span class="sub-label">Response Body</span>
            <button type="button" class="copy-btn" title="复制 Body" @click="copyText(httpResponseBody)">
              <i class="fas fa-copy"></i>
            </button>
          </div>
          <pre class="code-preview body selectable">{{ httpResponseBody }}</pre>
        </div>
      </section>

      <!-- WebSocket -->
      <section v-show="activeTab === 'ws'" class="tab-panel">
        <div class="request-row">
          <input v-model="wsUrl" class="url-input" placeholder="wss://host/path" />
          <button v-if="!wsActive" type="button" class="btn-primary" @click="connectWs">Connect</button>
          <button v-else type="button" class="btn-danger" @click="disconnectWs">
            {{ wsConnected ? 'Disconnect' : 'Cancel' }}
          </button>
          <Tooltip text="保存到集合" placement="bottom">
            <button type="button" class="btn-ghost" @click="openSaveRequestDialog()">
              <i class="fas fa-folder-plus"></i>
            </button>
          </Tooltip>
        </div>
        <div class="send-row">
          <input v-model="wsMessage" class="url-input" placeholder="Message to send" @keyup.enter="sendWs" />
          <button type="button" class="btn-primary" :disabled="!wsConnected" @click="sendWs">Send</button>
        </div>
      </section>

      <!-- SSE -->
      <section v-show="activeTab === 'sse'" class="tab-panel">
        <div class="request-row">
          <input v-model="sseUrl" class="url-input" placeholder="https://host/events" />
          <button v-if="!sseActive" type="button" class="btn-primary" @click="connectSse">Connect</button>
          <button v-else type="button" class="btn-danger" @click="disconnectSse">
            {{ sseConnected ? 'Disconnect' : 'Cancel' }}
          </button>
          <Tooltip text="保存到集合" placement="bottom">
            <button type="button" class="btn-ghost" @click="openSaveRequestDialog()">
              <i class="fas fa-folder-plus"></i>
            </button>
          </Tooltip>
        </div>
        <p class="hint">浏览器 EventSource 不支持自定义 Header；跨域需服务端允许 CORS。</p>
      </section>

      <!-- Socket.IO -->
      <section v-show="activeTab === 'socketio'" class="tab-panel">
        <div class="field-grid">
          <label class="field-label">Server URL</label>
          <input v-model="sioUrl" class="url-input full" placeholder="http://localhost:3000" />
          <label class="field-label">Path</label>
          <input v-model="sioPath" class="url-input full" placeholder="/socket.io/" />
        </div>
        <div class="request-row">
          <button v-if="!sioActive" type="button" class="btn-primary" @click="connectSio">Connect</button>
          <button v-else type="button" class="btn-danger" @click="disconnectSio">
            {{ sioConnected ? 'Disconnect' : 'Cancel' }}
          </button>
          <Tooltip text="保存到集合" placement="bottom">
            <button type="button" class="btn-ghost" @click="openSaveRequestDialog()">
              <i class="fas fa-folder-plus"></i>
            </button>
          </Tooltip>
        </div>
        <div class="send-row">
          <input v-model="sioEvent" class="field-input" placeholder="event" />
          <input v-model="sioPayload" class="url-input" placeholder='{"key":"value"}' />
          <button type="button" class="btn-primary" :disabled="!sioConnected" @click="emitSio">Emit</button>
        </div>
      </section>

      <!-- MQTT -->
      <section v-show="activeTab === 'mqtt'" class="tab-panel">
        <div class="field-grid">
          <label class="field-label">Broker (WebSocket)</label>
          <input v-model="mqttUrl" class="url-input full" placeholder="ws://broker:8083/mqtt" />
          <label class="field-label">Client ID</label>
          <input v-model="mqttClientId" class="url-input full" />
          <label class="field-label">Username / Password</label>
          <div class="kv-row">
            <input v-model="mqttUsername" class="kv-input" placeholder="username" />
            <input v-model="mqttPassword" class="kv-input" type="password" placeholder="password" />
          </div>
          <label class="field-label">Subscribe Topic</label>
          <input v-model="mqttSubTopic" class="url-input full" placeholder="test/topic" />
        </div>
        <div class="request-row">
          <button v-if="!mqttActive" type="button" class="btn-primary" @click="connectMqtt">Connect</button>
          <button v-else type="button" class="btn-danger" @click="disconnectMqtt">
            {{ mqttConnected ? 'Disconnect' : 'Cancel' }}
          </button>
          <Tooltip text="保存到集合" placement="bottom">
            <button type="button" class="btn-ghost" @click="openSaveRequestDialog()">
              <i class="fas fa-folder-plus"></i>
            </button>
          </Tooltip>
        </div>
        <div class="send-row">
          <input v-model="mqttPubTopic" class="field-input" placeholder="topic" />
          <input v-model="mqttPubMessage" class="url-input" placeholder="message" />
          <button type="button" class="btn-primary" :disabled="!mqttConnected" @click="publishMqtt">Publish</button>
        </div>
      </section>

      <!-- Shared log -->
      <section v-if="activeTab !== 'http' || logs.length" class="log-section">
        <div class="log-header">
          <span>Console</span>
          <button type="button" class="btn-ghost" @click="clearLogs">Clear</button>
        </div>
        <div ref="logBox" class="log-box custom-scrollbar selectable">
          <div v-for="entry in logs" :key="entry.id" class="log-line" :class="entry.direction">
            <span class="log-time">{{ entry.time }}</span>
            <span class="log-msg selectable">{{ entry.message }}</span>
          </div>
          <div v-if="!logs.length" class="log-empty">No messages yet</div>
        </div>
      </section>
      </template>
    </div>

    <ApiDebuggerSaveRequestDialog
      :visible="showSaveDialog"
      :collections="collections"
      :snapshot="currentSnapshot"
      :initial-collection-id="saveTargetCollectionId"
      :edit-request="editRequestTarget"
      @close="showSaveDialog = false"
      @save="handleSaveRequest"
    />
  </div>
</template>

<style lang="scss" scoped>
.api-debugger {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  overflow: hidden;
  background: var(--bg-primary);
  color: var(--text-main);
}

.panel-header {
  padding: 14px 16px 8px;
  border-bottom: 1px solid var(--border-30);

  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 700;

    i { color: var(--accent); }
  }
}

.meta-bar {
  display: flex;
  gap: 4px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border-30);
  overflow-x: auto;
  flex-shrink: 0;
  align-items: center;

  &::-webkit-scrollbar {
    height: 0;
  }

  .data-menu {
    margin-left: auto;
  }
}

.meta-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text-dim);
  font-size: 10px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;

  i { font-size: 10px; }

  &.active {
    background: var(--accent-10);
    border-color: var(--accent-20);
    color: var(--accent);
  }

  &:hover:not(.active) {
    background: var(--bg-input);
    color: var(--text-main);
  }
}

.env-banner {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  align-self: flex-start;
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--accent-08);
  border: 1px solid var(--accent-15);
  color: var(--accent);
  font-size: 10px;
  font-weight: 600;

  i { font-size: 9px; }
}

.tab-bar {
  display: flex;
  gap: 4px;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border-30);
  overflow-x: auto;
  flex-shrink: 0;
}

.tab-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text-dim);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;

  i { font-size: 10px; }

  &.active {
    background: var(--accent-10);
    border-color: var(--accent-20);
    color: var(--accent);
  }

  &:hover:not(.active) {
    background: var(--bg-input);
    color: var(--text-main);
  }
}

.panel-body {
  flex: 1;
  min-width: 0;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 12px;
}

.tab-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  width: 100%;
}

.request-row,
.send-row {
  display: flex;
  gap: 6px;
  align-items: center;
  min-width: 0;
  width: 100%;
}

.send-row {
  flex-wrap: wrap;

  .field-input {
    flex: 0 0 88px;
  }

  .url-input {
    flex: 1 1 140px;
    min-width: 0;
  }

  .btn-primary,
  .btn-danger {
    flex: 0 0 auto;
  }
}

.method-select-wrap {
  width: 96px;
  flex-shrink: 0;

  :deep(.app-select__trigger) {
    height: 32px;
    padding: 0 28px 0 10px;
    border-radius: 7px;
    border-color: var(--border-30);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-terminal);
    letter-spacing: 0.02em;
  }

  :deep(.app-select__arrow) {
    right: 9px;
    font-size: 9px;
  }

  :deep(.app-select.open .app-select__trigger) {
    box-shadow: 0 0 0 2px var(--accent-15);
  }
}

.field-input {
  width: 88px;
  flex-shrink: 0;
  height: 32px;
  padding: 0 10px;
  border-radius: 7px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 11px;
  font-weight: 600;
}

.url-input {
  flex: 1;
  min-width: 0;
  height: 32px;
  padding: 0 10px;
  border-radius: 7px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
  color: var(--accent);
  font-family: var(--font-terminal);
  font-size: 11px;
  user-select: text;
  -webkit-user-select: text;
  box-sizing: border-box;

  &.full {
    flex: none;
    width: 100%;
    max-width: 100%;
  }
}

.field-label {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-dim);
}

.field-grid {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  width: 100%;
}

.kv-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  width: 100%;
}

.kv-row {
  display: flex;
  gap: 6px;
  align-items: center;
  min-width: 0;
  width: 100%;

  &--head {
    padding: 0 2px;
    margin-bottom: -2px;
  }

  &.is-disabled {
    :deep(.header-combo__input) {
      opacity: 0.55;
    }
  }
}

.kv-head-cell {
  font-size: 9px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-dim);
  flex: 1;
  min-width: 0;

  &--check {
    flex: 0 0 32px;
    white-space: nowrap;
    text-align: center;
  }

  &--action {
    flex: 0 0 28px;
  }
}

.header-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex: 0 0 32px;

  &__input {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  &__box {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    border: 1.5px solid var(--border-30);
    background: var(--bg-input);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s;
  }

  &__icon {
    font-size: 9px;
    color: var(--bg-primary);
    opacity: 0;
    transform: scale(0.6);
    transition: opacity 0.15s, transform 0.15s;
  }

  &__input:checked + &__box {
    background: var(--accent);
    border-color: var(--accent);

    .header-toggle__icon {
      opacity: 1;
      transform: scale(1);
    }
  }

  &:hover &__box {
    border-color: var(--accent-30);
  }
}

.kv-input {
  flex: 1;
  min-width: 0;
  height: 30px;
  padding: 0 8px;
  border-radius: 6px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 11px;
  box-sizing: border-box;
  user-select: text;
  -webkit-user-select: text;
}

.segmented-control {
  display: inline-flex;
  align-self: flex-start;
  gap: 2px;
  padding: 3px;
  border-radius: 8px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
}

.segment-item {
  position: relative;
  cursor: pointer;

  &__input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }

  &__label {
    display: block;
    min-width: 44px;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    text-align: center;
    color: var(--text-dim);
    transition: background 0.15s, color 0.15s, box-shadow 0.15s;
    user-select: none;
  }

  &__input:checked + &__label {
    background: var(--accent);
    color: #fff;
    box-shadow: 0 1px 4px var(--accent-20);
  }

  &:hover &__input:not(:checked) + &__label {
    color: var(--text-main);
    background: var(--bg-secondary);
  }
}

.code-area {
  min-height: 100px;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid var(--border-30);
  background: var(--bg-input);
  color: var(--text-main);
  font-family: var(--font-terminal);
  font-size: 11px;
  line-height: 1.5;
  resize: vertical;
  user-select: text;
  -webkit-user-select: text;
}

.btn-primary,
.btn-danger,
.btn-ghost {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  height: 32px;
  padding: 0 12px;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  flex-shrink: 0;
}

.btn-primary {
  background: var(--accent);
  color: #fff;

  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

.btn-danger {
  background: var(--error-15);
  border-color: var(--error-30);
  color: var(--error);
}

.btn-ghost {
  background: transparent;
  border-color: var(--border-30);
  color: var(--text-dim);
  height: 28px;
}

.icon-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: var(--bg-secondary);
  color: var(--text-dim);
  cursor: pointer;

  &:hover { color: var(--error); }
}

.response-box {
  margin-top: 8px;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid var(--border-30);
  background: var(--bg-secondary);
}

.response-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 8px;
}

.status-chip {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 700;

  &.ok {
    background: color-mix(in srgb, var(--success) 15%, transparent);
    color: var(--success);
  }

  &.err {
    background: var(--error-15);
    color: var(--error);
  }
}

.meta-text {
  font-size: 10px;
  color: var(--text-dim);
}

.option-toggle {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  user-select: none;

  &__input {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  &__box {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1.5px solid var(--border-30);
    background: var(--bg-input);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s, border-color 0.2s;
  }

  &__icon {
    font-size: 8px;
    color: var(--bg-primary);
    opacity: 0;
    transform: scale(0.6);
    transition: opacity 0.15s, transform 0.15s;
  }

  &__text {
    font-size: 10px;
    color: var(--text-dim);
  }

  &__input:checked + &__box {
    background: var(--accent);
    border-color: var(--accent);

    .option-toggle__icon {
      opacity: 1;
      transform: scale(1);
    }
  }

  &:hover &__box {
    border-color: var(--accent-30);
  }
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 6px 0 4px;
}

.copy-btn {
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid var(--border-30);
  border-radius: 5px;
  background: var(--bg-input);
  color: var(--text-dim);
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s, border-color 0.15s, background 0.15s;

  &:hover {
    color: var(--accent);
    border-color: var(--accent-30);
    background: var(--accent-08);
  }
}

.sub-label {
  font-size: 10px;
  color: var(--text-dim);
  margin: 0;
}

.code-preview {
  margin: 0;
  padding: 8px;
  border-radius: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border-30);
  font-family: var(--font-terminal);
  font-size: 10px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 160px;
  overflow: auto;
  cursor: text;

  &.body { max-height: 240px; }

  &.selectable {
    user-select: text;
    -webkit-user-select: text;
  }
}

.hint {
  margin: 0;
  font-size: 10px;
  color: var(--text-dim);
  line-height: 1.4;
}

.log-section {
  margin-top: 12px;
  border-top: 1px solid var(--border-30);
  padding-top: 10px;
  min-width: 0;
  width: 100%;
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
}

.log-box {
  max-height: 180px;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 8px;
  border-radius: 8px;
  background: var(--bg-input);
  border: 1px solid var(--border-30);
  font-family: var(--font-terminal);
  font-size: 10px;
  cursor: text;
  min-width: 0;

  &.selectable {
    user-select: text;
    -webkit-user-select: text;
  }
}

.log-line {
  display: flex;
  gap: 8px;
  padding: 2px 0;
  line-height: 1.4;
  min-width: 0;

  &.in { color: var(--success); }
  &.out { color: var(--accent); }
  &.system { color: var(--text-dim); }
  &.error { color: var(--error); }
}

.log-time {
  flex-shrink: 0;
  opacity: 0.6;
}

.log-msg {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.log-empty {
  color: var(--text-dim);
  opacity: 0.7;
}
</style>
