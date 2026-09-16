import { describe, expect, it } from 'vitest';
import { formatCellText, parseCellText } from '../plugins/daily-tools/frontend/table/cell-text';
import {
  parseTextSource,
  previewTable,
  serializeTextTarget,
} from '../plugins/daily-tools/frontend/table/convert';
import { csvToTables, tablesToCsv } from '../plugins/daily-tools/frontend/table/csv';
import { formatDateValue } from '../plugins/daily-tools/frontend/table/dates';
import { jsonToTables, tablesToJson } from '../plugins/daily-tools/frontend/table/json-convert';
import { markdownToTables, tablesToMarkdown } from '../plugins/daily-tools/frontend/table/markdown';
import {
  boolCell,
  dateCell,
  emptyCell,
  TableError,
  uniqueColumnNames,
  type Table,
} from '../plugins/daily-tools/frontend/table/model';
import { findArrayPaths, resolvePath } from '../plugins/daily-tools/frontend/table/path';

const ISO_OPTIONS = { dateMode: 'iso', dateFormat: 'yyyy-MM-dd HH:mm:ss' } as const;

describe('uniqueColumnNames', () => {
  it('fills blanks and de-duplicates', () => {
    expect(uniqueColumnNames(['a', '', 'a', 'a'])).toEqual(['a', 'column2', 'a_2', 'a_3']);
  });
});

describe('parseCellText', () => {
  it('keeps everything as text when inference is off', () => {
    expect(parseCellText('12')).toEqual({ t: 's', v: '12' });
    expect(parseCellText('true')).toEqual({ t: 's', v: 'true' });
  });

  it('infers numbers and booleans conservatively', () => {
    expect(parseCellText('12', true)).toEqual({ t: 'n', v: 12 });
    expect(parseCellText(' -3.5 ', true)).toEqual({ t: 'n', v: -3.5 });
    expect(parseCellText('TRUE', true)).toEqual({ t: 'b', v: true });
    expect(parseCellText('', true)).toEqual({ t: 's', v: null });
  });

  it('refuses ambiguous numbers and unsafe precision', () => {
    for (const value of ['007', '1e5', '0x10', 'NaN', 'Infinity', '1,000', '12.']) {
      expect(parseCellText(value, true)).toEqual({ t: 's', v: value });
    }
    expect(parseCellText('12345678901234567890', true).t).toBe('s');
  });
});

describe('formatCellText / formatDateValue', () => {
  const date = dateCell('2024-03-05T08:30:05.250', 45356.3542);

  it('renders dates per mode', () => {
    expect(formatCellText(date, ISO_OPTIONS)).toBe('2024-03-05T08:30:05.250');
    expect(formatCellText(date, { dateMode: 'serial', dateFormat: '' })).toBe('45356.3542');
    expect(
      formatCellText(date, { dateMode: 'custom', dateFormat: 'yyyy/MM/dd HH:mm:ss.SSS' })
    ).toBe('2024/03/05 08:30:05.250');
  });

  it('renders other cell types', () => {
    expect(formatCellText(emptyCell(), ISO_OPTIONS)).toBe('');
    expect(formatCellText(boolCell(false), ISO_OPTIONS)).toBe('false');
  });

  it('formats date-only patterns and leaves unknown input untouched', () => {
    expect(formatDateValue('2024-01-02', 'yyyy年M月d日')).toBe('2024年1月2日');
    expect(formatDateValue('不是日期', 'yyyy-MM-dd')).toBe('不是日期');
  });
});

