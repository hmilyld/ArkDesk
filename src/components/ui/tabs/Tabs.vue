<script setup lang="ts">
import type { TabsRootEmits, TabsRootProps } from 'reka-ui';
import type { HTMLAttributes } from 'vue';
import { reactiveOmit } from '@vueuse/core';
import { TabsRoot, useForwardPropsEmits } from 'reka-ui';
import { cn } from '@/lib/utils';

const props = defineProps<TabsRootProps & { class?: HTMLAttributes['class'] }>();
const emits = defineEmits<TabsRootEmits>();

const delegatedProps = reactiveOmit(props, 'class');
const forwarded = useForwardPropsEmits(delegatedProps, emits);
</script>

<template>
  <TabsRoot
    v-slot="slotProps"
    data-slot="tabs"
    :data-orientation="forwarded.orientation || 'horizontal'"
    v-bind="forwarded"
    :class="
      cn(
        // reka-ui 2.10 输出 data-orientation='horizontal'（值形式），裸 data-horizontal: 变体匹配不上
        'gap-2 group/tabs flex data-[orientation=horizontal]:flex-col',
        props.class
      )
    "
  >
    <slot v-bind="slotProps" />
  </TabsRoot>
</template>
