<!--
  表单：全控件演示（span=6 居中档位示范）。
  覆盖 Input / Textarea / 分段控件 / mini 开关 / 复选框 / 滑块，
  标签与行内校验统一用 FormRow；提交后以结果区表达成功（不弹 toast）。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { kdb } from '@/core/db';
import { helloTasks } from '../schema';
import { STATUS_OPTIONS, type TaskStatus } from '../shared';
import FormRow from '@/components/native/FormRow.vue';
import Segmented from '@/components/native/Segmented.vue';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

/** 2–5 个互斥选项用分段控件（DESIGN §3）：状态 3 项、优先级 3 项 */
const STATUS_SEGMENTS = STATUS_OPTIONS.map((option) => ({
  value: option.value,
  label: option.label,
}));
const PRIORITY_SEGMENTS = [
  { value: 'high', label: '高' },
  { value: 'medium', label: '中' },
  { value: 'low', label: '低' },
];

const title = ref('');
const titleTouched = ref(false);
const status = ref<TaskStatus>('pending');
const urgent = ref(false);
const notify = ref(true);
const priority = ref<'high' | 'medium' | 'low'>('medium');
const estimate = ref([60]);
const description = ref('');
const result = ref('');

// Segmented 的 model 为 string，这里用计算属性在联合类型之间安全转换
const statusModel = computed({
  get: () => status.value,
  set: (value: string) => (status.value = value as TaskStatus),
});
const priorityModel = computed({
  get: () => priority.value,
  set: (value: string) => (priority.value = value as 'high' | 'medium' | 'low'),
});

const titleError = computed(() => {
  if (!title.value.trim()) return '任务标题不能为空';
  if (title.value.trim().length > 50) return '标题不能超过 50 个字符';
  return '';
});
/** 行内校验只在交互后呈现，避免一进页面就标红 */
const visibleTitleError = computed(() => (titleTouched.value ? titleError.value : ''));

const priorityLabel = computed(() => ({ high: '高', medium: '中', low: '低' })[priority.value]);

async function submit(): Promise<void> {
  titleTouched.value = true;
  if (titleError.value) return;
  result.value = '';
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

  // 成功用结果区表达，不弹 toast
  result.value = `已添加任务（ID: ${created?.id}） · 优先级：${priorityLabel.value} · 预估 ${estimate.value[0]}%`;

  // 重置表单
  title.value = '';
  description.value = '';
  status.value = 'pending';
  urgent.value = false;
  notify.value = true;
  priority.value = 'medium';
  estimate.value = [60];
  titleTouched.value = false;
}
</script>

<template>
  <ToolShell title="表单" description="全部表单控件演示：校验后参数化写入 hello_tasks 表">
    <div class="grid grid-cols-12">
      <form class="col-span-12 md:col-span-6 md:col-start-4" @submit.prevent="submit">
        <Panel title="任务信息" hint="hello_tasks">
          <FormRow
            label="任务标题"
            required
            description="最多 50 个字符"
            :error="visibleTitleError"
          >
            <template #default="{ id, invalid, describedBy }">
              <Input
                :id="id"
                v-model="title"
                placeholder="例如：整理周报"
                :aria-invalid="invalid"
                :aria-describedby="describedBy"
                @blur="titleTouched = true"
              />
            </template>
          </FormRow>

          <FormRow label="状态" description="2–5 个互斥选项用分段控件">
            <Segmented v-model="statusModel" :segments="STATUS_SEGMENTS" />
          </FormRow>

          <FormRow label="优先级">
            <Segmented v-model="priorityModel" :segments="PRIORITY_SEGMENTS" />
          </FormRow>

          <!-- 强调开关：mini 尺寸，开关与标题同行同区 -->
          <div>
            <div class="flex items-center gap-2">
              <Switch id="form-urgent" v-model="urgent" size="sm" />
              <Label for="form-urgent" class="text-sm">紧急优先</Label>
            </div>
            <p class="mt-1 text-xs text-muted-foreground">
              {{ urgent ? '将以「高」优先级写入' : '按上方选择的优先级写入' }}
            </p>
          </div>

          <!-- 细粒度布尔：复选框，标题在右 -->
          <div>
            <label class="flex cursor-pointer items-center gap-2 text-sm">
              <Checkbox v-model="notify" />
              写入时在标题后附加「已提醒」
            </label>
            <p class="mt-1 text-xs text-muted-foreground">细粒度布尔项用复选框，标题在右</p>
          </div>

          <FormRow label="预估进度" description="滑块仅用于粗略取值">
            <div class="flex items-center gap-3">
              <Slider v-model="estimate" :max="100" :step="5" class="w-52" />
              <span class="w-10 text-right font-mono text-xs tabular-nums text-muted-foreground">
                {{ estimate[0] }}%
              </span>
            </div>
          </FormRow>

          <FormRow label="备注（可选）">
            <Textarea v-model="description" placeholder="补充说明…" class="min-h-16" />
          </FormRow>

          <div class="flex items-center justify-end gap-3">
            <p v-if="result" role="status" class="mr-auto text-xs text-success">{{ result }}</p>
            <Button type="submit">添加任务</Button>
          </div>
        </Panel>
      </form>
    </div>
  </ToolShell>
</template>
