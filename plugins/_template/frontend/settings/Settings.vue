<!--
  可选：设置面板。
  实现后需在 plugin.json 中声明： "settings": { "entry": "frontend/settings/Settings.vue" }，
  设置页会自动出现本插件的 tab（tab 名 = plugin.json 中首个工具的 name）。
  配置读写用 useToolSettings(pluginId, defaults)（见 core/plugins/settings）。
  无设置需求的插件可删除本文件与对应声明。
-->

<template>
  <div class="space-y-3">
    <p class="text-base text-muted-foreground">此插件暂无设置项</p>
  </div>
</template>
