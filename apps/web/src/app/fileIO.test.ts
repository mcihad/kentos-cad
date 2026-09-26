import { describe, expect, it } from 'vitest';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import { isUuid } from '../core/uuid';
import { difference } from '../io/kcad';
import { unpackSnapshot } from '../io/columns';
import { formatsBuilt, kcadInProcess, v1IdentitiesInProcess } from '../io/testFormats';
import type { CadDocument } from '../model/document';
import { newProjectContent } from '../model/newProject';
import { toSnapshot, toSnapshotV2 } from '../model/snapshot';
import { snapshotSampleDocument } from '../model/snapshotSample';
import { fakeCloud, memoryFile, pick, setup } from './fileTesting';

/** The objects without their persistent ids, which v1 files do not hold. */
const bare = (doc: CadDocument) => [...doc.all()].map(({ uid: _uid, ...e }) => e);
const uids = (doc: CadDocument) => [...doc.all()].map((e) => e.uid);

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
/** A fixture's bytes (a plain Uint8Array: Node gives a Buffer). */
const fixture = (path: string) => new Uint8Array(fs.readFileSync(new URL(`../../../../fixtures/${path}`, import.meta.url)));

/** What a v2 file holds, read by the same module. */
async function readV2(bytes: Uint8Array): Promise<DocumentSnapshotV2> {
  return unpackSnapshot(await (await kcadInProcess()).decode(bytes));
}

const fresh = () => newProjectContent({ name: 'Ada 200', srid: 5254, plotScale: 500 });

const v1Text = (doc: CadDocument) => JSON.stringify(toSnapshot(doc));

