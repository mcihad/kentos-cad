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

  it('reads what a failure tells a caller to do: the field, the revision, whether and when to send again (ARCH-07)', async () => {
    const bad = (await recorder(422, { error: 'invalid', message: 'Arama en çok 100 karakter olabilir.', path: 'q', retryable: false })
      .api.candidates('t', 'p', 'x')
      .catch((e: unknown) => e)) as ApiFailure;
    expect([bad.code, bad.path, bad.retryable, bad.transient]).toEqual(['invalid', 'q', false, false]);
    const stale = (await recorder(409, { error: 'conflict', message: 'Başka biri değiştirdi.', revision: '42', retryable: false, conflicts: [] })
      .api.access('t', 'p')
      .catch((e: unknown) => e)) as ApiFailure;
    expect([stale.code, stale.revision, stale.transient]).toEqual(['conflict', '42', false]);
    const busy = (await recorder(429, { error: 'rate_limited', message: 'Çok fazla deneme.', retryable: true, retryAfter: 7 })
      .api.myProjects()
      .catch((e: unknown) => e)) as ApiFailure;
    expect([busy.retryable, busy.retryAfter, busy.transient]).toEqual([true, 7, true]);
    // A body without the fields (a proxy's page) is not retryable by itself; a gateway status still is.
    const proxy = (await recorder(502, { message: 'Bad gateway' }).api.myProjects().catch((e: unknown) => e)) as ApiFailure;
    expect([proxy.retryable, proxy.retryAfter, proxy.transient]).toEqual([false, undefined, true]);
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

describe('the cloud API client: files (docs/adr/0031, 0033, 0034, 0038)', () => {
  it('sends an upload’s bytes with the app header and hears how far it is; a refusal keeps the field', async () => {
    const sent: { url: string; headers: Record<string, string>; size: number }[] = [];
    let status = 200;
    const api = new HttpCloudApi(
      (async () => new Response('{}')) as unknown as typeof fetch,
      async (url, bytes, headers, progress) => {
        sent.push({ url, headers, size: bytes.byteLength });
        progress(bytes.byteLength / 2, bytes.byteLength);
        progress(bytes.byteLength, bytes.byteLength);
        return status === 200
          ? { status, text: JSON.stringify({ id: 'u1', size: bytes.byteLength, sha256: 'ab', createdAt: '', expiresAt: '', received: true, objects: '13' }) }
          : { status, text: JSON.stringify({ error: 'invalid', message: 'Gelen dosya bildirilenle aynı değil.', path: 'sha256', retryable: false }) };
      },
    );
    const heard: number[] = [];
    const up = await api.sendUpload('t 1', 'p', 'u/1', new Uint8Array(10), (done) => heard.push(done));
    expect([up.received, up.objects, heard]).toEqual([true, '13', [5, 10]]);
    expect(sent[0].url).toBe('/v1/tenants/t%201/projects/p/uploads/u%2F1');
    expect(sent[0].headers).toMatchObject({ 'content-type': 'application/octet-stream', 'x-kentos-client': 'web' });
    status = 422;
    const e = (await api.sendUpload('t', 'p', 'u', new Uint8Array(4)).catch((x: unknown) => x)) as ApiFailure;
    expect([e.code, e.path, e.status]).toEqual(['invalid', 'sha256', 422]);
  });

  it('asks one of its uploads as it stands (docs/adr/0040)', async () => {
    const calls: { url: string; method?: string }[] = [];
    const api = new HttpCloudApi((async (url: string, init: RequestInit) => {
      calls.push({ url, method: init.method });
      return new Response(JSON.stringify({ id: 'u/1', size: 3, sha256: 'ab', createdAt: '', expiresAt: '', received: true }));
    }) as unknown as typeof fetch);
    const up = await api.uploadState('t 1', 'p', 'u/1');
    expect([up.received, calls]).toEqual([true, [{ url: '/v1/tenants/t%201/projects/p/uploads/u%2F1', method: 'GET' }]]);
  });

  it('reads a .kcad with its hash, revision, cursor and file name; a refusal is the server’s words', async () => {
    const body = new Uint8Array([0x89, 0x4b, 0x43, 0x41, 0x44, 1, 2, 3]);
    const calls: string[] = [];
    const api = new HttpCloudApi((async (url: string) => {
      calls.push(url);
      if (url.endsWith('/snapshot'))
        return new Response(body, {
          headers: { etag: '"abc"', 'x-kentos-revision': '7', 'x-kentos-event-cursor': '42', 'content-length': '8', 'content-disposition': "attachment; filename=\"Ada _.kcad\"; filename*=UTF-8''Ada%20%C3%A7-r7.kcad" },
        });
      return new Response(JSON.stringify({ error: 'forbidden', message: 'İndirme izniniz yok (project.download).' }), { status: 403 });
    }) as unknown as typeof fetch);
    const seen: number[] = [];
    const d = await api.snapshot('t', 'p', (done) => seen.push(done));
    expect([[...d.bytes], d.sha256, d.revision, d.cursor, d.fileName, seen.at(-1)]).toEqual([[...body], 'abc', '7', '42', 'Ada ç-r7.kcad', 8]);
    const e = (await api.fileRevision('t', 'p', '3').catch((x: unknown) => x)) as ApiFailure;
    expect([e.code, e.status, e.message]).toEqual(['forbidden', 403, 'İndirme izniniz yok (project.download).']);
    expect(calls).toEqual(['/v1/tenants/t/projects/p/snapshot', '/v1/tenants/t/projects/p/files/3']);
  });
});

describe('a download cut short (docs/adr/0045)', () => {
  const file = new Uint8Array(Array.from({ length: 40 }, (_, i) => i));
  /** A body that gives `bytes`, then breaks when `cut` (the connection dropped). */
  const body = (bytes: Uint8Array, cut: boolean) => {
    let sent = false;
    return new ReadableStream<Uint8Array>({
      // The bytes are read before the break (an error at once would discard what was queued).
      pull(c) {
        if (!sent && bytes.byteLength) {
          sent = true;
          c.enqueue(bytes);
        } else if (cut) c.error(new TypeError('network'));
        else c.close();
      },
    });
  };
  /** A server that answers from `script`, recording each request's range. */
  function server(script: ((range: string | null) => Response)[]) {
    const ranges: (string | null)[] = [];
    const api = new HttpCloudApi((async (_url: string, init: RequestInit) => {
      const range = (init.headers as Record<string, string>).range ?? null;
      ranges.push(range);
      return script[ranges.length - 1](range);
    }) as unknown as typeof fetch);
    api.downloadWaits = [0];
    return { api, ranges };
  }
  const whole = (etag: string, cutAt?: number) => () =>
    new Response(body(cutAt === undefined ? file : file.slice(0, cutAt), cutAt !== undefined), { headers: { etag: `"${etag}"`, 'content-length': String(file.length), 'x-kentos-revision': '3' } });
  const rest = (etag: string, from: number) => () =>
    new Response(body(file.slice(from), false), { status: 206, headers: { etag: `"${etag}"`, 'content-range': `bytes ${from}-${file.length - 1}/${file.length}`, 'content-length': String(file.length - from) } });

  it('a revision goes on from the bytes that arrived with Range, and the parts join into the file', async () => {
    const { api, ranges } = server([whole('abc', 15), rest('abc', 15)]);
    const heard: number[] = [];
    const d = await api.fileRevision('t', 'p', '3', (done) => heard.push(done));
    expect([[...d.bytes], d.sha256, d.revision, ranges]).toEqual([[...file], 'abc', '3', [null, 'bytes=15-']]);
    expect(heard.at(-1)).toBe(40);
  });

  it('a continuation of another file, or the whole file again, starts over; nothing mixes', async () => {
    // Another tag in the 206 (the file changed): from the start.
    const other = server([whole('abc', 15), rest('xyz', 15), whole('xyz')]);
    const d = await other.api.fileRevision('t', 'p', '3');
    expect([[...d.bytes], d.sha256, other.ranges]).toEqual([[...file], 'xyz', [null, 'bytes=15-', null]]);
    // A server that does not continue sends the whole file: read from its start.
    const again = server([whole('abc', 15), whole('abc')]);
    const e = await again.api.checkpointFile('t', 'p', 'c1');
    expect([[...e.bytes], again.ranges]).toEqual([[...file], [null, 'bytes=15-']]);
  });

  it('a snapshot, made anew for each request, starts over without Range', async () => {
    const { api, ranges } = server([whole('s1', 15), whole('s2')]);
    const d = await api.snapshot('t', 'p');
    expect([[...d.bytes], d.sha256, ranges]).toEqual([[...file], 's2', [null, null]]);
  });

  it('gives up after five tries in a row that bring nothing, and a refusal is never tried again', async () => {
    const dead = server(Array.from({ length: 9 }, () => () => new Response(body(new Uint8Array(0), true), { headers: { etag: '"abc"' } })));
    const e = (await dead.api.fileRevision('t', 'p', '3').catch((x: unknown) => x)) as ApiFailure;
    expect([e.code, dead.ranges.length]).toEqual(['network', 5]);
    const refused = server([() => new Response(JSON.stringify({ error: 'forbidden', message: 'İndirme izniniz yok.' }), { status: 403 })]);
    const f = (await refused.api.fileRevision('t', 'p', '3').catch((x: unknown) => x)) as ApiFailure;
    expect([f.code, refused.ranges.length]).toEqual(['forbidden', 1]);
  });
});
