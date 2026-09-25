import { describe, expect, it } from 'vitest';
import { isUuid } from '../core/uuid';
import { v1IdentitiesInProcess } from '../io/testFormats';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { newProjectContent } from '../model/newProject';
import { toSnapshot } from '../model/snapshot';
import { snapshotSampleDocument } from '../model/snapshotSample';
import type { AppContext } from './context';
import { DocumentFiles, type DiscardChoice, type DrawingFileHandle, type DrawingFilePicker } from './fileIO';

/** The objects without their persistent ids, which v1 files do not hold. */
const bare = (doc: CadDocument) => [...doc.all()].map(({ uid: _uid, ...e }) => e);

/** An in-memory file; `fail` makes its writer throw on close, `during` runs while it is being written. */
function memoryFile(name: string, opts: { text?: string; fail?: boolean; during?: () => void } = {}) {
  const file = {
    name,
    text: opts.text ?? '',
    getFile: async () => new Blob([file.text]),
    createWritable: async () => {
      let buffer = '';
      return {
        write: async (data: string) => {
          buffer += data;
          opts.during?.();
        },
        close: async () => {
          if (opts.fail) throw new Error('disk dolu');
          file.text = buffer;
        },
      };
    },
  };
  return file satisfies DrawingFileHandle;
}

/** A cloud project as the file service sees it: open or not, autosaving or not, and how many changes stay unsent. */
function fakeCloud(open: { name: string; canWrite: boolean; unsent: number } | null) {
  const cloud = {
    project: { value: open ? { name: open.name, canWrite: open.canWrite } : null },
    sync: { value: null as { state: { value: string } } | null },
    left: 0,
    autosaves: () => !!cloud.project.value?.canWrite,
    leave: async () => {
      cloud.left++;
      cloud.project.value = null;
      return open?.unsent ?? 0;
    },
    detach: () => {
      cloud.project.value = null;
    },
  };
  return cloud;
}

function setup(doc = new CadDocument({ name: 'Proje', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 } }), cloud = fakeCloud(null)) {
  const messages: string[] = [];
  const say = (kind: string) => (text: string) => messages.push(`${kind}: ${text}`);
  const fitted: unknown[] = [];
  const ctx = {
    doc,
    log: { success: say('ok'), warn: say('uyarı'), error: say('hata'), info: say('bilgi') },
    tools: { activate: () => {} },
    selection: { clear: () => {} },
    view: { camera: { fit: (b: unknown) => fitted.push(b) }, zoomExtents: () => {} },
    cloud,
  } as unknown as AppContext;
  const files = new DocumentFiles(ctx);
  // The formats module in this process instead of its worker.
  files.identities = v1IdentitiesInProcess;
  // Every question is recorded and answered with the next choice (none left: the test did not expect one).
  const asked: string[] = [];
  const answers: DiscardChoice[] = [];
  files.ask = async (_name, after) => {
    asked.push(after);
    const next = answers.shift();
    if (!next) throw new Error(`beklenmeyen soru: ${after}`);
    return next;
  };
  return { doc, files, messages, asked, answers, cloud, fitted };
}

const fresh = () => newProjectContent({ name: 'Ada 200', srid: 5254, plotScale: 500 });

const pick = (save: DrawingFileHandle | null, open: DrawingFileHandle | null = null): DrawingFilePicker => ({ save: async () => save, open: async () => open });

