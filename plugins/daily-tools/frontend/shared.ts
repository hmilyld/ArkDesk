/**
 * daily-tools 插件内共享代码。
 */
import { normalizeError } from '@/core/errors';

/** 文件转换支持的格式 */
export const CONVERT_EXTENSIONS = ['docx', 'xlsx', 'xls', 'html', 'htm', 'csv', 'pdf', 'txt', 'md'];

/** OCR 支持的图片扩展名 */
export const OCR_EXTENSIONS = ['png', 'jpg', 'jpeg', 'webp', 'bmp'];

/**
 * 提取用户可读的错误信息。
 *
 * `ipc` 抛出的是规范化后的 `AppError` 对象（非 Error 实例），直接 `String(err)`
 * 会得到 `[object Object]`；此函数统一兼容 AppError / Error / 字符串 / 未知值。
 * 另外对 Tauri 参数反序列化错误（`invalid args ... missing field ...`）做通俗化翻译。
 */
const FIELD_LABELS: Record<string, string> = {
  req: '请求参数',
  key: '密钥',
  data: '数据',
  signature: '签名',
  inputPath: '输入文件',
  outputPath: '输出文件',
  password: '口令',
  salt: '盐值',
  algorithm: '算法',
  mode: '模式',
  operation: '操作',
  path: '文件路径',
  scheme: '编码方式',
  format: '格式',
};

export function errorMessage(value: unknown): string {
  const raw = normalizeError(value).message;
  const missing = raw.match(/missing field `([^`]+)`/);
  if (missing) {
    const label = FIELD_LABELS[missing[1]] ?? missing[1];
    return `参数不完整：缺少「${label}」，请检查对应输入后重试`;
  }
  if (/invalid args|invalid type|failed to deserialize/i.test(raw)) {
    return '参数格式不正确，请检查输入后重试';
  }
  return raw;
}