describe('JSON → Table', () => {
  it('uses the key union as the header and keeps first-seen order', () => {
    const tables = jsonToTables(
      JSON.stringify([
        { a: 1, b: 'x' },
        { b: 'y', c: true },
      ])
    );
    expect(tables).toHaveLength(1);
    expect(tables[0].columns).toEqual(['a', 'b', 'c']);
    expect(tables[0].rows[0]).toEqual([
      { t: 'n', v: 1 },
      { t: 's', v: 'x' },
      { t: 's', v: null },
    ]);
    expect(tables[0].rows[1][2]).toEqual({ t: 'b', v: true });
  });

  it('serialises nested objects and arrays into a single cell', () => {
    const tables = jsonToTables(JSON.stringify([{ a: { b: 1 }, c: [1, 2] }]));
    expect(tables[0].rows[0]).toEqual([
      { t: 's', v: '{"b":1}' },
      { t: 's', v: '[1,2]' },
    ]);
  });

  it('handles arrays of arrays and arrays of scalars', () => {
    const matrix = jsonToTables(JSON.stringify([[1, 2], [3]]))[0];
    expect(matrix.columns).toEqual(['column1', 'column2']);
    expect(matrix.rows[1][1]).toEqual({ t: 's', v: null });

    const scalars = jsonToTables(JSON.stringify([1, 'a', null]))[0];
    expect(scalars.columns).toEqual(['value']);
    expect(scalars.rows).toEqual([[{ t: 'n', v: 1 }], [{ t: 's', v: 'a' }], [{ t: 's', v: null }]]);
  });

  it('reports an empty array as an empty table', () => {
    const tables = jsonToTables('[]');
    expect(tables[0]).toEqual({ name: '数据', columns: [], rows: [] });
  });

  it('extracts the array at a dot path and one level of index', () => {
    const payload = JSON.stringify({ data: { list: [{ a: 1 }, { a: 2 }] } });
    expect(jsonToTables(payload, { path: 'data.list' })[0].rows).toHaveLength(2);
    expect(jsonToTables(payload, { path: '$.data.list' })[0].rows).toHaveLength(2);
    expect(jsonToTables(payload, { path: 'data.list[*]' })[0].rows).toHaveLength(2);

    const nested = JSON.stringify({ items: [{ rows: [{ x: 9 }] }] });
    expect(jsonToTables(nested, { path: 'items[0].rows' })[0].rows).toEqual([[{ t: 'n', v: 9 }]]);
  });

  it('suggests candidate paths when the root is not an array', () => {
    const payload = JSON.stringify({ meta: { total: 2 }, data: { rows: [{ a: 1 }] }, list: [] });
    try {
      jsonToTables(payload);
      throw new Error('应当抛出 TableError');
    } catch (err) {
      expect(err).toBeInstanceOf(TableError);
      // 浅层优先
      expect((err as TableError).candidates?.map((item) => item.path)).toEqual([
        '$.list',
        '$.data.rows',
      ]);
    }
  });

  it('fails with candidates when the path is wrong or not an array', () => {
    const payload = JSON.stringify({ data: { rows: [{ a: 1 }] }, other: 1 });
    expect(() => jsonToTables(payload, { path: 'data.nope' })).toThrowError(/找不到路径/);
    expect(() => jsonToTables(payload, { path: 'other' })).toThrowError(/不是数组/);

    try {
      jsonToTables(payload, { path: 'other' });
    } catch (err) {
      expect((err as TableError).candidates?.[0].path).toBe('$.data.rows');
    }
  });

  it('reports JSON syntax errors with a position', () => {
    expect(() => jsonToTables('{\n  "a": }')).toThrowError(/JSON 解析失败/);
    expect(() => jsonToTables('   ')).toThrowError(/请输入 JSON 内容/);
  });
});

describe('resolvePath / findArrayPaths', () => {
  it('walks keys, indexes and rejects unsupported syntax', () => {
    const root = { a: { b: [10, 20] } };
    expect(resolvePath(root, 'a.b[1]')).toBe(20);
    expect(resolvePath(root, '$.a.b')).toEqual([10, 20]);
    expect(() => resolvePath(root, 'a.b[9]')).toThrowError(/越界/);
    expect(() => resolvePath(root, 'a.c')).toThrowError(/找不到路径/);
    expect(() => resolvePath(root, 'a.b[*].c')).toThrowError(/\[\*\]/);
    expect(() => resolvePath(root, 'a.b[')).toThrowError(/「]」/);
  });

  it('ranks shallow object arrays first and respects the limit', () => {
    const root = { deep: { deeper: { rows: [{ a: 1 }] } }, top: [{ x: 1, y: 2 }] };
    const found = findArrayPaths(root);
    expect(found.map((item) => item.path)).toEqual(['$.top', '$.deep.deeper.rows']);
    expect(found[0]).toEqual({ path: '$.top', length: 1, keys: 2 });
    expect(findArrayPaths(root, { limit: 1 })).toHaveLength(1);
    expect(findArrayPaths(root, { maxDepth: 1 }).map((item) => item.path)).toEqual(['$.top']);
  });
});

