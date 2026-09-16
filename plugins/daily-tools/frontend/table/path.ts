/**
 * 数据路径（XPath 风格）解析。
 *
 * 语法：可选的 `$` 前缀 + 点分隔的键 + 数组下标 `[n]`（`[*]` 仅作为「此处必须是数组」
 * 的断言，只允许出现在路径末尾）。不接受谓词、通配与轴。
 *
 * 例：`data.list`、`$.data.list`、`items[0].rows`、`list[*]`
 */
import { isPlainObject, TableError, type ArrayPathCandidate } from './model';
import { PATH_LIMIT, PATH_MAX_DEPTH } from './options';

interface PathStep {
  kind: 'key' | 'index' | 'any';
  key: string;
  index: number;
}

/** 解析路径为步骤序列（路径非法时抛 TableError） */
function parsePath(path: string): PathStep[] {
  const source = path.trim().replace(/^\$/, '');
  const steps: PathStep[] = [];
  let cursor = 0;

  while (cursor < source.length) {
    const char = source[cursor];
    if (char === '.' || char === '\uFF0E') {
      cursor += 1;
      continue;
    }
    if (char === '[') {
      const close = source.indexOf(']', cursor);
      if (close === -1) throw new TableError('数据路径缺少匹配的「]」');
      const token = source.slice(cursor + 1, close).trim();
      if (token === '*') {
        steps.push({ kind: 'any', key: '', index: 0 });
      } else if (/^\d+$/.test(token)) {
        steps.push({ kind: 'index', key: '', index: Number(token) });
      } else {
        throw new TableError(`数据路径的方括号内只能是下标或 *，收到「${token}」`);
      }
      cursor = close + 1;
      continue;
    }

    let end = cursor;
    while (end < source.length && !'.[]'.includes(source[end])) end += 1;
    const key = source.slice(cursor, end);
    if (key === '') throw new TableError('数据路径中存在空的键名');
    steps.push({ kind: 'key', key, index: 0 });
    cursor = end;
  }

  if (steps.length === 0) throw new TableError('数据路径为空');
  return steps;
}

function describePath(steps: PathStep[], end: number): string {
  let text = '$';
  for (let i = 0; i < end; i += 1) {
    const step = steps[i];
    if (step.kind === 'key') text += `.${step.key}`;
    else if (step.kind === 'index') text += `[${step.index}]`;
    else text += '[*]';
  }
  return text;
}

/** 按路径取值（找不到 / 类型不符时抛 TableError） */
export function resolvePath(root: unknown, path: string): unknown {
  const steps = parsePath(path);
  let current: unknown = root;

  for (let i = 0; i < steps.length; i += 1) {
    const step = steps[i];
    if (step.kind === 'key') {
      if (!isPlainObject(current)) {
        throw new TableError(`路径「${describePath(steps, i)}」不是对象，无法取键「${step.key}」`);
      }
      if (!Object.prototype.hasOwnProperty.call(current, step.key)) {
        throw new TableError(`找不到路径「${describePath(steps, i + 1)}」对应的键`);
      }
      current = current[step.key];
      continue;
    }
    if (!Array.isArray(current)) {
      throw new TableError(`路径「${describePath(steps, i)}」不是数组`);
    }
    if (step.kind === 'index') {
      if (step.index >= current.length) {
        throw new TableError(
          `路径「${describePath(steps, i + 1)}」越界：数组长度 ${current.length}`
        );
      }
      current = current[step.index];
      continue;
    }
    if (i !== steps.length - 1) {
      throw new TableError('「[*]」只支持放在路径末尾（不支持对每项继续取字段）');
    }
  }

  return current;
}

function pathOf(segments: string[]): string {
  return segments.reduce(
    (text, segment) => (/^\d+$/.test(segment) ? `${text}[${segment}]` : `${text}.${segment}`),
    '$'
  );
}

function keysOf(value: unknown[]): number {
  const keys = new Set<string>();
  for (const item of value) {
    if (isPlainObject(item)) for (const key of Object.keys(item)) keys.add(key);
  }
  return keys.size;
}

/**
 * 扫描 JSON 中所有数组所在的路径（供「非数组输入」时提示用户选择）。
 *
 * 优先浅层、元素为对象的数组。
 */
export function findArrayPaths(
  root: unknown,
  options: { maxDepth?: number; limit?: number } = {}
): ArrayPathCandidate[] {
  const maxDepth = options.maxDepth ?? PATH_MAX_DEPTH;
  const limit = options.limit ?? PATH_LIMIT;
  const found: (ArrayPathCandidate & { depth: number })[] = [];

  function walk(value: unknown, segments: string[], depth: number): void {
    if (found.length >= limit || depth > maxDepth) return;
    if (Array.isArray(value)) {
      if (segments.length > 0) {
        found.push({
          path: pathOf(segments),
          length: value.length,
          keys: keysOf(value),
          depth,
        });
      }
      value.forEach((item, index) => walk(item, [...segments, String(index)], depth + 1));
      return;
    }
    if (isPlainObject(value)) {
      for (const key of Object.keys(value)) walk(value[key], [...segments, key], depth + 1);
    }
  }

  walk(root, [], 0);

  return found
    .sort((a, b) => a.depth - b.depth || b.keys - a.keys || b.length - a.length)
    .slice(0, limit)
    .map(({ path, length, keys }) => ({ path, length, keys }));
}
