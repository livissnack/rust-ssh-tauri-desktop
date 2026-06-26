<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
  drawSelection,
  dropCursor,
} from '@codemirror/view';
import { EditorState, type Extension } from '@codemirror/state';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching, foldGutter, indentOnInput } from '@codemirror/language';
import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search';
import { useI18n } from '../utils/i18n.ts';
import { detectAppPlatform } from '../utils/platform.ts';
import { formatSize } from '../utils/async.ts';
import { defaultTheme } from '../utils/theme.ts';
import { appCodemirrorHighlight, appCodemirrorTheme } from '../utils/codemirrorTheme.ts';
import { resolveLanguageForFile } from '../utils/codemirrorLanguage.ts';
import { loadSftpEditorCursor, saveSftpEditorCursor } from '../utils/sftpEditorCursor.ts';
import type { SftpEditorEncoding } from '../utils/sftpEditor.ts';
import AppSelect from './AppSelect.vue';

const props = defineProps<{
  visible: boolean;
  loading?: boolean;
  saving?: boolean;
  readonly?: boolean;
  source: 'local' | 'remote';
  path: string;
  fileName: string;
  fileSize?: number;
  content: string;
  dirty?: boolean;
  encoding: SftpEditorEncoding;
  savedAt?: number | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', closeAfter?: boolean): void;
  (e: 'update:content', value: string): void;
  (e: 'update:encoding', value: SftpEditorEncoding): void;
}>();

const { t } = useI18n();

const editorHost = ref<HTMLElement | null>(null);
const cursorLine = ref(1);
const cursorCol = ref(1);
const totalLines = ref(1);
const totalChars = ref(0);
const languageLabel = ref<string | null>(null);
const goToLineOpen = ref(false);
const goToLineInput = ref('');

let editorView: EditorView | null = null;
let mountToken = 0;

const sourceLabel = computed(() =>
  props.source === 'local' ? t('common.local') : t('common.remote'),
);

const saveShortcut = computed(() =>
  detectAppPlatform() === 'macos' ? '⌘S' : 'Ctrl+S',
);

const statusLineCol = computed(() =>
  t('sftp.editorLineCol', { line: cursorLine.value, col: cursorCol.value }),
);

const statusStats = computed(() =>
  t('sftp.editorStats', {
    lines: totalLines.value,
    chars: totalChars.value,
    size: formatSize(props.fileSize ?? 0),
  }),
);

const savedAtLabel = computed(() => {
  if (!props.savedAt) return '';
  return t('sftp.editorSavedAt', {
    time: new Date(props.savedAt).toLocaleTimeString(),
  });
});

const displayLanguage = computed(() =>
  languageLabel.value ?? t('sftp.editorPlainText'),
);

const encodingOptions = computed(() => [
  { value: 'utf-8', label: t('sftp.encodingUtf8') },
  { value: 'gbk', label: t('sftp.encodingGbk') },
  { value: 'latin1', label: t('sftp.encodingLatin1') },
]);

const updateDocStats = (view: EditorView) => {
  totalLines.value = view.state.doc.lines;
  totalChars.value = view.state.doc.length;
};

const applyGoToLine = () => {
  const n = parseInt(goToLineInput.value.trim(), 10);
  goToLineOpen.value = false;
  if (!editorView || !Number.isFinite(n) || n < 1) return;
  const line = Math.min(n, editorView.state.doc.lines);
  const lineObj = editorView.state.doc.line(line);
  editorView.dispatch({
    selection: { anchor: lineObj.from },
    scrollIntoView: true,
  });
  editorView.focus();
};

const destroyEditor = () => {
  if (editorView && props.path) {
    saveSftpEditorCursor(props.path, cursorLine.value, cursorCol.value);
  }
  mountToken += 1;
  editorView?.destroy();
  editorView = null;
};

