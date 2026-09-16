/**
 * Table ↔ GFM 管道表。
 *
 * 序列化：`|` 转义为 `\|`、单元格内换行写为 `<br>`（可关），保证表格结构不被破坏。
 * 解析：用 marked 的 table token（依赖已在仓库），还原 `<br>` 后按需做类型推断。
 */
import { marked, type Tokens } from 'marked';
import { formatCellText, parseCellText } from './cell-text';
import { padRows, TableError, uniqueColumnNames, type Table } from './model';
import type { TextFormatOptions } from './options';

export interface MarkdownTableOptions extends TextFormatOptions {
  /** 单元格内换行是否写为 `<br>`，默认 true */
  escapeNewlines?: boolean;
}

export interface MarkdownToTablesOptions extends TextFormatOptions {
  /** 是否做保守类型推断（数字 / 布尔），默认 false */
  inferTypes?: boolean;
  /** 是否把 `<br>` 还原为换行，默认 true */
  restoreBreaks?: boolean;
}

/** 仅用到 marked 表格单元格的原始文本 */
interface RawCell {
  text: string;
}

/** Table → Markdown 管道表（无列时返回空串） */
export function tablesToMarkdown(table: Table, options: MarkdownTableOptions): string {
  const escapeNewlines = options.escapeNewlines ?? true;
  if (table.columns.length === 0) return '';

  const header = table.columns.map((name) => escapeText(name, escapeNewlines));
  const separator = table.columns.map(() => '---');
  const lines = [
    `| ${header.join(' | ')} |`,
    `| ${separator.join(' | ')} |`,
    ...table.rows.map((row) => {
      const cells = padRows([row], table.columns.length)[0].map((cell) =>
        escapeText(formatCellText(cell, options), escapeNewlines)
      );
      return `| ${cells.join(' | ')} |`;
    }),
  ];

  return `${lines.join('\n')}\n`;
}

function escapeText(text: string, escapeNewlines: boolean): string {
  const normalized = text.replace(/\r\n|\r/g, '\n');
  const withBreaks = escapeNewlines ? normalized.replace(/\n/g, '<br>') : normalized;
  return withBreaks.replace(/\|/g, '\\|');
}

/** Markdown 文本 → Table[]（未找到管道表时抛 TableError） */
export function markdownToTables(text: string, options: MarkdownToTablesOptions): Table[] {
  if (text.trim() === '') throw new TableError('请输入 Markdown 表格内容');

  const tables = marked
    .lexer(text)
    .filter((token): token is Tokens.Table => token.type === 'table')
    .map((token, index) => tableFromToken(token, index, options));

  if (tables.length === 0) {
    throw new TableError(
      '未找到 Markdown 表格：需要带分隔行的 GFM 管道表，例如「| a | b |」+「| --- | --- |」'
    );
  }
  return tables;
}

function tableFromToken(
  token: Tokens.Table,
  index: number,
  options: MarkdownToTablesOptions
): Table {
  const restoreBreaks = options.restoreBreaks ?? true;
  const readCell = (cell: RawCell): string => {
    const raw = cell.text ?? '';
    return restoreBreaks ? raw.replace(/<br\s*\/?>/gi, '\n') : raw;
  };

  const header = token.header.map(readCell);
  const body = token.rows.map((row) => row.map(readCell));
  const width = body.reduce((max, row) => Math.max(max, row.length), header.length);

  const rows = padRows(
    body.map((row) => row.map((value) => parseCellText(value, options.inferTypes ?? false))),
    width
  );
  const columns = uniqueColumnNames(Array.from({ length: width }, (_, i) => header[i] ?? ''));

  return { name: `表${index + 1}`, columns, rows: columns.length === 0 ? [] : rows };
}
