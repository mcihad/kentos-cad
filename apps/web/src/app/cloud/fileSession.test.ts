import { afterEach, describe, expect, it } from 'vitest';
import { formatsBuilt } from '../../io/testFormats';
import { CadDocument } from '../../model/document';
import { LayerStore } from '../../model/layers';
import { snapshotSampleDocument } from '../../model/snapshotSample';
import { RecoveryCopies, type RecoveryCopy, type RecoveryStore } from '../recovery';
import { bytesOf, cloudSetup } from './cloudTesting';
import { UploadFailed } from './uploading';

/**
 * File projects and the import in the cloud session (docs/adr/0031, 0036,
 * 0038): the drawing on screen made a new file project (revision 1 on "0")
 * or a new database project (one import); a file project opened from its
 * newest revision, checked against the server's hash and read in stages;
 * an import refused by one object; a server without the import; Kaydet from
 * the unsaved-changes question; recovery copies of a file project's work.
 */

const point = (x: number, layerId = 'cizim') => ({ kind: 'point' as const, layerId, p: { x, y: 4420200 }, attrs: {} });
const open: { session: { detach(): void } }[] = [];
afterEach(() => {
  for (const o of open.splice(0)) o.session.detach();
});

function setup(opts: Parameters<typeof cloudSetup>[0] = {}) {
  const s = cloudSetup(opts);
  open.push(s);
  return s;
}

const emptyDoc = () => new CadDocument({ name: 'Boş', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 } });

describe.skipIf(!formatsBuilt)('file projects in the cloud session (docs/adr/0038)', () => {
  it('Buluta dosya olarak kaydet: a new file project whose revision 1 holds the drawing, attached, the drawing clean', async () => {
    const { session, server, doc, sockets } = setup();
    doc.add(point(486501));
    const stages: string[] = [];
    expect(await session.uploadFile('t', 'Ada 101 (dosya)', undefined, { tags: ['E2E'] }, (s) => stages.push(s))).toBe(true);
    expect([server.files.storage, server.files.revisions.map((r) => r.revision.revision)]).toEqual(['file', ['1']]);
    expect(stages.filter((s, i) => s !== stages[i - 1])).toEqual(['creating', 'encoding', 'uploading', 'verifying']);
    expect([session.project.value?.storage, session.file.value?.base.value, session.sync.value, doc.dirty.value, doc.name.value]).toEqual(['file', '1', null, false, 'Ada 101 (dosya)']);
    const back = await server.files.decode!(server.files.revisions[0].bytes);
    expect([back.entities.length, back.name]).toEqual([doc.size, 'Ada 101 (dosya)']);
    // Its own commit's event, heard on the live channel, is not someone else's revision.
    sockets.push([server.history.at(-1)!]);
    await new Promise((r) => setTimeout(r, 0));
    expect(session.file.value?.newer.value).toBeNull();
  });

  it('opens a file project from its newest revision, read in stages, based on it; Kaydet writes the next', async () => {
    const newest = snapshotSampleDocument();
    newest.add(point(486555));
    const { session, server, doc, messages } = setup({ doc: emptyDoc() });
    server.files.storage = 'file';
    server.meta.name = 'Ada 101';
    await server.files.commitAs(await bytesOf(snapshotSampleDocument()), 'Ayşe Yılmaz');
    await server.files.commitAs(await bytesOf(newest), 'Mehmet Demir');
    expect(await session.open('t', 'p', undefined, undefined, 'file')).toBe(true);
    expect([doc.size, doc.name.value, doc.dirty.value, session.file.value?.base.value, session.project.value?.storage]).toEqual([newest.size, 'Ada 101', false, '2', 'file']);
    expect(messages.at(-1)).toMatch(/^ok: “Ada 101” bulut projesi açıldı: revizyon 2, \d+ nesne\./);
    doc.add(point(486556));
    expect(await session.saveFile()).toBe(true);
    expect([server.files.revisions.length, session.file.value?.base.value, doc.dirty.value]).toEqual([3, '3', false]);
  });

  it('a download that changed on the way never becomes the drawing', async () => {
    const { session, server, doc, messages } = setup({ doc: emptyDoc() });
    server.files.storage = 'file';
    await server.files.commitAs(await bytesOf(snapshotSampleDocument()));
    server.files.corruptNextDownload = true;
    const before = doc.revision;
    expect(await session.open('t', 'p', undefined, undefined, 'file')).toBe(false);
    expect([doc.revision, doc.size, session.project.value]).toEqual([before, 0, null]);
    expect(messages.at(-1)).toMatch(/indirilemedi: İndirilen dosya sunucudakiyle aynı değil \(SHA-256 tutmuyor\)/);
  });

  it('a file project without a revision opens with its own metadata and no objects; the first Kaydet writes revision 1', async () => {
    const { session, server, doc } = setup({ doc: snapshotSampleDocument() });
    server.files.storage = 'file';
    expect(await session.open('t', 'p', undefined, undefined, 'file')).toBe(true);
    expect([doc.size, session.file.value?.base.value]).toEqual([0, '0']);
    doc.add(point(486501, doc.layers.leaves()[0].id));
    expect(await session.saveFile()).toBe(true);
    expect(server.files.revisions.map((r) => r.revision.revision)).toEqual(['1']);
  });

  it('Kaydet in the unsaved-changes question saves the open file project as its next revision', async () => {
    const { session, server, doc, files, answers } = setup();
    expect(await session.uploadFile('t', 'Ada', undefined, {})).toBe(true);
    doc.add(point(486501));
    answers.push('save');
    const content = { name: 'Yeni', settings: doc.settings.toJSON(), origin: doc.origin, homeView: null, layers: [...doc.layers.tree], activeLayer: 'cizim', entities: [], styles: { items: [], categories: [] } };
    expect(await files.newProject(content)).toBe(true);
    expect(server.files.revisions.map((r) => r.revision.revision)).toEqual(['1', '2']);
    expect([doc.name.value, session.project.value]).toEqual(['Yeni', null]);
  });
});

