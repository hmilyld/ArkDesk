import { describe, expect, it, vi } from 'vitest';

// logger 依赖 Tauri 运行时，这里以空实现替换
vi.mock('@/core/logger', () => ({
  logger: { error: () => {}, warn: () => {}, debug: () => {} },
}));

import { ErrorCode, normalizeError } from '@/core/errors';

describe('normalizeError', () => {
  it('保留结构化错误（code + message）', () => {
    expect(normalizeError({ code: 'X', message: 'm' })).toEqual({ code: 'X', message: 'm' });
  });

  it('Error 归一为 UNKNOWN 并保留 message', () => {
    const error = normalizeError(new Error('boom'));
    expect(error.code).toBe(ErrorCode.Unknown);
    expect(error.message).toBe('boom');
  });

  it('字符串归一为 UNKNOWN', () => {
    expect(normalizeError('oops').message).toBe('oops');
  });
});