const buildExtensions = (langExts: Extension[]): Extension[] => [
  lineNumbers(),
  highlightActiveLine(),
  highlightActiveLineGutter(),
  history(),
  drawSelection(),
  dropCursor(),
  indentOnInput(),
  bracketMatching(),
  foldGutter(),
  highlightSelectionMatches(),
  search({ top: false }),
  EditorView.lineWrapping,
  appCodemirrorTheme,
  appCodemirrorHighlight,
  ...(props.readonly ? [EditorState.readOnly.of(true)] : []),
  ...langExts,
  keymap.of([
    ...defaultKeymap,
    ...historyKeymap,
    ...searchKeymap,
    ...(props.readonly ? [] : [indentWithTab]),
    {
      key: 'Mod-s',
      run: () => {
        if (props.readonly) return true;
        emit('save', false);
        return true;
      },
    },
    {
      key: 'Mod-g',
      run: () => {
        goToLineInput.value = String(cursorLine.value);
        goToLineOpen.value = true;
        return true;
      },
    },
  ]),
  EditorView.updateListener.of((update) => {
    if (update.docChanged && !props.readonly) {
      emit('update:content', update.state.doc.toString());
    }
    if (update.selectionSet || update.docChanged) {
      const pos = update.state.selection.main.head;
      const line = update.state.doc.lineAt(pos);
      cursorLine.value = line.number;
      cursorCol.value = pos - line.from + 1;
      updateDocStats(update.view);
    }
  }),
];

const mountEditor = async (content: string, selectionPos?: number) => {
  const token = ++mountToken;
  if (!editorHost.value) return;

  editorView?.destroy();
  editorView = null;

  const { extensions: langExts, languageName } = await resolveLanguageForFile(props.fileName);
  if (token !== mountToken || !editorHost.value || !props.visible || props.loading) return;

  languageLabel.value = languageName;

  editorView = new EditorView({
    state: EditorState.create({
      doc: content,
      extensions: buildExtensions(langExts),
    }),
    parent: editorHost.value,
  });

  updateDocStats(editorView);

  const saved = loadSftpEditorCursor(props.path);
  let pos = selectionPos;
  if (pos == null && saved) {
    const line = Math.min(saved.line, editorView.state.doc.lines);
    const lineObj = editorView.state.doc.line(line);
    const col = Math.min(saved.col, lineObj.length + 1);
    pos = lineObj.from + col - 1;
  }
  pos ??= editorView.state.selection.main.head;

  const line = editorView.state.doc.lineAt(pos);
  cursorLine.value = line.number;
  cursorCol.value = pos - line.from + 1;
  editorView.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
  editorView.focus();
};

const syncEditorContent = (content: string) => {
  if (!editorView) return;
  const current = editorView.state.doc.toString();
  if (content === current) return;
  editorView.dispatch({
    changes: { from: 0, to: current.length, insert: content },
  });
  updateDocStats(editorView);
};

watch(
  () => [props.visible, props.loading, props.fileName, props.readonly] as const,
  async ([visible, loading]) => {
    if (visible && !loading) {
      await nextTick();
      await mountEditor(props.content);
    } else if (!visible) {
      destroyEditor();
    }
  },
);

watch(
  () => props.content,
  (content) => syncEditorContent(content),
);

watch(defaultTheme, async () => {
  if (!props.visible || props.loading || !editorView) return;
  const doc = editorView.state.doc.toString();
  const sel = editorView.state.selection.main.head;
  await mountEditor(doc, sel);
});

onBeforeUnmount(() => destroyEditor());
</script>

