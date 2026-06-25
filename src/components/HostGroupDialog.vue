<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "../utils/i18n.ts";

const { tr, t } = useI18n();

const props = defineProps<{
  visible: boolean;
  server: { id: string; name: string; host: string; username: string; port: number; group?: string | null } | null;
  servers: any[];
}>();

const emit = defineEmits<{
  close: [];
  confirm: [group: string | null];
}>();

const groupInput = ref("");

const existingGroups = computed(() => {
  const set = new Set<string>();
  for (const s of props.servers) {
    const g = s.group?.trim();
    if (g) set.add(g);
  }
  return [...set].sort((a, b) => a.localeCompare(b, "zh"));
});

watch(
  () => [props.visible, props.server?.group] as const,
  ([visible, group]) => {
    if (visible) groupInput.value = group?.trim() || "";
  },
  { immediate: true },
);

const selectGroup = (name: string) => {
  groupInput.value = name;
};

const clearGroup = () => {
  groupInput.value = "";
};

const handleConfirm = () => {
  const value = groupInput.value.trim();
  emit("confirm", value || null);
};

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === "Escape") emit("close");
};
</script>

<template>
  <Teleport to="body">
    <Transition name="host-group-fade">
      <div v-if="visible && server" class="host-group-overlay" @click="emit('close')">
        <div class="host-group-dialog" @click.stop @keydown="handleKeydown">
          <div class="host-group-dialog__header">
            <div class="host-group-dialog__icon">
              <i class="fas fa-folder-open"></i>
            </div>
            <div class="host-group-dialog__titles">
              <h3>{{ tr.hostGroup.title }}</h3>
              <p>{{ t('hostGroup.subtitle', { name: server.name }) }}</p>
            </div>
            <button type="button" class="host-group-dialog__close" :aria-label="tr.common.close" @click="emit('close')">
              <i class="fas fa-times"></i>
            </button>
          </div>

          <div class="host-group-dialog__host">
            <i class="fas fa-server"></i>
            <span>{{ server.username }}@{{ server.host }}:{{ server.port }}</span>
          </div>

          <div class="host-group-dialog__field">
            <label>{{ tr.hostGroup.groupName }}</label>
            <div class="host-group-dialog__input-wrap">
              <i class="fas fa-tag"></i>
              <input
                v-model="groupInput"
                type="text"
                :placeholder="tr.hostGroup.placeholder"
                autofocus
                @keyup.enter="handleConfirm"
              />
            </div>
            <p class="host-group-dialog__hint">{{ tr.hostGroup.emptyHint }}</p>
          </div>

          <div v-if="existingGroups.length" class="host-group-dialog__chips">
            <span class="host-group-dialog__chips-label">{{ tr.hostGroup.existingGroups }}</span>
            <div class="host-group-dialog__chip-list">
              <button
                v-for="g in existingGroups"
                :key="g"
                type="button"
                class="host-group-dialog__chip"
                :class="{ active: groupInput.trim() === g }"
                @click="selectGroup(g)"
              >
                <i class="fas fa-folder"></i>
                {{ g }}
              </button>
            </div>
          </div>

          <div class="host-group-dialog__footer">
            <button type="button" class="btn-ghost" @click="clearGroup">
              <i class="fas fa-eraser"></i>
              {{ tr.hostGroup.clearGroup }}
            </button>
            <div class="host-group-dialog__actions">
              <button type="button" class="btn-cancel" @click="emit('close')">{{ tr.common.cancel }}</button>
              <button type="button" class="btn-confirm" @click="handleConfirm">{{ tr.common.save }}</button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped lang="scss">
.host-group-overlay {
  position: fixed;
  inset: 0;
  z-index: 10060;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(10px);
}

