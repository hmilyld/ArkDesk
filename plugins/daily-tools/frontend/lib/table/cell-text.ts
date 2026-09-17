/**
 * 单元格 ↔ 文本。
 *
 * 文本 → 单元格：默认不做类型推断（一律字符串），开启后按保守规则识别数字与布尔；
 * 单元格 → 文本：按日期模式渲染，供 Markdown / CSV / JSON 输出复用。
 */
import { formatDateValue } from './dates';
import { boolCell, emptyCell, numberCell, textCell, type Cell } from './model';
import type { TextFormatOptions } from './options';

/** 整数位超过 15 位会超出 IEEE754 精确范围，保留原文本以免精度静默丢失 */
const MAX_INT_DIGITS = 15;

const NUMBER_PATTERN = /^-?(?:0|[1-9]\d*)(?:\.\d+)?$/;

/**
 * 文本 → 单元格。
 *
 * `inferTypes` 为 true 时（保守规则）：
 * - 空串 → 空单元格
 * - `true` / `false`（不区分大小写）→ 布尔
 * - 普通十进制数字（无前导零、无 `NaN`/`Infinity`/`0x`/科学计数法、整数位 ≤ 15）→ 数字
 * - 其余 → 原样字符串
 */
export function parseCellText(text: string, inferTypes = false): Cell {
  if (text === '') return emptyCell();
  if (!inferTypes) return textCell(text);

  const trimmed = text.trim();
  if (/^(?:true|false)$/i.test(trimmed)) return boolCell(trimmed.toLowerCase() === 'true');
  if (NUMBER_PATTERN.test(trimmed) && isPreciseNumber(trimmed)) return numberCell(Number(trimmed));
  return textCell(text);
}

function isPreciseNumber(text: string): boolean {
  const digits = text.replace('-', '').split('.')[0];
  return digits.length <= MAX_INT_DIGITS;
}

/** 单元格 → 文本（日期按 `dateMode` 渲染） */
export function formatCellText(cell: Cell, options: TextFormatOptions): string {
  if (cell.v === null) return '';
  switch (cell.t) {
    case 'b':
      return cell.v ? 'true' : 'false';
    case 'n':
      return String(cell.v);
    case 'd': {
      if (options.dateMode === 'serial') {
        return cell.n === undefined ? String(cell.v) : String(cell.n);
      }
      if (options.dateMode === 'custom') {
        return formatDateValue(String(cell.v), options.dateFormat);
      }
      return String(cell.v);
    }
    default:
      return String(cell.v);
  }
}
