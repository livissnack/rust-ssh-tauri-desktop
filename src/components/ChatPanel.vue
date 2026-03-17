<script setup lang="ts">
import { ref, onMounted, computed, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import {toast} from "../utils/toast.ts";

interface ChatMessage {
  content: string;
  fromMe: boolean;
  type: 'text' | 'file';
  time: string;
  isSearchHit?: boolean;
}

interface Peer {
  id: string;
  nickname?: string;
  lastMsg?: string;
  unreadCount: number;
  isOnline: boolean;
}

const myShortId = ref('Locating...');
const peers = ref<Peer[]>([]);
const currentPeer = ref<Peer | null>(null);
const inputMsg = ref('');
const messageBox = ref<HTMLElement | null>(null);

const peerSearchQuery = ref('');
const msgSearchQuery = ref('');
const remarks = ref<Record<string, string>>({});
const isSearchingMsg = ref(false);
const allChatHistories = ref<Record<string, ChatMessage[]>>({});

const filteredPeers = computed(() => {
  const query = peerSearchQuery.value.toLowerCase();

  return peers.value
      .filter(p => {
        const name = (remarks.value?.[p.id] || '匿名邻居').toLowerCase();
        return name.includes(query) || p.id.toLowerCase().includes(query);
      })
      .sort((a, b) => {
        if (a.isOnline === b.isOnline) return 0;
        return a.isOnline ? -1 : 1;
      });
});

const currentChatHistory = computed(() => {
  if (!currentPeer.value) return [];
  return allChatHistories.value[currentPeer.value.id] || [];
});

const emit = defineEmits(['update-online-count']);

const notifyOnlineCount = () => {
  const count = peers.value.filter(p => p.isOnline).length;
  emit('update-online-count', count);
};

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

const execSearch = async () => {
  if (!currentPeer.value) return;
  if (!msgSearchQuery.value.trim()) {
    isSearchingMsg.value = false;
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
    if (selected) {
      const fileName = selected.split(/[\\/]/).pop() || 'File';
      await invoke('send_p2p_file', { target: currentPeer.value.id, path: selected });
      pushMessage(currentPeer.value.id, { content: fileName, fromMe: true, type: 'file' });
    }
  } catch (err) {
    console.error("文件发送失败:", err);
  }
};

const addPeerToList = (id: string, online: boolean = true) => {
  const existing = peers.value.find(p => p.id === id);
  if (existing) {
    existing.isOnline = online;
  } else {
    peers.value.push({
      id,
      unreadCount: 0,
      isOnline: online
    });
  }
};

onMounted(async () => {
  await listen('p2p-peer-discovered', (e: any) => {
    console.log("TS 接收到发现事件，PeerID:", e.payload);
    addPeerToList(e.payload);
    notifyOnlineCount();
  });

  try {
    const existingPeers: string[] = await invoke('get_online_peers');
    existingPeers.forEach(id => addPeerToList(id));
    notifyOnlineCount();
  } catch (err) {
    console.error("拉取在线列表失败:", err);
  }

  await listen('p2p-my-id', (e: any) => myShortId.value = e.payload.substring(0, 8));

  try {
    remarks.value = await invoke('get_p2p_remarks');
  } catch (err) {
    console.log("备注加载失败");
  }

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

  await listen('p2p-peer-expired', (e: any) => {
    const offlinePeerId = e.payload;
    const peer = peers.value.find(p => p.id === offlinePeerId);
    if (peer) {
      peer.isOnline = false;
      notifyOnlineCount();
      if (currentPeer.value?.id === offlinePeerId) {
        toast.warning('对方已离线')
      }
    }
  });
});
</script>

<template>
  <div class="chat-wrapper">
    <header class="chat-header">
      <div class="header-left">
        <i class="fas fa-network-wired"></i>
        <span>P2P 加密通讯</span>
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
              <span class="status-dot" :class="{ 'is-online': peer.isOnline }"></span>
              <span v-if="peer.unreadCount > 0" class="unread-badge">{{ peer.unreadCount }}</span>
            </div>
            <div class="peer-meta" :class="{ 'is-offline': !peer.isOnline }">
              <div class="peer-top">
                <span class="name text-ellipsis">
                  {{ remarks[peer.id] || '匿名邻居' }}
                  <small v-if="!peer.isOnline" class="offline-tag">(离线)</small>
                </span>
              </div>
              <div class="last-msg text-ellipsis">...</div>
            </div>
          </div>
        </div>
      </aside>

      <section class="chat-view" v-if="currentPeer">
        <header class="view-header">
          <div class="peer-info" @click="editRemark(currentPeer.id)">
            <span class="current-name">{{ remarks[currentPeer.id] || '未命名的邻居' }}</span>
            <span class="current-id text-ellipsis">{{ currentPeer.id }}</span>
            <i class="fas fa-pen-nib edit-icon"></i>
          </div>
          <div class="msg-search-box">
            <input
                v-model="msgSearchQuery"
                @keyup.enter="execSearch"
                placeholder="搜索历史..."
            />
            <i class="fas fa-search search-trigger" @click="execSearch"></i>
          </div>
        </header>

        <div class="message-list" ref="messageBox">
          <div v-if="isSearchingMsg" class="search-mode-banner">
            <span>"{{ msgSearchQuery }}" 结果</span>
            <span @click="selectPeer(currentPeer)" class="close-search">退出</span>
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
                  <span class="file-name text-ellipsis">{{ msg.content }}</span>
                  <span class="file-desc">{{ msg.fromMe ? '已发送' : '已接收' }}</span>
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
              :placeholder="isSearchingMsg ? '搜索模式...' : '输入...'"
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
          <p>请选择一个在线邻居开始沟通</p>
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
  overflow: hidden;

  .chat-header {
    height: 44px; padding: 0 12px; display: flex; align-items: center; justify-content: space-between;
    background-color: var(--bg-secondary); border-bottom: 1px solid var(--border);
    flex-shrink: 0;

    .header-left { display: flex; align-items: center; gap: 8px; font-weight: 600; color: var(--accent); font-size: 13px; }
    .status-badge {
      display: flex; align-items: center; gap: 6px; padding: 2px 8px;
      background: var(--bg-input); border-radius: 12px; font-size: 10px;
      .pulse-dot { width: 6px; height: 6px; background: var(--success); border-radius: 50%; }
    }
  }

  .chat-container {
    flex: 1; display: flex; overflow: hidden;

    // 侧边栏优化
    .peer-sidebar {
      width: clamp(180px, 30%, 240px); background-color: var(--bg-secondary); border-right: 1px solid var(--border);
      display: flex; flex-direction: column; flex-shrink: 0;

      .sidebar-search {
        padding: 10px; position: relative;
        i { position: absolute; left: 18px; top: 19px; font-size: 11px; color: var(--text-dim); }
        input {
          box-sizing: border-box; /* 💡 关键：强制宽度包含 padding 和 border */
          background: var(--bg-input);
          border: 1px solid var(--border);
          border-radius: 6px;
          padding: 6px 8px 6px 26px;
          color: var(--text-main);
          font-size: 12px;
          width: 100%; /* 现在 100% 会完美适配容器 */
          outline: none; /* 建议顺便加上，防止点击时出现默认蓝框 */
        }
      }

      .peer-list {
        flex: 1; overflow-y: auto; padding: 0 6px;
        .peer-card {
          display: flex; align-items: center; gap: 8px; padding: 8px; border-radius: 6px;
          cursor: pointer; margin-bottom: 2px; transition: all 0.2s; border: 1px solid transparent;
          min-width: 0; // 核心：允许子元素溢出处理

          &:hover { background: var(--bg-card); }
          &.is-active { background: var(--accent-glow); border-color: var(--accent); }

          .avatar-area {
            position: relative;
            font-size: 28px;
            color: var(--text-dim);

            .status-dot {
              position: absolute;
              bottom: 2px;
              right: 0;
              width: 10px;
              height: 10px;
              border-radius: 50%;
              background: #94a3b8; // 默认灰色 (离线)
              border: 2px solid var(--bg-secondary); // 增加描边使其在头像上清晰
              transition: background 0.3s;

              &.is-online {
                background: #22c55e; // 绿色 (在线)
                box-shadow: 0 0 4px #22c55e;
              }
            }
          }

          .peer-meta {
            &.is-offline {
              opacity: 0.6; // 离线邻居整体变淡
              filter: grayscale(0.5); // 轻微灰度
            }

            .offline-tag {
              font-size: 10px;
              font-weight: normal;
              margin-left: 4px;
              color: var(--text-dim);
            }
          }
          .unread-badge {
            position: absolute; top: -4px; right: -4px; background: var(--danger);
            color: white; font-size: 9px; padding: 1px 4px; border-radius: 8px;
          }
        }
      }
    }

    // 聊天区优化
    .chat-view {
      flex: 1; display: flex; flex-direction: column; min-width: 0;

      .view-header {
        padding: 8px 12px; border-bottom: 1px solid var(--border); background: var(--bg-secondary);
        display: flex; justify-content: space-between; align-items: center; gap: 10px;

        .peer-info {
          cursor: pointer; display: flex; flex-direction: column; min-width: 0; flex: 1;
          &:hover .edit-icon { opacity: 1; }
          .current-name { font-weight: 600; font-size: 14px; }
          .current-id { font-size: 10px; color: var(--text-dim); opacity: 0.7; }
          .edit-icon { font-size: 10px; color: var(--accent); opacity: 0; transition: 0.2s; margin-top: 2px;}
        }

        .msg-search-box {
          position: relative; flex-shrink: 0;
          input {
            background: var(--bg-input); border: 1px solid var(--border); border-radius: 4px;
            padding: 4px 26px 4px 8px; font-size: 11px; color: var(--text-main); width: 80px;
            transition: width 0.3s ease;
            &:focus { width: 120px; }
          }
          .search-trigger { position: absolute; right: 8px; top: 6px; font-size: 11px; cursor: pointer; color: var(--text-dim); }
        }
      }

      .message-list {
        flex: 1; padding: 12px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px;

        .search-mode-banner {
          background: var(--accent-glow); padding: 6px; border-radius: 4px; text-align: center;
          font-size: 11px; margin-bottom: 8px; border: 1px dashed var(--accent);
          display: flex; justify-content: center; gap: 8px;
          .close-search { color: var(--accent); cursor: pointer; font-weight: 600; }
        }

        .msg-row {
          display: flex;
          &.is-mine { justify-content: flex-end; }
          &.search-hit .msg-bubble { border: 1px solid var(--accent); box-shadow: 0 0 6px var(--accent-glow); }

          .msg-bubble {
            max-width: 85%; padding: 8px 12px; border-radius: 10px;
            background: var(--bg-card); border: 1px solid var(--border); font-size: 13px;
            line-height: 1.4; word-break: break-word;

            &.is-mine { background: var(--accent); color: white; border: none; }
            &.file .file-box {
              display: flex; align-items: center; gap: 8px;
              i { font-size: 20px; color: var(--accent-orange); flex-shrink: 0; }
              .file-info {
                display: flex; flex-direction: column; min-width: 0;
                .file-name { font-weight: 600; font-size: 12px; }
                .file-desc { font-size: 9px; opacity: 0.8; }
              }
            }
            .msg-time { display: block; font-size: 9px; opacity: 0.5; margin-top: 4px; text-align: right; }
          }
        }
      }

      .input-panel {
        padding: 10px 12px; display: flex; gap: 8px; border-top: 1px solid var(--border);
        align-items: center;
        &.disabled { opacity: 0.5; }
        input {
          flex: 1; background: var(--bg-input); border: 1px solid var(--border);
          border-radius: 6px; padding: 8px 12px; color: var(--text-main); font-size: 13px;
          min-width: 0;
        }
        .tool-btn, .send-btn { background: none; border: none; font-size: 18px; cursor: pointer; color: var(--text-dim); flex-shrink: 0; }
        .send-btn { color: var(--accent); &:disabled { opacity: 0.3; } }
      }
    }

    .chat-empty {
      flex: 1; display: flex; align-items: center; justify-content: center;
      .empty-content {
        text-align: center; color: var(--text-dim); padding: 20px;
        i { font-size: 48px; opacity: 0.1; margin-bottom: 16px; }
        h3 { font-size: 16px; margin-bottom: 8px; }
        p { font-size: 12px; }
      }
    }
  }
}

.text-ellipsis { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>