<template>
  <Transition name="confirm-fade">
    <div v-if="visible" class="sftp-editor-overlay" @click="emit('close')">
      <div class="sftp-editor-shell" @click.stop>
        <header class="editor-toolbar">
          <div class="toolbar-left">
            <div class="file-badge" :class="{ readonly: readonly }">
              <i class="fas" :class="readonly ? 'fa-eye' : 'fa-file-code'"></i>
            </div>
            <div class="file-meta">
              <div class="file-title-row">
                <span class="file-name">{{ fileName }}</span>
                <span class="source-tag">{{ sourceLabel }}</span>
                <span v-if="readonly" class="readonly-tag">{{ t('sftp.editorReadonly') }}</span>
                <span v-else-if="dirty" class="dirty-tag">{{ t('sftp.editorDirty') }}</span>
              </div>
              <div class="file-path" :title="path">{{ path }}</div>
            </div>
          </div>
          <div class="toolbar-right">
            <div class="toolbar-tools">
              <div class="encoding-select">
                <span class="encoding-label">{{ t('sftp.editorEncoding') }}</span>
                <AppSelect
                  :model-value="encoding"
                  :options="encodingOptions"
                  :disabled="readonly || loading || saving"
                  @update:model-value="emit('update:encoding', $event as SftpEditorEncoding)"
                />
              </div>
              <span class="toolbar-divider" />
              <button
                class="btn-tool"
                type="button"
                :title="t('sftp.editorGoToLine')"
                :disabled="loading"
                @click="goToLineInput = String(cursorLine); goToLineOpen = true"
              >
                <i class="fas fa-arrow-right-to-line"></i>
              </button>
              <button class="btn-tool btn-tool-close" type="button" :title="t('common.close')" @click="emit('close')">
                <i class="fas fa-times"></i>
              </button>
            </div>
          </div>
        </header>

        <div class="editor-body">
          <div v-if="loading" class="editor-loading">
            <div class="loading-card">
              <i class="fas fa-spinner fa-spin"></i>
              <span>{{ t('common.loading') }}</span>
            </div>
          </div>
          <div v-show="!loading" ref="editorHost" class="editor-host" />
        </div>

        <footer class="editor-statusbar">
          <div class="status-left">
            <span class="status-item">
              <i class="fas fa-code"></i>
              {{ displayLanguage }}
            </span>
            <span class="status-divider" />
            <span class="status-item">{{ statusLineCol }}</span>
            <span class="status-divider" />
            <span class="status-item">{{ statusStats }}</span>
            <template v-if="savedAtLabel">
              <span class="status-divider" />
              <span class="status-item saved-at">
                <i class="fas fa-check"></i>
                {{ savedAtLabel }}
              </span>
            </template>
          </div>
          <div class="status-actions">
            <button class="btn-ghost" type="button" @click="emit('close')">
              {{ readonly ? t('common.close') : t('common.cancel') }}
            </button>
            <template v-if="!readonly">
              <button
                class="btn-ghost"
                type="button"
                :disabled="loading || saving || !dirty"
                @click="emit('save', true)"
              >
                {{ t('sftp.saveAndClose') }}
              </button>
              <button
                class="btn-primary"
                type="button"
                :disabled="loading || saving || !dirty"
                @click="emit('save', false)"
              >
                <i v-if="saving" class="fas fa-spinner fa-spin"></i>
                <template v-else>
                  <i class="fas fa-floppy-disk"></i>
                </template>
                {{ t('common.save') }}
                <kbd>{{ saveShortcut }}</kbd>
              </button>
            </template>
          </div>
        </footer>
      </div>

      <div v-if="goToLineOpen" class="goto-overlay" @click="goToLineOpen = false">
        <div class="goto-box" @click.stop>
          <label class="goto-label">{{ t('sftp.editorGoToLinePrompt') }}</label>
          <input
            v-model="goToLineInput"
            class="goto-input"
            type="number"
            min="1"
            autofocus
            @keyup.enter="applyGoToLine"
            @keyup.esc="goToLineOpen = false"
          />
          <div class="goto-actions">
            <button class="btn-ghost" type="button" @click="goToLineOpen = false">
              {{ t('common.cancel') }}
            </button>
            <button class="btn-primary" type="button" @click="applyGoToLine">
              {{ t('common.confirm') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.sftp-editor-overlay {
  position: fixed;
  inset: 0;
  z-index: 10002;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: var(--bg-secondary-60, rgba(0, 0, 0, 0.65));
  backdrop-filter: blur(10px);
}

.sftp-editor-shell {
  display: flex;
  flex-direction: column;
  width: min(980px, calc(100vw - 40px));
  height: min(82vh, 760px);
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  box-shadow: 0 20px 40px -16px var(--shadow, rgba(0, 0, 0, 0.45));
}

.editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 52px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  flex: 1;
}

.toolbar-right {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.toolbar-tools {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
}

.toolbar-divider {
  width: 1px;
  height: 22px;
  margin: 0 2px;
  background: var(--border);
  flex-shrink: 0;
}

.encoding-select {
  display: flex;
  align-items: center;
  gap: 6px;
  padding-left: 4px;

  :deep(.app-select) {
    min-width: 108px;
  }

  :deep(.app-select__trigger) {
    height: 28px;
    min-height: 28px;
    padding: 0 28px 0 10px;
    font-size: 12px;
    border-radius: 6px;
    background: var(--bg-input, var(--bg-secondary));
  }

  :deep(.app-select__arrow) {
    right: 8px;
    font-size: 9px;
  }
}

.encoding-label {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  color: var(--text-dim);
  white-space: nowrap;
}

.file-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 7px;
  border: 1px solid var(--accent-20, transparent);
  background: var(--accent-10);
  color: var(--accent);
  flex-shrink: 0;
  font-size: 14px;

  &.readonly {
    border-color: var(--border);
    background: var(--bg-card);
    color: var(--text-dim);
  }
}

.file-meta {
  min-width: 0;
}

.file-title-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.file-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-main);
  line-height: 1.3;
}

