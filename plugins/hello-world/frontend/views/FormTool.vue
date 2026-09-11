<!--
  表单：全控件演示（span=6 居中档位示范）。
  覆盖 Input / Textarea / Select / Switch / Checkbox / RadioGroup / Slider、
  行内校验与提交入库（hello_tasks），全部控件等高对齐（h-9 体系）。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { Slider } from '@/components/ui/slider';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { kdb } from '@/core/db';
import { helloTasks } from '../schema';
import { STATUS_OPTIONS } from '../shared';
import ToolShell from '@/components/tool/ToolShell.vue';

const title = ref('');
const status = ref<'pending' | 'in_progress' | 'done'>('pending');
const urgent = ref(false);
const notify = ref(true);
const priority = ref<'high' | 'medium' | 'low'>('medium');
const estimate = ref([60]);
const description = ref('');

const titleError = computed(() => {
  if (!title.value.trim()) return '任务标题不能为空';
  if (title.value.trim().length > 50) return '标题不能超过 50 个字符';
  return '';
});

const canSubmit = computed(() => !titleError.value && title.value.trim().length > 0);

const priorityLabel = computed(() => ({ high: '高', medium: '中', low: '低' })[priority.value]);

async function submit(): Promise<void> {
  if (!canSubmit.value) return;
  // 紧急开关直接提升优先级（演示「组件状态 → 写入参数」的组织方式）
  const finalPriority = urgent.value ? 'high' : priority.value;

  // .returning() 拿自增 id（proxy 驱动的 run 路径不回传 lastInsertId）
  const [created] = await kdb
    .insert(helloTasks)
    .values({
      title: notify.value ? `${title.value.trim()}（已提醒）` : title.value.trim(),
      status: status.value,
      priority: finalPriority,
    })
    .returning({ id: helloTasks.id });

  toast.success(`任务已添加（ID: ${created?.id}）`, {
    description: `优先级：${priorityLabel.value} · 预估 ${estimate.value[0]}%`,
  });

  // 重置表单
  title.value = '';
  description.value = '';
  status.value = 'pending';
  urgent.value = false;
  notify.value = true;
  priority.value = 'medium';
  estimate.value = [60];
}
</script>

<template>
  <ToolShell title="表单" description="全部表单控件演示：校验后参数化写入 hello_tasks 表">
    <div class="mx-auto grid w-full grid-cols-12">
      <form class="col-span-12 md:col-start-4 md:col-span-6 space-y-3" @submit.prevent="submit">
        <div class="divide-y divide-border/60 rounded-lg border border-border bg-card">
          <!-- 文本输入 + 校验 -->
          <div class="flex items-center justify-between gap-3 px-3 py-2.5">
            <Label for="task-title" class="shrink-0">任务标题</Label>
            <div class="w-52 space-y-1">
              <Input
                id="task-title"
                v-model="title"
                placeholder="例如：整理周报"
                :aria-invalid="!!titleError"
              />
              <p v-if="titleError" class="text-xs text-destructive">{{ titleError }}</p>
            </div>
          </div>

          <!-- 下拉选择 -->
          <div class="flex items-center justify-between gap-3 px-3 py-2.5">
            <Label for="task-status" class="shrink-0">状态</Label>
            <Select v-model="status">
              <SelectTrigger id="task-status" class="w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in STATUS_OPTIONS"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <!-- 单选组 -->
          <div class="flex items-center justify-between gap-3 px-3 py-2.5">
            <Label class="shrink-0">优先级</Label>
            <RadioGroup v-model="priority" class="flex gap-4">
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroupItem value="high" />
                高
              </label>
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroupItem value="medium" />
                中
              </label>
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroupItem value="low" />
                低
              </label>
            </RadioGroup>
          </div>

          <!-- 开关 -->
          <div class="flex items-center justify-between gap-3 px-3 py-2.5">
            <div>
              <Label class="text-sm">紧急优先</Label>
              <p class="text-xs text-muted-foreground">
                {{ urgent ? '将以「高」优先级写入' : '按上方选择的优先级写入' }}
              </p>
            </div>
            <Switch v-model="urgent" />
          </div>

          <!-- 复选框 -->
          <div class="flex items-center justify-between gap-3 px-3 py-2.5">
            <Label class="text-sm">标题附加提醒标记</Label>
            <Checkbox v-model="notify" />
          </div>

          <!-- 滑块 -->
          <div class="flex items-center justify-between gap-3 px-3 py-2.5">
            <Label class="shrink-0 text-sm">预估进度</Label>
            <div class="flex w-52 items-center gap-3">
              <Slider v-model="estimate" :max="100" :step="5" />
              <span class="w-10 text-right font-mono text-xs text-muted-foreground">
                {{ estimate[0] }}%
              </span>
            </div>
          </div>

          <!-- 多行文本 -->
          <div class="flex items-start justify-between gap-3 px-3 py-2.5">
            <Label for="task-desc" class="shrink-0 pt-1">备注（可选）</Label>
            <Textarea
              id="task-desc"
              v-model="description"
              placeholder="补充说明…"
              class="min-h-16 w-52"
            />
          </div>
        </div>

        <div class="flex justify-end">
          <Button type="submit" :disabled="!canSubmit">添加任务</Button>
        </div>
      </form>
    </div>
  </ToolShell>
</template>
