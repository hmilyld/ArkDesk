<!--
  网络工具设置：请求默认值、响应/历史、代理、环境变量管理。
-->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { Plus, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { useEnvironments } from '../composables/useEnvironments';
import { httpSettings } from '../settings-store';
import type { HttpSettings } from '../shared';

const settings = httpSettings;
const {
  environments,
  refresh,
  varsOf,
  createEnvironment,
  renameEnvironment,
  deleteEnvironment,
  addVar,
  updateVar,
  deleteVar,
} = useEnvironments();

const VAR_PLACEHOLDER = '{{name}}';

const selectedEnvId = ref<number | null>(null);
const envName = ref('');

const selectedVars = computed(() => varsOf(selectedEnvId.value));
const selectedEnvValue = computed({
  get: () => (selectedEnvId.value === null ? 'none' : String(selectedEnvId.value)),
  set: (value: string) => (selectedEnvId.value = value === 'none' ? null : Number(value)),
});

onMounted(async () => {
  await refresh();
  selectedEnvId.value = environments.value[0]?.id ?? null;
});

watch(
  selectedEnvId,
  (id) => {
    envName.value = environments.value.find((item) => item.id === id)?.name ?? '';
  },
  { immediate: true }
);

async function commitEnvName(): Promise<void> {
  if (selectedEnvId.value !== null && envName.value.trim()) {
    await renameEnvironment(selectedEnvId.value, envName.value.trim());
  }
}

function setNumber(key: keyof HttpSettings, value: unknown): void {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < 0) return;
  (settings.value[key] as number) = Math.floor(parsed);
}
function setBool(key: keyof HttpSettings, value: boolean | 'indeterminate'): void {
  (settings.value[key] as boolean) = value === true;
}
function setString(key: keyof HttpSettings, value: unknown): void {
  (settings.value[key] as string) = String(value);
}

async function handleCreateEnvironment(): Promise<void> {
  const id = await createEnvironment('新环境');
  selectedEnvId.value = id;
}
async function handleDeleteEnvironment(id: number): Promise<void> {
  await deleteEnvironment(id);
  if (settings.value.activeEnvironmentId === id) settings.value.activeEnvironmentId = null;
  if (selectedEnvId.value === id) selectedEnvId.value = environments.value[0]?.id ?? null;
}
</script>

