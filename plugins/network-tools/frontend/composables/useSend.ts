/**
 * 发送请求：loading / 响应 / 错误 / 取消 状态。
 */

import { ref } from 'vue';
import { http, type HttpSendOptions } from '@/core/http';
import { logger } from '@/core/logger';
import { errorMessage } from '../lib/error';
import type { HttpResponseView } from '../shared';

export function useSend() {
  const loading = ref(false);
  const response = ref<HttpResponseView | null>(null);
  const error = ref('');
  let activeTaskId = '';

  async function run(options: HttpSendOptions, taskId: string): Promise<HttpResponseView | null> {
    loading.value = true;
    error.value = '';
    response.value = null;
    activeTaskId = taskId;
    try {
      const result = await http.send({ ...options, taskId });
      response.value = result;
      return result;
    } catch (err) {
      error.value = errorMessage(err);
      logger.error(`HTTP 请求失败: ${error.value}`);
      return null;
    } finally {
      loading.value = false;
      activeTaskId = '';
    }
  }

  async function cancel(): Promise<void> {
    if (!activeTaskId) return;
    try {
      await http.cancel(activeTaskId);
    } catch (err) {
      logger.warn(`取消请求失败: ${errorMessage(err)}`);
    }
  }

  function reset(): void {
    response.value = null;
    error.value = '';
  }

  return { loading, response, error, run, cancel, reset };
}
