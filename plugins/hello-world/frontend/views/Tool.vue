<!--
  示例工具：Rust 集成大全（12 列栅格居中 8 列示范）。
  每个功能模块用 Panel 包裹；错误就地呈现并提供重试，演示用提示不再走 toast。
-->
<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue';
import { NotebookPen, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ipc } from '@/core/ipc';
import { kdb } from '@/core/db';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import { desc, eq } from 'drizzle-orm';
import { useToolSettings } from '@/core/plugins';
import { helloNotes, type HelloNote } from '../schema';
import { HELLO_CONFIG_DEFAULTS, type HelloWorldConfig } from '../shared';
import EmptyState from '@/components/native/EmptyState.vue';
import ErrorState from '@/components/native/ErrorState.vue';
import FormRow from '@/components/native/FormRow.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

/** 配置来自设置页（Settings.vue / shared.ts），此处只读 */
const config = useToolSettings<HelloWorldConfig>('hello-world', HELLO_CONFIG_DEFAULTS);

// ── 链路一：IPC 命令调用 ──────────────────────────────────────
const name = ref('');
const greeting = ref('');
const greetingError = ref('');
const greetingLoading = ref(false);
const greetingErrorMessage = computed(() =>
  greetingError.value ? `${greetingError.value}，请输入名字后重试。` : ''
);

async function callGreet(): Promise<void> {
  greetingLoading.value = true;
  greetingError.value = '';
  greeting.value = '';
  try {
    // 设置页的自定义模板作为可选参数传入；空输入由 Rust 返回 AppError，
    // 此处就地呈现并给出重试入口（错误三分法：可定位错误不弹 toast）
    const template = config.value.greetingTemplate.trim();
    greeting.value = await ipc<string>('hello_world_greet', {
      name: name.value,
      template: template || undefined,
    });
  } catch (err) {
    greetingError.value = normalizeError(err).message;
  } finally {
    greetingLoading.value = false;
  }
}

// ── 链路二：日志管道（原控制台页并入） ────────────────────────
const lastLog = ref('');

function demoLog(level: 'info' | 'warn' | 'error'): void {
  const text = `手动写入的 ${level.toUpperCase()} 日志（${Date.now()}）`;
  logger[level](text);
  lastLog.value = text;
}

// ── 链路三：SQLite 读写（Drizzle 对象化查询） ─────────────────
const notes = ref<HelloNote[]>([]);
const noteDraft = ref('');
const noteError = ref('');
const notesLoading = ref(false);

async function loadNotes(): Promise<void> {
  notesLoading.value = true;
  try {
    notes.value = await kdb.select().from(helloNotes).orderBy(desc(helloNotes.id));
  } finally {
    notesLoading.value = false;
  }
}

async function addNote(): Promise<void> {
  const content = noteDraft.value.trim();
  if (!content) {
    noteError.value = '记录内容不能为空';
    return;
  }
  noteError.value = '';
  await kdb.insert(helloNotes).values({ content });
  noteDraft.value = '';
  await loadNotes();
}

async function removeNote(id: number): Promise<void> {
  await kdb.delete(helloNotes).where(eq(helloNotes.id, id));
  await loadNotes();
}

onMounted(loadNotes);
// KeepAlive 场景：从其他工具切回时刷新数据（与 TableTool 一致的模式）
onActivated(loadNotes);
</script>

<template>
  <ToolShell
    title="示例工具"
    description="Rust 集成大全：IPC 命令、日志管道、SQLite 读写——插件开发的最小完整链路"
  >
    <div class="grid grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 分区一：IPC 命令调用（错误就地呈现 + 重试） -->
        <Panel title="IPC 命令调用" hint="hello_world_greet">
          <p class="text-xs text-muted-foreground">
            前端 ipc() → Rust 命令 → 返回结果；留空提交会触发 Rust 侧参数校验，错误在此就地展示
          </p>
          <FormRow label="名字" description="将替换设置页问候语模板里的 {name} 占位符">
            <template #default="{ id }">
              <div class="flex gap-2">
                <Input
                  :id="id"
                  v-model="name"
                  placeholder="例如：小明"
                  class="max-w-xs"
                  @keydown.enter.prevent="callGreet"
                />
                <Button :disabled="greetingLoading" @click="callGreet">
                  {{ greetingLoading ? '问候中…' : '问候' }}
                </Button>
              </div>
            </template>
          </FormRow>
          <ErrorState
            v-if="greetingErrorMessage"
            :message="greetingErrorMessage"
            :on-retry="callGreet"
          />
          <p
            v-else-if="greeting"
            class="rounded-md border bg-muted px-3 py-2 font-mono text-sm break-words"
          >
            {{ greeting }}
          </p>
        </Panel>

        <!-- 分区二：日志管道（写入真实日志文件，结果以状态文本反馈） -->
        <Panel title="日志管道" hint="logger">
          <p class="text-xs text-muted-foreground">
            logger 各级别写入真实日志文件，级别阈值在设置页调整
          </p>
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" @click="demoLog('info')">Info 日志</Button>
            <Button variant="secondary" size="sm" @click="demoLog('warn')">警告日志</Button>
            <Button variant="secondary" size="sm" @click="demoLog('error')">错误日志</Button>
          </div>
          <p v-if="lastLog" class="font-mono text-xs break-all text-muted-foreground">
            最近写入：{{ lastLog }}
          </p>
        </Panel>

        <!-- 分区三：SQLite 读写（三态齐全：加载 / 空 / 列表） -->
        <Panel title="SQLite 读写" hint="hello_notes">
          <p class="text-xs text-muted-foreground">
            kdb 对象化查询（hello_notes 表），迁移由 Rust 侧 hello-world 模块定义
          </p>
          <FormRow label="新记录" description="写入后按自增 id 倒序展示" :error="noteError">
            <template #default="{ id, invalid, describedBy }">
              <div class="flex gap-2">
                <Input
                  :id="id"
                  v-model="noteDraft"
                  placeholder="写点什么…"
                  class="max-w-xs"
                  :aria-invalid="invalid"
                  :aria-describedby="describedBy"
                  @keydown.enter.prevent="addNote"
                />
                <Button variant="secondary" @click="addNote">添加</Button>
              </div>
            </template>
          </FormRow>

          <LoadingState v-if="notesLoading" :rows="3" />
          <EmptyState
            v-else-if="notes.length === 0"
            :icon="NotebookPen"
            title="还没有记录"
            description="在上方输入一条内容，写入 hello_notes 表"
          />
          <div v-else class="divide-y divide-border">
            <div
              v-for="note in notes"
              :key="note.id"
              class="flex items-center justify-between gap-3 py-2"
            >
              <div class="min-w-0">
                <p class="truncate text-sm">{{ note.content }}</p>
                <p class="font-mono text-xs tabular-nums text-muted-foreground">
                  {{ note.createdAt }}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon-sm"
                class="shrink-0 text-muted-foreground hover:text-destructive"
                title="删除记录"
                aria-label="删除记录"
                @click="removeNote(note.id)"
              >
                <Trash2 class="size-4" />
              </Button>
            </div>
          </div>
        </Panel>
      </div>
    </div>
  </ToolShell>
</template>
