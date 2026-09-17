/**
 * 组合式函数示例（需要响应式/生命周期时放本目录）。
 * 纯逻辑请不要放这里，放 `frontend/lib/`。
 */
import { computed, ref } from 'vue';

export function useCounter(initial = 0) {
  const count = ref(initial);
  const doubled = computed(() => count.value * 2);
  function increment(): void {
    count.value += 1;
  }
  function reset(): void {
    count.value = initial;
  }
  return { count, doubled, increment, reset };
}
