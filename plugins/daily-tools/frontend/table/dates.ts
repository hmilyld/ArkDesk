/**
 * 日期格式化（输出侧）。
 *
 * Excel 日期在读取时已由后端换算为 ISO 8601（`v`）并保留原始序列号（`n`），
 * 因此这里只需要「ISO 字符串 + 自定义格式 → 展示文本」的纯逻辑。
 */

const ISO_PATTERN = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?)?/;

const TOKEN_PATTERN = /yyyy|yy|SSS|MM|M|dd|d|HH|H|mm|m|ss|s/g;

interface DateParts {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
  milli: number;
}

function parseIso(value: string): DateParts | null {
  const matched = ISO_PATTERN.exec(value.trim());
  if (!matched) return null;
  return {
    year: Number(matched[1]),
    month: Number(matched[2]),
    day: Number(matched[3]),
    hour: Number(matched[4] ?? 0),
    minute: Number(matched[5] ?? 0),
    second: Number(matched[6] ?? 0),
    milli: Number((matched[7] ?? '0').padEnd(3, '0')),
  };
}

function pad(value: number, length = 2): string {
  return String(value).padStart(length, '0');
}

/**
 * 用常见格式串格式化 ISO 日期时间。
 *
 * 支持：`yyyy` `yy` `MM` `M` `dd` `d` `HH` `H` `mm` `m` `ss` `s` `SSS`；
 * 其余字符原样输出。无法解析的输入原样返回。
 */
export function formatDateValue(iso: string, pattern: string): string {
  const parts = parseIso(iso);
  if (!parts) return iso;
  const format = pattern.trim() === '' ? 'yyyy-MM-dd HH:mm:ss' : pattern;

  const tokens: Record<string, string> = {
    yyyy: pad(parts.year, 4),
    yy: pad(parts.year % 100),
    MM: pad(parts.month),
    M: String(parts.month),
    dd: pad(parts.day),
    d: String(parts.day),
    HH: pad(parts.hour),
    H: String(parts.hour),
    mm: pad(parts.minute),
    m: String(parts.minute),
    ss: pad(parts.second),
    s: String(parts.second),
    SSS: pad(parts.milli, 3),
  };

  return format.replace(TOKEN_PATTERN, (token) => tokens[token] ?? token);
}
