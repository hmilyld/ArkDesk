import { describe, expect, it } from 'vitest';
import {
  ALL_METHODS,
  classifyFlow,
  EMPTY_FILTER,
  filterFlows,
  flowToResponseView,
  flowToSpec,
  isBinaryRequest,
  type FlowRecord,
  type FlowSummary,
} from '../plugins/network-tools/frontend/intercept-shared';

function summary(patch: Partial<FlowSummary> = {}): FlowSummary {
  return {
    id: 1,
    startedAt: 0,
    method: 'GET',
    scheme: 'https',
    host: 'api.example.com',
    url: 'https://api.example.com/users',
    status: 200,
    durationMs: 12,
    reqSize: 0,
    resSize: 10,
    error: null,
    isUpgrade: false,
    contentType: 'application/json',
    ...patch,
  };
}

function record(patch: Partial<FlowRecord> = {}): FlowRecord {
  return {
    summary: summary(),
    clientAddr: '127.0.0.1:12345',
    reqHeaders: [{ name: 'Accept', value: 'application/json' }],
    resHeaders: [{ name: 'Content-Type', value: 'application/json' }],
    reqBody: null,
    resBody: null,
    ...patch,
  };
}

describe('filterFlows', () => {
  const list = [
    summary({ id: 1, host: 'a.com', method: 'GET', status: 200, url: 'https://a.com/x' }),
    summary({ id: 2, host: 'b.com', method: 'POST', status: 404, url: 'https://b.com/y' }),
    summary({
      id: 3,
      host: 'b.com',
      method: 'GET',
      status: null,
      url: 'https://b.com/z',
      error: 'timeout',
    }),
  ];

  it('empty filter keeps all (ALL_METHODS is not a method)', () => {
    expect(filterFlows(list, { ...EMPTY_FILTER })).toHaveLength(3);
    expect(EMPTY_FILTER.method).toBe(ALL_METHODS);
  });

  it('filters by host, method, status class and url', () => {
    expect(filterFlows(list, { ...EMPTY_FILTER, host: 'b.com' }).map((f) => f.id)).toEqual([2, 3]);
    expect(filterFlows(list, { ...EMPTY_FILTER, method: 'POST' }).map((f) => f.id)).toEqual([2]);
    expect(filterFlows(list, { ...EMPTY_FILTER, status: '2xx' }).map((f) => f.id)).toEqual([1]);
    expect(filterFlows(list, { ...EMPTY_FILTER, status: 'err' }).map((f) => f.id)).toEqual([3]);
    expect(filterFlows(list, { ...EMPTY_FILTER, url: '/y' }).map((f) => f.id)).toEqual([2]);
  });
});

describe('flowToSpec', () => {
  it('maps method/url/headers/text body', () => {
    const spec = flowToSpec(
      record({
        reqBody: {
          text: '{"a":1}',
          base64: null,
          isBinary: false,
          size: 7,
          truncated: false,
          contentType: 'application/json',
          decoded: false,
          note: null,
        },
      })
    );
    expect(spec.method).toBe('GET');
    expect(spec.url).toBe('https://api.example.com/users');
    expect(spec.headers[0]?.name).toBe('Accept');
    expect(spec.body.type).toBe('raw');
    expect(spec.body.raw).toBe('{"a":1}');
    expect(spec.body.rawLang).toBe('json');
  });

  it('detects binary request bodies', () => {
    const binary = record({
      reqBody: {
        text: null,
        base64: 'AAEC',
        isBinary: true,
        size: 3,
        truncated: false,
        contentType: 'application/octet-stream',
        decoded: false,
        note: null,
      },
    });
    expect(isBinaryRequest(binary)).toBe(true);
    expect(isBinaryRequest(record())).toBe(false);
  });
});

describe('flowToResponseView', () => {
  it('builds a ResponsePanel-compatible view', () => {
    const view = flowToResponseView(
      record({
        resBody: {
          text: 'ok',
          base64: null,
          isBinary: false,
          size: 2,
          truncated: false,
          contentType: 'text/plain',
          decoded: false,
          note: null,
        },
      })
    );
    expect(view.status).toBe(200);
    expect(view.ok).toBe(true);
    expect(view.bodyText).toBe('ok');
    expect(view.isBinary).toBe(false);
  });

  it('keeps an HTML body (never empty) and classifies it as doc', () => {
    const html = '<!DOCTYPE html><html><body>hi</body></html>';
    const rec = record({
      summary: summary({ status: 200, contentType: 'text/html; charset=utf-8' }),
      resBody: {
        text: html,
        base64: null,
        isBinary: false,
        size: html.length,
        truncated: false,
        contentType: 'text/html; charset=utf-8',
        decoded: true,
        note: null,
      },
    });
    expect(flowToResponseView(rec).bodyText).toBe(html);
    expect(classifyFlow(rec.summary)).toBe('doc');
  });
});

describe('classifyFlow', () => {
  it('classifies by content type', () => {
    expect(classifyFlow(summary({ contentType: 'text/html' }))).toBe('doc');
    expect(classifyFlow(summary({ contentType: 'text/css' }))).toBe('css');
    expect(classifyFlow(summary({ contentType: 'text/javascript' }))).toBe('js');
    expect(classifyFlow(summary({ contentType: 'image/png' }))).toBe('img');
    expect(classifyFlow(summary({ contentType: 'font/woff2' }))).toBe('font');
    expect(classifyFlow(summary({ contentType: 'video/mp4' }))).toBe('media');
    expect(classifyFlow(summary({ contentType: 'application/octet-stream' }))).toBe('other');
    // 重定向体常见 text/plain，不应归入 JSON/XHR
    expect(classifyFlow(summary({ contentType: 'text/plain', status: 307 }))).toBe('other');
  });

  it('falls back to url extension and marks upgrades as ws', () => {
    expect(classifyFlow(summary({ contentType: null, url: 'https://a.com/app.css' }))).toBe('css');
    expect(classifyFlow(summary({ contentType: null, url: 'https://a.com/a.js' }))).toBe('js');
    expect(classifyFlow(summary({ contentType: null, isUpgrade: true }))).toBe('ws');
  });
});

describe('filterFlows by type', () => {
  const list = [
    summary({ id: 1, contentType: 'text/css', url: 'https://a.com/a.css' }),
    summary({ id: 2, contentType: 'image/png', url: 'https://a.com/a.png' }),
    summary({ id: 3, contentType: 'application/json', url: 'https://a.com/api' }),
  ];
  it('keeps only the requested kind', () => {
    expect(filterFlows(list, { ...EMPTY_FILTER, type: 'img' }).map((f) => f.id)).toEqual([2]);
    expect(filterFlows(list, { ...EMPTY_FILTER, type: 'css' }).map((f) => f.id)).toEqual([1]);
    expect(filterFlows(list, { ...EMPTY_FILTER }).length).toBe(3);
  });
});
