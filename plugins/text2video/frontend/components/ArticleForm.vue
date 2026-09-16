<!--
  标题 / 作者 / 正文 表单（生成页手动输入、草稿箱新建/编辑共用）。
  compact：用于弹窗（标题短、正文框内部滚动）。
-->
<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import FormRow from '@/components/native/FormRow.vue';
import type { ArticleValue } from '../shared';

const model = defineModel<ArticleValue>({ required: true });
const props = withDefaults(defineProps<{ compact?: boolean }>(), { compact: false });

function setField(key: keyof ArticleValue, value: string | number): void {
  model.value = { ...model.value, [key]: String(value) };
}
</script>

<template>
  <div class="space-y-3">
    <div
      :class="
        props.compact
          ? 'grid grid-cols-1 gap-3 sm:grid-cols-2'
          : 'grid grid-cols-1 gap-4 sm:grid-cols-3'
      "
    >
      <FormRow label="标题" :class="props.compact ? '' : 'sm:col-span-2'">
        <template #default="{ id }">
          <Input
            :id="id"
            :model-value="model.title"
            placeholder="视频标题"
            @update:model-value="(value) => setField('title', value)"
          />
        </template>
      </FormRow>
      <FormRow label="作者">
        <template #default="{ id }">
          <Input
            :id="id"
            :model-value="model.author"
            placeholder="留空则显示「佚名」"
            @update:model-value="(value) => setField('author', value)"
          />
        </template>
      </FormRow>
    </div>
    <FormRow label="正文" description="段落之间用空行分隔；正文会按标点自动分句排版。">
      <template #default="{ id }">
        <Textarea
          :id="id"
          :model-value="model.content"
          :rows="props.compact ? 10 : 14"
          :class="props.compact ? 'max-h-[45vh] min-h-40 resize-none overflow-auto' : ''"
          placeholder="段落之间用空行分隔"
          @update:model-value="(value) => setField('content', value)"
        />
      </template>
    </FormRow>
  </div>
</template>
