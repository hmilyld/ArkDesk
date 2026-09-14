<!--
  设置分组：与系统设置页一致的「小节标题 + 分组卡片」结构。
  插件设置面板应统一使用本组件，勿自造标题/卡片样式。

    <SettingsSection title="外观" description="可选说明">
      <SettingsRow title="主题">…</SettingsRow>
    </SettingsSection>

  卡片内为同一分组的多行设置；若需栅格排布的密集字段，直接在默认插槽里
  放带内边距的 grid（card 的 divide-y 只作用于直接子节点）。
-->
<script setup lang="ts">
import type { HTMLAttributes } from 'vue';

defineProps<{
  /** 小节标题；省略且无 actions 插槽时标题区整行隐藏 */
  title?: string;
  /** 标题下方的一行说明 */
  description?: string;
  /** 追加到卡片容器的类（一般无需使用） */
  class?: HTMLAttributes['class'];
}>();
</script>

<template>
  <section>
    <div v-if="title || $slots.actions" class="flex items-center justify-between gap-2 px-1 pb-1.5">
      <div class="min-w-0">
        <h2 class="text-xs font-medium text-muted-foreground">{{ title }}</h2>
        <p v-if="description" class="mt-0.5 text-xs text-muted-foreground/70">
          {{ description }}
        </p>
      </div>
      <div v-if="$slots.actions" class="flex shrink-0 items-center gap-2">
        <slot name="actions" />
      </div>
    </div>

    <div
      class="divide-y divide-border rounded-lg border border-border bg-card"
      :class="$props.class"
    >
      <slot />
    </div>

    <p v-if="$slots.footer" class="px-1 pt-1.5 text-xs text-muted-foreground">
      <slot name="footer" />
    </p>
  </section>
</template>
