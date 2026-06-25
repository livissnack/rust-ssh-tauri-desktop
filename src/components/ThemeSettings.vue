<template>
  <div class="theme-settings">
    <div class="panel-header">
      <i class="fas fa-palette"></i>
      <span>{{ tr.theme.title }}</span>
    </div>

    <div class="settings-scroll custom-scrollbar">
      <section class="settings-section">
        <h3 class="section-title">
          <i class="fas fa-language"></i>
          {{ tr.theme.language }}
        </h3>
        <p class="section-hint">{{ tr.theme.languageHint }}</p>
        <div class="language-list">
          <div
              v-for="opt in localeOptions"
              :key="opt.id"
              class="language-option"
              :class="{ active: locale === opt.id }"
              @click="setLocale(opt.id)"
          >
            <span class="language-label">{{ opt.nativeLabel }}</span>
            <i v-if="locale === opt.id" class="fas fa-check-circle check-icon"></i>
          </div>
        </div>
      </section>

      <section class="settings-section">
        <h3 class="section-title">
          <i class="fas fa-swatchbook"></i>
          {{ tr.theme.appearance }}
        </h3>
        <div class="settings-list">
          <div
              v-for="theme in themeOptions"
              :key="theme.id"
              class="theme-option"
              :class="{ active: defaultTheme === theme.id }"
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
              <span class="theme-type">{{ theme.isLight ? tr.theme.lightTheme : tr.theme.darkTheme }}</span>
            </div>

            <i v-if="defaultTheme === theme.id" class="fas fa-check-circle check-icon"></i>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { themeOptions, applyTheme, defaultTheme } from "../utils/theme.ts";
import { useI18n } from "../utils/i18n.ts";

const { locale, tr, setLocale, localeOptions } = useI18n();
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
    flex-shrink: 0;
    i { color: var(--accent); }
  }

  .settings-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .settings-section {
    .section-title {
      margin: 0 0 6px;
      font-size: 13px;
      font-weight: 600;
      display: flex;
      align-items: center;
      gap: 8px;
      color: var(--text-main);

      i {
        color: var(--accent);
        font-size: 12px;
      }
    }

    .section-hint {
      margin: 0 0 12px;
      font-size: 11px;
      color: var(--text-dim);
      line-height: 1.4;
    }
  }

  .language-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .language-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;

    &:hover {
      border-color: var(--accent);
      background: var(--bg-secondary);
    }

    &.active {
      border-color: var(--accent);
      background: var(--accent-glow);
    }

    .language-label {
      font-size: 14px;
      font-weight: 500;
    }

    .check-icon {
      color: var(--accent);
      font-size: 1rem;
    }
  }

  .settings-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
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
      background: var(--accent-glow);
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
      background: var(--bg-primary);

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
