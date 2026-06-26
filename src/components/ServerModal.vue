<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "../utils/toast.ts";
import { generateStrongPassword } from "../utils/password.ts";
import { t, localeCompareTag } from "../utils/i18n.ts";

const props = defineProps<{
  isOpen: boolean;
  isEditing: boolean;
  server: any;
  servers: any[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', server: any): void;
}>();

const showPassword = ref(false);
const formData = ref({ ...props.server });
const jumpHostOpen = ref(false);
const jumpHostSelectRef = ref<HTMLElement | null>(null);
const jumpHostMenuRef = ref<HTMLElement | null>(null);
const modalBodyRef = ref<HTMLElement | null>(null);
const menuPosition = ref({ top: 0, left: 0, width: 0 });

const filteredServers = computed(() => {
  return props.servers.filter(x => x.id !== formData.value.id);
});

const existingGroups = computed(() => {
  const set = new Set<string>();
  for (const s of props.servers) {
    const g = s.group?.trim();
    if (g) set.add(g);
  }
  return [...set].sort((a, b) => a.localeCompare(b, localeCompareTag()));
});

const jumpHostOptions = computed(() => [
  { value: '', label: t('serverModal.directNoJump') },
  ...filteredServers.value.map(s => ({ value: s.id, label: s.name }))
]);

const selectedJumpLabel = computed(() => {
  const id = formData.value.jump_host_id ?? '';
  return jumpHostOptions.value.find(o => o.value === id)?.label ?? t('serverModal.directNoJump');
});

const modalTitle = computed(() => t(props.isEditing ? 'serverModal.editTitle' : 'serverModal.newTitle'));
const modalSubtitle = computed(() =>
  t(props.isEditing ? 'serverModal.editSubtitle' : 'serverModal.newSubtitle')
);

const menuStyle = computed(() => ({
  top: `${menuPosition.value.top}px`,
  left: `${menuPosition.value.left}px`,
  width: `${menuPosition.value.width}px`,
}));

watch(() => props.isOpen, (isOpen) => {
  if (isOpen) {
    formData.value = { ...props.server };
    showPassword.value = false;
  } else {
    jumpHostOpen.value = false;
  }
});

const updateMenuPosition = () => {
  const root = jumpHostSelectRef.value;
  if (!root) return;
  const trigger = root.querySelector('.custom-select__trigger') as HTMLElement | null;
  if (!trigger) return;
  const rect = trigger.getBoundingClientRect();
  menuPosition.value = {
    top: rect.bottom + 6,
    left: rect.left,
    width: rect.width,
  };
};

const closeJumpHost = () => {
  jumpHostOpen.value = false;
};

const openJumpHost = () => {
  jumpHostOpen.value = true;
  nextTick(updateMenuPosition);
};

const toggleJumpHost = (e: Event) => {
  e.stopPropagation();
  if (jumpHostOpen.value) {
    closeJumpHost();
  } else {
    openJumpHost();
  }
};

const selectJumpHost = (value: string) => {
  formData.value.jump_host_id = value;
  closeJumpHost();
};

const handlePointerDownOutside = (e: PointerEvent) => {
  if (!jumpHostOpen.value) return;
  const target = e.target as Node;
  if (jumpHostSelectRef.value?.contains(target)) return;
  if (jumpHostMenuRef.value?.contains(target)) return;
  closeJumpHost();
};

const handleModalScroll = () => {
  if (jumpHostOpen.value) {
    updateMenuPosition();
  }
};

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDownOutside);
  window.addEventListener('resize', handleModalScroll);
});

onUnmounted(() => {
  document.removeEventListener('pointerdown', handlePointerDownOutside);
  window.removeEventListener('resize', handleModalScroll);
});

const selectKeyFile = async () => {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'SSH Private Key', extensions: ['*', 'pem', 'key'] }]
  });
  if (selected) formData.value.private_key_path = selected as string;
};

