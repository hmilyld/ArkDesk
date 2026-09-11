/**
 * 数据维护共享：类型、标识符白名单、SQL 构建、单元格值解析。
 *
 * 全部 SQL 走 core/db 的 db.select / db.execute（$1 占位符参数化）；
 * 表名/列名是标识符无法参数化，校验 + 双引号包裹防注入
 * （与 core/db 的 IDENTIFIER_RE 同一套白名单规则，core 未导出故在此自建）。
 */
import { ErrorCode } from '@/core/errors';

export interface TableInfo {
  name: string;
  rowCount: number;
}

export interface ColumnInfo {
  cid: number;
  name: string;
  /** 声明类型（SQLite 动态类型，仅亲和性提示，可能为空串） */
  type: string;
  notnull: number;
  /** 默认值表达式文本，无默认值为 null */
  dfltValue: string | null;
  /** 联合主键中的位置（1 起；0 = 非主键） */
  pk: number;
}

export interface TableSchemaInfo {
  name: string;
  columns: ColumnInfo[];
  ddl: string | null;
  rowCount: number;
}

/** pragma_table_info 原始行（notnull 列已别名为 not_null，规避 SQLite 关键字） */
export interface PragmaColumnRow {
  cid: number;
  name: string;
  type: string | null;
  not_null: number;
  dfltValue: string | null;
  pk: number;
}

/** 原始行 → ColumnInfo（声明类型可能为 null，数值列做兜底转换） */
export function toColumnInfo(row: PragmaColumnRow): ColumnInfo {
  return {
    cid: Number(row.cid) || 0,
    name: row.name,
    type: row.type ?? '',
    notnull: Number(row.not_null) || 0,
    dfltValue: row.dfltValue,
    pk: Number(row.pk) || 0,
  };
}

/** 行数据：列名 → 值（rowid 可用时额外含 __rid 行标识，用于定位编辑/删除） */
export type DataRow = Record<string, unknown>;

export interface InsertEntry {
  column: string;
  value: unknown;
}

export interface SqlStatement {
  sql: string;
  params: unknown[];
}

/** 每页行数（数据分页） */
export const PAGE_SIZE = 50;

// ─────────────────────────── 标识符 ───────────────────────────

const IDENTIFIER_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

function invalid(label: string, name: string): Error {
  return Object.assign(new Error(`非法的${label}: ${name}`), { code: ErrorCode.InvalidInput });
}

function quoteIdentifier(name: string, label: string): string {
  if (!IDENTIFIER_RE.test(name)) throw invalid(label, name);
  return `"${name}"`;
}

function throwIfInvalid(name: string, label: string): void {
  if (!IDENTIFIER_RE.test(name)) throw invalid(label, name);
}

// ─────────────────────────── SQL 构建 ───────────────────────────

/** 表清单（排除 sqlite_ 内部表；表名来自 sqlite_master 数据而非标识符，可参数化） */
export function sqlTableList(): SqlStatement {
  return {
    sql: `SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' ORDER BY name`,
    params: [],
  };
}

export function sqlRowCount(table: string): SqlStatement {
  return { sql: `SELECT COUNT(*) AS n FROM ${quoteIdentifier(table, '表名')}`, params: [] };
}

/** 表结构（pragma_table_info 表值函数支持参数化，返回 cid/name/type/notnull/dflt_value/pk）。
 *  notnull 是 SQLite 关键字（后缀操作符，大小写不敏感），裸列名与裸别名均会语法错误：
 *  列名须加引号，别名须换成 not_null 这类非关键字 */
export function sqlColumns(table: string): SqlStatement {
  return {
    sql: `SELECT cid, name, type, "notnull" AS not_null, dflt_value AS dfltValue, pk FROM pragma_table_info($1)`,
    params: [table],
  };
}

/** 建表 DDL 原文 */
export function sqlDdl(table: string): SqlStatement {
  return {
    sql: `SELECT sql FROM sqlite_master WHERE type = 'table' AND name = $1`,
    params: [table],
  };
}

