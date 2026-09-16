/**
 * JSON ↔ Table。
 *
 * - 解析：根为数组时直接取用；否则按点路径取数组。取不到时抛错并附带候选数组路径。
 * - 生成：单元格按类型还原（日期按 `dateMode`），单表输出数组，多表输出 `{ 表名: [...] }`。
 */
import { formatCellText } from './cell-text';
import {
  collectColumns,
  emptyCell,
  generateColumnNames,
  isPlainObject,
  jsonToCell,
  padRows,
  TableError,
  type Cell,
  type Table,
} from './model';
import { findArrayPaths, resolvePath } from './path';
import type { TextFormatOptions } from './options';

export interface JsonToTablesOptions {
  /** 目标数组的点路径；留空表示根节点必须是数组 */
  path?: string;
}

/** JSON 文本 → Table（根/路径解析失败时抛 TableError） */
export function jsonToTables(text: string, options: JsonToTablesOptions = {}): Table[] {
  if (text.trim() === '') throw new TableError('请输入 JSON 内容');

  let root: unknown;
  try {
    root = JSON.parse(text);
  } catch (err) {
    throw new TableError(`JSON 解析失败：${describeParseError(err, text)}`);
  }

  const path = (options.path ?? '').trim();
  if (Array.isArray(root)) return [tableFromArray(root, '数据')];

  if (path === '') {
    throw new TableError(
      'JSON 根节点不是数组：请在「数据路径」中指定数组所在位置',
      findArrayPaths(root)
    );
  }

  let value: unknown;
  try {
    value = resolvePath(root, path);
  } catch (err) {
    const reason = err instanceof Error ? err.message : String(err);
    throw new TableError(`${reason}；可尝试下列路径`, findArrayPaths(root));
  }

  if (!Array.isArray(value)) {
    throw new TableError(`路径「${path}」的结果不是数组，无法作为表格数据`, findArrayPaths(root));
  }

  return [tableFromArray(value, path)];
}

/** 数组 → Table：对象数组取键并集，数组数组按列号，其余单列 `value` */
export function tableFromArray(items: unknown[], name: string): Table {
  if (items.length === 0) return { name, columns: [], rows: [] };

  if (items.every(isPlainObject)) {
    const columns = collectColumns(items);
    const rows = items.map((item) =>
      columns.map((column) =>
        Object.prototype.hasOwnProperty.call(item, column) ? jsonToCell(item[column]) : emptyCell()
      )
    );
    return { name, columns, rows };
  }

  if (items.every(Array.isArray)) {
    const width = items.reduce((max, item) => Math.max(max, item.length), 0);
    return {
      name,
      columns: generateColumnNames(width),
      rows: padRows(
        items.map((item) => item.map(jsonToCell)),
        width
      ),
    };
  }

  return {
    name,
    columns: ['value'],
    rows: items.map((item) => [jsonToCell(item)]),
  };
}

/** Table → JSON 文本 */
export function tablesToJson(tables: Table[], options: TextFormatOptions): string {
  if (tables.length === 1) {
    return JSON.stringify(tableToRecords(tables[0], options), null, 2);
  }

  const payload: Record<string, unknown[]> = {};
  const used = new Set<string>();
  tables.forEach((table, index) => {
    let name = table.name.trim() === '' ? `sheet${index + 1}` : table.name;
    let suffix = 2;
    while (used.has(name)) {
      name = `${table.name}_${suffix}`;
      suffix += 1;
    }
    used.add(name);
    payload[name] = tableToRecords(table, options);
  });
  return JSON.stringify(payload, null, 2);
}

/** 行对象数组（列名 → JSON 值） */
export function tableToRecords(table: Table, options: TextFormatOptions): unknown[] {
  return table.rows.map((row) => {
    const record: Record<string, unknown> = {};
    table.columns.forEach((column, index) => {
      record[column] = cellToJsonValue(row[index] ?? emptyCell(), options);
    });
    return record;
  });
}

function cellToJsonValue(cell: Cell, options: TextFormatOptions): unknown {
  if (cell.v === null) return null;
  switch (cell.t) {
    case 'n':
      return cell.v;
    case 'b':
      return cell.v;
    case 'd':
      if (options.dateMode === 'serial') return cell.n ?? cell.v;
      return formatCellText(cell, options);
    default:
      return String(cell.v);
  }
}

function describeParseError(err: unknown, text: string): string {
  const message = err instanceof Error ? err.message : String(err);
  const matched = /position (\d+)/.exec(message);
  if (!matched) return message;
  const position = Number(matched[1]);
  const before = text.slice(0, position);
  const line = before.split(/\r\n|\r|\n/).length;
  const column = position - before.lastIndexOf('\n');
  return `${message}（第 ${line} 行 第 ${column} 列）`;
}