const wouldCreateJumpCycle = (hostId: string, jumpId: string | null | undefined) => {
  if (!jumpId) return false;
  if (jumpId === hostId) return true;
  const visited = new Set<string>();
  let current: string | null | undefined = jumpId;
  while (current) {
    if (current === hostId) return true;
    if (visited.has(current)) return true;
    visited.add(current);
    const hop = props.servers.find((s) => s.id === current);
    current = hop?.jump_host_id || null;
    if (!current) break;
  }
  return false;
};

const saveHost = async () => {
  if (!formData.value.name?.trim() || !formData.value.host?.trim()) {
    toast.warning(t('toast.fillRequired'));
    return;
  }
  const hostId = formData.value.id || '__new__';
  const jumpId = formData.value.jump_host_id || null;
  if (wouldCreateJumpCycle(hostId, jumpId)) {
    toast.error(t('serverModal.jumpCycleError'));
    return;
  }
  const payload = {
    ...formData.value,
    port: Number(formData.value.port),
    group: formData.value.group?.trim() || null,
    updated_at: formData.value.updated_at || 0,
    deleted: formData.value.deleted ?? false
  };
  emit('save', payload);
};

const closeModal = () => {
  showPassword.value = false;
  emit('close');
};

const generatePassword = () => {
  formData.value.password = generateStrongPassword(30);
  showPassword.value = true;
  toast.success(t('serverModal.passwordGenerated'));
};
</script>