.host-group-dialog {
  width: min(420px, 100%);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  box-shadow:
    0 24px 48px rgba(0, 0, 0, 0.35),
    inset 0 1px 0 var(--border-30);
  overflow: hidden;

  &__header {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 20px 20px 16px;
    border-bottom: 1px solid var(--border-30);
  }

  &__icon {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 12px;
    background: linear-gradient(135deg, var(--accent-15), var(--accent-purple-15, rgba(187, 154, 247, 0.15)));
    color: var(--accent);
    font-size: 16px;
  }

  &__titles {
    flex: 1;
    min-width: 0;

    h3 {
      margin: 0 0 4px;
      font-size: 16px;
      font-weight: 700;
      color: var(--text-main);
    }

    p {
      margin: 0;
      font-size: 12px;
      color: var(--text-dim);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }

  &__close {
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.15s ease;

    &:hover {
      background: var(--bg-input);
      color: var(--text-main);
    }
  }

  &__host {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 16px 20px 0;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-30);
    font-family: var(--font-terminal);
    font-size: 11px;
    color: var(--text-dim);

    i {
      color: var(--accent);
      font-size: 10px;
    }
  }

  &__field {
    padding: 16px 20px 0;

    label {
      display: block;
      margin-bottom: 8px;
      font-size: 11px;
      font-weight: 600;
      color: var(--text-dim);
      letter-spacing: 0.02em;
    }
  }

  &__input-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 12px;
    border-radius: 10px;
    border: 1px solid var(--border-30);
    background: var(--bg-input);
    transition: border-color 0.2s, box-shadow 0.2s;

    &:focus-within {
      border-color: var(--accent-30);
      box-shadow: 0 0 0 3px var(--accent-10);
    }

    i {
      color: var(--text-dim);
      font-size: 12px;
    }

    input {
      flex: 1;
      min-width: 0;
      border: none;
      background: transparent;
      color: var(--text-main);
      font-size: 13px;
      outline: none;

      &::placeholder {
        color: var(--text-dim);
        opacity: 0.7;
      }
    }
  }

  &__hint {
    margin: 8px 0 0;
    font-size: 11px;
    color: var(--text-dim);
    opacity: 0.85;
  }

  &__chips {
    padding: 14px 20px 0;
  }

  &__chips-label {
    display: block;
    margin-bottom: 8px;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  &__chip-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  &__chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--border-30);
    background: var(--bg-secondary);
    color: var(--text-main);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;

    i {
      font-size: 10px;
      color: var(--text-dim);
    }

    &:hover {
      border-color: var(--accent-30);
      background: var(--accent-08);
      color: var(--accent);
    }

    &.active {
      border-color: var(--accent-40);
      background: var(--accent-12);
      color: var(--accent);
      font-weight: 600;

      i {
        color: var(--accent);
      }
    }
  }

  &__footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 18px 20px 20px;
    margin-top: 16px;
  }

  &__actions {
    display: flex;
    gap: 8px;
  }
}

.btn-ghost,
.btn-cancel,
.btn-confirm {
  height: 34px;
  padding: 0 14px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s ease;
}

.btn-ghost {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: transparent;
  border-color: var(--border-30);
  color: var(--text-dim);

  i {
    font-size: 11px;
  }

  &:hover {
    background: var(--bg-input);
    color: var(--text-main);
  }
}

.btn-cancel {
  background: transparent;
  border-color: var(--border-30);
  color: var(--text-dim);

  &:hover {
    background: var(--bg-input);
    color: var(--text-main);
  }
}

.btn-confirm {
  background: var(--accent);
  color: #fff;

  &:hover {
    filter: brightness(1.08);
  }
}

.host-group-fade-enter-active,
.host-group-fade-leave-active {
  transition: opacity 0.2s ease;

  .host-group-dialog {
    transition: transform 0.2s ease, opacity 0.2s ease;
  }
}

.host-group-fade-enter-from,
.host-group-fade-leave-to {
  opacity: 0;

  .host-group-dialog {
    transform: scale(0.96) translateY(8px);
    opacity: 0;
  }
}
</style>
