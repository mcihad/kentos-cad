import { describe, expect, it } from 'vitest';
import { unpackSnapshot } from '../io/columns';
import { formatsBuilt, kcadInProcess } from '../io/testFormats';
import type { DrawingFileHandle } from './fileIO';
import { memoryFile, pick, setup } from './fileTesting';
import { RecoveryCopies, readCopy, type RecoveryCopy, type RecoveryStore } from './recovery';

/**
 * Saving when something goes wrong (TODOS.md FILE-18, docs/adr/0030): the
 * permission of a chosen file refused or taken back, a full disk, the tab
 * closing or hiding while the file is written. In every case the previous
 * file stays as it was (the browser writes to a copy and puts it in place
 * only when the writer closes), the drawing stays unsaved, and the message
 * says what happened and what to do.
 */

const OLD = 'önceki sağlam dosya';
const text = (b: Uint8Array) => new TextDecoder().decode(b);

/** A file whose permission the browser asks about; `answer` is the user's (or the browser's) reply. */
function permissioned(state: { now: PermissionState; answer: PermissionState }) {
  const file = memoryFile('pafta.kcad', { data: OLD });
  const asked: string[] = [];
  return Object.assign(file, {
    asked,
    queryPermission: async () => (asked.push('query'), state.now),
    requestPermission: async () => {
      asked.push('request');
      state.now = state.answer;
      return state.answer;
    },
  });
}

/** A file whose writer fails at `step` with a DOMException named `name`; records whether it was aborted. */
function failing(step: 'open' | 'write' | 'close', name: string) {
  const file = memoryFile('pafta.kcad', { data: OLD });
  const seen = { aborted: false };
  const handle: DrawingFileHandle & { bytes: Uint8Array } = Object.assign(file, {
    createWritable: async () => {
      if (step === 'open') throw new DOMException('tarayıcı izni geri aldı', name);
      return {
        write: async () => {
          if (step === 'write') throw new DOMException('yazılamadı', name);
        },
        close: async () => {
          if (step === 'close') throw new DOMException('kapatılamadı', name);
          file.bytes = new TextEncoder().encode('YENİ');
        },
        abort: async () => {
          seen.aborted = true;
        },
      };
    },
  });
  return { handle, seen };
}

function memoryStore(): RecoveryStore & { items: Map<string, RecoveryCopy> } {
  const items = new Map<string, RecoveryCopy>();
  return {
    items,
    list: async () => [...items.values()],
    put: async (c) => void items.set(c.id, c),
    delete: async (id) => void items.delete(id),
  };
}

describe.skipIf(!formatsBuilt)('saving when something goes wrong (TODOS.md FILE-18)', () => {
  it('asks for a forgotten permission again, and writes nothing when it is refused', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    const state = { now: 'prompt' as PermissionState, answer: 'denied' as PermissionState };
    const file = permissioned(state);
    files.handle = file;
    expect(await files.save()).toBe(false);
    expect([doc.dirty.value, text(file.bytes), file.asked]).toEqual([true, OLD, ['query', 'request']]);
    expect(messages.at(-1)).toMatch(/^hata: “pafta\.kcad” dosyasına yazma izni verilmedi; çizim kaydedilmedi ve önceki dosya olduğu gibi duruyor\. Kaydet'e yeniden basıp izni verin ya da Farklı kaydet/);
    // Given the next time: written.
    state.answer = 'granted';
    expect(await files.save()).toBe(true);
    expect(doc.dirty.value).toBe(false);
    expect(unpackSnapshot(await (await kcadInProcess()).decode(file.bytes)).entities).toHaveLength(1);
  });

  it('a permission taken back while the file is written: the previous file stays, the drawing stays unsaved', async () => {
    for (const step of ['open', 'write', 'close'] as const) {
      const { doc, files, messages } = setup();
      doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
      const { handle, seen } = failing(step, 'NotAllowedError');
      files.handle = handle;
      expect(await files.save(), step).toBe(false);
      expect([doc.dirty.value, text(handle.bytes)], step).toEqual([true, OLD]);
      // A writer that was opened is aborted: the browser drops its copy.
      expect(seen.aborted, step).toBe(step !== 'open');
      expect(messages.at(-1), step).toMatch(/^hata: “pafta\.kcad” için yazma izni yok ya da geri alındı\. Önceki dosya olduğu gibi duruyor; .*Değişiklikler kaydedilmemiş sayılıyor\.$/);
    }
  });

  it('a full disk: the writer is aborted, the previous file stays, the message says to make room', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    const { handle, seen } = failing('write', 'QuotaExceededError');
    files.handle = handle;
    expect(await files.save()).toBe(false);
    expect([doc.dirty.value, text(handle.bytes), seen.aborted]).toEqual([true, OLD, true]);
    expect(messages.at(-1)).toMatch(/^hata: “pafta\.kcad” yazılamadı: diskte ya da tarayıcının depolama alanında yer kalmadı\. Önceki dosya olduğu gibi duruyor; yer açıp yeniden kaydedin/);
  });

  it('the tab closing or hidden while the file is written: nothing is replaced before the writer closes, and the recovery copy holds the work', async () => {
    const { ctx, doc, files } = setup();
    const store = memoryStore();
    const recovery = new RecoveryCopies({ ...ctx, files }, { store, live: async () => new Set(), quietMs: 1e9, maxMs: 1e9 });
    recovery.start();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: { Ad: 'P1' } });
    // The tab is hidden: the copy is written at once (the page may be gone next).
    await recovery.flush();
    expect(store.items.size).toBe(1);
    // A writer that never closes: the tab goes away in the middle of the save.
    const file = memoryFile('pafta.kcad', { data: OLD });
    let wrote = false;
    files.handle = Object.assign(file, {
      createWritable: async () => ({
        write: async () => void (wrote = true),
        close: () => new Promise<void>(() => {}),
      }),
    });
    const saving = files.save();
    for (let i = 0; i < 50 && !wrote; i++) await new Promise((r) => setTimeout(r, 1));
    expect(wrote).toBe(true);
    // Everything as it was: the old file, an unsaved drawing, the save still waiting.
    expect([text(file.bytes), doc.dirty.value, files.busy.value]).toEqual([OLD, true, true]);
    // What the next start offers is the drawing itself.
    const copy = readCopy([...store.items.values()][0])!;
    const back = unpackSnapshot(await (await kcadInProcess()).decode(copy.bytes));
    expect(back.entities.map((e) => e.attrs)).toEqual([{ Ad: 'P1' }]);
    expect(copy.name).toBe('Proje');
    void saving;
    recovery.dispose();
  });

  it('a save that fails keeps the recovery copy; one that succeeds removes it', async () => {
    const { ctx, doc, files } = setup();
    const store = memoryStore();
    const recovery = new RecoveryCopies({ ...ctx, files }, { store, live: async () => new Set(), quietMs: 1e9, maxMs: 1e9 });
    recovery.start();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    await recovery.flush();
    files.picker = pick(memoryFile('a.kcad', { fail: true }));
    expect(await files.saveAs()).toBe(false);
    expect(store.items.size).toBe(1);
    files.picker = pick(memoryFile('b.kcad'));
    expect(await files.saveAs()).toBe(true);
    expect(store.items.size).toBe(0);
    recovery.dispose();
  });
});
