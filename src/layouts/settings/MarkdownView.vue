<!--
  设置页 Markdown 渲染视图：关于 / 更新日志等静态内容页共用。
  内容为打包内置的 .md 文件（?raw 导入），非用户输入，无 XSS 面。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { marked } from 'marked';

const props = defineProps<{
  /** 原始 Markdown 文本（src/content/*.md?raw） */
  source: string;
}>();

const html = computed(() => marked.parse(props.source, { async: false }));
</script>

<template>
  <!-- eslint-disable-next-line vue/no-v-html —— 内容为内置静态 Markdown -->
  <div class="markdown-body" v-html="html" />
</template>

<!-- v-html 内容无 scoped 属性，用全局命名空间选择器 -->
<style>
.markdown-body {
  max-width: 42rem;
  font-size: 0.923rem;
  line-height: 1.7;
}
.markdown-body h1 {
  font-size: 1.25rem;
  font-weight: 600;
  letter-spacing: -0.01em;
  margin: 0 0 1rem;
}
.markdown-body h2 {
  font-size: 1rem;
  font-weight: 600;
  margin: 1.75rem 0 0.75rem;
  padding-bottom: 0.375rem;
  border-bottom: 1px solid var(--border);
}
.markdown-body h3 {
  font-size: 0.923rem;
  font-weight: 600;
  margin: 1.25rem 0 0.5rem;
}
.markdown-body p {
  margin: 0.5rem 0;
  color: var(--foreground);
}
.markdown-body ul,
.markdown-body ol {
  margin: 0.5rem 0;
  padding-left: 1.375rem;
}
.markdown-body ul {
  list-style: disc;
}
.markdown-body ol {
  list-style: decimal;
}
.markdown-body li {
  margin: 0.25rem 0;
}
.markdown-body li::marker {
  color: var(--muted-foreground);
}
.markdown-body a {
  color: var(--primary);
  text-underline-offset: 2px;
}
.markdown-body strong {
  font-weight: 600;
}
.markdown-body code {
  font-family: var(--font-mono);
  font-size: 0.846rem;
  background: var(--muted);
  border-radius: calc(var(--radius) - 4px);
  padding: 0.1rem 0.35rem;
}
.markdown-body pre {
  background: var(--console);
  color: var(--console-foreground);
  border-radius: var(--radius);
  padding: 0.75rem 1rem;
  overflow-x: auto;
  margin: 0.75rem 0;
}
.markdown-body pre code {
  background: transparent;
  padding: 0;
  font-size: 0.846rem;
}
.markdown-body blockquote {
  border-left: 2px solid var(--border);
  padding-left: 0.75rem;
  margin: 0.75rem 0;
  color: var(--muted-foreground);
}
.markdown-body hr {
  border: 0;
  border-top: 1px solid var(--border);
  margin: 1.5rem 0;
}
.markdown-body table {
  width: 100%;
  border-collapse: collapse;
  margin: 0.75rem 0;
  font-size: 0.846rem;
}
.markdown-body th,
.markdown-body td {
  border: 1px solid var(--border);
  padding: 0.375rem 0.625rem;
  text-align: left;
}
.markdown-body th {
  background: var(--muted);
  font-weight: 600;
}
</style>