describe('Table → JSON', () => {
  const table: Table = {
    name: '数据',
    columns: ['名称', '数量', '日期', '空'],
    rows: [
      [{ t: 's', v: '甲' }, { t: 'n', v: 3 }, dateCell('2024-01-01T00:00:00', 45292), emptyCell()],
    ],
  };

  it('keeps native types and renders dates per mode', () => {
    const iso = JSON.parse(tablesToJson([table], ISO_OPTIONS));
    expect(iso).toEqual([{ 名称: '甲', 数量: 3, 日期: '2024-01-01T00:00:00', 空: null }]);

    const serial = JSON.parse(
      tablesToJson([table], { dateMode: 'serial', dateFormat: 'yyyy-MM-dd' })
    );
    expect(serial[0].日期).toBe(45292);

    const custom = JSON.parse(
      tablesToJson([table], { dateMode: 'custom', dateFormat: 'yyyy-MM-dd' })
    );
    expect(custom[0].日期).toBe('2024-01-01');
  });

  it('keys multi-table output by sheet name', () => {
    const payload = JSON.parse(tablesToJson([table, { ...table, name: '其他' }], ISO_OPTIONS));
    expect(Object.keys(payload)).toEqual(['数据', '其他']);
  });
});

describe('Markdown', () => {
  const table: Table = {
    name: '数据',
    columns: ['a', 'b'],
    rows: [
      [
        { t: 's', v: 'x|y' },
        { t: 's', v: 'l1\nl2' },
      ],
    ],
  };

  it('escapes pipes and newlines', () => {
    expect(tablesToMarkdown(table, ISO_OPTIONS)).toBe(
      '| a | b |\n| --- | --- |\n| x\\|y | l1<br>l2 |\n'
    );
    expect(tablesToMarkdown(table, { ...ISO_OPTIONS, escapeNewlines: false })).toBe(
      '| a | b |\n| --- | --- |\n| x\\|y | l1\nl2 |\n'
    );
  });

  it('parses back to the same values', () => {
    const parsed = markdownToTables(tablesToMarkdown(table, ISO_OPTIONS), ISO_OPTIONS);
    expect(parsed[0].columns).toEqual(['a', 'b']);
    expect(parsed[0].rows).toEqual([
      [
        { t: 's', v: 'x|y' },
        { t: 's', v: 'l1\nl2' },
      ],
    ]);
  });

  it('infers types only when asked', () => {
    const source = '| n | b |\n| --- | --- |\n| 12 | true |\n';
    expect(markdownToTables(source, ISO_OPTIONS)[0].rows[0]).toEqual([
      { t: 's', v: '12' },
      { t: 's', v: 'true' },
    ]);
    expect(markdownToTables(source, { ...ISO_OPTIONS, inferTypes: true })[0].rows[0]).toEqual([
      { t: 'n', v: 12 },
      { t: 'b', v: true },
    ]);
  });

  it('reads every table in a document and rejects table-less input', () => {
    const document = '| a |\n| --- |\n| 1 |\n\n| b |\n| --- |\n| 2 |\n';
    expect(markdownToTables(document, ISO_OPTIONS)).toHaveLength(2);
    expect(() => markdownToTables('纯文本', ISO_OPTIONS)).toThrowError(/未找到 Markdown 表格/);
  });

  it('pads short rows and names extra columns', () => {
    const parsed = markdownToTables('| a | b |\n| --- | --- |\n| 1 |\n', ISO_OPTIONS)[0];
    expect(parsed.rows[0][1]).toEqual({ t: 's', v: null });
  });
});