describe.skipIf(!formatsBuilt)('a new database project filled by one import (docs/adr/0036, 0038)', () => {
  it('uploads the drawing and imports it in one transaction: every object at version 1, no batches', async () => {
    const { session, server, doc } = setup();
    const stages: string[] = [];
    expect(await session.upload('t', 'Ada 101', undefined, {}, (s) => stages.push(s))).toBe(true);
    expect(stages.filter((s, i) => s !== stages[i - 1])).toEqual(['creating', 'encoding', 'uploading', 'verifying', 'importing']);
    expect([server.store.size, server.commits, server.lifecycleLog.map((e) => e.commandName)]).toEqual([doc.size, 0, ['project.import']]);
    expect([session.project.value?.storage, session.file.value, doc.dirty.value]).toEqual(['database', null, false]);
    const some = [...doc.all()][0];
    expect(session.sync.value?.versionOf(some.uid!)).toBe('1');
    // The autosave goes on from there: an edit is sent over version 1.
    doc.add(point(486501));
    expect(await session.flush()).toBe(true);
    expect(server.store.size).toBe(doc.size);
  });

  it('an import refused by one object leaves the new project empty and the drawing local, and says which object', async () => {
    const { session, server, doc } = setup();
    doc.add(point(486501));
    server.files.refuseImportAt = 13;
    const e = await session.upload('t', 'Ada 101', undefined, {}).catch((x: unknown) => x);
    expect(e).toBeInstanceOf(UploadFailed);
    const failed = e as UploadFailed;
    expect([failed.refused, failed.project.id, failed.message]).toEqual([{ index: 13, path: 'entities[13]' }, 'p', 'Dosyanın 14. nesnesi içe aktarılamadı: koordinat ±1 000 000 000 sınırının dışında.']);
    expect([server.store.size, session.project.value, doc.dirty.value, doc.name.value]).toEqual([0, null, true, 'Örnek pafta.kcad']);
  });

  it('a server without the import gets the objects in batches, then the layer tree', async () => {
    const { session, server, doc, messages } = setup();
    server.files.noImport = true;
    expect(await session.upload('t', 'Ada 101', undefined, {})).toBe(true);
    expect([server.store.size, server.commits > 0, session.project.value?.storage]).toEqual([doc.size, true, 'database']);
    expect(messages.some((m) => /eski sürüm\); nesneler parça parça gönderiliyor/.test(m))).toBe(true);
  });
});

describe.skipIf(!formatsBuilt)('recovery copies of a file project’s unsaved work (docs/adr/0030, 0038)', () => {
  function memoryStore(): RecoveryStore & { items: Map<string, RecoveryCopy> } {
    const items = new Map<string, RecoveryCopy>();
    return { items, list: async () => [...items.values()], put: async (c) => void items.set(c.id, c), delete: async (id) => void items.delete(id) };
  }

  it('a file project’s unsaved work gets a copy; a database project’s is in its device draft', async () => {
    const file = setup();
    const store = memoryStore();
    const recovery = new RecoveryCopies({ ...file.ctx, cloud: file.session, files: file.files }, { store, live: async () => new Set(), quietMs: 1e9, maxMs: 1e9 });
    recovery.start();
    expect(await file.session.uploadFile('t', 'Ada', undefined, {})).toBe(true);
    file.doc.add(point(486501));
    await recovery.flush();
    expect(store.items.size).toBe(1);
    // Kaydet removes it.
    expect(await file.session.saveFile()).toBe(true);
    expect(store.items.size).toBe(0);
    recovery.dispose();

    const db = setup();
    const dbStore = memoryStore();
    const dbRecovery = new RecoveryCopies({ ...db.ctx, cloud: db.session, files: db.files }, { store: dbStore, live: async () => new Set(), quietMs: 1e9, maxMs: 1e9 });
    dbRecovery.start();
    expect(await db.session.upload('t', 'Ada', undefined, {})).toBe(true);
    db.doc.add(point(486501));
    await dbRecovery.flush();
    expect(dbStore.items.size).toBe(0);
    dbRecovery.dispose();
  });
});
