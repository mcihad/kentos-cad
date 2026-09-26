import { describe, expect, it } from 'vitest';
import { KcadError } from '../io/kcad';
import { formatsBuilt, kcadInProcess } from '../io/testFormats';
import { toSnapshot } from '../model/snapshot';
import { snapshotSampleDocument } from '../model/snapshotSample';
import type { DrawingCodec } from './drawingFile';
import { memoryFile, pick, recordingView, setup } from './fileTesting';

/**
 * Opening in stages (TODOS.md FILE-20, docs/adr/0030; CLAUDE.md §21.2): the
 * open's window hears the project before its objects, then the reading and
 * the checks; Vazgeç stops the open at any stage; a late result, or one that
 * finds the drawing changed or replaced meanwhile, replaces nothing; a file
 * read in part never becomes the drawing.
 */

/** A saved drawing of `n` extra points (the sample's objects first), as a v2 file. */
async function savedDrawing(n: number) {
  const src = snapshotSampleDocument();
  for (let i = 0; i < n; i++) src.add({ kind: 'point', layerId: 'cizim', p: { x: 486500 + i, y: 4420200 }, attrs: { Ad: `N${i}` } });
  const file = memoryFile('Büyük.kcad');
  const writer = setup(src);
  writer.files.picker = pick(file);
  expect(await writer.files.saveAs()).toBe(true);
  return { file, size: src.size };
}

describe.skipIf(!formatsBuilt)('opening in stages (TODOS.md FILE-20)', () => {
  it('shows the project before its objects, then the checks, and replaces the drawing once, whole', async () => {
    const { file, size } = await savedDrawing(3000);
    const { doc, files, messages } = setup();
    const seen: string[] = [];
    files.opening = async () => recordingView(seen);
    let replaced = 0;
    doc.events.on('reset', () => replaced++);
    files.picker = pick(null, file);
    expect(await files.open()).toBe(true);
    expect([doc.size, replaced, doc.dirty.value]).toEqual([size, 1, false]);
    const at = (pattern: RegExp) => seen.findIndex((s) => pattern.test(s));
    // Save as named the drawing after its file.
    const project = at(/^project: “Büyük”: 3\.013 nesne, \d+ üst katman$/);
    expect(project).toBeGreaterThan(at(/^step: Dosya denetleniyor/));
    expect(project).toBeLessThan(at(/^step: Nesneler okunuyor/));
    expect(at(/^step: Nesneler okunuyor/)).toBeLessThan(at(/^step: Nesneler denetleniyor: 3\.013 \/ 3\.013$/));
    expect(seen.slice(-2)).toEqual(['step: Çizim ekrana getiriliyor…', 'close']);
    expect(messages.at(-1)).toMatch(/^ok: “Büyük\.kcad” açıldı: 3013 nesne, \d+ katman\.$/);
  });

  it('Vazgeç stops it at any stage: the drawing on screen stays as it was', async () => {
    const { file } = await savedDrawing(3000);
    for (const stage of [/^step: Dosya denetleniyor/, /^step: Nesneler okunuyor/, /^step: Nesneler denetleniyor/, /^step: Çizim ekrana/]) {
      const { doc, files, messages } = setup();
      doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
      const before = [doc.size, doc.name.value, doc.revision];
      const seen: string[] = [];
      files.opening = async (_name, cancel) => recordingView(seen, (text) => stage.test(`step: ${text}`) && cancel());
      files.ask = async () => 'drop';
      files.picker = pick(null, file);
      expect(await files.open(), stage.source).toBe(false);
      expect([doc.size, doc.name.value, doc.revision], stage.source).toEqual(before);
      expect(seen.at(-1)).toBe('close');
      expect(messages.at(-1), stage.source).toBe('bilgi: “Büyük.kcad” açılışı durduruldu; ekrandaki çizim olduğu gibi duruyor.');
      expect(files.handle).toBeNull();
    }
  });

  it('a decode stopped in the worker, or one that answers after Vazgeç, replaces nothing', async () => {
    const { file } = await savedDrawing(10);
    const inProcess = await kcadInProcess();
    for (const late of [false, true]) {
      const { doc, files, messages } = setup();
      const before = doc.revision;
      let release = null as (() => void) | null;
      // A codec whose reading waits: Vazgeç ends the worker (rejects), or the answer comes anyway, late.
      const codec: DrawingCodec = {
        encode: inProcess.encode,
        decode: (bytes, progress) =>
          new Promise((resolve, reject) => {
            release = () => void inProcess.decode(bytes, progress).then(resolve, reject);
            if (!late) codec.cancel = () => reject(new KcadError('cancelled', 'İşlem durduruldu.'));
          }),
      };
      files.kcad = async () => codec;
      let stop: () => void = () => {};
      files.opening = async (_name, cancel) => ((stop = cancel), recordingView([]));
      files.picker = pick(null, file);
      const opening = files.open();
      // Until the worker has the file.
      for (let i = 0; i < 200 && !release; i++) await new Promise((r) => setTimeout(r, 1));
      expect(release).not.toBeNull();
      stop();
      release!();
      expect(await opening, `late ${late}`).toBe(false);
      expect([doc.revision, doc.size]).toEqual([before, 0]);
      expect(messages.at(-1)).toMatch(/açılışı durduruldu/);
    }
  });

  it('gives way when the drawing on screen changed or another was opened meanwhile', async () => {
    const { file } = await savedDrawing(3000);
    const { doc, files, messages } = setup();
    const other = snapshotSampleDocument();
    // A cloud project put on screen while the file was being checked (it asks nobody, app/cloud/session.ts).
    files.opening = async () =>
      recordingView([], (text) => {
        if (/^Nesneler denetleniyor/.test(text) && doc.name.value !== other.name.value)
          doc.replaceWith({ name: other.name.value, settings: other.settings.toJSON(), origin: other.origin, homeView: null, layers: [...other.layers.tree], activeLayer: 'cizim', entities: [], styles: { items: [], categories: [] } });
      });
    files.picker = pick(null, file);
    expect(await files.open()).toBe(false);
    expect([doc.name.value, doc.size]).toEqual([other.name.value, 0]);
    expect(messages.at(-1)).toMatch(/^uyarı: “Büyük\.kcad” açılmadı: açılış sürerken ekrandaki çizim değişti ya da başka bir çizim açıldı; o çizim olduğu gibi duruyor/);
  });

  it('a v1 file is read in stages too, and a broken one never becomes the drawing', async () => {
    const { doc, files, messages } = setup();
    const text = JSON.stringify(toSnapshot(snapshotSampleDocument()));
    const seen: string[] = [];
    files.opening = async () => recordingView(seen);
    files.picker = pick(null, memoryFile('Eski.kcad', { data: text }));
    expect(await files.open()).toBe(true);
    expect(seen.some((s) => /^project: “Örnek pafta\.kcad”: 13 nesne, 2 üst katman$/.test(s)), seen.join(' | ')).toBe(true);
    expect(seen.some((s) => /^step: Nesneler denetleniyor: 13 \/ 13$/.test(s))).toBe(true);
    const size = doc.size;
    const broken = JSON.parse(text);
    broken.entities[12].layerId = 'yok';
    files.picker = pick(null, memoryFile('Bozuk.kcad', { data: JSON.stringify(broken) }));
    expect(await files.open()).toBe(false);
    expect(doc.size).toBe(size);
    expect(messages.at(-1)).toMatch(/^hata: “Bozuk\.kcad” açılamadı: Nesne 13 \(\w+\) › katman: “yok” katmanı dosyada yok$/);
  });
});
