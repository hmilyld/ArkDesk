<!--
  hello-world 设置面板：演示插件设置约定。
  约定要点：useToolSettings(toolId, defaults) 读写配置，修改自动持久化。
-->
<script setup lang="ts">
import { Label } from '@/components/ui/label';
import { useToolSettings } from '@/core/plugins';
import { HELLO_CONFIG_DEFAULTS, type HelloWorldConfig } from '../shared';
import { Textarea } from '@/components/ui/textarea';

const config = useToolSettings<HelloWorldConfig>('hello-world', HELLO_CONFIG_DEFAULTS);
</script>

<template>
  <div class="space-y-3">
    <div class="flex items-center justify-between gap-4">
      <div class="min-w-0">
        <Label for="greeting-template" class="text-sm">问候语模板</Label>
        <p class="text-xs text-muted-foreground">{name} 会被替换为输入的名字，留空使用默认文案</p>
      </div>
      <Textarea
        id="greeting-template"
        v-model="config.greetingTemplate"
        class="max-w-60"
        :placeholder="HELLO_CONFIG_DEFAULTS.greetingTemplate"
      />
    </div>
  </div>
</template>