<template>
  <div class="space-y-6">
    <!-- 请求默认 -->
    <section class="space-y-3">
      <h3 class="text-sm font-medium">请求默认</h3>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1">
          <Label class="text-xs">超时（毫秒）</Label>
          <Input
            type="number"
            :model-value="settings.timeoutMs"
            min="0"
            @update:model-value="(value) => setNumber('timeoutMs', value)"
          />
        </div>
        <div class="space-y-1">
          <Label class="text-xs">默认 User-Agent（留空用框架默认）</Label>
          <Input
            :model-value="settings.defaultUserAgent"
            placeholder="ArkDesk/..."
            spellcheck="false"
            @update:model-value="(value) => setString('defaultUserAgent', value)"
          />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <div>
          <Label class="text-sm">跟随重定向</Label>
          <p class="text-xs text-muted-foreground">关闭后可在响应中查看 3xx 与 Location</p>
        </div>
        <Switch
          :model-value="settings.followRedirects"
          @update:model-value="(value) => setBool('followRedirects', value)"
        />
      </div>
      <div class="flex items-center justify-between">
        <div>
          <Label class="text-sm">校验 SSL 证书</Label>
          <p class="text-xs text-muted-foreground">关闭仅用于自签证书/内网调试</p>
        </div>
        <Switch
          :model-value="settings.verifySsl"
          @update:model-value="(value) => setBool('verifySsl', value)"
        />
      </div>
    </section>

    <!-- 响应与历史 -->
    <section class="space-y-3">
      <h3 class="text-sm font-medium">响应与历史</h3>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1">
          <Label class="text-xs">响应体大小上限（KB，0 = 框架默认）</Label>
          <Input
            type="number"
            :model-value="settings.maxResponseKb"
            min="0"
            @update:model-value="(value) => setNumber('maxResponseKb', value)"
          />
        </div>
        <div class="space-y-1">
          <Label class="text-xs">历史保留条数</Label>
          <Input
            type="number"
            :model-value="settings.historyLimit"
            min="1"
            :disabled="!settings.historyEnabled"
            @update:model-value="(value) => setNumber('historyLimit', value)"
          />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <div>
          <Label class="text-sm">记录请求历史</Label>
          <p class="text-xs text-muted-foreground">每次发送后自动落库，超出上限自动裁剪</p>
        </div>
        <Switch
          :model-value="settings.historyEnabled"
          @update:model-value="(value) => setBool('historyEnabled', value)"
        />
      </div>
    </section>

    <!-- 代理 -->
    <section class="space-y-3">
      <h3 class="text-sm font-medium">代理</h3>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1">
          <Label class="text-xs">模式</Label>
          <Select
            :model-value="settings.proxyMode"
            @update:model-value="(value) => setString('proxyMode', value)"
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="global">跟随全局设置</SelectItem>
              <SelectItem value="custom">自定义</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="space-y-1">
          <Label class="text-xs">代理地址</Label>
          <Input
            :model-value="settings.proxyUrl"
            placeholder="http://127.0.0.1:7890"
            spellcheck="false"
            :disabled="settings.proxyMode !== 'custom'"
            @update:model-value="(value) => setString('proxyUrl', value)"
          />
        </div>
      </div>
    </section>

    <!-- 请求拦截 -->
    <section class="space-y-3">
      <h3 class="text-sm font-medium">请求拦截</h3>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1">
          <Label class="text-xs">代理端口（0 = 首次启动时询问）</Label>
          <Input
            type="number"
            :model-value="settings.interceptorPort"
            min="0"
            max="65535"
            @update:model-value="(value) => setNumber('interceptorPort', value)"
          />
        </div>
        <div class="space-y-1">
          <Label class="text-xs">捕获体大小上限（KB）</Label>
          <Input
            type="number"
            :model-value="settings.interceptorMaxBodyKb"
            min="1"
            :disabled="!settings.interceptorRecordBodies"
            @update:model-value="(value) => setNumber('interceptorMaxBodyKb', value)"
          />
        </div>
        <div class="space-y-1">
          <Label class="text-xs">流量条数上限</Label>
          <Input
            type="number"
            :model-value="settings.interceptorMaxFlows"
            min="1"
            @update:model-value="(value) => setNumber('interceptorMaxFlows', value)"
          />
        </div>
        <div class="space-y-1">
          <Label class="text-xs">WebSocket 每连接帧数上限</Label>
          <Input
            type="number"
            :model-value="settings.interceptorMaxWsFrames"
            min="1"
            @update:model-value="(value) => setNumber('interceptorMaxWsFrames', value)"
          />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <div>
          <Label class="text-sm">捕获请求/响应体</Label>
          <p class="text-xs text-muted-foreground">关闭后仅记录头部与元信息，降低内存占用</p>
        </div>
        <Switch
          :model-value="settings.interceptorRecordBodies"
          @update:model-value="(value) => setBool('interceptorRecordBodies', value)"
        />
      </div>
      <div class="flex items-center justify-between">
        <div>
          <Label class="text-sm">启动时自动设置系统代理</Label>
          <p class="text-xs text-muted-foreground">
            启动代理时自动把系统 HTTP(S) 代理指向本地（macOS 会弹框授权；停止时自动还原）
          </p>
        </div>
        <Switch
          :model-value="settings.interceptorAutoSystemProxy"
          @update:model-value="(value) => setBool('interceptorAutoSystemProxy', value)"
        />
      </div>
    </section>

    <!-- 环境变量 -->
    <section class="space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-medium">环境变量</h3>
        <Button variant="outline" size="sm" @click="handleCreateEnvironment">
          <Plus class="size-4" />
          新建环境
        </Button>
      </div>
      <p class="text-xs text-muted-foreground">
        在请求中以 <code>{{ VAR_PLACEHOLDER }}</code> 引用；认证等敏感值以明文保存。
      </p>

      <Select v-if="environments.length > 0" v-model="selectedEnvValue">
        <SelectTrigger class="w-64">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="env in environments" :key="env.id" :value="String(env.id)">
            {{ env.name }}
          </SelectItem>
        </SelectContent>
      </Select>

      <template v-if="selectedEnvId !== null">
        <div class="flex items-center gap-2">
          <Input
            v-model="envName"
            class="w-64"
            @blur="commitEnvName"
            @keydown.enter="commitEnvName"
          />
          <Button variant="destructive" size="sm" @click="handleDeleteEnvironment(selectedEnvId)">
            <Trash2 class="size-4" />
            删除环境
          </Button>
          <Button variant="outline" size="sm" @click="addVar(selectedEnvId)">
            <Plus class="size-4" />
            添加变量
          </Button>
        </div>

        <div class="space-y-2">
          <div v-for="row in selectedVars" :key="row.id" class="flex items-center gap-2">
            <Checkbox
              :model-value="row.enabled === 1"
              @update:model-value="
                (value) => updateVar(row.id, { enabled: value === true ? 1 : 0 })
              "
            />
            <Input
              :model-value="row.key"
              placeholder="变量名"
              spellcheck="false"
              class="w-48 font-mono text-xs"
              @update:model-value="(value) => updateVar(row.id, { key: String(value) })"
            />
            <Input
              :model-value="row.value"
              :type="row.is_secret === 1 ? 'password' : 'text'"
              placeholder="变量值"
              spellcheck="false"
              class="flex-1 font-mono text-xs"
              @update:model-value="(value) => updateVar(row.id, { value: String(value) })"
            />
            <label class="flex shrink-0 items-center gap-1 text-xs text-muted-foreground">
              <Checkbox
                :model-value="row.is_secret === 1"
                @update:model-value="
                  (value) => updateVar(row.id, { is_secret: value === true ? 1 : 0 })
                "
              />
              敏感
            </label>
            <Button variant="ghost" size="icon-sm" @click="deleteVar(row.id)">
              <Trash2 class="size-4" />
            </Button>
          </div>
          <p
            v-if="selectedVars.length === 0"
            class="py-3 text-center text-xs text-muted-foreground"
          >
            该环境暂无变量
          </p>
        </div>
      </template>
      <p v-else class="py-3 text-xs text-muted-foreground">尚未创建环境。</p>
    </section>
  </div>
</template>
