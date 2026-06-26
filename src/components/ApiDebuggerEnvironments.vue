<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from '../utils/toast.ts';
import { confirm } from '../utils/confirm.ts';
import { t } from '../utils/i18n.ts';
import AppSelect from './AppSelect.vue';
import ApiDebuggerInputDialog from './ApiDebuggerInputDialog.vue';
import type { ApiEnvironment, EnvVariable } from '../utils/apiDebuggerStorage.ts';
import { createEnvVariable, createId } from '../utils/apiDebuggerStorage.ts';

const props = defineProps<{
  environments: ApiEnvironment[];
  activeEnvId: string | null;
}>();

const emit = defineEmits<{
  update: [environments: ApiEnvironment[]];
  'update:activeEnvId': [id: string | null];
}>();

const showCreateDialog = ref(false);

const activeEnv = computed(() =>
  props.environments.find((item) => item.id === props.activeEnvId) ?? props.environments[0] ?? null,
);

const setActive = (id: string) => {
  emit('update:activeEnvId', id);
};

const envOptions = computed(() =>
  props.environments.map((env) => ({ value: env.id, label: env.name })),
);

const selectedEnvId = computed({
  get: () => props.activeEnvId ?? activeEnv.value?.id ?? '',
  set: (id: string | number) => setActive(String(id)),
});

const addEnvironment = () => {
  showCreateDialog.value = true;
};

const confirmCreateEnvironment = (name: string) => {
  const env: ApiEnvironment = { id: createId(), name, variables: [] };
  const next = [...props.environments, env];
  emit('update', next);
  emit('update:activeEnvId', env.id);
  showCreateDialog.value = false;
  toast.success(t('apiDebugger.environments.created'));
};

const deleteEnvironment = async (env: ApiEnvironment) => {
  if (props.environments.length <= 1) {
    toast.warning(t('apiDebugger.environments.keepOne'));
    return;
  }
  const ok = await confirm(
    t('apiDebugger.environments.deleteConfirm', { name: env.name }),
    'warning',
    t('apiDebugger.environments.deleteTitle'),
  );
  if (!ok) return;
  const next = props.environments.filter((item) => item.id !== env.id);
  emit('update', next);
  if (props.activeEnvId === env.id) {
    emit('update:activeEnvId', next[0]?.id ?? null);
  }
  toast.success(t('apiDebugger.environments.deleted'));
};

const updateEnvName = (env: ApiEnvironment, name: string) => {
  emit('update', props.environments.map((item) => (item.id === env.id ? { ...item, name } : item)));
};

const addVariable = (env: ApiEnvironment) => {
  emit('update', props.environments.map((item) => {
    if (item.id !== env.id) return item;
    return { ...item, variables: [...item.variables, createEnvVariable()] };
  }));
};

const removeVariable = (env: ApiEnvironment, variableId: string) => {
  emit('update', props.environments.map((item) => {
    if (item.id !== env.id) return item;
    return { ...item, variables: item.variables.filter((v) => v.id !== variableId) };
  }));
};

const patchVariable = (env: ApiEnvironment, variable: EnvVariable, patch: Partial<EnvVariable>) => {
  emit('update', props.environments.map((item) => {
    if (item.id !== env.id) return item;
    return {
      ...item,
      variables: item.variables.map((v) => (v.id === variable.id ? { ...v, ...patch } : v)),
    };
  }));
};
</script>

