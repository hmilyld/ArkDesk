/**
 * 加解密面板统一调用封装：ipc + loading/error 状态。
 *
 * 带请求序号：乱序返回的旧请求不会覆盖新结果（文本类面板有防抖多次触发）。
 */
import { ref, type Ref } from 'vue';
import { ipc, type CommandName } from '@/core/ipc';
import { errorMessage } from '../shared';

export function useCryptoCall<T>(command: CommandName) {
  const result = ref<T | null>(null) as Ref<T | null>;
  const error = ref('');
  const loading = ref(false);
  let seq = 0;

  async function run(args: Record<string, unknown>): Promise<void> {
    const current = ++seq;
    loading.value = true;
    error.value = '';
    try {
      const value = await ipc<T>(command, args);
      if (current === seq) result.value = value;
    } catch (err) {
      if (current === seq) {
        error.value = errorMessage(err);
        result.value = null;
      }
    } finally {
      if (current === seq) loading.value = false;
    }
  }

  function reset(): void {
    seq += 1;
    result.value = null;
    error.value = '';
    loading.value = false;
  }

  return { result, error, loading, run, reset };
}
