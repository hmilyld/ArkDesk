import { describe, expect, it } from 'vitest';
import {
  BUNDLE_TYPE,
  createBundle,
  parseBundle,
  stringifyBundle,
} from '../plugins/network-tools/frontend/transfer';
import { createRequestSpec, normalizeSpec } from '../plugins/network-tools/frontend/shared';

describe('normalizeSpec', () => {
  it('fills missing nested structures', () => {
    const spec = normalizeSpec({ method: 'POST', url: 'https://x' });
    expect(spec.method).toBe('POST');
    expect(spec.query).toEqual([]);
    expect(spec.body.type).toBe('none');
    expect(spec.auth.type).toBe('none');
  });

  it('falls back on non-object input', () => {
    expect(normalizeSpec(null).method).toBe('GET');
    expect(normalizeSpec('nope').url).toBe('');
  });
});

describe('transfer bundle', () => {
  it('round-trips a bundle', () => {
    const bundle = createBundle({ requests: [{ name: 'r1', spec: createRequestSpec() }] });
    const parsed = parseBundle(stringifyBundle(bundle));
    expect(parsed.type).toBe(BUNDLE_TYPE);
    expect(parsed.requests?.[0]?.name).toBe('r1');
  });

  it('rejects foreign or malformed files', () => {
    expect(() => parseBundle('{"type":"other"}')).toThrow();
    expect(() => parseBundle('not json')).toThrow();
  });
});
