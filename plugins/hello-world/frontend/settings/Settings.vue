<!--
  hello-world 设置面板：演示插件设置约定与统一的设置页 UI。
  约定要点：useToolSettings(toolId, defaults) 读写配置，修改自动持久化；
  设置项一律用 @/components/settings 的 SettingsSection / SettingsRow / SettingsField，
  保持与系统设置页视觉一致。
-->
<script setup lang="ts">
import { useToolSettings } from '@/core/plugins';
import { SettingsRow, SettingsSection } from '@/components/settings';
import { Textarea } from '@/components/ui/textarea';
import { HELLO_CONFIG_DEFAULTS, type HelloWorldConfig } from '../shared';

const config = useToolSettings<HelloWorldConfig>('hello-world', HELLO_CONFIG_DEFAULTS);
</script>

<template>
  <div class="space-y-6">
    <SettingsSection title="问候">
      <SettingsRow title="问候语模板" description="{name} 会被替换为输入的名字，留空使用默认文案">
        <Textarea
          id="greeting-template"
          v-model="config.greetingTemplate"
          class="w-72"
          :rows="2"
          :placeholder="HELLO_CONFIG_DEFAULTS.greetingTemplate"
        />
      </SettingsRow>
    </SettingsSection>
  </div>
</template>
