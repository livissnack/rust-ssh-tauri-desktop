<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const closeApp = () => appWindow.close();
</script>

<template>
  <header class="titlebar">
    <div class="titlebar-drag-handle" data-tauri-drag-region></div>
    <div class="titlebar-ui-layer">
      <div class="window-controls">
        <div class="dot close" @click="closeApp"></div>
        <div class="dot minimize" @click="minimize"></div>
        <div class="dot maximize" @click="toggleMaximize"></div>
      </div>
      <div class="title-text">Hiphup SSH v2.0</div>
      <div class="titlebar-spacer"></div>
    </div>
  </header>
</template>

<style lang="scss" scoped>
$bg-sidebar: #16161e;
$text-dim: #565f89;

.titlebar {
  height: 38px;
  background-color: $bg-sidebar;
  position: relative;
  z-index: 1000;
  flex-shrink: 0;

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

      .dot {
        width: 12px;
        height: 12px;
        border-radius: 50%;
        cursor: pointer;
        transition: filter 0.2s;

        &.close {
          background: #ff5f56;
        }

        &.minimize {
          background: #ffbd2e;
        }

        &.maximize {
          background: #27c93f;
        }

        &:hover {
          filter: brightness(1.2);
        }
      }
    }

    .title-text {
      font-size: 11px;
      color: $text-dim;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 1px;
    }

    .titlebar-spacer {
      width: 60px;
    }
  }
}
</style>
