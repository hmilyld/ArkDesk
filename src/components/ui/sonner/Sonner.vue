<script lang="ts" setup>
import type { ToasterProps } from 'vue-sonner';

// vue-sonner v2 样式独立分包：不引入则 toast 容器失去 fixed 定位，掉到页面左下角
import 'vue-sonner/style.css';

import {
  CircleCheckIcon,
  InfoIcon,
  Loader2Icon,
  OctagonXIcon,
  TriangleAlertIcon,
  XIcon,
} from '@lucide/vue';
import { reactiveOmit } from '@vueuse/core';
import { Toaster as Sonner } from 'vue-sonner';
import { cn } from '@/lib/utils';

const props = defineProps<ToasterProps>();
const delegatedProps = reactiveOmit(props, 'class', 'toastOptions');
</script>

<template>
  <Sonner
    :class="cn('toaster group', props.class)"
    :style="{
      '--normal-bg': 'color-mix(in oklab, var(--popover) 88%, transparent)',
      '--normal-text': 'var(--popover-foreground)',
      '--normal-border': 'var(--border)',
      '--border-radius': 'var(--radius)',
      '--gray2': 'var(--border)',
      '--gray3': 'var(--border)',
      '--gray4': 'var(--border)',
      '--gray5': 'var(--border)',
      '--gray12': 'var(--popover-foreground)',
      '--shadow-opacity': '0.12',
    }"
    :toast-options="
      props.toastOptions ?? {
        classes: {
          toast: 'rounded-md backdrop-blur-md shadow-lg',
          description: '!text-muted-foreground',
        },
      }
    "
    v-bind="delegatedProps"
  >
    <template #success-icon>
      <CircleCheckIcon class="size-4" />
    </template>
    <template #info-icon>
      <InfoIcon class="size-4" />
    </template>
    <template #warning-icon>
      <TriangleAlertIcon class="size-4" />
    </template>
    <template #error-icon>
      <OctagonXIcon class="size-4" />
    </template>
    <template #loading-icon>
      <div>
        <Loader2Icon class="size-4 animate-spin" />
      </div>
    </template>
    <template #close-icon>
      <XIcon class="size-4" />
    </template>
  </Sonner>
</template>
