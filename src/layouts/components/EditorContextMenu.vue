<!--
  编辑区右键菜单：接管 input / textarea / contenteditable 的系统右键，
  仅提供剪切 / 复制 / 粘贴，屏蔽系统菜单中的查看源代码等调试项；
  剪切/复制走 execCommand（用户手势内同步有效），粘贴经 Tauri clipboard 插件
  预取文本后 execCommand 写入（await 跨用户手势会失效，故必须预取）；
  非编辑区的右键由 main.ts 全局拦截。
-->
<script setup lang="ts">
import { onUnmounted, ref } from 'vue';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import { isMac } from '@/core/platform';

const open = ref(false);
const x = ref(0);
const y = ref(0);
const canCutCopy = ref(false);
const canPaste = ref(false);

/** 打开菜单时捕获的聚焦编辑区与 selection（菜单项点击后恢复） */
let captured: HTMLElement | null = null;
let savedStart = 0;
let savedEnd = 0;
let pasteText = '';

function isEditable(target: EventTarget | null): target is HTMLElement {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target.isContentEditable
  );
}

function selectionOf(el: HTMLElement): { start: number; end: number; hasSelection: boolean } {
  if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? start;
    return { start, end, hasSelection: start !== end };
  }
  const selection = window.getSelection();
  if (!selection) return { start: 0, end: 0, hasSelection: false };
  return {
    start: 0,
    end: 0,
    hasSelection: !selection.isCollapsed && el.contains(selection.anchorNode),
  };
}

async function onContextMenu(event: MouseEvent): Promise<void> {
  const el = isEditable(event.target) ? event.target : null;
  if (!el) return;
  // capture 阶段拦断：系统菜单不弹出，main.ts 的全局拦截也不会再触发
  event.preventDefault();
  event.stopPropagation();

  captured = el;
  const { start, end, hasSelection } = selectionOf(el);
  savedStart = start;
  savedEnd = end;
  canCutCopy.value = hasSelection;
  x.value = Math.min(event.clientX, window.innerWidth - 152);
  y.value = Math.min(event.clientY, window.innerHeight - 116);
  try {
    pasteText = await readText();
  } catch {
    pasteText = '';
  }
  canPaste.value = pasteText.length > 0;
  open.value = true;
}

/** 菜单项动作：全部同步执行（保持 click 手势内 execCommand 有效） */
function exec(kind: 'cut' | 'copy' | 'paste'): void {
  const el = captured;
  open.value = false;
  if (!el) return;
  el.focus();
  if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
    el.setSelectionRange(savedStart, savedEnd);
  }
  if (kind === 'cut') document.execCommand('cut');
  if (kind === 'copy') document.execCommand('copy');
  if (kind === 'paste' && pasteText) document.execCommand('insertText', false, pasteText);
}

function onPointerDown(event: PointerEvent): void {
  if (!open.value) return;
  if (!(event.target instanceof HTMLElement) || !event.target.closest('[data-editor-menu]')) {
    open.value = false;
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (open.value && event.key === 'Escape') open.value = false;
}

document.addEventListener('contextmenu', onContextMenu, true);
document.addEventListener('pointerdown', onPointerDown, true);
document.addEventListener('keydown', onKeydown, true);

onUnmounted(() => {
  document.removeEventListener('contextmenu', onContextMenu, true);
  document.removeEventListener('pointerdown', onPointerDown, true);
  document.removeEventListener('keydown', onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      data-editor-menu
      class="fixed z-[100] min-w-36 rounded-md border border-border bg-popover p-1 shadow-md"
      :style="{ left: `${x}px`, top: `${y}px` }"
    >
      <button
        type="button"
        class="flex w-full items-center justify-between gap-4 rounded-sm px-2 py-1.5 text-left text-sm transition-colors hover:bg-accent disabled:pointer-events-none disabled:opacity-50"
        :disabled="!canCutCopy"
        @click="exec('cut')"
      >
        剪切
        <span class="text-xs text-muted-foreground">{{ isMac ? '⌘X' : 'Ctrl+X' }}</span>
      </button>
      <button
        type="button"
        class="flex w-full items-center justify-between gap-4 rounded-sm px-2 py-1.5 text-left text-sm transition-colors hover:bg-accent disabled:pointer-events-none disabled:opacity-50"
        :disabled="!canCutCopy"
        @click="exec('copy')"
      >
        复制
        <span class="text-xs text-muted-foreground">{{ isMac ? '⌘C' : 'Ctrl+C' }}</span>
      </button>
      <button
        type="button"
        class="flex w-full items-center justify-between gap-4 rounded-sm px-2 py-1.5 text-left text-sm transition-colors hover:bg-accent disabled:pointer-events-none disabled:opacity-50"
        :disabled="!canPaste"
        @click="exec('paste')"
      >
        粘贴
        <span class="text-xs text-muted-foreground">{{ isMac ? '⌘V' : 'Ctrl+V' }}</span>
      </button>
    </div>
  </Teleport>
</template>