<template>
  <section class="manager-view">
    <div class="manager-toolbar">
      <button type="button" class="btn-ghost" @click="addEnvironment">
        <i class="fas fa-plus"></i> {{ t('apiDebugger.environments.newEnv') }}
      </button>
    </div>

    <p class="hint-box">
      {{ t('apiDebugger.environments.hintBefore') }}<code v-pre>{{name}}</code>{{ t('apiDebugger.environments.hintAfter') }}
    </p>

    <div class="env-selector">
      <span class="field-label">{{ t('apiDebugger.environments.currentEnv') }}</span>
      <div class="env-select-wrap">
        <AppSelect
          v-model="selectedEnvId"
          :options="envOptions"
          icon="fas fa-sliders-h"
        />
      </div>
    </div>

    <div class="item-list">
      <div
        v-for="env in environments"
        :key="env.id"
        class="env-card"
        :class="{ active: (activeEnvId ?? activeEnv?.id) === env.id }"
      >
        <div class="env-head">
          <input
            class="env-name-input"
            :value="env.name"
            @change="updateEnvName(env, ($event.target as HTMLInputElement).value)"
          />
          <span v-if="(activeEnvId ?? activeEnv?.id) === env.id" class="env-badge">{{ t('apiDebugger.activeEnv') }}</span>
          <button
            v-if="(activeEnvId ?? activeEnv?.id) !== env.id"
            type="button"
            class="btn-ghost"
            @click="setActive(env.id)"
          >
            {{ t('apiDebugger.enableEnv') }}
          </button>
          <button type="button" class="icon-btn" :title="t('apiDebugger.environments.deleteEnv')" @click="deleteEnvironment(env)">
            <i class="fas fa-trash-alt"></i>
          </button>
        </div>

        <div class="kv-row kv-row--head">
          <span class="kv-head-label">{{ t('apiDebugger.enabled') }}</span>
          <span class="kv-head-label flex">Key</span>
          <span class="kv-head-label flex">Value</span>
          <span class="kv-head-label action"></span>
        </div>

        <div v-for="variable in env.variables" :key="variable.id" class="kv-row">
          <label class="header-toggle">
            <input
              :checked="variable.enabled"
              type="checkbox"
              class="header-toggle__input"
              @change="patchVariable(env, variable, { enabled: ($event.target as HTMLInputElement).checked })"
            />
            <span class="header-toggle__box">
              <i class="fas fa-check header-toggle__icon"></i>
            </span>
          </label>
          <input
            class="kv-input"
            :value="variable.key"
            placeholder="KEY"
            @input="patchVariable(env, variable, { key: ($event.target as HTMLInputElement).value })"
          />
          <input
            class="kv-input"
            :value="variable.value"
            placeholder="value"
            @input="patchVariable(env, variable, { value: ($event.target as HTMLInputElement).value })"
          />
          <button type="button" class="icon-btn" @click="removeVariable(env, variable.id)">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <button type="button" class="btn-ghost" @click="addVariable(env)">
          <i class="fas fa-plus"></i> {{ t('apiDebugger.environments.addVariable') }}
        </button>
      </div>
    </div>
  </section>

  <ApiDebuggerInputDialog
    :visible="showCreateDialog"
    :title="t('apiDebugger.environments.createDialogTitle')"
    :label="t('apiDebugger.environments.createDialogLabel')"
    :placeholder="t('apiDebugger.environments.createDialogPlaceholder')"
    icon="fa-sliders-h"
    initial-value="New Environment"
    @close="showCreateDialog = false"
    @confirm="confirmCreateEnvironment"
  />
</template>

<style scoped lang="scss">
@use './api-debugger-manager.scss';

.field-label {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-dim);
  white-space: nowrap;
}

.kv-row--head {
  margin-top: 4px;
}

.kv-head-label {
  font-size: 9px;
  font-weight: 600;
  color: var(--text-dim);
  flex: 0 0 32px;
  text-align: center;
  white-space: nowrap;

  &.flex {
    flex: 1;
    text-align: left;
  }

  &.action {
    flex: 0 0 28px;
  }
}

code {
  font-family: var(--font-terminal);
  color: var(--accent);
}

.env-select-wrap {
  :deep(.app-select__trigger) {
    height: 32px;
    padding: 0 32px 0 10px;
    border-radius: 7px;
    border-color: var(--border-30);
    font-size: 11px;
  }

  :deep(.app-select__arrow) {
    right: 10px;
    font-size: 9px;
  }

  :deep(.app-select.open .app-select__trigger) {
    box-shadow: 0 0 0 2px var(--accent-15);
  }

  :deep(.app-select__icon) {
    font-size: 11px;
  }
}
</style>
