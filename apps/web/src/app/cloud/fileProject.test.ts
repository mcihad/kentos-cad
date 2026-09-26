import { afterEach, describe, expect, it } from 'vitest';
import { formatsBuilt, kcadInProcess } from '../../io/testFormats';
import { snapshotSampleDocument } from '../../model/snapshotSample';
import type { CloudApi } from './api';
import { bytesOf, serverFor } from './cloudTesting';
import { FileProjectSave, type FileSaveState, type NewerRevision } from './fileProject';

/**
 * Kaydet on a file project (docs/adr/0031, 0038; TODOS.md SYNC-02, SYNC-04,
 * SYNC-06): a new revision on the one the drawing came from, in stages the
 * status bar shows apart; nothing called saved before the commit; the dirty
 * rule; a cut connection, a lost answer, someone else's revision first, and
 * someone else's revision while the project is open.
 */

const made: FileProjectSave[] = [];
afterEach(() => {
  for (const f of made.splice(0)) f.dispose();
});

async function setup(opts: { base?: string; revisions?: number; canWrite?: boolean; api?: (a: CloudApi) => CloudApi } = {}) {
  const doc = snapshotSampleDocument();
  const server = serverFor(doc);
  server.files.storage = 'file';
  for (let i = 0; i < (opts.revisions ?? 1); i++) await server.files.commitAs(await bytesOf(doc), 'Ayşe Yılmaz', `ilk-${i}`);
  const warnings: string[] = [];
  const told = { newer: [] as NewerRevision[], deleted: 0, archived: 0, asked: 0 };
  const states: FileSaveState[] = [];
  const file = new FileProjectSave({
    doc,
    api: opts.api ? opts.api(server) : server,
    tenantId: 't',
    projectId: 'p',
    base: opts.base ?? String(opts.revisions ?? 1),
    cursor: String(server.history.length),
    canWrite: opts.canWrite ?? true,
    codec: kcadInProcess,
    warn: (t) => warnings.push(t),
    onNewer: (n) => told.newer.push(n),
    onDeleted: () => told.deleted++,
    onArchived: () => told.archived++,
    onAccessChanged: () => told.asked++,
    waits: [1, 1, 1],
  });
  made.push(file);
  file.state.subscribe((s) => states.push(s));
  return { doc, server, file, warnings, told, states };
}

const point = (x: number) => ({ kind: 'point' as const, layerId: 'cizim', p: { x, y: 4420200 }, attrs: {} });

