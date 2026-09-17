<!--
  表单行（docs/design.md §4）：标签 + 可选说明 + 控件 + **行内校验**。
  校验就地呈现（控件下方 text-xs text-destructive），并带上 aria 关联；不要只在提交时用 toast 报错。
  行内小动作（清空/插入变量等）放 `#label-action` 插槽。

    <FormRow v-model:error="nameError" label="公司名称" required>
      <template #default="{ id, invalid, describedBy }">
        <Input :id="id" :aria-invalid="invalid" :aria-describedby="describedBy" v-model="name" />
      </template>
    </FormRow>
-->
<script setup lang="ts">
import { computed, useId } from 'vue';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

const props = withDefaults(
  defineProps<{
    label: string;
    /** 标签下方的补充说明（什么时候填、格式要求） */
    description?: string;
    /** 校验错误：非空即进入错误态 */
    error?: string;
    required?: boolean;
    class?: string;
  }>(),
  { description: undefined, error: undefined, required: false, class: undefined }
);

const id = useId();
const errorId = `${id}-error`;
const invalid = computed(() => Boolean(props.error));
</script>

<template>
  <div :class="cn('space-y-1.5', props.class)">
    <div class="flex items-center justify-between gap-2">
      <Label :for="id">
        {{ label }}
        <span v-if="required" class="text-destructive" aria-hidden="true">*</span>
      </Label>
      <slot name="label-action" />
    </div>
    <p v-if="description" class="text-xs text-muted-foreground">{{ description }}</p>
    <slot :id="id" :invalid="invalid" :described-by="invalid ? errorId : undefined" />
    <p v-if="error" :id="errorId" class="text-xs text-destructive">{{ error }}</p>
  </div>
</template>
