<!--
  标题 / 作者 / 正文 表单（生成页手动输入、草稿箱新建/编辑共用）。
  compact：用于弹窗（标题短、正文框内部滚动）。
-->
<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
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
      <div :class="props.compact ? 'space-y-1.5' : 'space-y-1.5 sm:col-span-2'">
        <Label>标题</Label>
        <Input
          :model-value="model.title"
          placeholder="视频标题"
          @update:model-value="(value) => setField('title', value)"
        />
      </div>
      <div class="space-y-1.5">
        <Label>作者</Label>
        <Input
          :model-value="model.author"
          placeholder="留空则显示「佚名」"
          @update:model-value="(value) => setField('author', value)"
        />
      </div>
    </div>
    <div class="space-y-1.5">
      <Label>正文</Label>
      <Textarea
        :model-value="model.content"
        :rows="props.compact ? 10 : 14"
        :class="props.compact ? 'max-h-[45vh] min-h-40 resize-none overflow-auto' : ''"
        placeholder="段落之间用空行分隔"
        @update:model-value="(value) => setField('content', value)"
      />
      <p class="text-xs text-muted-foreground">段落之间用空行分隔；正文会按标点自动分句排版。</p>
    </div>
  </div>
</template>
