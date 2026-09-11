<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { Keyboard } from '@lucide/vue';
import { isMac } from '@/core/platform';
import { acceleratorFromEvent, formatAccelerator } from '@/core/global-shortcut';

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const recording = ref(false);
const display = computed(() => formatAccelerator(props.modelValue, isMac));

function onKeydown(event: KeyboardEvent): void {
  event.preventDefault();
  event.stopPropagation();
  if (event.key === 'Escape') {
    recording.value = false;
    return;
  }
  if (event.key === 'Backspace' || event.key === 'Delete') {
    emit('update:modelValue', '');
    recording.value = false;
    return;
  }
  const accelerator = acceleratorFromEvent(event, isMac);
  if (accelerator) {
    emit('update:modelValue', accelerator);
    recording.value = false;
  }
}

function stop(): void {
  recording.value = false;
}

watch(recording, (value) => {
  if (value) {
    window.addEventListener('keydown', onKeydown, true);
    window.addEventListener('mousedown', stop, true);
  } else {
    window.removeEventListener('keydown', onKeydown, true);
    window.removeEventListener('mousedown', stop, true);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown, true);
  window.removeEventListener('mousedown', stop, true);
});
</script>

<template>
  <button
    type="button"
    class="flex h-9 w-64 items-center justify-between gap-2 rounded-md border border-border bg-background px-3 text-sm transition-colors hover:bg-accent/50"
    :aria-label="'全局快捷键：' + (display || '未设置')"
    @click="recording = true"
  >
    <span
      class="truncate"
      :class="
        recording
          ? 'text-muted-foreground'
          : props.modelValue
            ? 'font-mono'
            : 'text-muted-foreground'
      "
    >
      {{ recording ? '按下快捷键…' : display || '点击录制' }}
    </span>
    <Keyboard class="size-4 shrink-0 text-muted-foreground" />
  </button>
</template>
