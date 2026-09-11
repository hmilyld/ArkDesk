<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { CornerDownLeft, Search } from '@lucide/vue';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { search } from '@/core/search';
import { openPalette, paletteOpen } from '@/core/search/palette';
import { registerShortcut } from '@/core/shortcuts';

const router = useRouter();
const query = ref('');
const active = ref(0);
const results = computed(() => search(query.value));

let dispose: (() => void) | null = null;
onMounted(() => {
  dispose = registerShortcut('mod+k', openPalette);
});
onUnmounted(() => dispose?.());

watch(paletteOpen, (value) => {
  if (value) {
    query.value = '';
    active.value = 0;
  }
});
watch(results, () => {
  active.value = 0;
});

async function run(index: number): Promise<void> {
  const item = results.value[index];
  if (!item) return;
  paletteOpen.value = false;
  await item.run(router);
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    active.value = Math.min(active.value + 1, results.value.length - 1);
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    active.value = Math.max(active.value - 1, 0);
  } else if (event.key === 'Enter') {
    event.preventDefault();
    void run(active.value);
  }
}
</script>

<template>
  <Dialog v-model:open="paletteOpen">
    <DialogContent :show-close-button="false" class="gap-0 overflow-hidden p-0 sm:max-w-lg">
      <div class="flex items-center gap-2 border-b px-3">
        <Search class="size-4 shrink-0 text-muted-foreground" />
        <input
          v-model="query"
          class="h-11 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          placeholder="搜索工具、页面或命令…"
          @keydown="onKeydown"
        />
      </div>

      <div class="max-h-80 overflow-y-auto py-1">
        <button
          v-for="(item, index) in results"
          :key="item.id"
          type="button"
          class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm"
          :class="index === active ? 'bg-accent text-accent-foreground' : 'hover:bg-accent/50'"
          @mousemove="active = index"
          @click="run(index)"
        >
          <span class="min-w-0">
            <span class="block truncate">{{ item.title }}</span>
            <span v-if="item.subtitle" class="block truncate text-xs text-muted-foreground">
              {{ item.subtitle }}
            </span>
          </span>
          <span class="shrink-0 text-xs text-muted-foreground">{{ item.group }}</span>
        </button>
        <p v-if="results.length === 0" class="px-3 py-6 text-center text-sm text-muted-foreground">
          无匹配结果
        </p>
      </div>

      <div class="flex items-center gap-2 border-t px-3 py-1.5 text-[11px] text-muted-foreground">
        <CornerDownLeft class="size-3" />
        <span>打开</span>
        <span class="ml-auto">Esc 关闭</span>
      </div>
    </DialogContent>
  </Dialog>
</template>
