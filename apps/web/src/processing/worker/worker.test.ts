import { describe, expect, it } from 'vitest';
import { CadDocument } from '../../model/document';
import type { Entity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { LayerStore } from '../../model/layers';
import { BUILTIN_TOOLS } from '../builtin';
import { calculateField } from '../builtin/calculateField';
import { edgeLengths } from '../builtin/edgeLengths';
import { vertexNumbering } from '../builtin/vertexNumbering';
import { clientExecutor } from '../job';
import { defaultValues } from '../parameters';
import { ProcessingRunner, WORKER_THRESHOLD } from '../runner';
import { defineTool } from '../types';
import { handleJob } from './handleJob';
import type { WorkerRequest } from './protocol';
import { workerExecutor, type WorkerLike } from './workerExecutor';

const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x: number, y: number, s: number) => [v(x, y), v(x + s, y), v(x + s, y + s), v(x, y + s)];
const TOOLS = new Map(BUILTIN_TOOLS.map((t) => [t.id, t]));

/** A Worker stand-in: messages are structured-cloned both ways, as the browser does. */
function fakeWorker(opts: { silent?: boolean } = {}) {
  const w: WorkerLike & { terminated: boolean; sent: WorkerRequest[] } = {
    onmessage: null,
    onerror: null,
    terminated: false,
    sent: [],
    postMessage(m) {
      const copy = structuredClone(m);
      w.sent.push(copy);
      if (opts.silent) return;
      queueMicrotask(() => void handleJob(copy, (r) => !w.terminated && w.onmessage?.({ data: structuredClone(r) }), (id) => TOOLS.get(id)));
    },
    terminate() {
      w.terminated = true;
    },
  };
  return w;
}

function setup(spawn: () => WorkerLike) {
  const doc = new CadDocument({ name: 't.kcad', layers: new LayerStore([{ id: 'a', name: 'Parseller' }], 'a'), origin: v(0, 0) });
  const e1 = doc.add({ kind: 'polygon', pts: square(0, 0, 10), layerId: 'a', label: '1', attrs: { Parsel: '1' } });
  const e2 = doc.add({ kind: 'polygon', pts: square(10, 0, 10), layerId: 'a', label: '2', attrs: { Parsel: '2' } });
  const worker = workerExecutor(spawn, new Set(TOOLS.keys()));
  const runner = new ProcessingRunner({ doc, selectedIds: () => [e1.id, e2.id], visibleBounds: () => null }, [clientExecutor, worker]);
  return { doc, runner, e1, e2 };
}

