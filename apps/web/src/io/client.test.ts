import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CoordReadOptions } from '../contracts/generated/CoordReadOptions';
import { FormatsClient, type WorkerLike } from './client';
import type { FormatsReply, FormatsRequest } from './protocol';

/**
 * The page's side of the formats worker (io/client.ts) with a fake worker:
 * requests answered by id, what is handed over and what is copied, a trap
 * or a failed load starting a fresh worker, the idle stop and cancel.
 */

class FakeWorker implements WorkerLike {
  onmessage: ((e: { data: FormatsReply }) => void) | null = null;
  onerror: ((e: unknown) => void) | null = null;
  readonly sent: { message: FormatsRequest; transfer: Transferable[] }[] = [];
  terminated = false;
  refuse: Error | null = null;

  postMessage(message: FormatsRequest, transfer: Transferable[]): void {
    if (this.refuse) throw this.refuse;
    this.sent.push({ message, transfer });
  }

  terminate(): void {
    this.terminated = true;
  }

  reply(r: FormatsReply): void {
    this.onmessage?.({ data: r });
  }

  /** The id of the n-th request sent. */
  id(n = 0): number {
    return this.sent[n].message.id;
  }
}

const encode = (v: unknown) => new TextEncoder().encode(JSON.stringify(v)).buffer as ArrayBuffer;
const options: CoordReadOptions = { delimiter: 'auto', decimal: 'auto', header: 'auto', columns: [], previewRows: 0, entities: false };

function setup() {
  const workers: FakeWorker[] = [];
  const client = new FormatsClient(() => {
    const w = new FakeWorker();
    workers.push(w);
    return w;
  });
  return { client, workers };
}

afterEach(() => {
  vi.useRealTimers();
});

describe('FormatsClient', () => {
  it('starts one worker with the first request and answers each request by its id', async () => {
    const { client, workers } = setup();
    expect(workers).toHaveLength(0);
    const bytes = new Uint8Array([49, 32, 50]);
    const first = client.readCoords(bytes, options);
    const second = client.readDxf(new Uint8Array(4), { maxEntities: 0 });
    expect(workers).toHaveLength(1);
    const w = workers[0];
    expect(w.sent.map((s) => s.message.op)).toEqual(['readCoords', 'readDxf']);
    // The caller keeps its bytes: the coordinate list goes as a copy.
    const sent = w.sent[0];
    expect(sent.transfer).toHaveLength(1);
    expect(sent.transfer[0]).not.toBe(bytes.buffer);
    expect(bytes).toEqual(new Uint8Array([49, 32, 50]));
    // Answers in any order; one for an unknown id is ignored.
    w.reply({ id: 999, ok: true, json: encode('?') });
    w.reply({ id: w.id(1), ok: true, json: encode({ entities: [], layers: [] }) });
    w.reply({ id: w.id(0), ok: true, json: encode({ points: 2 }) });
    await expect(first).resolves.toEqual({ points: 2 });
    await expect(second).resolves.toEqual({ entities: [], layers: [] });
  });

  it('hands a DXF file over without a copy, and copies a view into a larger buffer', () => {
    const { client, workers } = setup();
    const whole = new Uint8Array([1, 2, 3, 4]);
    void client.readDxf(whole, { maxEntities: 0 });
    expect(workers[0].sent[0].transfer[0]).toBe(whole.buffer);
    const big = new ArrayBuffer(16);
    new Uint8Array(big).set([9, 9, 7, 8, 9, 10], 0);
    void client.readDxf(new Uint8Array(big, 2, 4), { maxEntities: 0 });
    const copy = workers[0].sent[1].transfer[0] as ArrayBuffer;
    expect(copy).not.toBe(big);
    expect([...new Uint8Array(copy)]).toEqual([7, 8, 9, 10]);
  });

  it('passes a written file and its report on', async () => {
    const { client, workers } = setup();
    const written = client.writeCoords({ points: [], delimiter: 'space', columns: ['name', 'y', 'x'], header: false, encoding: 'utf8' });
    const w = workers[0];
    w.reply({ id: w.id(), ok: true, file: new Uint8Array([65, 13, 10]).buffer, report: JSON.stringify({ counts: { point: 1 }, notes: [], skipped: [] }) });
    const out = await written;
    expect([...out.bytes]).toEqual([65, 13, 10]);
    expect(out.report.counts).toEqual({ point: 1 });
  });

  it("rejects a reader's error without stopping the worker", async () => {
    const { client, workers } = setup();
    const r = client.readDxf(new Uint8Array(1), { maxEntities: 0 });
    const w = workers[0];
    w.reply({ id: w.id(), ok: false, fatal: false, message: 'Bu bir DWG dosyası.' });
    await expect(r).rejects.toThrow('Bu bir DWG dosyası.');
    expect(w.terminated).toBe(false);
    void client.readCoords(new Uint8Array(1), options);
    expect(workers).toHaveLength(1);
  });

  it('after a trap every waiting request fails and the next one starts a fresh worker', async () => {
    const { client, workers } = setup();
    const a = client.readCoords(new Uint8Array(1), options);
    const b = client.readCoords(new Uint8Array(1), options);
    const w = workers[0];
    w.reply({ id: w.id(0), ok: false, fatal: true, message: 'Dosya biçimi modülü beklenmedik biçimde durdu.' });
    await expect(a).rejects.toThrow('beklenmedik biçimde durdu');
    await expect(b).rejects.toThrow('beklenmedik biçimde durdu');
    expect(w.terminated).toBe(true);
    void client.readCoords(new Uint8Array(1), options);
    expect(workers).toHaveLength(2);
    expect(workers[1].sent).toHaveLength(1);
  });

  it('a worker that fails to load fails what waits with a Turkish message', async () => {
    const { client, workers } = setup();
    const r = client.readCoords(new Uint8Array(1), options);
    workers[0].onerror?.(new Event('error'));
    await expect(r).rejects.toThrow('Dosya biçimi modülü yüklenemedi');
    expect(workers[0].terminated).toBe(true);
  });

  it('a request the worker cannot take fails at once', async () => {
    const { client, workers } = setup();
    void client.readCoords(new Uint8Array(1), options);
    workers[0].refuse = new Error('DataCloneError');
    await expect(client.readCoords(new Uint8Array(1), options)).rejects.toThrow('Dosya biçimi modülüne gönderilemedi: DataCloneError');
  });

  it('stops an idle worker after half a minute, not while a request waits', async () => {
    vi.useFakeTimers();
    const { client, workers } = setup();
    const first = client.readCoords(new Uint8Array(1), options);
    const w = workers[0];
    w.reply({ id: w.id(0), ok: true, json: encode(1) });
    await first;
    vi.advanceTimersByTime(29_999);
    expect(w.terminated).toBe(false);
    // A new request before the half minute keeps the worker, however long it takes.
    const second = client.readCoords(new Uint8Array(1), options);
    vi.advanceTimersByTime(60_000);
    expect(w.terminated).toBe(false);
    w.reply({ id: w.id(1), ok: true, json: encode(2) });
    await second;
    vi.advanceTimersByTime(30_000);
    expect(w.terminated).toBe(true);
    expect(workers).toHaveLength(1);
  });

  it('cancel stops the worker and fails what waits', async () => {
    const { client, workers } = setup();
    const r = client.readDxf(new Uint8Array(1), { maxEntities: 0 });
    client.cancel();
    await expect(r).rejects.toThrow('İşlem durduruldu.');
    expect(workers[0].terminated).toBe(true);
  });
});
