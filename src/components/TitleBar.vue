<script setup lang="ts">
import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const props = defineProps<{
  activeSessionId?: string | null;
}>();

const controlsHover = ref(false);

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const closeApp = () => appWindow.close();
</script>

<template>
  <header class="titlebar">
    <div class="titlebar-drag-handle" data-tauri-drag-region></div>
    <div class="titlebar-ui-layer">
      <div class="window-controls" @mouseenter="controlsHover = true" @mouseleave="controlsHover = false">
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
      <div class="titlebar-spacer"></div>
    </div>
  </header>
</template>

<style lang="scss" scoped>
@use "sass:color";
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
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 10;
  }

  .titlebar-ui-layer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 15px;
    z-index: 20;
    pointer-events: none;

    .window-controls {
      display: flex;
      gap: 8px;
      pointer-events: auto;

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
        position: relative;
        border: 0.5px solid rgba(0, 0, 0, 0.12); // macOS 特有的微细边框

        i {
          font-size: 7px; // 符号非常小
          opacity: 0; // 默认隐藏
          color: rgba(0, 0, 0, 0.5); // 符号颜色是半透明黑
          transition: opacity 0.1s ease;
        }

        &.close {
          background: #ff5f57;
          &:active { background: #bf4942; } // 点击时变深
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

    .title-text {
      font-size: 10px;
      color: var(--text-dim);
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 1.5px;
      opacity: 0.8;
      transition: color 0.3s ease;

      .is-inactive & {
        color: var(--text-dim-40);
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

      .app-icon {
        font-size: 14px;
        color: var(--accent);
        filter: drop-shadow(0 0 4px var(--accent-30));
        opacity: 0.9;
      }

      .title-main {
        font-size: 12px;
        color: var(--text-main);
        font-weight: 600;
        letter-spacing: 0.5px;
        text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
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
      }
    }

    .is-inactive .title-text-container {
      opacity: 0.5;
      filter: grayscale(1);
      transition: all 0.4s ease;
    }

    .titlebar-spacer {
      width: 60px;
    }
  }
}
</style>