/** rowid 可用性探测（WITHOUT ROWID 表会报错，降级只读） */
export function sqlRowidProbe(table: string): SqlStatement {
  return { sql: `SELECT rowid FROM ${quoteIdentifier(table, '表名')} LIMIT 1`, params: [] };
}

/** 分页取数：rowid 别名 __rid 用于行定位；排序稳定分页 */
export function sqlPage(table: string, limit: number, offset: number): SqlStatement {
  return {
    sql: `SELECT rowid AS __rid, * FROM ${quoteIdentifier(table, '表名')} ORDER BY rowid LIMIT $1 OFFSET $2`,
    params: [limit, offset],
  };
}

/** 分页取数（无 rowid 表：只读降级） */
export function sqlPageNoRowid(table: string, limit: number, offset: number): SqlStatement {
  return {
    sql: `SELECT * FROM ${quoteIdentifier(table, '表名')} LIMIT $1 OFFSET $2`,
    params: [limit, offset],
  };
}

export function sqlUpdateCell(
  table: string,
  column: string,
  value: unknown,
  rowid: unknown
): SqlStatement {
  return {
    sql: `UPDATE ${quoteIdentifier(table, '表名')} SET ${quoteIdentifier(column, '列名')} = $1 WHERE rowid = $2`,
    params: [value, rowid],
  };
}

/** 整行更新（entries 为发生变更的列，rowid 定位行） */
export function sqlUpdateRow(table: string, entries: InsertEntry[], rowid: unknown): SqlStatement {
  if (entries.length === 0) throw invalid('更新内容', '（至少修改一列）');
  const setClause = entries
    .map((entry, i) => `${quoteIdentifier(entry.column, '列名')} = $${i + 1}`)
    .join(', ');
  return {
    sql: `UPDATE ${quoteIdentifier(table, '表名')} SET ${setClause} WHERE rowid = $${entries.length + 1}`,
    params: [...entries.map((entry) => entry.value), rowid],
  };
}

export function sqlDeleteRow(table: string, rowid: unknown): SqlStatement {
  return {
    sql: `DELETE FROM ${quoteIdentifier(table, '表名')} WHERE rowid = $1`,
    params: [rowid],
  };
}

export function sqlInsertRow(table: string, entries: InsertEntry[]): SqlStatement {
  if (entries.length === 0) throw invalid('插入内容', '（至少填写一列）');
  const columns = entries.map((entry) => quoteIdentifier(entry.column, '列名')).join(', ');
  const placeholders = entries.map((_, index) => `$${index + 1}`).join(', ');
  return {
    sql: `INSERT INTO ${quoteIdentifier(table, '表名')} (${columns}) VALUES (${placeholders})`,
    params: entries.map((entry) => entry.value),
  };
}

// ─────────────────────────── 值处理 ───────────────────────────

const NUMERIC_TYPE_RE = /INT|REAL|FLOA|DOUB|DEC|NUM|BOOL/i;
const BLOB_TYPE_RE = /BLOB/i;

export function isBlobColumn(column: ColumnInfo): boolean {
  return BLOB_TYPE_RE.test(column.type);
}

/** INTEGER 单列主键是 rowid 别名，插入留空自动赋值 */
export function isRowidAlias(column: ColumnInfo): boolean {
  return column.pk === 1 && /^INTEGER$/i.test(column.type);
}

/** 校验列名（新增行对话框提交前调用，提前拦截不合法标识符） */
export function assertColumnName(name: string): void {
  throwIfInvalid(name, '列名');
}

/** 内联编辑提交值：NULL 开关优先；数值亲和列可解析时转数字，其余按文本（交由 SQLite 亲和性转换） */
export function parseCellValue(
  raw: string,
  isNull: boolean,
  column: ColumnInfo
): number | string | null {
  if (isNull) return null;
  if (NUMERIC_TYPE_RE.test(column.type)) {
    const parsed = Number(raw);
    if (raw.trim() !== '' && Number.isFinite(parsed)) return parsed;
  }
  return raw;
}
