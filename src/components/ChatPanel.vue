<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// 模拟数据及状态
const myShortId = ref('8a2f1b...');
const peers = ref<{id: string, nickname?: string}[]>([]);
const currentPeer = ref<any>(null);
const inputMsg = ref('');
const chatHistory = ref<{content: string, fromMe: boolean, time: string}[]>([]);

const selectPeer = (peer: any) => {
  currentPeer.value = peer;
  // 这里可以加载 redb 里的历史记录
};

const sendMsg = async () => {
  if (!inputMsg.value.trim() || !currentPeer.value) return;

  const content = inputMsg.value;
  try {
    // 调用刚才暴露的后端命令
    await invoke('send_p2p_message', {
      target: currentPeer.value.id,
      content: content
    });

    chatHistory.value.push({
      content: content,
      fromMe: true,
      time: new Date().toLocaleTimeString()
    });
    inputMsg.value = '';
  } catch (err) {
    console.error("发送失败:", err);
  }
};

onMounted(async () => {
  // 监听 Rust 端 mDNS 发现的事件
  await listen('p2p-peer-discovered', (event: any) => {
    const peerId = event.payload;
    if (!peers.value.find(p => p.id === peerId)) {
      peers.value.push({ id: peerId });
    }
  });

  await listen('p2p-receive-msg', (event) => {
    const { from, content } = event.payload;
    // 如果当前正在和这个人聊天，则存入历史
    chatHistory.value.push({
      content: content,
      fromMe: false,
      time: new Date().toLocaleTimeString()
    });
  });

  await listen('p2p-peer-expired', (event: any) => {
    const peerId = event.payload;
    peers.value = peers.value.filter(p => p.id !== peerId);
  });
});
</script>

<template>
  <div class="chat-settings">
    <div class="panel-header">
      <i class="fas fa-comments"></i>
      <span>局域网邻居 (P2P)</span>
      <div class="my-status">
        <span class="status-dot"></span>
        <span class="my-id">我的ID: {{ myShortId }}</span>
      </div>
    </div>

    <div class="chat-container">
      <div class="peer-list">
        <div
            v-for="peer in peers"
            :key="peer.id"
            class="peer-item"
            :class="{ active: currentPeer?.id === peer.id }"
            @click="selectPeer(peer)"
        >
          <div class="peer-avatar">
            <i class="fas fa-user"></i>
            <span class="online-indicator"></span>
          </div>
          <div class="peer-info">
            <span class="peer-name">{{ peer.nickname || '匿名邻居' }}</span>
            <span class="peer-id-text">{{ peer.id.substring(0, 8) }}...</span>
          </div>
        </div>
        <div v-if="peers.length === 0" class="empty-state">
          正在探测局域网邻居...
        </div>
      </div>

      <div class="chat-main" v-if="currentPeer">
        <div class="chat-messages" ref="messageBox">
          <div
              v-for="(msg, index) in chatHistory"
              :key="index"
              class="message-wrapper"
              :class="{ 'mine': msg.fromMe }"
          >
            <div class="message-bubble">
              {{ msg.content }}
              <span class="message-time">{{ msg.time }}</span>
            </div>
          </div>
        </div>

        <div class="chat-input-area">
          <input
              v-model="inputMsg"
              @keyup.enter="sendMsg"
              placeholder="输入消息内容..."
              type="text"
          />
          <button @click="sendMsg" :disabled="!inputMsg.trim()">
            <i class="fas fa-paper-plane"></i>
          </button>
        </div>
      </div>

      <div class="chat-main-empty" v-else>
        <i class="fas fa-terminal"></i>
        <p>选择一个邻居开始加密会话</p>
      </div>
    </div>
  </div>
</template>


<style lang="scss" scoped>
.chat-settings {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  color: var(--text-main);

  .panel-header {
    padding: 20px;
    font-size: 1rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid var(--border);

    i { color: var(--accent); }

    .my-status {
      margin-left: auto;
      display: flex;
      align-items: center;
      gap: 6px;
      background: var(--bg-primary);
      padding: 4px 10px;
      border-radius: 20px;
      border: 1px solid var(--border);

      .status-dot {
        width: 8px;
        height: 8px;
        background: #4caf50;
        border-radius: 50%;
        box-shadow: 0 0 8px #4caf50;
      }
      .my-id { font-size: 11px; color: var(--text-dim); font-family: monospace; }
    }
  }

  .chat-container {
    flex: 1;
    display: flex;
    overflow: hidden;

    .peer-list {
      width: 260px;
      border-right: 1px solid var(--border);
      overflow-y: auto;
      padding: 12px;
      display: flex;
      flex-direction: column;
      gap: 8px;

      .peer-item {
        display: flex;
        align-items: center;
        padding: 10px;
        border-radius: 8px;
        cursor: pointer;
        transition: all 0.2s;
        border: 1px solid transparent;

        &:hover { background: var(--bg-primary); }
        &.active {
          background: var(--accent-glow);
          border-color: var(--accent);
        }

        .peer-avatar {
          position: relative;
          width: 36px;
          height: 36px;
          background: var(--bg-secondary);
          border-radius: 50%;
          display: flex;
          align-items: center;
          justify-content: center;
          margin-right: 12px;
          border: 1px solid var(--border);

          i { color: var(--text-dim); }
          .online-indicator {
            position: absolute;
            bottom: 0;
            right: 0;
            width: 10px;
            height: 10px;
            background: #4caf50;
            border-radius: 50%;
            border: 2px solid var(--bg-primary);
          }
        }

        .peer-info {
          display: flex;
          flex-direction: column;
          .peer-name { font-size: 13px; font-weight: 500; }
          .peer-id-text { font-size: 11px; color: var(--text-dim); font-family: monospace; }
        }
      }
    }

    .chat-main {
      flex: 1;
      display: flex;
      flex-direction: column;
      background: var(--bg-primary);

      .chat-messages {
        flex: 1;
        padding: 20px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 16px;

        .message-wrapper {
          display: flex;
          &.mine { justify-content: flex-end; }

          .message-bubble {
            max-width: 70%;
            padding: 10px 14px;
            border-radius: 12px;
            background: var(--bg-secondary);
            border: 1px solid var(--border);
            font-size: 14px;
            position: relative;

            .message-time {
              display: block;
              font-size: 10px;
              color: var(--text-dim);
              margin-top: 4px;
            }
          }

          &.mine .message-bubble {
            background: var(--accent);
            color: white;
            border: none;
            .message-time { color: rgba(255,255,255,0.7); }
          }
        }
      }

      .chat-input-area {
        padding: 16px;
        border-top: 1px solid var(--border);
        display: flex;
        gap: 12px;

        input {
          flex: 1;
          background: var(--bg-secondary);
          border: 1px solid var(--border);
          border-radius: 6px;
          padding: 8px 12px;
          color: var(--text-main);
          outline: none;
          &:focus { border-color: var(--accent); }
        }

        button {
          background: var(--accent);
          color: white;
          border: none;
          border-radius: 6px;
          width: 40px;
          cursor: pointer;
          transition: opacity 0.2s;
          &:disabled { opacity: 0.5; cursor: not-allowed; }
        }
      }
    }

    .chat-main-empty {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: var(--text-dim);
      i { font-size: 3rem; margin-bottom: 16px; opacity: 0.2; }
    }
  }
}
</style>