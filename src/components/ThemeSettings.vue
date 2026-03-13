<template>
  <div class="theme-settings">
    <div class="panel-header">
      <i class="fas fa-palette"></i>
      <span>主题视觉</span>
    </div>

    <div class="settings-list">
      <div
          v-for="theme in themeOptions"
          :key="theme.id"
          class="theme-option"
          :class="{ active: currentTheme === theme.id }"
          @click="applyTheme(theme.id)"
      >
        <div class="theme-preview" :class="`${theme.id}-theme`">
          <div class="preview-sidebar"></div>
          <div class="preview-content">
            <div class="preview-accent"></div>
          </div>
        </div>

        <div class="theme-info">
          <span class="theme-label">{{ theme.name }}</span>
          <span class="theme-type">{{ theme.isLight ? '亮色主题' : '暗色主题' }}</span>
        </div>

        <i v-if="currentTheme === theme.id" class="fas fa-check-circle check-icon"></i>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { themeOptions, applyTheme } from "../utils/theme.ts";
</script>

<style lang="scss" scoped>
.theme-settings {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  color: var(--text-main);
  transition: background 0.3s ease;

  .panel-header {
    padding: 20px;
    font-size: 1rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid var(--border);
    i { color: var(--accent); }
  }

  .settings-list {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
  }

  .theme-option {
    display: flex;
    align-items: center;
    padding: 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;

    &:hover {
      border-color: var(--accent);
      transform: translateY(-1px);
      background: var(--bg-secondary);
    }

    &.active {
      border-color: var(--accent);
      background: var(--accent-glow); // 使用透明度混合色
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    }

    .theme-preview {
      width: 60px;
      height: 40px;
      border-radius: 4px;
      border: 1px solid var(--border);
      display: flex;
      overflow: hidden;
      margin-right: 16px;
      background: var(--bg-primary); // 这里会受预览框上的主题类控制

      .preview-sidebar {
        width: 30%;
        background: var(--bg-secondary);
        border-right: 1px solid var(--border);
      }
      .preview-content {
        flex: 1;
        background: var(--bg-primary);
        display: flex;
        align-items: center;
        justify-content: center;
        .preview-accent {
          width: 10px;
          height: 10px;
          border-radius: 50%;
          background: var(--accent);
        }
      }
    }

    .theme-info {
      display: flex;
      flex-direction: column;
      .theme-label {
        font-size: 14px;
        font-weight: 500;
        margin-bottom: 2px;
        color: var(--text-main);
      }
      .theme-type {
        font-size: 11px;
        color: var(--text-dim);
      }
    }

    .check-icon {
      position: absolute;
      right: 16px;
      color: var(--accent);
      font-size: 1.1rem;
    }
  }
}
</style>