<template>
  <Transition name="modal">
    <div v-if="isOpen" class="modal-overlay" @click.self="closeModal">
      <div class="modal-card" role="dialog" aria-modal="true" :aria-label="modalTitle">
        <div class="modal-accent" aria-hidden="true"></div>

        <header class="modal-header">
          <div class="modal-header__main">
            <div class="modal-header__icon">
              <i :class="isEditing ? 'fas fa-pen-to-square' : 'fas fa-server'"></i>
            </div>
            <div class="modal-header__text">
              <h3>{{ modalTitle }}</h3>
              <p>{{ modalSubtitle }}</p>
            </div>
          </div>
          <Tooltip :text="t('common.close')">
            <button type="button" class="modal-header__close" @click="closeModal">
              <i class="fas fa-xmark"></i>
            </button>
          </Tooltip>
        </header>

        <div ref="modalBodyRef" class="modal-body" @scroll="handleModalScroll">
          <section class="form-section">
            <div class="form-section__title">{{ t('serverModal.connectionInfo') }}</div>

            <div class="form-group">
              <label>{{ t('serverModal.displayName') }}</label>
              <div class="input-with-icon">
                <i class="fas fa-tag input-icon"></i>
                <input
                    v-model="formData.name"
                    class="field-control"
                    type="text"
                    :placeholder="t('serverModal.displayNamePlaceholder')"
                />
              </div>
            </div>

            <div class="form-group">
              <label>{{ t('serverModal.group') }} <span class="label-hint">{{ t('serverModal.optional') }}</span></label>
              <div class="input-with-icon">
                <i class="fas fa-folder input-icon"></i>
                <input
                    v-model="formData.group"
                    class="field-control"
                    type="text"
                    list="host-group-suggestions"
                    :placeholder="t('serverModal.groupPlaceholder')"
                />
                <datalist id="host-group-suggestions">
                  <option v-for="g in existingGroups" :key="g" :value="g" />
                </datalist>
              </div>
            </div>

            <div class="form-row">
              <div class="form-group flex-3">
                <label>{{ t('serverModal.host') }}</label>
                <div class="input-with-icon">
                  <i class="fas fa-globe input-icon"></i>
                  <input v-model="formData.host" class="field-control" type="text" placeholder="192.168.1.100" />
                </div>
              </div>
              <div class="form-group flex-1">
                <label>{{ t('serverModal.port') }}</label>
                <NumberInput
                    v-model="formData.port"
                    :min="1"
                    :max="65535"
                    class="number-input--lg"
                />
              </div>
            </div>

            <div class="form-group">
              <label>{{ t('serverModal.username') }}</label>
              <div class="input-with-icon">
                <i class="fas fa-user input-icon"></i>
                <input v-model="formData.username" class="field-control" type="text" placeholder="root" />
              </div>
            </div>
          </section>

          <section class="form-section">
            <div class="form-section__title">{{ t('serverModal.authSection') }}</div>

            <div class="auth-tabs">
              <button
                  type="button"
                  :class="['auth-tab', { active: formData.auth_type === 'password' }]"
                  @click="formData.auth_type = 'password'"
              >
                <i class="fas fa-key"></i>
                <span>{{ t('serverModal.password') }}</span>
              </button>
              <button
                  type="button"
                  :class="['auth-tab', { active: formData.auth_type === 'key' }]"
                  @click="formData.auth_type = 'key'"
              >
                <i class="fas fa-fingerprint"></i>
                <span>{{ t('serverModal.sshKey') }}</span>
              </button>
            </div>

            <div v-if="formData.auth_type === 'password'" class="form-group">
              <label>{{ t('serverModal.password') }}</label>
              <div class="password-wrapper">
                <i class="fas fa-lock input-icon"></i>
                <input
                    v-model="formData.password"
                    class="field-control"
                    :type="showPassword ? 'text' : 'password'"
                    :placeholder="t('serverModal.passwordPlaceholder')"
                />
                <Tooltip :text="t('serverModal.generatePassword')">
                  <button type="button" class="gen-btn" @click="generatePassword">
                    <i class="fas fa-wand-magic-sparkles"></i>
                  </button>
                </Tooltip>
                <Tooltip :text="showPassword ? t('serverModal.hidePassword') : t('serverModal.showPassword')">
                  <button type="button" class="eye-btn" @click="showPassword = !showPassword">
                    <i class="fas" :class="showPassword ? 'fa-eye-slash' : 'fa-eye'"></i>
                  </button>
                </Tooltip>
              </div>
            </div>

            <div v-else class="form-group">
              <label>{{ t('serverModal.privateKeyPath') }}</label>
              <div class="file-picker">
                <div class="input-with-icon file-input">
                  <i class="fas fa-file-code input-icon"></i>
                  <input
                      v-model="formData.private_key_path"
                      class="field-control"
                      type="text"
                      readonly
                      :placeholder="t('serverModal.selectKeyPlaceholder')"
                  />
                </div>
                <button type="button" class="browse-btn" @click="selectKeyFile">
                  <i class="fas fa-folder-open"></i>
                  {{ t('serverModal.browse') }}
                </button>
              </div>
            </div>
          </section>

          <section class="form-section form-section--last">
            <div class="form-section__title">{{ t('serverModal.advanced') }}</div>

            <div class="form-group">
              <label>{{ t('serverModal.jumpHost') }} <span class="label-hint">{{ t('serverModal.optional') }}</span></label>
              <div ref="jumpHostSelectRef" class="custom-select" :class="{ open: jumpHostOpen }">
                <button type="button" class="custom-select__trigger" @click="toggleJumpHost">
                  <i class="fas fa-diagram-project custom-select__icon"></i>
                  <span class="custom-select__label">{{ selectedJumpLabel }}</span>
                  <i class="fas fa-chevron-down custom-select__arrow"></i>
                </button>
              </div>

              <Teleport to="body">
                <ul
                    v-show="jumpHostOpen"
                    ref="jumpHostMenuRef"
                    class="custom-select__menu custom-select__menu--portal"
                    :style="menuStyle"
                >
                  <li
                      v-for="opt in jumpHostOptions"
                      :key="opt.value || 'direct'"
                      :class="['custom-select__option', { active: (formData.jump_host_id ?? '') === opt.value }]"
                      @pointerdown.prevent.stop="selectJumpHost(opt.value)"
                  >
                    <i class="fas fa-check custom-select__check"></i>
                    <span>{{ opt.label }}</span>
                  </li>
                </ul>
              </Teleport>
            </div>
          </section>
        </div>

        <footer class="modal-footer">
          <button type="button" class="btn btn--ghost" @click="closeModal">{{ t('common.cancel') }}</button>
          <button type="button" class="btn btn--primary" @click="saveHost">
            <i class="fas fa-check"></i>
            {{ t('common.save') }}
          </button>
        </footer>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(10px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  padding: 24px;
}

