/**
 * JSON 表格工具的中间表示。
 *
 * 所有格式（JSON / Markdown / CSV / Excel）都先解析成 `Table`，再由序列化器输出，
 * 因此任意两个格式之间互转只需各自实现「解析」与「序列化」。
 */

/** 单元格类型：字符串 / 数字 / 布尔 / 日期 / Excel 错误值 */
export type CellType = 's' | 'n' | 'b' | 'd' | 'e';

export interface Cell {
  t: CellType;
  /** 值；`null` 表示空单元格 */
  v: string | number | boolean | null;
  /** 日期原始序列号（Excel 1900 日期系统），仅 `t === 'd'` 时有值 */
  n?: number;
  /** 自定义数字格式，仅 `t === 'd'` 时有值（写 xlsx 时使用） */
  f?: string;
}

export interface Table {
  name: string;
  columns: string[];
  rows: Cell[][];
}

/** 候选数组路径：解析不到数据时提示用户一键填入 */
export interface ArrayPathCandidate {
  /** 形如 `$.data.rows` */
  path: string;
  /** 数组长度 */
  length: number;
  /** 元素为对象时键的数量，其余为 0 */
  keys: number;
}

/** 用户可读的转换错误（界面直接展示 message） */
export class TableError extends Error {
  candidates?: ArrayPathCandidate[];

  constructor(message: string, candidates?: ArrayPathCandidate[]) {
    super(message);
    this.name = 'TableError';
    this.candidates = candidates;
  }
}

export function emptyCell(): Cell {
  return { t: 's', v: null };
}

export function textCell(value: string): Cell {
  return { t: 's', v: value };
}

export function numberCell(value: number): Cell {
  return { t: 'n', v: value };
}

export function boolCell(value: boolean): Cell {
  return { t: 'b', v: value };
}

export function errorCell(value: string): Cell {
  return { t: 'e', v: value };
}

export function dateCell(iso: string, serial?: number, format?: string): Cell {
  const cell: Cell = { t: 'd', v: iso };
  if (serial !== undefined) cell.n = serial;
  if (format !== undefined && format !== '') cell.f = format;
  return cell;
}

export function isEmptyCell(cell: Cell): boolean {
  return cell.v === null || cell.v === '';
}

export function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** JSON 值 → 单元格（嵌套对象/数组整体 JSON 序列化，与「键为表头」约定一致） */
export function jsonToCell(value: unknown): Cell {
  if (value === null || value === undefined) return emptyCell();
  if (typeof value === 'string') return textCell(value);
  if (typeof value === 'number') {
    return Number.isFinite(value) ? numberCell(value) : textCell(String(value));
  }
  if (typeof value === 'boolean') return boolCell(value);
  return textCell(JSON.stringify(value));
}

/** `column1`、`column2` … */
export function generateColumnNames(count: number): string[] {
  return Array.from({ length: count }, (_, index) => `column${index + 1}`);
}

/** 表头规范化：空名补 `columnN`，重名追加 `_2`、`_3` */
export function uniqueColumnNames(names: string[]): string[] {
  const used = new Set<string>();
  return names.map((name, index) => {
    const base = name.trim() === '' ? `column${index + 1}` : name;
    let candidate = base;
    let suffix = 2;
    while (used.has(candidate)) {
      candidate = `${base}_${suffix}`;
      suffix += 1;
    }
    used.add(candidate);
    return candidate;
  });
}

/** 对象数组的键并集（按首次出现顺序） */
export function collectColumns(items: Record<string, unknown>[]): string[] {
  const columns: string[] = [];
  const seen = new Set<string>();
  for (const item of items) {
    for (const key of Object.keys(item)) {
      if (!seen.has(key)) {
        seen.add(key);
        columns.push(key);
      }
    }
  }
  return columns;
}

/** 行宽补齐到 `width`，缺失补空单元格 */
export function padRows(rows: Cell[][], width: number): Cell[][] {
  return rows.map((row) => {
    if (row.length === width) return row;
    const next = row.slice(0, width);
    while (next.length < width) next.push(emptyCell());
    return next;
  });
}

/** 所有单元格都为空的行 */
export function isBlankRow(row: Cell[]): boolean {
  return row.every((cell) => isEmptyCell(cell));
}