describe('CSV', () => {
  it('quotes fields and protects against formula injection', () => {
    const table: Table = {
      name: '数据',
      columns: ['a', 'b', 'c'],
      rows: [
        [
          { t: 's', v: 'x,y' },
          { t: 's', v: '=1+1' },
          { t: 's', v: 'he said "hi"' },
        ],
      ],
    };
    expect(tablesToCsv(table, ISO_OPTIONS)).toBe('a,b,c\r\n"x,y",\'=1+1,"he said ""hi"""');
    expect(tablesToCsv(table, { ...ISO_OPTIONS, preventInjection: false })).toContain('=1+1');
  });

  it('guards formula-like column names too', () => {
    const table: Table = {
      name: '数据',
      columns: ['=A1'],
      rows: [[{ t: 's', v: 'x' }]],
    };
    expect(tablesToCsv(table, ISO_OPTIONS)).toBe("'=A1\r\nx");
  });

  it('parses quoted commas, quotes and newlines', () => {
    const source = 'a,b\r\n"x,y","line1\nline2"\r\n"he said ""hi""",z';
    const table = csvToTables(source, ISO_OPTIONS)[0];
    expect(table.columns).toEqual(['a', 'b']);
    expect(table.rows).toEqual([
      [
        { t: 's', v: 'x,y' },
        { t: 's', v: 'line1\nline2' },
      ],
      [
        { t: 's', v: 'he said "hi"' },
        { t: 's', v: 'z' },
      ],
    ]);
  });

  it('round-trips a table with CRLF and a trailing newline', () => {
    const table: Table = {
      name: '数据',
      columns: ['名称', '数量'],
      rows: [
        [
          { t: 's', v: '甲' },
          { t: 'n', v: 1 },
        ],
        [
          { t: 's', v: '乙' },
          { t: 'n', v: 2 },
        ],
      ],
    };
    const csv = `${tablesToCsv(table, ISO_OPTIONS)}\r\n`;
    const parsed = csvToTables(csv, { ...ISO_OPTIONS, inferTypes: true })[0];
    expect(parsed.columns).toEqual(['名称', '数量']);
    expect(parsed.rows).toEqual(table.rows);
  });

  it('generates column names without a header row and supports inference', () => {
    const table = csvToTables('1,true\n2,false', {
      ...ISO_OPTIONS,
      hasHeader: false,
      inferTypes: true,
    })[0];
    expect(table.columns).toEqual(['column1', 'column2']);
    expect(table.rows[0]).toEqual([
      { t: 'n', v: 1 },
      { t: 'b', v: true },
    ]);
  });

  it('rejects unbalanced quotes and empty input', () => {
    expect(() => csvToTables('"a,b', ISO_OPTIONS)).toThrowError(/未闭合的引号/);
    expect(() => csvToTables('  ', ISO_OPTIONS)).toThrowError(/请输入 CSV 内容/);
  });
});

describe('convert 入口', () => {
  it('rejects xlsx in the text-only helpers', () => {
    expect(() => parseTextSource('xlsx', 'x')).toThrowError(/Excel 文件请先选择工作表/);
    expect(() => serializeTextTarget('xlsx', [], ISO_OPTIONS)).toThrowError(/导出/);
  });

  it('parses JSON and serialises Markdown through the shared options', () => {
    const tables = parseTextSource('json', '[{"a":1}]');
    expect(serializeTextTarget('markdown', tables, ISO_OPTIONS)).toBe('| a |\n| --- |\n| 1 |\n');
  });

  it('truncates previews without touching the source table', () => {
    const table: Table = {
      name: '数据',
      columns: ['a'],
      rows: Array.from({ length: 5 }, (_, index) => [{ t: 'n' as const, v: index }]),
    };
    expect(previewTable(table, 2).rows).toHaveLength(2);
    expect(table.rows).toHaveLength(5);
  });
});
