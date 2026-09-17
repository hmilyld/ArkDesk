/**
 * Table ↔ CSV（UTF-8，RFC 4180）。
 *
 * 序列化：含逗号/引号/换行的字段加引号并转义；字符串以 `= + - @` 开头时加单引号前缀，
 * 避免被 Excel 当作公式执行（可用 `preventInjection` 关闭）。
 * 解析：手写状态机，支持引号内逗号与换行。
 */
import { formatCellText, parseCellText } from './cell-text';
import { padRows, TableError, uniqueColumnNames, type Cell, type Table } from './model';
import type { TextFormatOptions } from './options';

export interface CsvTableOptions extends TextFormatOptions {
  /** 防公式注入（默认 true） */
  preventInjection?: boolean;
  /** 行分隔符，默认 CRLF（Excel 友好） */
  eol?: string;
}

export interface CsvToTablesOptions extends TextFormatOptions {
  /** 首行是否为表头，默认 true */
  hasHeader?: boolean;
  /** 是否做保守类型推断，默认 false */
  inferTypes?: boolean;
  name?: string;
}

/** Table → CSV 文本 */
export function tablesToCsv(table: Table, options: CsvTableOptions): string {
  const preventInjection = options.preventInjection ?? true;
  const eol = options.eol ?? '\r\n';

  const lines = [
    table.columns.map((name) => quoteField(guardInjection(name, 's', preventInjection))),
    ...table.rows.map((row) => {
      const cells = padRows([row], table.columns.length)[0];
      return cells.map((cell) => quoteField(fieldText(cell, options, preventInjection)));
    }),
  ];

  return lines.map((fields) => fields.join(',')).join(eol);
}

function fieldText(cell: Cell, options: TextFormatOptions, preventInjection: boolean): string {
  return guardInjection(formatCellText(cell, options), cell.t, preventInjection);
}

/** 字符串以 `= + - @` 开头时加 `'` 前缀（数字/布尔不加，避免破坏负号） */
function guardInjection(text: string, type: Cell['t'], preventInjection: boolean): string {
  if (preventInjection && type === 's' && /^[=+\-@\t\r]/.test(text)) return `'${text}`;
  return text;
}

function quoteField(text: string): string {
  if (!/[",\r\n]/.test(text)) return text;
  return `"${text.replace(/"/g, '""')}"`;
}

/** CSV 文本 → Table[]（仅一张表） */
export function csvToTables(text: string, options: CsvToTablesOptions): Table[] {
  if (text.trim() === '') throw new TableError('请输入 CSV 内容');

  const rows = parseCsvRows(text);
  while (rows.length > 0 && rows[rows.length - 1].every((field) => field === '')) rows.pop();
  if (rows.length === 0) throw new TableError('CSV 内容为空');

  const hasHeader = options.hasHeader ?? true;
  const header = hasHeader ? rows[0] : [];
  const body = hasHeader ? rows.slice(1) : rows;
  const width = body.reduce((max, row) => Math.max(max, row.length), header.length);
  const columns = uniqueColumnNames(
    Array.from({ length: width }, (_, index) => header[index] ?? '')
  );
  const cells: Cell[][] = padRows(
    body.map((row) => row.map((value) => parseCellText(value, options.inferTypes ?? false))),
    width
  );

  return [{ name: options.name ?? '数据', columns, rows: cells }];
}

/** RFC 4180 状态机：支持引号字段内的逗号、换行与 `""` 转义 */
function parseCsvRows(text: string): string[][] {
  const source = text.startsWith('\uFEFF') ? text.slice(1) : text;
  const rows: string[][] = [];
  let row: string[] = [];
  let field = '';
  let inQuotes = false;
  let cursor = 0;

  while (cursor < source.length) {
    const char = source[cursor];

    if (inQuotes) {
      if (char === '"') {
        if (source[cursor + 1] === '"') {
          field += '"';
          cursor += 2;
          continue;
        }
        inQuotes = false;
        cursor += 1;
        continue;
      }
      field += char;
      cursor += 1;
      continue;
    }

    if (char === '"' && field === '') {
      inQuotes = true;
      cursor += 1;
      continue;
    }
    if (char === ',') {
      row.push(field);
      field = '';
      cursor += 1;
      continue;
    }
    if (char === '\r' || char === '\n') {
      if (char === '\r' && source[cursor + 1] === '\n') cursor += 1;
      row.push(field);
      rows.push(row);
      row = [];
      field = '';
      cursor += 1;
      continue;
    }

    field += char;
    cursor += 1;
  }

  if (inQuotes) throw new TableError('CSV 存在未闭合的引号');
  if (field !== '' || row.length > 0) {
    row.push(field);
    rows.push(row);
  }
  return rows;
}
