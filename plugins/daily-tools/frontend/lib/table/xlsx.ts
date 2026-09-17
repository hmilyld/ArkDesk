/**
 * Excel（.xlsx）读写：经 IPC 调用 Rust 侧 calamine / rust_xlsxwriter。
 *
 * 文本类格式（JSON / Markdown / CSV）的解析与生成都在前端，只有 xlsx 需要后端。
 */
import { ipc } from '@/core/ipc';
import { padRows, uniqueColumnNames, type Table } from './model';

export interface ReadSheetOptions {
  /** 工作表名；缺省用第一个 */
  sheet?: string;
  /** 首行是否为表头，默认 true */
  hasHeader?: boolean;
  /** 合并单元格是否用左上角的值填充，默认 false */
  fillMerged?: boolean;
  /** 是否裁除尾部全空行列，默认 true */
  trimEmpty?: boolean;
}

/** 工作簿内的工作表名 */
export function listSheets(path: string): Promise<string[]> {
  return ipc<string[]>('daily_tools_xlsx_list_sheets', { path });
}

/** 读取单个工作表为 Table（列名与行宽与文本格式解析结果保持一致） */
export async function readSheet(path: string, options: ReadSheetOptions = {}): Promise<Table> {
  const table = await ipc<Table>('daily_tools_xlsx_read', {
    path,
    sheet: options.sheet ?? null,
    hasHeader: options.hasHeader ?? true,
    fillMerged: options.fillMerged ?? false,
    trimEmpty: options.trimEmpty ?? true,
  });

  const columns = uniqueColumnNames(table.columns ?? []);
  return {
    name: table.name,
    columns,
    rows: padRows(table.rows ?? [], columns.length),
  };
}

/** 写出工作簿（每个 Table 一个工作表） */
export function writeWorkbook(path: string, tables: Table[], style = true): Promise<void> {
  return ipc<void>('daily_tools_xlsx_write', { path, sheets: tables, style });
}