describe('processing in a worker', () => {
  it('gives the same result as in the page, applied as one undo step', async () => {
    const page = setup(() => fakeWorker());
    const bg = setup(() => fakeWorker());
    const values = defaultValues(vertexNumbering, page.runner.defaults());
    const a = await page.runner.run(vertexNumbering, values, { target: 'client' });
    const b = await bg.runner.run(vertexNumbering, values, { target: 'worker' });
    expect(a.status === 'ok' && a.record.target).toBe('client');
    expect(b.status === 'ok' && b.record.target).toBe('worker');
    const labels = (d: CadDocument) => [...d.all()].filter((e) => e.kind === 'point').map((e) => [e.label, (e as Extract<Entity, { kind: 'point' }>).p]);
    expect(labels(bg.doc)).toEqual(labels(page.doc));
    expect(labels(bg.doc)).toHaveLength(6);
    bg.doc.undo();
    expect(labels(bg.doc)).toHaveLength(0);
  });
  it('takes geometry from a store the worker builds of its copies: the page’s edge labels and areas', async () => {
    const page = setup(() => fakeWorker());
    const bg = setup(() => fakeWorker());
    const texts = (d: CadDocument) => [...d.all()].flatMap((e) => (e.kind === 'text' ? [[e.text, e.p, e.rotation]] : []));
    const edges = defaultValues(edgeLengths, page.runner.defaults());
    await page.runner.run(edgeLengths, edges, { target: 'client' });
    await bg.runner.run(edgeLengths, edges, { target: 'worker' });
    expect(texts(bg.doc)).toEqual(texts(page.doc));
    expect(texts(bg.doc)).toHaveLength(7);
    const area = { ...defaultValues(calculateField, page.runner.defaults()), value: 'metin($alan, 3)' };
    await page.runner.run(calculateField, area, { target: 'client' });
    const out = await bg.runner.run(calculateField, area, { target: 'worker' });
    expect(out.status === 'ok' && out.record.target).toBe('worker');
    expect(bg.doc.get(bg.e2.id)!.attrs['Hesap alanı']).toBe('100.000');
    expect(bg.doc.get(bg.e2.id)!.attrs).toEqual(page.doc.get(page.e2.id)!.attrs);
  });
  it('compiles expressions in the worker', async () => {
    const { doc, runner, e2 } = setup(() => fakeWorker());
    const out = await runner.run(calculateField, { ...defaultValues(calculateField, runner.defaults()), field: 'Etiket', value: "'P-' || Parsel" }, { target: 'worker' });
    expect(out.status === 'ok' && out.record.summary).toBe('2 nesnede “Etiket” yazıldı.');
    expect(doc.get(e2.id)!.attrs.Etiket).toBe('P-2');
  });
  it('"auto" sends big inputs to the worker and keeps small ones in the page', () => {
    const { runner } = setup(() => fakeWorker());
    expect(runner.executorFor(vertexNumbering, 'auto', 10)?.target).toBe('client');
    expect(runner.executorFor(vertexNumbering, 'auto', WORKER_THRESHOLD)?.target).toBe('worker');
    const pageOnly = defineTool({ id: 'x.pageOnly', label: 'X', category: 'points', description: '', targets: ['client'], parameters: [] as const, run: () => ({}) });
    expect(runner.executorFor(pageOnly, 'auto', 1e6)?.target).toBe('client');
    expect(runner.executorFor(pageOnly, 'worker')).toBeNull();
  });
  it('a tool the worker does not have, and a crashed worker, end as errors', async () => {
    const { runner } = setup(() => fakeWorker());
    const stranger = defineTool({ id: 'x.stranger', label: 'Yabancı', category: 'points', description: '', targets: ['worker'], parameters: [] as const, run: () => ({}) });
    const out = await runner.run(stranger, {});
    expect(out.status === 'error' && out.message).toContain('bu ortamda çalıştırılamıyor');
    let crash: WorkerLike | null = null;
    const broken = setup(() => (crash = fakeWorker({ silent: true })));
    const values = defaultValues(vertexNumbering, broken.runner.defaults());
    const pending = broken.runner.run(vertexNumbering, values, { target: 'worker' });
    await new Promise((r) => setTimeout(r, 0));
    crash!.onerror?.(new Error('boom'));
    const failed = await pending;
    expect(failed.status === 'error' && failed.message).toContain('beklenmedik biçimde durdu');
  });
  it('Durdur terminates the worker and leaves the drawing unchanged', async () => {
    const workers: ReturnType<typeof fakeWorker>[] = [];
    const { doc, runner } = setup(() => {
      const w = fakeWorker({ silent: true });
      workers.push(w);
      return w;
    });
    const size = doc.size;
    const pending = runner.run(vertexNumbering, defaultValues(vertexNumbering, runner.defaults()), { target: 'worker' });
    await new Promise((r) => setTimeout(r, 10));
    runner.cancel();
    const out = await pending;
    expect(out.status).toBe('canceled');
    expect(workers[0].terminated).toBe(true);
    expect(doc.size).toBe(size);
    // The next job starts a fresh worker.
    void runner.run(vertexNumbering, defaultValues(vertexNumbering, runner.defaults()), { target: 'worker' });
    await new Promise((r) => setTimeout(r, 0));
    expect(workers).toHaveLength(2);
    expect(workers[1].sent[0].entities).toHaveLength(2);
    runner.cancel();
  });
});
