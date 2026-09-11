<script setup lang="ts">
import MainLayout from '@/layouts/MainLayout.vue';
import { Toaster } from '@/components/ui/sonner';
</script>

<template>
  <MainLayout />
  <!-- 自动消失（默认 3.5s），无关闭按钮；错误类单独延长（见 main.ts 全局错误通道） -->
  <Toaster position="top-center" :duration="3500" />
</template>