describe.skipIf(!formatsBuilt)('local drawing files', () => {
  it('saves KCAD v2, clears dirty only once the file is written, and saves there again without asking', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    const file = memoryFile('Pafta 12.kcad');
    files.picker = pick(file);
    expect(await files.save()).toBe(true);
    expect(doc.dirty.value).toBe(false);
    // The file is KCAD v2 and holds the drawing, persistent ids included.
    expect([...file.bytes.slice(0, 5)]).toEqual([0x89, 0x4b, 0x43, 0x41, 0x44]);
    expect(difference(await readV2(file.bytes), toSnapshotV2(doc))).toBeNull();
    // The project takes the file's name, without the extension.
    expect(doc.name.value).toBe('Pafta 12');
    expect(messages.at(-1)).toBe('ok: “Pafta 12.kcad” kaydedildi.');
    // The next save goes to the same file; the picker is not asked.
    files.picker = pick(null);
    doc.add({ kind: 'point', layerId: 'x', p: { x: 3, y: 4 }, attrs: {} });
    expect(await files.save()).toBe(true);
    expect((await readV2(file.bytes)).entities).toHaveLength(2);
    expect(doc.dirty.value).toBe(false);
  });

  it('keeps the drawing unsaved when the write fails or the user cancels', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    files.picker = pick(memoryFile('a.kcad', { fail: true }));
    expect(await files.saveAs()).toBe(false);
    expect(doc.dirty.value).toBe(true);
    expect(messages.at(-1)).toMatch(/^hata: “a\.kcad” yazılamadı: disk dolu\. Önceki dosya olduğu gibi duruyor; .*Değişiklikler kaydedilmemiş sayılıyor\.$/);
    files.picker = pick(null);
    expect(await files.saveAs()).toBe(false);
    expect(doc.dirty.value).toBe(true);
  });

  it('an edit made while the file is being written stays unsaved', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: 1, y: 2 }, attrs: {} });
    const file = memoryFile('a.kcad', { during: () => doc.add({ kind: 'point', layerId: 'x', p: { x: 5, y: 5 }, attrs: {} }) });
    files.picker = pick(file);
    expect(await files.save()).toBe(true);
    expect(doc.dirty.value).toBe(true);
    expect(messages.at(-1)).toMatch(/^uyarı: .*kayıt sürerken yapılan değişiklikler henüz kaydedilmedi/);
    // The file holds the drawing of the moment the save began.
    expect((await readV2(file.bytes)).entities).toHaveLength(1);
  });

  it('a drawing the file cannot hold is not written, and stays unsaved', async () => {
    const { doc, files, messages } = setup();
    doc.add({ kind: 'point', layerId: 'x', p: { x: Number.NaN, y: 2 }, attrs: {} });
    const file = memoryFile('a.kcad', { data: 'eski içerik' });
    files.picker = pick(file);
    expect(await files.save()).toBe(false);
    expect(doc.dirty.value).toBe(true);
    expect(new TextDecoder().decode(file.bytes)).toBe('eski içerik');
    expect(messages.at(-1)).toMatch(/^hata: “a\.kcad” yazılamadı: Çizim KCAD 2 olarak yazılamıyor: entities\/0\/p\/x: sayı NaN ya da sonsuz/);
  });

  it('Save As leaves a cloud project that ended (deleted, access taken away, archived) before the file takes the drawing', async () => {
    for (const state of ['deleted', 'revoked', 'archived']) {
      const cloud = fakeCloud({ name: 'Bulut', canWrite: true, unsent: 0 });
      cloud.sync.value = { state: { value: state } };
      const { files } = setup(undefined, cloud);
      const file = memoryFile('kopya.kcad');
      files.picker = pick(file);
      expect(await files.saveAs(), state).toBe(true);
      expect(cloud.project.value, state).toBeNull();
      expect(files.handle, state).toBe(file);
    }
  });

  it('opens v2 and v1 by content, and refuses a broken or foreign file without touching the open drawing', async () => {
    const { doc, files, messages } = setup();
    const src = snapshotSampleDocument();
    const v2 = memoryFile('Örnek.kcad');
    const writer = setup(src);
    writer.files.picker = pick(v2);
    expect(await writer.files.saveAs()).toBe(true);
    files.picker = pick(null, v2);
    expect(await files.open()).toBe(true);
    expect(bare(doc)).toEqual(bare(src));
    expect(uids(doc)).toEqual(uids(src));
    expect(doc.dirty.value).toBe(false);
    expect(files.handle).toBe(v2);
    const size = doc.size;
    const cases: [string, Uint8Array | string, RegExp][] = [
      ['bozuk.kcad', fixture('kcad/v2/broken/bad-hash.kcad'), /SHA-256/],
      ['eksik.kcad', v2.bytes.slice(0, 100), /Dosya eksik/],
      ['sürüm.kcad', '{"format":"kentos.document","version":9}', /sürümü 9/],
      ['boş.kcad', '', /Dosya boş/],
      ['plan.kcad', fixture('formats/v1/blocks.dxf'), /İçe aktar/],
    ];
    for (const [name, data, says] of cases) {
      files.picker = pick(null, memoryFile(name, { data }));
      expect(await files.open(), name).toBe(false);
      expect(messages.at(-1), name).toMatch(new RegExp(`^hata: “${name.replace('.', '\\.')}” açılamadı: .*${says.source}`));
      expect(doc.size).toBe(size);
      expect(files.handle).toBe(v2);
    }
  });

  it('keeps every persistent id across v2 saves and opens, edited or not', async () => {
    const { doc, files } = setup();
    const file = memoryFile('Örnek.kcad');
    const writer = setup(snapshotSampleDocument());
    writer.files.picker = pick(file);
    await writer.files.saveAs();
    await files.load(file.bytes, file);
    const first = uids(doc);
    expect(first).toEqual(uids(writer.doc));
    // Edited and saved: the same objects keep their ids, a new one keeps its own.
    doc.update(2, { attrs: { Ad: 'P9' } });
    const added = doc.add({ kind: 'point', layerId: 'cizim', p: { x: 1, y: 1 }, attrs: {} });
    expect(await files.save()).toBe(true);
    await files.load(file.bytes, file);
    expect(uids(doc)).toEqual([...first, added.uid]);
    expect(doc.byUid(first[1]!)?.attrs).toEqual({ Ad: 'P9' });
  });

  it('a v1 file is never written over by Save: Save asks where, and the ids and their source go into the v2 file', async () => {
    const { doc, files, messages } = setup();
    const text = v1Text(snapshotSampleDocument());
    const old = memoryFile('Örnek.kcad', { data: text });
    files.picker = pick(null, old);
    expect(await files.open()).toBe(true);
    expect(files.handle).toBeNull();
    expect(messages.at(-1)).toMatch(/^bilgi: Dosya eski biçimde \(KCAD v1\)\. Kaydet, yeni biçimde \(v2\) yazmak için yer sorar/);
    const ids = await v1IdentitiesInProcess(text);
    expect(uids(doc)).toEqual(ids.entities.map((e) => e.uid));
    expect([doc.projectId, doc.migratedFrom]).toEqual([ids.project, { format: 'kentos.document', version: 1, sourceSha256: ids.sourceSha256 }]);
    // Save asks (the old file's name is suggested); the old file is untouched.
    const suggested: string[] = [];
    const next = memoryFile('Örnek-v2.kcad');
    files.picker = pick(next, null, suggested);
    expect(await files.save()).toBe(true);
    expect(suggested).toEqual(['Örnek pafta.kcad']);
    expect(new TextDecoder().decode(old.bytes)).toBe(text);
    const saved = await readV2(next.bytes);
    expect(saved.uids).toEqual(ids.entities.map((e) => e.uid));
    expect([saved.projectId, saved.migratedFrom?.sourceSha256]).toEqual([ids.project, ids.sourceSha256]);
    // Now the drawing lives in the v2 file: Save writes there without asking.
    expect(files.handle).toBe(next);
    // The user may choose the old file itself: then it becomes the v2 file (their decision, verified bytes).
    await files.load(text, old);
    files.picker = pick(old);
    expect(await files.save()).toBe(true);
    const replaced = await readV2(old.bytes);
    expect([replaced.uids, replaced.migratedFrom]).toEqual([saved.uids, saved.migratedFrom]);
  });

  it('the web saves the v1 sample as the reference migration, byte for byte', async () => {
    const { files } = setup();
    const text = new TextDecoder().decode(fixture('document/v1/sample.json'));
    await files.load(text, null);
    // Written where it is told (Save as would name the project after the file).
    const file = memoryFile('Örnek pafta.kcad');
    files.handle = file;
    expect(await files.save()).toBe(true);
    // fixtures/kcad/v2/migrated.kcad: written by the independent Python writer; the desktop writes it too.
    expect(file.bytes).toEqual(fixture('kcad/v2/migrated.kcad'));
  });

  it('says what KCAD v2 does not keep of a v1 file, instead of dropping it silently', async () => {
    const { files, messages } = setup();
    // An object with a field no contract knows (“note”), which the v1 reader keeps as it is.
    await files.load(new TextDecoder().decode(fixture('document/v1/identity/minimal.kcad')), null);
    files.picker = pick(memoryFile('Küçük.kcad'));
    expect(await files.save()).toBe(true);
    expect(messages).toContain('uyarı: “Küçük.kcad”: KCAD v2\'nin tanımadığı alanlar yazılmadı: polyline.note.');
  });

  it('a file read without write access is not where Save writes', async () => {
    const { files } = setup();
    const bytes = new Uint8Array();
    const writer = setup(snapshotSampleDocument());
    const file = memoryFile('salt.kcad', { data: bytes });
    writer.files.picker = pick(file);
    await writer.files.saveAs();
    expect(await files.load(file.bytes, file, true)).toBe(true);
    expect(files.handle).toBeNull();
  });

  it('opens the same v1 file with the same ids every time', async () => {
    const { doc, files } = setup();
    const text = v1Text(snapshotSampleDocument());
    await files.load(text, memoryFile('Örnek.kcad', { data: text }));
    const first = uids(doc);
    doc.add({ kind: 'point', layerId: 'cizim', p: { x: 1, y: 1 }, attrs: {} });
    await files.load(text, memoryFile('Örnek.kcad', { data: text }));
    expect(uids(doc)).toEqual(first);
  });

  it('opens a v1 drawing whose ids cannot be derived with new ones, and says so', async () => {
    const { doc, files, messages } = setup();
    files.identities = async () => {
      throw new Error('Dosya biçimi modülü yüklenemedi');
    };
    const text = v1Text(snapshotSampleDocument());
    expect(await files.load(text, memoryFile('Örnek.kcad', { data: text }))).toBe(true);
    expect(doc.size).toBe(13);
    expect([...doc.all()].every((e) => isUuid(e.uid))).toBe(true);
    expect([doc.projectId, doc.migratedFrom]).toEqual([null, null]);
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

describe.skipIf(!formatsBuilt)('new project', () => {
  it('replaces a clean drawing without asking and has no file yet', async () => {
    const { doc, files, asked, messages, fitted } = setup();
    files.handle = memoryFile('eski.kcad');
    doc.projectId = '0192f5a0-0000-7000-8000-00000000abcd';
    expect(await files.newProject(fresh())).toBe(true);
    expect([doc.size, doc.name.value, doc.crs.value.srid, doc.settings.plotScale.value, doc.dirty.value]).toEqual([0, 'Ada 200', 5254, 500, false]);
    expect([files.handle, asked.length, doc.projectId, doc.migratedFrom]).toEqual([null, 0, null, null]);
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
    expect((await readV2(file.bytes)).entities).toHaveLength(1);
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
    files.picker = pick(null, memoryFile('Örnek.kcad', { data: v1Text(src) }));
    // Dirty, but the cloud project keeps the changes: no question.
    expect(await files.open()).toBe(true);
    expect(doc.size).toBe(src.size);
  });
});