.modal-card {
  position: relative;
  width: 480px;
  max-width: 100%;
  max-height: min(90vh, 720px);
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  box-shadow:
    0 24px 48px -12px var(--shadow),
    0 0 0 1px var(--border-30);
  overflow: hidden;
}

.modal-accent {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, var(--accent), var(--accent-purple));
  opacity: 0.9;
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 20px 12px;
  border-bottom: 1px solid var(--border-30);

  &__main {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
  }

  &__icon {
    width: 38px;
    height: 38px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 11px;
    background: linear-gradient(135deg, var(--accent-15), var(--accent-08));
    border: 1px solid var(--accent-20);
    color: var(--accent);
    font-size: 16px;
  }

  &__text {
    min-width: 0;

    h3 {
      margin: 0 0 4px;
      font-size: 16px;
      font-weight: 700;
      color: var(--text-main);
      line-height: 1.2;
    }

    p {
      margin: 0;
      font-size: 12px;
      color: var(--text-dim);
      line-height: 1.4;
    }
  }

  &__close {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s ease;

    &:hover {
      background: var(--error-15);
      border-color: var(--error-30);
      color: var(--error);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 2px var(--accent-glow);
    }
  }
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0 20px 4px;

  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 4px;
  }
}

.form-section {
  padding: 10px 0;
  border-bottom: 1px solid var(--border-30);

  &:first-child {
    padding-top: 12px;
  }

  &--last {
    border-bottom: none;
    padding-bottom: 8px;
  }

  &__title {
    margin-bottom: 12px;
    padding-left: 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
    letter-spacing: 0.02em;
    position: relative;

    &::before {
      content: '';
      position: absolute;
      left: 0;
      top: 50%;
      transform: translateY(-50%);
      width: 3px;
      height: 3px;
      border-radius: 50%;
      background: var(--accent);
      opacity: 0.65;
    }
  }
}

.form-group {
  margin-bottom: 14px;

  &:last-child { margin-bottom: 0; }

  label {
    display: block;
    margin-bottom: 7px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-main);
    opacity: 0.85;
  }

  .label-hint {
    font-size: 11px;
    font-weight: 400;
    color: var(--text-dim);
    opacity: 0.7;
  }
}

.form-row {
  display: flex;
  gap: 12px;

  .flex-3 { flex: 3; min-width: 0; }
  .flex-1 { flex: 1.2; min-width: 108px; }
}

.input-with-icon {
  position: relative;

  .input-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 12px;
    color: var(--text-dim);
    opacity: 0.65;
    pointer-events: none;
    z-index: 1;
  }

  .field-control {
    padding-left: 36px;
  }

  &.file-input .field-control {
    cursor: default;
  }
}

.field-control {
  width: 100%;
  height: 38px;
  padding: 0 12px;
  box-sizing: border-box;
  background-color: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 9px;
  color: var(--text-main);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;

  &:focus {
    border-color: var(--accent);
    background-color: var(--accent-05);
    box-shadow: 0 0 0 3px var(--accent-15);
  }

  &::placeholder {
    color: var(--text-dim);
    opacity: 0.5;
  }

  &:disabled,
  &[readonly] {
    opacity: 0.85;
  }
}

