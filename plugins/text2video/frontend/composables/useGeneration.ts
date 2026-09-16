/**
 * 生成流程的公共状态与操作：进度、日志、结果、取消、打开产物。
 * 生成页与草稿箱共用，避免重复实现。
 */
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Channel } from '@tauri-apps/api/core';
import { ipc } from '@/core/ipc';
import { openArtifact, STAGE_LABELS, type ProgressMsg, type RunSummary } from '../shared';

export function useGeneration() {
  const running = ref(false);
  const progress = ref<ProgressMsg>({ stage: 'cleaning', current: 0, total: 0, message: '' });
  const logs = ref<string[]>([]);
  const summary = ref<RunSummary | null>(null);

  const progressPercent = computed(() => {
    if (!progress.value.total) return 0;
    return Math.min(100, Math.round((progress.value.current / progress.value.total) * 100));
  });

  function pushLog(message: string): void {
    logs.value.push(message);
    if (logs.value.length > 300) logs.value.splice(0, logs.value.length - 300);
  }

  /** 开始一次生成：重置进度态并返回用于上报的 Channel */
  function begin(total: number): Channel<ProgressMsg> {
    running.value = true;
    summary.value = null;
    logs.value = [];
    progress.value = { stage: 'cleaning', current: 0, total, message: '准备中…' };
    const channel = new Channel<ProgressMsg>();
    channel.onmessage = (msg) => {
      progress.value = msg;
      pushLog(`[${STAGE_LABELS[msg.stage] ?? msg.stage}] ${msg.message}`);
    };
    return channel;
  }

  function finish(): void {
    running.value = false;
  }

  async function cancel(): Promise<void> {
    await ipc('text2video_cancel');
    toast.info('已请求取消，将在当前步骤结束后停止');
  }

  return {
    running,
    progress,
    logs,
    summary,
    progressPercent,
    begin,
    finish,
    cancel,
    openPath: openArtifact,
  };
}