.file-path {
  margin-top: 2px;
  font-size: 11px;
  color: var(--text-dim);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.source-tag,
.dirty-tag,
.readonly-tag {
  display: inline-flex;
  align-items: center;
  padding: 1px 7px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
  line-height: 1.5;
}

.source-tag {
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
}

.dirty-tag {
  border: 1px solid rgba(245, 158, 11, 0.25);
  background: rgba(245, 158, 11, 0.12);
  color: #f59e0b;
}

.readonly-tag {
  border: 1px solid var(--accent-20, var(--border));
  background: var(--accent-08, var(--accent-10));
  color: var(--accent);
}

.btn-tool {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  flex-shrink: 0;
  font-size: 12px;
  transition: background 0.15s ease, color 0.15s ease;

  &:hover:not(:disabled) {
    background: var(--accent-08, var(--bg-secondary));
    color: var(--text-main);
  }

  &:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  &.btn-tool-close:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.12);
    color: var(--error, #ef4444);
  }
}

.editor-body {
  position: relative;
  flex: 1;
  min-height: 0;
  background: var(--bg-input, var(--bg-secondary));
}

.editor-host {
  height: 100%;

  :deep(.cm-editor) {
    height: 100%;
    outline: none;
  }
}

.editor-loading {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-input, var(--bg-secondary));
}

.loading-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
}

.editor-statusbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 44px;
  padding: 8px 12px;
  border-top: 1px solid var(--border);
  background: var(--bg-secondary);
}

.status-left {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  min-width: 0;
}

.status-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-main);

  i {
    color: var(--accent);
    font-size: 10px;
  }

  &.saved-at i {
    color: var(--success);
  }
}

.status-divider {
  width: 1px;
  height: 12px;
  background: var(--border);
}

.status-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.btn-ghost,
.btn-primary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 30px;
  padding: 0 12px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}

.btn-ghost {
  background: transparent;
  border-color: var(--border);
  color: var(--text-dim);

  &:hover:not(:disabled) {
    background: var(--bg-card);
    color: var(--text-main);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-primary {
  background: var(--accent);
  color: #fff;

  kbd {
    margin-left: 2px;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.18);
    font-family: inherit;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.goto-overlay {
  position: fixed;
  inset: 0;
  z-index: 10003;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.35);
}

.goto-box {
  width: min(320px, calc(100vw - 32px));
  padding: 16px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: 0 12px 28px var(--shadow, rgba(0, 0, 0, 0.3));
}

.goto-label {
  display: block;
  margin-bottom: 10px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main);
}

.goto-input {
  width: 100%;
  height: 36px;
  padding: 0 12px;
  margin-bottom: 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 14px;
  outline: none;

  &:focus {
    border-color: var(--accent);
  }
}

.goto-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.confirm-fade-enter-active,
.confirm-fade-leave-active {
  transition: all 0.25s ease;
}

.confirm-fade-enter-from,
.confirm-fade-leave-to {
  opacity: 0;

  .sftp-editor-shell {
    transform: scale(0.97) translateY(10px);
  }
}
</style>
