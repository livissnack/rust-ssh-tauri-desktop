<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  detectAppPlatform,
  useMacTitlebar,
  useWinStyleTitlebar,
} from "../utils/platform";

const appWindow = getCurrentWindow();

defineProps<{
  activeSessionId?: string | null;
}>();

const platform = detectAppPlatform();
const isMacOs = useMacTitlebar(platform);
const isWinLayout = useWinStyleTitlebar(platform);

const controlsHover = ref(false);
const isMaximized = ref(false);

let unlistenResized: (() => void) | null = null;

onMounted(async () => {
  try {
    isMaximized.value = await appWindow.isMaximized();
    unlistenResized = await appWindow.onResized(async () => {
      isMaximized.value = await appWindow.isMaximized();
    });
  } catch (err) {
    console.warn("TitleBar: window state unavailable", err);
  }
});

onUnmounted(() => {
  unlistenResized?.();
});

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const closeApp = () => appWindow.close();
</script>

<template>
  <header
    class="titlebar"
    :class="{
      'is-mac': isMacOs,
      'is-win-layout': isWinLayout,
      [`platform-${platform}`]: true,
    }"
  >
    <div class="titlebar-drag-handle" data-tauri-drag-region></div>
    <div class="titlebar-ui-layer">
      <!-- macOS：左侧交通灯 -->
      <div
        v-if="isMacOs"
        class="window-controls mac-controls"
        @mouseenter="controlsHover = true"
        @mouseleave="controlsHover = false"
      >
        <div class="dot close" @click="closeApp">
          <i v-show="controlsHover" class="fas fa-times"></i>
        </div>
        <div class="dot minimize" @click="minimize">
          <i v-show="controlsHover" class="fas fa-minus"></i>
        </div>
        <div class="dot maximize" @click="toggleMaximize">
          <i v-show="controlsHover" class="fas fa-expand-alt"></i>
        </div>
      </div>

      <div class="title-text-container">
        <div class="app-icon">
          <i class="fas fa-terminal"></i>
        </div>
        <div class="title-main">Hiphup Terminal</div>
        <div class="session-badge" v-if="activeSessionId">SSH</div>
      </div>

      <!-- macOS：右侧占位，与左侧交通灯等宽，保证标题居中 -->
      <div v-if="isMacOs" class="titlebar-spacer" aria-hidden="true"></div>

      <!-- Windows / Linux / 其他：右侧窗口按钮 -->
      <div v-if="isWinLayout" class="window-controls win-controls">
        <button type="button" class="win-btn" title="最小化" aria-label="最小化" @click="minimize">
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <path d="M0 4.5h10v1H0z" fill="currentColor" />
          </svg>
        </button>
        <button
          type="button"
          class="win-btn"
          :title="isMaximized ? '向下还原' : '最大化'"
          :aria-label="isMaximized ? '向下还原' : '最大化'"
          @click="toggleMaximize"
        >
          <svg v-if="isMaximized" viewBox="0 0 10 10" aria-hidden="true">
            <path
              d="M2 2h6v6H2V2zm1 1v4h4V3H3zm2.5-2h4.5v1H7V1H3v1h2.5z"
              fill="currentColor"
            />
          </svg>
          <svg v-else viewBox="0 0 10 10" aria-hidden="true">
            <path
              d="M0 0h10v10H0V0zm1 1v8h8V1H1z"
              fill="none"
              stroke="currentColor"
              stroke-width="1"
            />
          </svg>
        </button>
        <button type="button" class="win-btn close" title="关闭" aria-label="关闭" @click="closeApp">
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <path
              d="M1.5 1.5l7 7M8.5 1.5l-7 7"
              fill="none"
              stroke="currentColor"
              stroke-width="1.2"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>
    </div>
  </header>
</template>

<style lang="scss" scoped>
@use '../assets/css/base.scss';

.titlebar {
  height: 38px;
  background-color: var(--bg-sidebar);
  border-bottom: 1px solid var(--border-50);
  position: relative;
  z-index: 1000;
  flex-shrink: 0;
  user-select: none;

  .titlebar-drag-handle {
    position: absolute;
    inset: 0;
    z-index: 10;
  }

  .titlebar-ui-layer {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 15px;
    z-index: 20;
    pointer-events: none;

    .window-controls {
      pointer-events: auto;
      flex-shrink: 0;
      position: relative;
      z-index: 2;
    }

    .mac-controls {
      display: flex;
      gap: 8px;
      width: 52px;

      &:hover .dot i {
        opacity: 1;
      }

      .dot {
        width: 12px;
        height: 12px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 0.5px solid rgba(0, 0, 0, 0.12);
        cursor: default;

        i {
          font-size: 7px;
          opacity: 0;
          color: rgba(0, 0, 0, 0.5);
          transition: opacity 0.1s ease;
        }

        &.close {
          background: #ff5f57;
          &:active { background: #bf4942; }
        }

        &.minimize {
          background: #febc2e;
          &:active { background: #be8e25; }
        }

        &.maximize {
          background: #28c840;
          &:active { background: #1e9530; }
        }
      }
    }

    .title-text-container {
      position: absolute;
      left: 50%;
      top: 50%;
      transform: translate(-50%, -50%);
      display: flex;
      align-items: center;
      gap: 10px;
      pointer-events: none;
      max-width: calc(100% - 140px);
      white-space: nowrap;

      .app-icon {
        font-size: 14px;
        color: var(--accent);
        filter: drop-shadow(0 0 4px var(--accent-30));
        opacity: 0.9;
        flex-shrink: 0;
      }

      .title-main {
        font-size: 12px;
        color: var(--text-main);
        font-weight: 600;
        letter-spacing: 0.5px;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .session-badge {
        font-size: 9px;
        padding: 1px 6px;
        border-radius: 4px;
        background: var(--accent-15);
        color: var(--accent);
        border: 1px solid var(--accent-20);
        text-transform: uppercase;
        font-weight: 800;
        letter-spacing: 1px;
        flex-shrink: 0;
      }
    }

    .titlebar-spacer {
      width: 52px;
      flex-shrink: 0;
    }
  }

  /* macOS：左右对称，标题视觉居中 */
  &.is-mac .titlebar-ui-layer {
    padding: 0 15px;
  }

  /* Windows / Linux / 其他：右侧三键 */
  &.is-win-layout .titlebar-ui-layer {
    padding: 0;
    justify-content: flex-end;

    .win-controls {
      display: flex;
      align-items: stretch;
      height: 100%;
      margin-left: auto;

      .win-btn {
        width: 46px;
        height: 100%;
        border: none;
        background: transparent;
        color: var(--text-main);
        display: inline-flex;
        align-items: center;
        justify-content: center;
        cursor: default;
        padding: 0;
        transition: background-color 0.12s ease, color 0.12s ease;

        svg {
          width: 10px;
          height: 10px;
          display: block;
          flex-shrink: 0;
        }

        &:hover {
          background: var(--border-50);
        }

        &:active {
          background: var(--border);
        }

        &.close:hover {
          background: #e81123;
          color: #fff;
        }

        &.close:active {
          background: #bf0f1d;
          color: #fff;
        }
      }
    }
  }
}
</style>
