/**
 * JSON 制表工具的前端选项。
 */

/** 支持的表格格式 */
export type TableFormat = 'json' | 'markdown' | 'csv' | 'xlsx';

export const TABLE_FORMATS: TableFormat[] = ['json', 'markdown', 'csv', 'xlsx'];

export const FORMAT_LABELS: Record<TableFormat, string> = {
  json: 'JSON',
  markdown: 'Markdown 表格',
  csv: 'CSV',
  xlsx: 'Excel（.xlsx）',
};

/** 日期输出形态 */
export type DateMode = 'iso' | 'serial' | 'custom';

export const DATE_MODES: { value: DateMode; label: string }[] = [
  { value: 'iso', label: 'ISO 8601' },
  { value: 'serial', label: 'Excel 序列号' },
  { value: 'custom', label: '自定义格式' },
];

export const DEFAULT_DATE_FORMAT = 'yyyy-MM-dd HH:mm:ss';

/** 文本类格式（Markdown / CSV / JSON）共用的输出选项 */
export interface TextFormatOptions {
  dateMode: DateMode;
  dateFormat: string;
}

/** 预览最多渲染的行数（导出始终全量） */
export const PREVIEW_ROWS = 200;

/** xlsx 读写上限（与后端 `xlsx::limits` 对齐） */
export const MAX_ROWS = 200_000;
export const MAX_COLS = 2048;

/** 候选数组路径扫描范围 */
export const PATH_MAX_DEPTH = 6;
export const PATH_LIMIT = 50;
