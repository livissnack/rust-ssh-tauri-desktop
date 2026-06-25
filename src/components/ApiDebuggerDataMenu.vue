<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import { toast } from '../utils/toast.ts';
import { confirm } from '../utils/confirm.ts';
import type { ApiDebuggerStore } from '../utils/apiDebuggerStorage.ts';
import {
  exportApiDebuggerData,
  importApiDebuggerData,
  type ExportFormat,
} from '../utils/apiDebuggerStorage.ts';

const emit = defineEmits<{
  imported: [store: ApiDebuggerStore];
}>();

const rootRef = ref<HTMLElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const menuOpen = ref(false);
const menuPosition = ref({ top: 0, left: 0, width: 0 });

const menuStyle = computed(() => ({
  top: `${menuPosition.value.top}px`,
  left: `${menuPosition.value.left}px`,
  width: `${Math.max(menuPosition.value.width, 210)}px`,
}));

const updateMenuPosition = () => {
  const trigger = rootRef.value?.querySelector('.meta-btn') as HTMLElement | null;
  if (!trigger) return;
  const rect = trigger.getBoundingClientRect();
  const panelWidth = Math.max(rect.width, 210);
  menuPosition.value = {
    top: rect.bottom + 6,
    left: Math.max(8, rect.right - panelWidth),
    width: panelWidth,
  };
};

const closeMenu = () => {
  menuOpen.value = false;
};

const toggleMenu = () => {
  if (menuOpen.value) {
    closeMenu();
    return;
  }
  menuOpen.value = true;
  nextTick(updateMenuPosition);
};

const handleExport = async (format: ExportFormat) => {
  closeMenu();
  try {
    const ok = await exportApiDebuggerData(format);
    if (ok) toast.success('导出成功');
  } catch (err) {
    toast.error(String(err));
  }
};

const handleImport = async (format: 'auto' | ExportFormat) => {
  closeMenu();
  const merge = await confirm('选择「确定」合并导入，选择「取消」覆盖现有数据', 'warning', '导入 API 数据');
  try {
    const store = await importApiDebuggerData(format, merge ? 'merge' : 'replace');
    if (!store) return;
    emit('imported', store);
    toast.success('导入成功');
  } catch (err) {
    toast.error(String(err));
  }
};

const handlePointerDownOutside = (event: PointerEvent) => {
  if (!menuOpen.value) return;
  const target = event.target as Node;
  if (rootRef.value?.contains(target)) return;
  if (menuRef.value?.contains(target)) return;
  closeMenu();
};

watch(menuOpen, (open) => {
  if (open) nextTick(updateMenuPosition);
});

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDownOutside);
  window.addEventListener('resize', updateMenuPosition);
  window.addEventListener('scroll', updateMenuPosition, true);
});

onUnmounted(() => {
  document.removeEventListener('pointerdown', handlePointerDownOutside);
  window.removeEventListener('resize', updateMenuPosition);
  window.removeEventListener('scroll', updateMenuPosition, true);
});
</script>

<template>
  <div ref="rootRef" class="data-menu" :class="{ open: menuOpen }">
    <button type="button" class="meta-btn" @click="toggleMenu">
      <i class="fas fa-database"></i>
      <span>数据</span>
      <i class="fas fa-chevron-down caret"></i>
    </button>

    <Teleport to="body">
      <div
        v-if="menuOpen"
        ref="menuRef"
        class="data-menu__panel"
        :style="menuStyle"
      >
        <div class="menu-group">
          <span class="menu-label">导出</span>
          <button type="button" class="menu-item" @click="handleExport('hiphup')">
            <i class="fas fa-file-export"></i>
            Hiphup 完整备份
          </button>
          <button type="button" class="menu-item" @click="handleExport('postman-collection')">
            <i class="fas fa-layer-group"></i>
            Postman Collection
          </button>
          <button type="button" class="menu-item" @click="handleExport('postman-environment')">
            <i class="fas fa-sliders-h"></i>
            Postman Environment
          </button>
        </div>
        <div class="menu-group">
          <span class="menu-label">导入</span>
          <button type="button" class="menu-item" @click="handleImport('auto')">
            <i class="fas fa-file-import"></i>
            自动识别格式
          </button>
          <button type="button" class="menu-item" @click="handleImport('hiphup')">
            <i class="fas fa-file-code"></i>
            Hiphup 备份
          </button>
          <button type="button" class="menu-item" @click="handleImport('postman-collection')">
            <i class="fas fa-layer-group"></i>
            Postman Collection
          </button>
          <button type="button" class="menu-item" @click="handleImport('postman-environment')">
            <i class="fas fa-sliders-h"></i>
            Postman Environment
          </button>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped lang="scss">
.data-menu {
  position: relative;
  flex-shrink: 0;

  &.open .caret {
    transform: rotate(180deg);
  }

  &.open .meta-btn {
    color: var(--accent);
    border-color: var(--accent-30);
    background: var(--accent-08);
  }
}

.meta-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--border-30);
  border-radius: 7px;
  background: var(--bg-input);
  color: var(--text-dim);
  font-size: 10px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;

  .caret {
    font-size: 8px;
    transition: transform 0.2s;
  }

  &:hover {
    color: var(--accent);
    border-color: var(--accent-30);
    background: var(--accent-08);
  }
}

.data-menu__panel {
  position: fixed;
  z-index: 10050;
  padding: 8px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: 0 12px 28px var(--shadow);
  max-height: min(420px, calc(100vh - 24px));
  overflow-y: auto;
}

.menu-group + .menu-group {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border-30);
}

.menu-label {
  display: block;
  padding: 0 8px 4px;
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-dim);
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text-main);
  font-size: 11px;
  text-align: left;
  cursor: pointer;

  i {
    width: 14px;
    font-size: 10px;
    color: var(--text-dim);
  }

  &:hover {
    background: var(--accent-08);
    color: var(--accent);

    i { color: var(--accent); }
  }
}
</style>