describe('local drawing files', () => {
  it('clears dirty only once the file is written, and saves there again without asking', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    const file = memoryFile('Pafta 12.kcad');
    files.picker = pick(file);
    expect(await files.save()).toBe(true);
    expect(doc.dirty.value).toBe(false);
    expect(JSON.parse(file.text)).toEqual(JSON.parse(JSON.stringify(toSnapshot(doc))));
    // The project takes the file's name, without the extension.
    expect(doc.name.value).toBe('Pafta 12');
    expect(messages.at(-1)).toBe('ok: “Pafta 12.kcad” kaydedildi.');
    // The next save goes to the same file; the picker is not asked.
    files.picker = pick(null);
    doc.add({ kind: 'point', layerId: 'x', p: { x: 3, y: 4 }, attrs: {} });
    expect(await files.save()).toBe(true);
    expect(JSON.parse(file.text).entities).toHaveLength(2);
    expect(doc.dirty.value).toBe(false);
  });

  it('keeps the drawing unsaved when the write fails or the user cancels', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    files.picker = pick(memoryFile('a.kcad', { fail: true }));
    expect(await files.saveAs()).toBe(false);
    expect(doc.dirty.value).toBe(true);
    expect(messages.at(-1)).toMatch(/^hata: “a\.kcad” yazılamadı: disk dolu\. Değişiklikler kaydedilmemiş sayılıyor/);
    files.picker = pick(null);
    expect(await files.saveAs()).toBe(false);
    expect(doc.dirty.value).toBe(true);
  });

  it('an edit made while the file is being written stays unsaved', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    files.picker = pick(memoryFile('a.kcad', { during: () => doc.add({ kind: 'point', layerId: 'x', p: { x: 5, y: 5 }, attrs: {} }) }));
    expect(await files.save()).toBe(true);
    expect(doc.dirty.value).toBe(true);
    expect(messages.at(-1)).toMatch(/^uyarı: .*kayıt sürerken yapılan değişiklikler henüz kaydedilmedi/);
  });

  it('opens a drawing, and refuses a broken one without touching the open drawing', async () => {
    const { doc, files, messages } = setup();
    const src = snapshotSampleDocument();
    const good = memoryFile('Örnek.kcad', { text: JSON.stringify(toSnapshot(src)) });
    files.picker = pick(null, good);
    expect(await files.open()).toBe(true);
    expect(bare(doc)).toEqual(bare(src));
    expect(doc.dirty.value).toBe(false);
    expect(files.handle).toBe(good);
    // The objects' persistent ids are the ones the file's content gives (ADR 0014), not the source drawing's.
    const ids = await v1IdentitiesInProcess(good.text);
    expect([...doc.all()].map((e) => [e.id, e.uid])).toEqual(ids.entities.map((e) => [e.id, e.uid]));
    expect(messages.some((m) => m.startsWith('uyarı'))).toBe(false);
    const size = doc.size;
    files.picker = pick(null, memoryFile('bozuk.kcad', { text: '{"format":"kentos.document","version":9}' }));
    expect(await files.open()).toBe(false);
    expect(messages.at(-1)).toMatch(/^hata: “bozuk\.kcad” açılamadı: /);
    expect(doc.size).toBe(size);
    expect(files.handle).toBe(good);
  });

  it('a file read without write access is not where Save writes', async () => {
    const { files } = setup();
    const text = JSON.stringify(toSnapshot(snapshotSampleDocument()));
    expect(await files.load(text, memoryFile('salt.kcad', { text }), true)).toBe(true);
    expect(files.handle).toBeNull();
  });

  it('opens the same file with the same ids every time, also after an unchanged save; an edited file is another drawing', async () => {
    const { doc, files } = setup();
    const text = JSON.stringify(toSnapshot(snapshotSampleDocument()));
    const file = memoryFile('Örnek.kcad', { text });
    await files.load(text, file);
    const first = [...doc.all()].map((e) => e.uid);
    doc.add({ kind: 'point', layerId: 'cizim', p: { x: 1, y: 1 }, attrs: {} });
    await files.load(text, file);
    expect([...doc.all()].map((e) => e.uid)).toEqual(first);
    expect(await files.save()).toBe(true);
    await files.load(file.text, file);
    expect([...doc.all()].map((e) => e.uid)).toEqual(first);
    // v1 keeps no ids, so an edited and saved file is another snapshot: every object gets other ids
    // when it is opened again. The binary v2 file stores them (TODOS.md FILE-05, docs/adr/0014).
    doc.update(1, { attrs: { Ad: 'P9' } });
    expect(await files.save()).toBe(true);
    await files.load(file.text, file);
    expect([...doc.all()].map((e) => e.uid).filter((u) => first.includes(u!))).toEqual([]);
  });

  it('opens a drawing whose ids cannot be derived with new ones, and says so', async () => {
    const { doc, files, messages } = setup();
    files.identities = async () => {
      throw new Error('Dosya biçimi modülü yüklenemedi');
    };
    const text = JSON.stringify(toSnapshot(snapshotSampleDocument()));
    expect(await files.load(text, memoryFile('Örnek.kcad', { text }))).toBe(true);
    expect(doc.size).toBe(13);
    expect([...doc.all()].every((e) => isUuid(e.uid))).toBe(true);
    expect(messages).toContain(
      'uyarı: “Örnek.kcad”: nesnelerin kalıcı kimlikleri dosyadan türetilemedi (Dosya biçimi modülü yüklenemedi); bu açılış için yeni kimlik verildi ve dosya yeniden açılınca kimlikler değişir. Dosyayı yeniden açmayı deneyin.',
    );
  });

  it('one file command at a time', async () => {
    const { doc, files } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    files.picker = pick(memoryFile('a.kcad'));
    const first = files.save();
    expect(files.busy.value).toBe(true);
    expect(await files.save()).toBe(false);
    expect(await first).toBe(true);
    expect(files.busy.value).toBe(false);
  });
});