.custom-select {
  position: relative;

  &__trigger {
    width: 100%;
    height: 38px;
    padding: 0 36px 0 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    box-sizing: border-box;
    background-color: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 9px;
    color: var(--text-main);
    font-size: 13px;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;

    &:hover {
      border-color: var(--border-50);
    }
  }

  &__icon {
    flex-shrink: 0;
    font-size: 12px;
    color: var(--text-dim);
    opacity: 0.65;
  }

  &__label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__arrow {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 10px;
    color: var(--text-dim);
    opacity: 0.7;
    transition: transform 0.2s ease;
  }

  &.open {
    .custom-select__trigger {
      border-color: var(--accent);
      background-color: var(--accent-05);
      box-shadow: 0 0 0 3px var(--accent-15);
    }

    .custom-select__arrow {
      transform: translateY(-50%) rotate(180deg);
      color: var(--accent);
    }
  }

  &__menu {
    margin: 0;
    padding: 6px;
    list-style: none;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 12px 28px var(--shadow);
    max-height: 200px;
    overflow-y: auto;

    &::-webkit-scrollbar { width: 4px; }
    &::-webkit-scrollbar-thumb {
      background: var(--border);
      border-radius: 4px;
    }

    &--portal {
      position: fixed;
      z-index: 3000;
    }
  }

  &__option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 7px;
    font-size: 13px;
    color: var(--text-main);
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;

    .custom-select__check {
      width: 12px;
      font-size: 10px;
      color: var(--accent);
      opacity: 0;
      flex-shrink: 0;
    }

    &:hover {
      background: var(--accent-08);
      color: var(--text-main);
    }

    &.active {
      background: var(--accent-12);
      color: var(--accent);
      font-weight: 500;

      .custom-select__check {
        opacity: 1;
      }
    }
  }
}

.auth-tabs {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 14px;
  padding: 4px;
  border-radius: 10px;
  background: var(--bg-input);
  border: 1px solid var(--border-30);
}

.auth-tab {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  height: 36px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;

  i { font-size: 11px; }

  &:hover {
    color: var(--text-main);
    background: var(--bg-primary-30);
  }

  &.active {
    background: var(--accent-12);
    border-color: var(--accent-20);
    color: var(--accent);
    box-shadow: 0 1px 4px var(--accent-10);
  }

  &:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
}

.password-wrapper {
  position: relative;

  .field-control {
    padding-right: 72px;
    padding-left: 36px;
  }

  .input-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 12px;
    color: var(--text-dim);
    opacity: 0.65;
    pointer-events: none;
  }

  .gen-btn,
  .eye-btn {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.15s ease;

    i {
      font-size: 12px;
    }

    &:hover {
      background: var(--accent-10);
      color: var(--accent);
    }
  }

  .gen-btn {
    right: 34px;
  }

  .eye-btn {
    right: 4px;
  }
}

.file-picker {
  display: flex;
  gap: 8px;

  .input-with-icon { flex: 1; min-width: 0; }

  .browse-btn {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 38px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--bg-secondary);
    color: var(--text-main);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;

    i { font-size: 11px; color: var(--text-dim); }

    &:hover {
      border-color: var(--accent-30);
      background: var(--accent-08);
      color: var(--accent);

      i { color: var(--accent); }
    }
  }
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 20px 18px;
  border-top: 1px solid var(--border-30);
  background: var(--bg-secondary-60);
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  height: 38px;
  padding: 0 20px;
  border-radius: 9px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;

  &--ghost {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-dim);

    &:hover {
      background: var(--bg-input);
      color: var(--text-main);
      border-color: var(--border-50);
    }
  }

  &--primary {
    border: none;
    background: var(--accent);
    color: #fff;
    box-shadow: 0 4px 14px var(--accent-20);

    i { font-size: 11px; }

    &:hover {
      filter: brightness(1.08);
      box-shadow: 0 6px 18px var(--accent-30);
      transform: translateY(-1px);
    }

    &:active { transform: translateY(0); }
  }

  &:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.22s ease;

  .modal-card {
    transition: transform 0.28s cubic-bezier(0.34, 1.2, 0.64, 1), opacity 0.22s ease;
  }
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;

  .modal-card {
    opacity: 0;
    transform: scale(0.96) translateY(12px);
  }
}
</style>
