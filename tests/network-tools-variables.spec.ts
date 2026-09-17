import { describe, expect, it } from 'vitest';
import {
  collectVars,
  resolveSpec,
  substitute,
} from '../plugins/network-tools/frontend/lib/variables';
import { createRequestSpec, kv, prettyBody } from '../plugins/network-tools/frontend/shared';

describe('substitute', () => {
  it('replaces known variables and reports missing ones', () => {
    const result = substitute('{{base}}/u/{{id}}?x={{missing}}', { base: 'https://a', id: '7' });
    expect(result.text).toBe('https://a/u/7?x={{missing}}');
    expect(result.missing).toEqual(['missing']);
  });

  it('tolerates inner whitespace', () => {
    expect(substitute('{{ base }}', { base: 'x' }).text).toBe('x');
  });
});

describe('collectVars', () => {
  it('ignores disabled entries and blank keys', () => {
    const vars = collectVars([
      { key: 'a', value: '1', enabled: true },
      { key: 'b', value: '2', enabled: 0 },
      { key: '  ', value: '3', enabled: true },
    ]);
    expect(vars).toEqual({ a: '1' });
  });
});

describe('resolveSpec', () => {
  it('substitutes enabled fields across the spec', () => {
    const spec = createRequestSpec();
    spec.url = '{{base}}/x';
    spec.headers.push(kv('Auth', 'Bearer {{token}}'));
    spec.query.push(kv('q', '{{term}}'));
    spec.body.type = 'raw';
    spec.body.raw = '{"id":"{{id}}"}';

    const result = resolveSpec(spec, { base: 'https://a', token: 'T', term: 'z', id: '9' });
    expect(result.missing).toEqual([]);
    expect(result.spec.url).toBe('https://a/x');
    expect(result.spec.headers.find((row) => row.name === 'Auth')?.value).toBe('Bearer T');
    expect(result.spec.query[0]?.value).toBe('z');
    expect(result.spec.body.raw).toBe('{"id":"9"}');
  });

  it('ignores disabled rows when collecting missing variables', () => {
    const spec = createRequestSpec();
    spec.headers.push(kv('Skip', '{{nope}}', false));
    const result = resolveSpec(spec, {});
    expect(result.missing).toEqual([]);
  });

  it('does not mutate the original spec', () => {
    const spec = createRequestSpec();
    spec.url = '{{base}}/x';
    resolveSpec(spec, { base: 'https://a' });
    expect(spec.url).toBe('{{base}}/x');
  });
});

describe('prettyBody', () => {
  it('formats JSON', () => {
    expect(prettyBody('{"a":1}')).toBe('{\n  "a": 1\n}');
  });

  it('leaves plain text untouched', () => {
    expect(prettyBody('hello', 'text/plain')).toBe('hello');
  });

  it('degrades gracefully for markup without DOMParser (node)', () => {
    const xml = '<?xml version="1.0"?><a><b>1</b></a>';
    expect(prettyBody(xml, 'application/xml')).toBeTruthy();
  });
});