describe('new project', () => {
  it('replaces a clean drawing without asking and has no file yet', async () => {
    const { doc, files, asked, messages, fitted } = setup();
    files.handle = memoryFile('eski.kcad');
    expect(await files.newProject(fresh())).toBe(true);
    expect([doc.size, doc.name.value, doc.crs.value.srid, doc.settings.plotScale.value, doc.dirty.value]).toEqual([0, 'Ada 200', 5254, 500, false]);
    expect([files.handle, asked.length]).toEqual([null, 0]);
    // One sheet at 1:500 around the zone's work-area centre.
    expect(fitted).toEqual([{ minX: 499_875, minY: 4_319_906.25, maxX: 500_125, maxY: 4_320_093.75 }]);
    expect(messages.at(-1)).toBe('ok: “Ada 200” yeni projesi açıldı: TUREF / TM30 (EPSG:5254), 1:500. İlk kayıtta dosyanın yeri sorulur.');
  });

  it('asks about unsaved changes: stay keeps everything, drop replaces, save writes first', async () => {
    const { doc, files, asked, answers } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    answers.push('stay');
    expect(await files.newProject(fresh())).toBe(false);
    expect([doc.size, doc.dirty.value, asked[0]]).toEqual([1, true, 'Yeni proje açılırsa bu değişiklikler kaybolur.']);
    answers.push('save');
    const file = memoryFile('Proje.kcad');
    files.picker = pick(file);
    expect(await files.newProject(fresh())).toBe(true);
    expect(JSON.parse(file.text).entities).toHaveLength(1);
    expect([doc.size, files.handle]).toEqual([0, null]);
    doc.add({ kind: 'point', layerId: 'taslak', p: { x: 1, y: 2 }, attrs: {} });
    answers.push('drop');
    expect(await files.newProject(fresh())).toBe(true);
    expect([doc.size, doc.dirty.value]).toEqual([0, false]);
    // A failed save stays: nothing is replaced.
    doc.add({ kind: 'point', layerId: 'taslak', p: { x: 1, y: 2 }, attrs: {} });
    answers.push('save');
    files.picker = pick(memoryFile('dolu.kcad', { fail: true }));
    expect(await files.newProject(fresh())).toBe(false);
    expect(doc.size).toBe(1);
  });

  it('leaves an autosaving cloud project without asking and says what stayed on the device', async () => {
    const cloud = fakeCloud({ name: 'Ada 101', canWrite: true, unsent: 2 });
    const { doc, files, asked, messages } = setup(undefined, cloud);
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    expect(await files.newProject(fresh())).toBe(true);
    expect([asked.length, cloud.left, cloud.project.value]).toEqual([0, 1, null]);
    expect(messages).toContain('bilgi: “Ada 101” bulut projesi kapatıldı. Gönderilemeyen 2 değişiklik bu cihazda saklanıyor; proje yeniden açılınca geri gelir.');
  });

  it('asks a viewer, whose cloud edits are not kept', async () => {
    const cloud = fakeCloud({ name: 'Ada 101', canWrite: false, unsent: 0 });
    const { doc, files, asked, answers } = setup(undefined, cloud);
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    answers.push('stay');
    expect(await files.newProject(fresh())).toBe(false);
    expect([asked.length, cloud.left, doc.size]).toEqual([1, 0, 1]);
  });

  it('waits while an edit is open, and opening a file leaves the cloud project too', async () => {
    const { doc, files, messages } = setup(undefined, fakeCloud({ name: 'Ada 101', canWrite: true, unsent: 0 }));
    const group = doc.beginGroup('işlem');
    expect(await files.newProject(fresh())).toBe(false);
    expect(messages.at(-1)).toMatch(/^uyarı: Bir işlem sürerken yeni proje açılamaz/);
    group.cancel();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    const src = snapshotSampleDocument();
    files.picker = pick(null, memoryFile('Örnek.kcad', { text: JSON.stringify(toSnapshot(src)) }));
    // Dirty, but the cloud project keeps the changes: no question.
    expect(await files.open()).toBe(true);
    expect(doc.size).toBe(src.size);
  });
});