describe.skipIf(!formatsBuilt)('Kaydet on a file project (docs/adr/0038)', () => {
  it('writes the next revision on its base, in stages, and calls it saved only once the server committed it', async () => {
    const { doc, server, file, states } = await setup();
    doc.add(point(486501));
    expect(file.state.value).toBe('pending');
    let committedWhenSaved = -1;
    file.state.subscribe((s) => s === 'saved' && (committedWhenSaved = server.files.revisions.length));
    expect(await file.save()).toBe('saved');
    expect(states.slice(states.indexOf('encoding'))).toEqual(['encoding', 'uploading', 'verifying', 'saved']);
    // “Saved” came after the commit, never before (SYNC-04).
    expect(committedWhenSaved).toBe(2);
    expect([file.base.value, file.lastSaved.value?.revision, doc.dirty.value]).toEqual(['2', '2', false]);
    // The revision holds the drawing, the new point included.
    const back = await server.files.decode!(server.files.revisions[1].bytes);
    expect(back.entities).toHaveLength(doc.size);
    // Nothing changed: nothing is written.
    expect(await file.save()).toBe('unchanged');
    expect(server.files.revisions).toHaveLength(2);
  });

  it('an edit made while the file goes up stays unsaved (CLAUDE.md §4.8)', async () => {
    const { doc, server, file } = await setup({
      api: (a) =>
        Object.assign(Object.create(a) as CloudApi, {
          sendUpload: async (...args: Parameters<CloudApi['sendUpload']>) => {
            // The user draws while the bytes are on their way.
            doc.add(point(486777));
            return a.sendUpload(...args);
          },
        }),
    });
    doc.add(point(486501));
    expect(await file.save()).toBe('saved');
    expect([file.base.value, doc.dirty.value, file.state.value]).toEqual(['2', true, 'pending']);
    // The revision holds the drawing as it was when Kaydet took it: without the later point.
    const back = await server.files.decode!(server.files.revisions[1].bytes);
    expect(back.entities).toHaveLength(doc.size - 1);
  });

  it('a connection cut while the bytes go: the same upload is sent again, and one revision is written', async () => {
    const { doc, server, file } = await setup();
    server.files.cutNextSend = 1;
    doc.add(point(486501));
    expect(await file.save()).toBe('saved');
    expect([server.files.sends, server.files.revisions.length, server.files.uploads.size]).toEqual([2, 2, 0]);
  });

  it('a commit whose answer is lost goes again with its key: written once, never a conflict with itself', async () => {
    const { doc, server, file } = await setup();
    server.files.loseNextCommitAnswer = true;
    doc.add(point(486501));
    expect(await file.save()).toBe('saved');
    expect([server.files.commits, server.files.revisions.length, file.base.value, file.conflict.value]).toEqual([1, 2, '2', null]);
  });

  it('someone else saved first: nothing is written, the drawing stays, and both revisions are known (SYNC-06)', async () => {
    const { doc, server, file } = await setup();
    await server.files.commitAs(await bytesOf(snapshotSampleDocument()));
    doc.add(point(486501));
    const before = doc.size;
    expect(await file.save()).toBe('conflict');
    expect(server.files.revisions).toHaveLength(2);
    expect([file.state.value, file.conflict.value, doc.dirty.value, doc.size]).toEqual(['conflict', { expected: '1', actual: '2' }, true, before]);
    // Until the user chooses, Kaydet does not try again over it.
    expect(await file.save()).toBe('conflict');
    expect(server.files.commits).toBe(0);
    // Chosen (a copy elsewhere, a local file, or the newest revision opened): Kaydet goes on from the new base.
    file.settle('2');
    expect(await file.save()).toBe('saved');
    expect(file.base.value).toBe('3');
  });

  it('someone else’s revision while the project is open is said and offered, never loaded by itself', async () => {
    const { doc, server, file, told } = await setup();
    const before = doc.size;
    const e = await server.files.commitAs(await bytesOf(snapshotSampleDocument()), 'Mehmet Demir');
    file.receive([e]);
    await new Promise((r) => setTimeout(r, 0));
    expect(file.newer.value).toEqual({ revision: '2', by: 'Mehmet Demir' });
    expect(file.state.value).toBe('outdated');
    expect(told.newer).toEqual([{ revision: '2', by: 'Mehmet Demir' }]);
    expect(doc.size).toBe(before);
  });

  it('the event of this window’s own commit is not someone else’s', async () => {
    const { doc, server, file, told } = await setup();
    doc.add(point(486501));
    expect(await file.save()).toBe('saved');
    file.receive([server.history.at(-1)!]);
    await new Promise((r) => setTimeout(r, 0));
    expect([file.newer.value, told.newer.length, file.state.value]).toEqual([null, 0, 'saved']);
  });

  it('the role lowered refuses Kaydet; raised, it saves again; a deletion ends it', async () => {
    const { doc, server, file, told } = await setup();
    doc.add(point(486501));
    expect(file.setAccess(false)).toBe('held');
    expect(await file.save()).toBe('readonly');
    expect(server.files.commits).toBe(0);
    expect(file.setAccess(true)).toBe('resumed');
    expect(await file.save()).toBe('saved');
    file.receive([server.deleteAs('biri')]);
    expect([file.state.value, told.deleted]).toEqual(['deleted', 1]);
    doc.add(point(486502));
    expect(await file.save()).toBe('ended');
  });

  it('the server out of reach: the drawing stays, the state says so, and the next Kaydet saves it', async () => {
    const { doc, server, file, warnings } = await setup();
    server.offline = true;
    doc.add(point(486501));
    expect(await file.save()).toBe('failed');
    expect([file.state.value, doc.dirty.value]).toEqual(['error', true]);
    expect(warnings.at(-1)).toMatch(/^Dosya kaydedilemedi: sunucuya ulaşılamadı .*yeniden Kaydet'e basın\.$/);
    server.offline = false;
    expect(await file.save()).toBe('saved');
    expect([file.state.value, doc.dirty.value, file.error.value]).toEqual(['saved', false, '']);
  });
});
