import { describe, expect, it } from 'vitest';
import { CONTRACTS_VERSION } from '../contracts/version';
import { readHealth, ServerStatus } from './server';

const answer = (status: number, body: unknown) => async () => new Response(typeof body === 'string' ? body : JSON.stringify(body), { status });
const good = { status: 'ok', service: 'kentos-api', version: '0.1.0', contracts: CONTRACTS_VERSION };

describe('server health', () => {
  it('reads the health contract as untrusted data', () => {
    expect(readHealth(good)).toEqual({ ok: true, health: { ...good, commit: undefined } });
    expect(readHealth({ ...good, commit: 'abc123' })).toMatchObject({ ok: true, health: { commit: 'abc123' } });
    expect(readHealth(null)).toEqual({ ok: false, error: 'yanıt bir nesne değil' });
    expect(readHealth({ ...good, status: 'down' })).toEqual({ ok: false, error: 'durum “ok” değil' });
    expect(readHealth({ ...good, contracts: '1' })).toEqual({ ok: false, error: 'sözleşme sürümü eksik' });
    expect(readHealth({ ...good, commit: 7 })).toEqual({ ok: false, error: 'commit metin değil' });
  });

  it('is online only for a valid answer in the same contracts version', async () => {
    const s = new ServerStatus(answer(200, good));
    expect(s.state.value).toBe('checking');
    expect(await s.check()).toBe('online');
    expect(s.health.value?.version).toBe('0.1.0');
    expect(s.detail.value).toBe('');
    const other = new ServerStatus(answer(200, { ...good, contracts: CONTRACTS_VERSION + 1 }));
    expect(await other.check()).toBe('incompatible');
    expect(other.detail.value).toMatch(/sözleşme sürümü/);
    expect(other.health.value?.contracts).toBe(CONTRACTS_VERSION + 1);
  });

  it('is offline, with the reason, when nothing or something else answers', async () => {
    const cases: [ServerStatus, RegExp][] = [
      [new ServerStatus(answer(503, '')), /^API çalışmıyor\.$/],
      [new ServerStatus(answer(404, 'yok')), /404/],
      // A static host that sends its index page for any path.
      [new ServerStatus(answer(200, '<!doctype html><html></html>')), /KentOS sunucusundan gelmiyor/],
      [new ServerStatus(answer(200, { hello: 1 })), /sağlık sözleşmesine uymuyor/],
      [
        new ServerStatus(async () => {
          throw new TypeError('Failed to fetch');
        }),
        /ulaşılamadı/,
      ],
    ];
    for (const [s, why] of cases) {
      expect(await s.check()).toBe('offline');
      expect(s.detail.value).toMatch(why);
      expect(s.health.value).toBeNull();
    }
  });

  it('shares a check already under way', async () => {
    let calls = 0;
    const s = new ServerStatus(async () => (calls++, new Response(JSON.stringify(good))));
    const [a, b] = await Promise.all([s.check(), s.check()]);
    expect([a, b, calls]).toEqual(['online', 'online', 1]);
    await s.check();
    expect(calls).toBe(2);
  });
});
