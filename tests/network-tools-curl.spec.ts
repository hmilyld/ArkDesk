import { describe, expect, it } from 'vitest';
import { buildCurl, parseCurl, tokenizeCurl } from '../plugins/network-tools/frontend/curl';
import { createRequestSpec, kv } from '../plugins/network-tools/frontend/shared';

describe('tokenizeCurl', () => {
  it('splits quoted args and joins continuation lines', () => {
    const tokens = tokenizeCurl("curl 'https://a.com' \\\n  -H 'A: b'");
    expect(tokens).toEqual(['curl', 'https://a.com', '-H', 'A: b']);
  });

  it('handles escaped quotes inside double quotes', () => {
    expect(tokenizeCurl('curl "a \\"b\\" c"')).toEqual(['curl', 'a "b" c']);
  });
});

describe('parseCurl', () => {
  it('parses method, url, headers, cookies and raw body', () => {
    const spec = parseCurl(
      `curl 'https://api.example.com/login' -X POST -H 'Content-Type: application/json' -H 'Accept: */*' -b 'a=1; b=2' --data-raw '{"u":"x"}'`
    );
    expect(spec.method).toBe('POST');
    expect(spec.url).toBe('https://api.example.com/login');
    expect(spec.headers.find((row) => row.name === 'Content-Type')?.value).toBe('application/json');
    expect(spec.cookies.map((row) => `${row.name}=${row.value}`)).toEqual(['a=1', 'b=2']);
    expect(spec.body.type).toBe('raw');
    expect(spec.body.rawLang).toBe('json');
  });

  it('parses multipart form with a file part', () => {
    const spec = parseCurl(`curl 'https://x/upload' -F 'name=foo' -F 'file=@/tmp/a.png'`);
    expect(spec.method).toBe('POST');
    expect(spec.body.type).toBe('multipart');
    expect(spec.body.multipart[0]?.kind).toBe('text');
    expect(spec.body.multipart[1]?.kind).toBe('file');
    expect(spec.body.multipart[1]?.filePath).toBe('/tmp/a.png');
  });

  it('converts urlencoded body to form fields', () => {
    const spec = parseCurl(
      `curl 'https://x' -H 'content-type: application/x-www-form-urlencoded' --data-raw 'a=1&b=hello%20world'`
    );
    expect(spec.body.type).toBe('form');
    expect(spec.body.form.map((row) => `${row.name}=${row.value}`)).toEqual([
      'a=1',
      'b=hello world',
    ]);
  });

  it('parses basic auth and a bare url, defaulting to GET', () => {
    const spec = parseCurl(`curl https://x -u user:pass`);
    expect(spec.url).toBe('https://x');
    expect(spec.method).toBe('GET');
    expect(spec.auth.type).toBe('basic');
    expect(spec.auth.basicUsername).toBe('user');
    expect(spec.auth.basicPassword).toBe('pass');
  });

  it('skips values of ignored flags instead of treating them as url', () => {
    const spec = parseCurl(`curl -o /dev/null -s https://real.example.com/api`);
    expect(spec.url).toBe('https://real.example.com/api');
  });

  it('parses multipart file with type and filename params', () => {
    const spec = parseCurl(`curl 'https://x' -F 'pic=@/tmp/a.png;type=image/png;filename=a.png'`);
    expect(spec.body.multipart[0]?.filePath).toBe('/tmp/a.png');
    expect(spec.body.multipart[0]?.contentType).toBe('image/png');
    expect(spec.body.multipart[0]?.fileName).toBe('a.png');
  });
});

describe('buildCurl', () => {
  it('builds a curl that round-trips through parseCurl', () => {
    const spec = createRequestSpec();
    spec.method = 'POST';
    spec.url = 'https://api.example.com/x';
    spec.headers.push(kv('X-A', '1'));
    spec.body.type = 'raw';
    spec.body.raw = '{"a":1}';
    spec.body.rawLang = 'json';

    const text = buildCurl(spec);
    expect(text).toContain('curl');
    expect(text).toContain('-X POST');
    expect(text).toContain('X-A: 1');

    const parsed = parseCurl(text);
    expect(parsed.method).toBe('POST');
    expect(parsed.url).toBe('https://api.example.com/x');
    expect(parsed.body.raw).toBe('{"a":1}');
    expect(parsed.body.rawLang).toBe('json');
  });

  it('includes API key in query when configured', () => {
    const spec = createRequestSpec();
    spec.url = 'https://api.example.com/x';
    spec.auth.type = 'apikey';
    spec.auth.apiKeyIn = 'query';
    spec.auth.apiKeyName = 'key';
    spec.auth.apiKeyValue = 'v1';
    const text = buildCurl(spec);
    expect(text).toContain('https://api.example.com/x?key=v1');
  });
});
