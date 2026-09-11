/**
 * 命令面板开关状态（TitleBar 按钮与快捷键共用）。
 */
import { ref } from 'vue';

export const paletteOpen = ref(false);

export function openPalette(): void {
  paletteOpen.value = true;
}
