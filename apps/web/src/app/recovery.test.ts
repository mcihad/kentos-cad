import { describe, expect, it } from 'vitest';
import { unpackSnapshot } from '../io/columns';
import { formatsBuilt, kcadInProcess } from '../io/testFormats';
import { snapshotSampleDocument } from '../model/snapshotSample';
import { fakeCloud, memoryFile, pick, setup } from './fileTesting';
import { RecoveryCopies, readCopy, type RecoveryAnswer, type RecoveryCopy, type RecoveryStore } from './recovery';

/**
 * Local recovery copies (TODOS.md FILE-19, docs/adr/0030): unsaved work is
 * kept on the device apart from any `.kcad` file, removed when the drawing
 * is saved or its changes are dropped on purpose, and offered when the app
 * starts after a crash; a restored copy is unsaved and has no file, so no
 * file is ever written over by itself.
 */

function memoryStore(): RecoveryStore & { items: Map<string, unknown> } {
  const items = new Map<string, unknown>();
  return {
    items,
    list: async () => [...items.values()],
    put: async (c) => void items.set(c.id, c),
    delete: async (id) => void items.delete(id),
  };
}

const decode = async (bytes: Uint8Array) => unpackSnapshot(await (await kcadInProcess()).decode(bytes));

/** The app's file service, a store, and copies whose questions are answered from `answers`. */
function app(o: { alive?: string[]; answers?: RecoveryAnswer[]; store?: ReturnType<typeof memoryStore>; cloud?: ReturnType<typeof fakeCloud> } = {}) {
  const s = setup(undefined, o.cloud);
  const store = o.store ?? memoryStore();
  const asked: string[] = [];
  const answers = [...(o.answers ?? [])];
  const recovery = new RecoveryCopies(
    { ...s.ctx, files: s.files },
    {
      store,
      live: async () => new Set(o.alive ?? []),
      ask: async (c) => (asked.push(c.name), answers.shift() ?? 'later'),
      quietMs: 1e9,
      maxMs: 1e9,
    },
  );
  s.files.discarded = () => recovery.discard();
  recovery.start();
  return { ...s, store, recovery, asked };
}

/** A copy another tab left: the sample drawing, edited, as KCAD v2. */
async function copyOf(name: string, tab: string, savedAt: number): Promise<RecoveryCopy> {
  const doc = snapshotSampleDocument();
  doc.name.set(name);
  const file = memoryFile(`${name}.kcad`);
  const writer = setup(doc);
  writer.files.picker = pick(file);
  await writer.files.saveAs();
  return { id: `${tab}/1`, tab, name, file: `${name}.kcad`, savedAt, objects: doc.size, bytes: file.bytes, version: 1 };
}

describe.skipIf(!formatsBuilt)('local recovery copies (TODOS.md FILE-19)', () => {
  it('keeps a copy of unsaved work; a save removes it', async () => {
    const { doc, files, store, recovery } = app();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: { Ad: 'P1' } });
    await recovery.flush();
    const copy = readCopy(store.items.get(recovery.current))!;
    expect([copy.name, copy.objects, copy.file, copy.tab]).toEqual(['Proje', 1, null, recovery.tab]);
    expect((await decode(copy.bytes)).entities.map((e) => e.attrs)).toEqual([{ Ad: 'P1' }]);
    // A later edit replaces the copy (one per drawing), holding the new state.
    doc.add({ kind: 'point', layerId: 'x', p: { x: 3, y: 4 }, attrs: {} });
    await recovery.flush();
    expect(store.items.size).toBe(1);
    expect((await decode(readCopy(store.items.get(recovery.current))!.bytes)).entities).toHaveLength(2);
    files.picker = pick(memoryFile('Pafta.kcad'));
    expect(await files.saveAs()).toBe(true);
    expect(store.items.size).toBe(0);
    recovery.dispose();
  });

  it('dropping the changes on purpose removes the copy; a drawing replaced without the question keeps it', async () => {
    const { doc, files, store, recovery, answers } = app();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    await recovery.flush();
    const first = recovery.current;
    expect(store.items.has(first)).toBe(true);
    // Kaydetmeden devam et (Yeni proje): the copy goes with the changes.
    answers.push('drop');
    const sample = snapshotSampleDocument();
    const content = { name: 'Yeni', settings: sample.settings.toJSON(), origin: sample.origin, homeView: null, layers: [...sample.layers.tree], activeLayer: 'cizim', entities: [], styles: { items: [], categories: [] } };
    expect(await files.newProject(content)).toBe(true);
    expect(store.items.size).toBe(0);
    // Unsaved work replaced without that question (a cloud project opened over it) stays, to be offered later.
    doc.add({ kind: 'point', layerId: 'cizim', p: { x: 1, y: 2 }, attrs: {} });
    await recovery.flush();
    const second = recovery.current;
    doc.replaceWith({ ...content, name: 'Bulut' });
    expect(recovery.current).not.toBe(second);
    doc.add({ kind: 'point', layerId: 'cizim', p: { x: 5, y: 6 }, attrs: {} });
    await recovery.flush();
    expect([...store.items.keys()].sort()).toEqual([second, recovery.current].sort());
    recovery.dispose();
  });

  it('keeps no copy of an open cloud project: its device draft keeps the changes', async () => {
    const { doc, store, recovery } = app({ cloud: fakeCloud({ name: 'Ada 101', canWrite: true, unsent: 0 }) });
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    await recovery.flush();
    expect(store.items.size).toBe(0);
    recovery.dispose();
  });

  it('offers what gone tabs left, newest first; restore puts it on screen unsaved and without a file', async () => {
    const store = memoryStore();
    const old = await copyOf('Eski', 'olu-sekme-1', 1000);
    const newest = await copyOf('Yeni', 'olu-sekme-2', 2000);
    const live = await copyOf('Canlı', 'canli-sekme', 3000);
    for (const c of [old, newest, live]) await store.put(c);
    store.items.set('bozuk', { id: 'bozuk', version: 9 });
    const { doc, files, recovery, asked, messages } = app({ store, alive: ['canli-sekme'], answers: ['later', 'restore'] });
    expect(await recovery.offer()).toBe(true);
    // The live tab's copy and the unreadable record are not offered.
    expect(asked).toEqual(['Yeni', 'Eski']);
    expect([doc.name.value, doc.size, doc.dirty.value, files.handle]).toEqual(['Eski', old.objects, true, null]);
    expect(messages.at(-1)).toMatch(/^ok: “Eski” kaydedilmemiş çalışması geri yüklendi: 13 nesne\. .*Kaydet dosyanın yerini sorar, hiçbir dosyanın üzerine kendiliğinden yazılmaz\.$/);
    // Kept for later, left alone, the restored one gone; the drawing on screen has its own copy now.
    expect([...store.items.keys()].sort()).toEqual([newest.id, live.id, 'bozuk', recovery.current].sort());
    recovery.dispose();
  });

  it('Sil deletes a copy; Sonra keeps it for the next start', async () => {
    const store = memoryStore();
    const a = await copyOf('A', 'olu-1', 1000);
    const b = await copyOf('B', 'olu-2', 2000);
    await store.put(a);
    await store.put(b);
    const first = app({ store, answers: ['delete', 'later'] });
    expect(await first.recovery.offer()).toBe(false);
    expect([...store.items.keys()]).toEqual([a.id]);
    first.recovery.dispose();
    const next = app({ store, answers: ['later'] });
    await next.recovery.offer();
    expect(next.asked).toEqual(['A']);
    next.recovery.dispose();
  });
});
