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
      <div class="title-text">Hiphup Terminal</div>
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
  border-bottom: 1px solid var(--border-50); // 修复点：使用预计算的 50% 透明边框
  position: relative;
  z-index: 1000;
  flex-shrink: 0;
  user-select: none;

  /* Tauri 拖拽层 */
  .titlebar-drag-handle {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 10;
  }

  /* 交互层 */
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
    pointer-events: none; // 关键：允许点击穿透到下方的拖拽层

    .window-controls {
      display: flex;
      gap: 8px;
      pointer-events: auto; // 恢复控件区域的交互

      .dot {
        width: 12px;
        height: 12px;
        border-radius: 50%;
        cursor: pointer;
        transition: all 0.2s ease;
        position: relative;
        display: flex;
        align-items: center;
        justify-content: center;

        /* 标准 macOS 风格配色：通过 filter 替代 Sass 函数实现动态变色 */
        &.close {
          background: #ff5f56;
          &:hover { filter: brightness(1.1); }
        }

        &.minimize {
          background: #ffbd2e;
          &:hover { filter: brightness(1.1); }
        }

        &.maximize {
          background: #27c93f;
          &:hover { filter: brightness(1.1); }
        }

        &:active {
          transform: scale(0.9);
          filter: brightness(0.8);
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

      // 修复点：配合 CSS 变量处理失焦状态
      .is-inactive & {
        color: var(--text-dim-40);
      }
    }

    .titlebar-spacer {
      width: 60px; // 视觉平衡块
    }
  }
}
</style>
