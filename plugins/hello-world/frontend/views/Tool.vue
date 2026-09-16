<!--
  示例工具：Rust 集成大全（Tailwind 栅格居中 8 列示范）。
-->
<script setup lang="ts">
import { onActivated, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ipc } from '@/core/ipc';
import { kdb } from '@/core/db';
import { logger } from '@/core/logger';
import { desc, eq } from 'drizzle-orm';
import { useToolSettings } from '@/core/plugins';
import { helloNotes, type HelloNote } from '../schema';
import { HELLO_CONFIG_DEFAULTS, type HelloWorldConfig } from '../shared';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

/** 配置来自设置页（Settings.vue / shared.ts），此处只读 */
const config = useToolSettings<HelloWorldConfig>('hello-world', HELLO_CONFIG_DEFAULTS);

// ── 链路一：IPC 命令调用 ──────────────────────────────────────
const name = ref('');
const greeting = ref('');
const greetingLoading = ref(false);

async function callGreet(): Promise<void> {
  greetingLoading.value = true;
  try {
    // 设置页的自定义模板作为可选参数传入；空输入由 Rust 返回 AppError，
    // 经 ipc 封装抛出，未捕获时由全局处理兜底 toast（错误 6s 自动消失）
    const template = config.value.greetingTemplate.trim();
    greeting.value = await ipc<string>('hello_world_greet', {
      name: name.value,
      template: template || undefined,
    });
  } finally {
    greetingLoading.value = false;
  }
}

// ── 链路二：日志管道（原控制台页并入） ────────────────────────
function demoLog(level: 'info' | 'warn' | 'error'): void {
  const text = `手动写入的 ${level.toUpperCase()} 日志（${Date.now()}）`;
  logger[level](text);
  const hint = { description: '设置页可打开日志目录查看' };
  if (level === 'info') toast.info(text, hint);
  else if (level === 'warn') toast.warning(text, hint);
  else toast.error(text, hint);
}

// ── 链路三：SQLite 读写（Drizzle 对象化查询） ─────────────────
const notes = ref<HelloNote[]>([]);
const noteDraft = ref('');
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
  if (!content) return;
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
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 分区一：IPC 命令调用 -->
        <Panel title="IPC 命令调用">
          <p class="text-xs text-muted-foreground">
            前端 ipc() → Rust 命令 → 返回结果（空输入触发错误路径，Rust 侧 hello_world_greet）
          </p>
          <form class="flex gap-2" @submit.prevent="callGreet">
            <Input v-model="name" placeholder="输入名字" class="max-w-xs" />
            <Button type="submit" :disabled="greetingLoading">问候</Button>
          </form>
          <p
            v-if="greeting"
            class="rounded-md border border-primary/20 bg-primary/5 px-3 py-2 font-mono text-sm text-primary"
          >
            {{ greeting }}
          </p>
        </Panel>

        <!-- 分区二：日志管道 -->
        <Panel title="日志管道">
          <p class="text-xs text-muted-foreground">
            logger 各级别写入真实日志文件，级别阈值在设置页调整
          </p>
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" @click="demoLog('info')">Info 日志</Button>
            <Button variant="secondary" size="sm" @click="demoLog('warn')">警告日志</Button>
            <Button variant="secondary" size="sm" @click="demoLog('error')">错误日志</Button>
          </div>
        </Panel>

        <!-- 分区三：SQLite 读写 -->
        <Panel title="SQLite 读写">
          <p class="text-xs text-muted-foreground">
            kdb 对象化查询（hello_notes 表），迁移由 Rust 侧 hello_world 模块定义
          </p>
          <form class="flex gap-2" @submit.prevent="addNote">
            <Input v-model="noteDraft" placeholder="写点什么…" class="max-w-xs" />
            <Button type="submit" variant="secondary">添加</Button>
          </form>

          <p v-if="notesLoading" class="text-xs text-muted-foreground">加载中…</p>
          <p v-else-if="notes.length === 0" class="text-xs text-muted-foreground">
            还没有记录，添加一条试试
          </p>
          <div v-else class="divide-y divide-border/60">
            <div
              v-for="note in notes"
              :key="note.id"
              class="flex items-center justify-between gap-3 px-3 py-2 transition-colors hover:bg-accent/50"
            >
              <div class="min-w-0">
                <p class="truncate text-sm">{{ note.content }}</p>
                <p class="font-mono text-xs text-muted-foreground">
                  {{ note.createdAt }}
                </p>
              </div>
              <Button
                variant="ghost"
                size="sm"
                class="text-muted-foreground"
                @click="removeNote(note.id)"
              >
                删除
              </Button>
            </div>
          </div>
        </Panel>
      </div>
    </div>
  </ToolShell>
</template>
