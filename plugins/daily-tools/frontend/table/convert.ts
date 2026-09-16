/**
 * 文本类格式（JSON / Markdown / CSV）的解析与生成总入口。
 *
 * Excel 不在这里：它是异步 IPC，由页面在「选表 / 导出」时单独调用。
 */
import { csvToTables, tablesToCsv } from './csv';
import { jsonToTables, tablesToJson } from './json-convert';
import { markdownToTables, tablesToMarkdown } from './markdown';
import { TableError, type Table } from './model';
import type { TableFormat, TextFormatOptions } from './options';

/** 解析阶段不需要日期选项（日期只在输出时有意义） */
const EMPTY_DATE_OPTIONS: TextFormatOptions = { dateMode: 'iso', dateFormat: '' };

export interface SourceParseOptions {
  /** JSON：目标数组的点路径 */
  jsonPath?: string;
  /** Markdown / CSV：是否做保守类型推断 */
  inferTypes?: boolean;
  /** CSV：首行是否为表头 */
  hasHeader?: boolean;
  /** Markdown：是否把 `<br>` 还原为换行 */
  restoreBreaks?: boolean;
}

/** 文本源 → Table[]（xlsx 请用 `readSheet`） */
export function parseTextSource(
  format: TableFormat,
  text: string,
  options: SourceParseOptions = {}
): Table[] {
  switch (format) {
    case 'json':
      return jsonToTables(text, { path: options.jsonPath });
    case 'markdown':
      return markdownToTables(text, {
        ...EMPTY_DATE_OPTIONS,
        inferTypes: options.inferTypes,
        restoreBreaks: options.restoreBreaks,
      });
    case 'csv':
      return csvToTables(text, {
        ...EMPTY_DATE_OPTIONS,
        inferTypes: options.inferTypes,
        hasHeader: options.hasHeader,
      });
    default:
      throw new TableError('Excel 文件请先选择工作表');
  }
}

export interface TargetSerializeOptions extends TextFormatOptions {
  /** Markdown：单元格内换行是否写为 `<br>` */
  escapeNewlines?: boolean;
  /** CSV：防公式注入 */
  preventInjection?: boolean;
}

/** Table[] → 文本（多表仅 JSON 目标有意义；Markdown / CSV 由调用方传入选中的单表） */
export function serializeTextTarget(
  format: TableFormat,
  tables: Table[],
  options: TargetSerializeOptions
): string {
  switch (format) {
    case 'json':
      return tablesToJson(tables, options);
    case 'markdown':
      return tablesToMarkdown(tables[0] ?? { name: '数据', columns: [], rows: [] }, options);
    case 'csv':
      return tablesToCsv(tables[0] ?? { name: '数据', columns: [], rows: [] }, options);
    default:
      throw new TableError('Excel 目标请使用「导出 .xlsx」');
  }
}

/** 预览用：只保留前 `limit` 行 */
export function previewTable(table: Table, limit: number): Table {
  if (table.rows.length <= limit) return table;
  return { ...table, rows: table.rows.slice(0, limit) };
}
