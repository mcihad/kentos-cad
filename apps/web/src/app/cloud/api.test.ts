import { describe, expect, it } from 'vitest';
import { ApiFailure, HttpCloudApi } from './api';
import { revokeEnvelope, shareEnvelope } from './sharing';

/** A fetch that records each request and answers with `status` and `body`. */
function recorder(status = 200, body: unknown = {}) {
  const calls: { url: string; init: RequestInit }[] = [];
  const fetcher = (async (url: string, init: RequestInit) => {
    calls.push({ url, init });
    return new Response(status === 204 ? null : JSON.stringify(body), { status });
  }) as unknown as typeof fetch;
  return { calls, api: new HttpCloudApi(fetcher) };
}

describe('the cloud API client: sharing', () => {
  it('asks the share dialog routes of the project, with the app header', async () => {
    const { calls, api } = recorder(200, { projects: [] });
    await api.myProjects();
    await api.access('t 1', 'p/2');
    await api.candidates('t', 'p', 'Şeyma %_ ışık');
    expect(calls.map((c) => [c.init.method, c.url])).toEqual([
      ['GET', '/v1/me/projects'],
      // Ids are path segments, never raw.
      ['GET', '/v1/tenants/t%201/projects/p%2F2/access'],
      ['GET', '/v1/tenants/t/projects/p/access/candidates?q=%C5%9Eeyma+%25_+%C4%B1%C5%9F%C4%B1k'],
    ]);
    for (const c of calls) expect((c.init.headers as Record<string, string>)['x-kentos-client']).toBe('web');
    expect(new URL(calls[2].url, 'http://x').searchParams.get('q')).toBe('Şeyma %_ ışık');
  });

  it('sends the share and revoke commands to the command route, as they are', async () => {
    const { calls, api } = recorder(200, { userId: 'u2', changed: true, replayed: false });
    const share = shareEnvelope('t', 'p', 'u2', 'editor', '2026-12-31T20:59:59.000Z');
    const answer = await api.accessCommand(share);
    await api.accessCommand(revokeEnvelope('t', 'p', 'u2'));
    expect(answer.changed).toBe(true);
    expect(calls.map((c) => [c.init.method, c.url])).toEqual([
      ['POST', '/v1/tenants/t/projects/p/commands'],
      ['POST', '/v1/tenants/t/projects/p/commands'],
    ]);
    const sent = calls.map((c) => JSON.parse(String(c.init.body)));
    expect(sent[0]).toMatchObject({
      commandName: 'project.share',
      version: 1,
      tenantId: 't',
      projectId: 'p',
      expectedVersions: {},
      input: { userId: 'u2', role: 'editor', expiresAt: '2026-12-31T20:59:59.000Z' },
    });
    expect(sent[1]).toMatchObject({ commandName: 'project.access.revoke', version: 1, input: { userId: 'u2' } });
    // Every command has its own request id and idempotency key.
    expect(sent[0].idempotencyKey).not.toBe(sent[1].idempotencyKey);
    expect(sent[0].requestId).toMatch(/^web-/);
    // No end: no expiresAt at all (not null).
    expect(shareEnvelope('t', 'p', 'u2', 'viewer').input).toStrictEqual({ userId: 'u2', role: 'viewer' });
  });

  it('turns refusals into failures with the server’s words and code', async () => {
    const lost = recorder(404, { error: 'not_found', message: 'Proje bulunamadı.', requestId: 'r1' });
    const e404 = await lost.api.access('t', 'p').catch((e: unknown) => e);
    expect(e404).toBeInstanceOf(ApiFailure);
    expect([(e404 as ApiFailure).notFound, (e404 as ApiFailure).message, (e404 as ApiFailure).requestId]).toEqual([true, 'Proje bulunamadı.', 'r1']);
    const refused = recorder(403, { error: 'forbidden', message: '“Ada” projesinde bu işlem için yetkiniz yok (project.share).' });
    const e403 = (await refused.api.candidates('t', 'p', 'me').catch((e: unknown) => e)) as ApiFailure;
    expect([e403.code, e403.notFound, e403.status]).toEqual(['forbidden', false, 403]);
    const down = new HttpCloudApi((async () => {
      throw new TypeError('Failed to fetch');
    }) as unknown as typeof fetch);
    const net = (await down.myProjects().catch((e: unknown) => e)) as ApiFailure;
    expect([net.code, net.transient, net.status]).toEqual(['network', true, 0]);
  });

  it('a search given up is aborted, not reported as a dead network', async () => {
    const api = new HttpCloudApi(((_url: string, init: RequestInit) =>
      new Promise((_resolve, reject) => init.signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError'))))) as unknown as typeof fetch);
    const abort = new AbortController();
    const asked = api.candidates('t', 'p', 'meh', abort.signal).catch((e: unknown) => e);
    abort.abort();
    const e = (await asked) as ApiFailure;
    expect([e.code, e.status, e.transient]).toEqual(['aborted', 499, false]);
  });
});
