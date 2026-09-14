<!--
  可选：设置面板。
  实现后需在 plugin.json 中声明： "settings": { "entry": "frontend/settings/Settings.vue" }，
  设置页会自动出现本插件的 tab（tab 名 = plugin.json 中首个工具的 name）。

  UI 统一使用 @/components/settings 的 SettingsSection / SettingsRow / SettingsField，
  与系统设置页保持一致，勿自造标题 / 卡片样式：
    - 分组：<SettingsSection title="…">（可加 description / #actions / #footer）
    - 单行设置：<SettingsRow title="…" description="…"> 右侧放控件
    - 栅格密集字段：在 <SettingsSection> 内放 <div class="grid … gap-3 p-3"> + <SettingsField>

  配置读写用 useToolSettings(pluginId, defaults)（见 core/plugins/settings）。
  无设置需求的插件可删除本文件与对应声明。
-->
<script setup lang="ts">
import { SettingsRow, SettingsSection } from '@/components/settings';
// import { useToolSettings } from '@/core/plugins';
// import { Input } from '@/components/ui/input';
// import { MY_PLUGIN_DEFAULTS, type MyPluginConfig } from '../shared';
//
// const config = useToolSettings<MyPluginConfig>('my-plugin', MY_PLUGIN_DEFAULTS);
</script>

<template>
  <div class="space-y-6">
    <SettingsSection title="分组标题">
      <SettingsRow title="示例设置项" description="一行说明文字">
        <!-- 右侧控件，如：<Input v-model="config.value" class="w-72" /> -->
        <span class="text-sm text-muted-foreground">此插件暂无设置项</span>
      </SettingsRow>
    </SettingsSection>
  </div>
</template>
