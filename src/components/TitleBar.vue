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
  background-color: base.$bg-sidebar; // 标题栏通常与侧边栏融为一体
  border-bottom: 1px solid rgba(base.$border, 0.5); // 增加极其细微的分割感
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
    // 注意：在 Tauri 中需在 HTML 标签上添加 data-tauri-drag-region
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
    pointer-events: none; // 穿透到拖拽层

    .window-controls {
      display: flex;
      gap: 8px;
      pointer-events: auto; // 恢复控件的点击事件

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

        /* 标准 macOS 风格配色，但在不同主题下会自动调整明度 */
        &.close {
          background: #ff5f56;
          &:hover { background: color.scale(#ff5f56, $lightness: 10%); }
        }

        &.minimize {
          background: #ffbd2e;
          &:hover { background: color.scale(#ffbd2e, $lightness: 10%); }
        }

        &.maximize {
          background: #27c93f;
          &:hover { background: color.scale(#27c93f, $lightness: 10%); }
        }

        /* 悬停时可以增加一个小图标感（可选） */
        &:active {
          transform: scale(0.9);
          filter: brightness(0.8);
        }
      }
    }

    .title-text {
      font-size: 10px; // 稍微缩小一点更精致
      color: base.$text-dim;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 1.5px; // 增加间距营造高级感
      opacity: 0.8;

      // 当窗口失去焦点时，可以配合全局类名变色（需 JS 配合）
      .is-inactive & {
        color: rgba(base.$text-dim, 0.4);
      }
    }

    .titlebar-spacer {
      width: 60px; // 用于平衡左侧控制按钮的视觉重量，保持文字居中
    }
  }
}
</style>
