<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from '../utils/toast.ts';
import { confirm } from '../utils/confirm.ts';
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
  toast.success('环境已创建');
};

const deleteEnvironment = async (env: ApiEnvironment) => {
  if (props.environments.length <= 1) {
    toast.warning('至少保留一个环境');
    return;
  }
  const ok = await confirm(`删除环境「${env.name}」？`, 'warning', '删除环境');
  if (!ok) return;
  const next = props.environments.filter((item) => item.id !== env.id);
  emit('update', next);
  if (props.activeEnvId === env.id) {
    emit('update:activeEnvId', next[0]?.id ?? null);
  }
  toast.success('环境已删除');
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
        <i class="fas fa-plus"></i> 新建环境
      </button>
    </div>

    <p class="hint-box">
      在 URL、Header、Body 中使用 <code v-pre>{{变量名}}</code> 引用环境变量，发送请求时自动替换。
    </p>

    <div class="env-selector">
      <span class="field-label">当前环境</span>
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
          <span v-if="(activeEnvId ?? activeEnv?.id) === env.id" class="env-badge">Active</span>
          <button
            v-if="(activeEnvId ?? activeEnv?.id) !== env.id"
            type="button"
            class="btn-ghost"
            @click="setActive(env.id)"
          >
            启用
          </button>
          <button type="button" class="icon-btn" title="删除环境" @click="deleteEnvironment(env)">
            <i class="fas fa-trash-alt"></i>
          </button>
        </div>

        <div class="kv-row kv-row--head">
          <span class="kv-head-label">启用</span>
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
          <i class="fas fa-plus"></i> 添加变量
        </button>
      </div>
    </div>
  </section>

  <ApiDebuggerInputDialog
    :visible="showCreateDialog"
    title="新建环境"
    label="环境名称"
    placeholder="例如 Production"
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
