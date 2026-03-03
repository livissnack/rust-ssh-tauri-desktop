<script setup lang="ts">
import { computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  activeId: string | null;
  servers: any[];
}>();

const emit = defineEmits<{
  (e: 'update:activeId', id: string): void;
  (e: 'connect'): void;
  (e: 'edit', server: any): void;
  (e: 'delete', id: string): void;
  (e: 'openAddModal'): void;
}>();

const activeId = computed({
  get: () => props.activeId,
  set: (id) => emit('update:activeId', id)
});

const deleteServer = async (id: string) => {
  if (confirm("Delete this server configuration?")) {
    await invoke("delete_server", { id });
    emit('delete', id);
  }
};

const handleDoubleClick = () => {
  emit('connect');
};
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="logo-hex">H</div>
      <span class="brand-text">Hiphup</span>
    </div>
    <div class="sidebar-scroll-area">
      <nav class="nav-groups">
        <div class="group-label">HOSTS</div>
        <div
            v-for="s in servers"
            :key="s.id"
            :class="['host-card', { active: activeId === s.id }]"
            @click="activeId = s.id"
            @dblclick="handleDoubleClick"
        >
          <div class="host-icon-wrapper">
            <div v-if="activeId === s.id" class="pulse-ring"></div>
            <span class="icon">🖥️</span>
          </div>
          <div class="host-meta">
            <div class="name">{{ s.name }}</div>
            <div class="ip">{{ s.host }}</div>
          </div>
          <div class="host-actions">
            <span @click.stop="emit('edit', s)">⚙️</span>
            <span @click.stop="deleteServer(s.id)">×</span>
          </div>
        </div>
      </nav>
    </div>
    <div class="sidebar-footer">
      <button class="add-host-btn" @click="emit('openAddModal')">+ Add New Host</button>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
$bg-sidebar: #16161e;
$accent: #7aa2f7;
$border-color: #292e42;
$text-dim: #565f89;

.sidebar {
  width: 260px;
  background-color: $bg-sidebar;
  border-right: 1px solid #1a1b26;
  display: flex;
  flex-direction: column;
  height: 100%;

  .brand {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 20px;

    .logo-hex {
      width: 30px;
      height: 30px;
      background: linear-gradient(45deg, $accent, #bb9af7);
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: #fff;
      font-weight: 800;
    }

    .brand-text {
      font-weight: 700;
      color: #fff;
      font-size: 18px;
    }
  }

  .sidebar-scroll-area {
    flex: 1;
    overflow-y: auto;
    padding: 0 12px;
  }

  .sidebar-footer {
    flex-shrink: 0;
    padding: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.03);
  }
}

.host-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 12px;
  margin-bottom: 6px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;

  &:hover {
    background: #1a1b26;
    .host-actions {
      opacity: 1;
    }
  }

  &.active {
    background: #24283b;
    box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);

    .host-icon-wrapper {
      background: $accent;
      color: #fff;
    }

    .pulse-ring {
      border-color: #e0af68;
      display: block;
    }
  }

  .host-icon-wrapper {
    position: relative;
    width: 36px;
    height: 36px;
    background: #1a1b26;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .host-meta {
    .name {
      font-size: 13px;
      font-weight: 600;
      color: #c0caf5;
    }

    .ip {
      font-size: 11px;
      color: $text-dim;
    }
  }

  .host-actions {
    position: absolute;
    right: 10px;
    display: flex;
    gap: 8px;
    opacity: 0;
    transition: opacity 0.2s;

    span {
      cursor: pointer;
      color: $text-dim;

      &:hover {
        color: $accent;
      }
    }
  }
}

.add-host-btn {
  width: 100%;
  background: #24283b;
  border: 1px dashed $text-dim;
  color: $accent;
  padding: 12px;
  border-radius: 12px;
  cursor: pointer;

  &:hover {
    border-color: $accent;
    background: #2d334a;
  }
}

.pulse-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  border: 2px solid $accent;
  border-radius: 10px;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0% {
    transform: scale(1);
    opacity: 0.5;
  }
  100% {
    transform: scale(1.4);
    opacity: 0;
  }
}

.group-label {
  font-size: 11px;
  color: $text-dim;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 12px 0 8px 12px;
  font-weight: 600;
}
</style>
