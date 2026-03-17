<script setup lang="ts">
import { ref, onMounted, computed, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

// --- 接口定义 ---
interface ChatMessage {
  content: string;
  fromMe: boolean;
  type: 'text' | 'file';
  time: string;
  isSearchHit?: boolean; // 搜索高亮标记
}

interface Peer {
  id: string;
  nickname?: string;
  lastMsg?: string;
  unreadCount: number;
}

// --- 状态定义 ---
const myShortId = ref('Locating...');
const peers = ref<Peer[]>([]);
const currentPeer = ref<Peer | null>(null);
const inputMsg = ref('');
const messageBox = ref<HTMLElement | null>(null);

// 搜索与备注相关状态
const peerSearchQuery = ref('');      // 邻居列表搜索
const msgSearchQuery = ref('');       // 消息内容搜索
const remarks = ref<Record<string, string>>({}); // PeerID -> Remark 映射
const isSearchingMsg = ref(false);    // 是否处于消息搜索模式

/**
 * 消息结构隔离存储
 */
const allChatHistories = ref<Record<string, ChatMessage[]>>({});

// --- 计算属性 ---

// 1. 过滤后的邻居列表（支持 ID 或 备注搜索）
const filteredPeers = computed(() => {
  const query = peerSearchQuery.value.toLowerCase();
  return peers.value.filter(p => {
    const name = (remarks.value[p.id] || '匿名邻居').toLowerCase();
    return name.includes(query) || p.id.toLowerCase().includes(query);
  });
});

// 2. 当前显示的聊天记录
const currentChatHistory = computed(() => {
  if (!currentPeer.value) return [];
  return allChatHistories.value[currentPeer.value.id] || [];
});

// --- 核心逻辑 ---

const scrollToBottom = async (force = false) => {
  await nextTick();
  if (messageBox.value) {
    const { scrollTop, scrollHeight, clientHeight } = messageBox.value;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 150;
    if (force || isAtBottom) {
      messageBox.value.scrollTo({ top: scrollHeight, behavior: 'smooth' });
    }
  }
};

const pushMessage = (peerId: string, msg: { content: string, fromMe: boolean, type: 'text' | 'file' }, timestamp?: number) => {
  if (!allChatHistories.value[peerId]) allChatHistories.value[peerId] = [];

  const timeStr = timestamp
      ? new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      : new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  allChatHistories.value[peerId].push({ ...msg, time: timeStr });

  const peer = peers.value.find(p => p.id === peerId);
  if (peer) {
    peer.lastMsg = msg.type === 'file' ? '[文件]' + msg.content : msg.content;
    if (!msg.fromMe && currentPeer.value?.id !== peerId) {
      peer.unreadCount++;
    }
  }
  if (!isSearchingMsg.value) scrollToBottom(msg.fromMe);
};

// --- API 交互 ---

// 切换联系人
const selectPeer = async (peer: Peer) => {
  currentPeer.value = peer;
  peer.unreadCount = 0;
  msgSearchQuery.value = '';
  isSearchingMsg.value = false;

  try {
    const history: any[] = await invoke('get_p2p_messages', { peerId: peer.id });
    allChatHistories.value[peer.id] = history.map(m => ({
      content: m.content,
      fromMe: m.direction === 'send',
      type: m.msg_type,
      time: new Date(m.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    }));
  } catch (err) {
    console.error("加载历史失败:", err);
  }
  scrollToBottom(true);
};

// 修改备注
const editRemark = async (peerId: string) => {
  const currentRemark = remarks.value[peerId] || '';
  const newRemark = prompt('设置邻居备注:', currentRemark);
  if (newRemark !== null) {
    try {
      await invoke('set_p2p_remark', { peerId, remark: newRemark });
      remarks.value[peerId] = newRemark;
    } catch (err) {
      console.error("备注设置失败:", err);
    }
  }
};

// 搜索消息
const execSearch = async () => {
  if (!currentPeer.value) return;
  if (!msgSearchQuery.value.trim()) {
    isSearchingMsg.value = false;
    // 重新加载完整历史
    selectPeer(currentPeer.value);
    return;
  }

  isSearchingMsg.value = true;
  try {
    const results: any[] = await invoke('search_p2p_messages', {
      peerId: currentPeer.value.id,
      keyword: msgSearchQuery.value
    });
    allChatHistories.value[currentPeer.value.id] = results.map(m => ({
      content: m.content,
      fromMe: m.direction === 'send',
      type: m.msg_type,
      time: new Date(m.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      isSearchHit: true
    }));
  } catch (err) {
    console.error("搜索失败:", err);
  }
};

const sendMsg = async () => {
  if (!inputMsg.value.trim() || !currentPeer.value || isSearchingMsg.value) return;
  const content = inputMsg.value;
  const targetId = currentPeer.value.id;
  try {
    await invoke('send_p2p_message', { target: targetId, content });
    pushMessage(targetId, { content, fromMe: true, type: 'text' });
    inputMsg.value = '';
  } catch (err) {
    console.error("发送失败:", err);
  }
};

const sendFile = async () => {
  if (!currentPeer.value) return;
  try {
    const selected = await open({ multiple: false, title: '选择文件' });
    if (selected && typeof selected === 'string') {
      const fileName = selected.split(/[\\/]/).pop() || 'File';
      await invoke('send_p2p_file', { target: currentPeer.value.id, path: selected });
      pushMessage(currentPeer.value.id, { content: fileName, fromMe: true, type: 'file' });
    }
  } catch (err) {
    console.error("文件发送失败:", err);
  }
};

// --- 初始化 ---
onMounted(async () => {
  // 获取我的 ID
  await listen('p2p-my-id', (e: any) => myShortId.value = e.payload.substring(0, 8));

  // 获取所有备注
  try {
    remarks.value = await invoke('get_p2p_remarks');
  } catch (err) {
    console.log("备注为空或加载失败");
  }

  // 发现邻居
  await listen('p2p-peer-discovered', (e: any) => {
    const id = e.payload;
    if (!peers.value.find(p => p.id === id)) {
      peers.value.push({ id, unreadCount: 0 });
    }
  });

  // 接收消息
  await listen('p2p-receive-msg', (e: any) => {
    const record = e.payload;
    if (record.direction === 'receive') {
      pushMessage(record.peer_id, {
        content: record.content,
        fromMe: false,
        type: record.msg_type
      }, record.timestamp);
    }
  });
});
</script>

<template>
  <div class="chat-wrapper">
    <header class="chat-header">
      <div class="header-left">
        <i class="fas fa-network-wired"></i>
        <span>P2P 局域网加密通讯</span>
      </div>
      <div class="header-right">
        <div class="status-badge">
          <span class="pulse-dot"></span>
          <span class="my-id">ME: {{ myShortId }}</span>
        </div>
      </div>
    </header>

    <main class="chat-container">
      <aside class="peer-sidebar">
        <div class="sidebar-search">
          <i class="fas fa-search"></i>
          <input v-model="peerSearchQuery" placeholder="搜索邻居..." />
        </div>

        <div class="peer-list">
          <div
              v-for="peer in filteredPeers"
              :key="peer.id"
              class="peer-card"
              :class="{ 'is-active': currentPeer?.id === peer.id }"
              @click="selectPeer(peer)"
          >
            <div class="avatar-area">
              <i class="fas fa-user-circle"></i>
              <span v-if="peer.unreadCount > 0" class="unread-badge">{{ peer.unreadCount }}</span>
            </div>
            <div class="peer-meta">
              <div class="peer-top">
                <span class="name text-ellipsis">{{ remarks[peer.id] || '匿名邻居' }}</span>
              </div>
              <div class="last-msg text-ellipsis">{{ peer.lastMsg || peer.id.substring(0, 12) + '...' }}</div>
            </div>
          </div>
        </div>
      </aside>

      <section class="chat-view" v-if="currentPeer">
        <header class="view-header">
          <div class="peer-info" @click="editRemark(currentPeer.id)">
            <span class="current-name">{{ remarks[currentPeer.id] || '未命名的邻居' }}</span>
            <span class="current-id">{{ currentPeer.id }}</span>
            <i class="fas fa-pen-nib edit-icon"></i>
          </div>
          <div class="msg-search-box">
            <input
                v-model="msgSearchQuery"
                @keyup.enter="execSearch"
                placeholder="搜索聊天记录..."
            />
            <i class="fas fa-search search-trigger" @click="execSearch"></i>
          </div>
        </header>

        <div class="message-list" ref="messageBox">
          <div v-if="isSearchingMsg" class="search-mode-banner">
            显示 "{{ msgSearchQuery }}" 的搜索结果
            <span @click="selectPeer(currentPeer)" class="close-search">退出搜索</span>
          </div>

          <div
              v-for="(msg, index) in currentChatHistory"
              :key="index"
              class="msg-row"
              :class="{ 'is-mine': msg.fromMe, 'search-hit': msg.isSearchHit }"
          >
            <div class="msg-bubble" :class="msg.type">
              <div v-if="msg.type === 'file'" class="file-box">
                <i class="fas fa-file-invoice"></i>
                <div class="file-info">
                  <span class="file-name">{{ msg.content }}</span>
                  <span class="file-desc">{{ msg.fromMe ? '已发送' : '已存至下载目录' }}</span>
                </div>
              </div>
              <div v-else class="text-content">{{ msg.content }}</div>
              <span class="msg-time">{{ msg.time }}</span>
            </div>
          </div>
        </div>

        <footer class="input-panel" :class="{ 'disabled': isSearchingMsg }">
          <button class="tool-btn" @click="sendFile" :disabled="isSearchingMsg">
            <i class="fas fa-paperclip"></i>
          </button>
          <input
              v-model="inputMsg"
              @keyup.enter="sendMsg"
              :placeholder="isSearchingMsg ? '搜索模式下无法发送消息' : '输入消息...'"
              type="text"
              :disabled="isSearchingMsg"
          />
          <button class="send-btn" @click="sendMsg" :disabled="!inputMsg.trim() || isSearchingMsg">
            <i class="fas fa-paper-plane"></i>
          </button>
        </footer>
      </section>

      <section class="chat-empty" v-else>
        <div class="empty-content">
          <i class="fas fa-user-secret main-icon"></i>
          <h3>开启私密会话</h3>
          <p>从左侧列表选择一个在线邻居开始沟通</p>
        </div>
      </section>
    </main>
  </div>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

.chat-wrapper {
  display: flex; flex-direction: column; height: 100%;
  background-color: var(--bg-primary); color: var(--text-main);

  .chat-header {
    height: 50px; padding: 0 20px; display: flex; align-items: center; justify-content: space-between;
    background-color: var(--bg-secondary); border-bottom: 1px solid var(--border);
    .header-left { display: flex; align-items: center; gap: 10px; font-weight: 600; color: var(--accent); }
    .status-badge {
      display: flex; align-items: center; gap: 8px; padding: 4px 12px;
      background: var(--bg-input); border-radius: 20px; font-size: 11px;
      .pulse-dot { width: 8px; height: 8px; background: var(--success); border-radius: 50%; box-shadow: 0 0 8px var(--success); }
    }
  }

  .chat-container {
    flex: 1; display: flex; overflow: hidden;

    // 侧边栏
    .peer-sidebar {
      width: 280px; background-color: var(--bg-secondary); border-right: 1px solid var(--border);
      display: flex; flex-direction: column;

      .sidebar-search {
        padding: 15px; position: relative;
        i { position: absolute; left: 25px; top: 25px; font-size: 12px; color: var(--text-dim); }
        input {
          background: var(--bg-input); border: 1px solid var(--border);
          border-radius: 6px; padding: 8px 10px 8px 30px; color: var(--text-main); font-size: 13px;
        }
      }

      .peer-list {
        flex: 1; overflow-y: auto; padding: 0 10px;
        .peer-card {
          display: flex; align-items: center; gap: 12px; padding: 12px; border-radius: 8px;
          cursor: pointer; margin-bottom: 4px; transition: all 0.2s;
          &:hover { background: var(--bg-card); }
          &.is-active { background: var(--accent-glow); border: 1px solid var(--accent); }
          .avatar-area { position: relative; font-size: 32px; color: var(--text-dim); }
          .unread-badge {
            position: absolute; top: -2px; right: -2px; background: var(--danger);
            color: white; font-size: 10px; padding: 2px 6px; border-radius: 10px;
          }
          .peer-meta {
            flex: 1; overflow: hidden;
            .name { font-size: 14px; font-weight: 600; }
            .last-msg { font-size: 12px; color: var(--text-dim); margin-top: 4px; }
          }
        }
      }
    }

    // 聊天区
    .chat-view {
      flex: 1; display: flex; flex-direction: column;

      .view-header {
        padding: 10px 20px; border-bottom: 1px solid var(--border); background: var(--bg-secondary);
        display: flex; justify-content: space-between; align-items: center;

        .peer-info {
          cursor: pointer; display: flex; flex-direction: column;
          &:hover .edit-icon { opacity: 1; }
          .current-name { font-weight: 600; font-size: 15px; }
          .current-id { font-size: 10px; color: var(--text-dim); font-family: monospace; }
          .edit-icon { font-size: 12px; color: var(--accent); opacity: 0; transition: 0.2s; margin-top: 2px;}
        }

        .msg-search-box {
          position: relative;
          input {
            background: var(--bg-input); border: 1px solid var(--border); border-radius: 4px;
            padding: 6px 30px 6px 10px; font-size: 12px; color: var(--text-main); width: 180px;
          }
          .search-trigger { position: absolute; right: 10px; top: 8px; font-size: 12px; cursor: pointer; color: var(--text-dim); }
        }
      }

      .message-list {
        flex: 1; padding: 20px; overflow-y: auto; display: flex; flex-direction: column; gap: 12px;

        .search-mode-banner {
          background: var(--accent-glow); padding: 8px; border-radius: 4px; text-align: center;
          font-size: 12px; margin-bottom: 10px; border: 1px dashed var(--accent);
          .close-search { color: var(--accent); cursor: pointer; margin-left: 10px; font-weight: 600; }
        }

        .msg-row {
          display: flex;
          &.is-mine { justify-content: flex-end; }
          &.search-hit .msg-bubble { border: 1px solid var(--accent); box-shadow: 0 0 10px var(--accent-glow); }

          .msg-bubble {
            max-width: 70%; padding: 10px 14px; border-radius: 12px;
            background: var(--bg-card); border: 1px solid var(--border);

            &.is-mine { background: var(--accent); color: white; border: none; }
            &.file .file-box {
              display: flex; align-items: center; gap: 10px;
              i { font-size: 24px; color: var(--accent-orange); }
              .file-info { display: flex; flex-direction: column; .file-name { font-weight: 600; } .file-desc { font-size: 10px; opacity: 0.8; } }
            }
            .msg-time { display: block; font-size: 10px; opacity: 0.6; margin-top: 6px; text-align: right; }
          }
        }
      }

      .input-panel {
        padding: 20px; display: flex; gap: 15px; border-top: 1px solid var(--border);
        &.disabled { opacity: 0.5; }
        input {
          flex: 1; background: var(--bg-input); border: 1px solid var(--border);
          border-radius: 8px; padding: 10px 15px; color: var(--text-main);
        }
        .tool-btn, .send-btn { background: none; border: none; font-size: 20px; cursor: pointer; color: var(--text-dim); }
        .send-btn { color: var(--accent); &:disabled { opacity: 0.3; } }
      }
    }

    .chat-empty {
      flex: 1; display: flex; align-items: center; justify-content: center;
      .empty-content {
        text-align: center; color: var(--text-dim);
        i { font-size: 64px; opacity: 0.1; margin-bottom: 20px; }
      }
    }
  }
}

.text-ellipsis { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>