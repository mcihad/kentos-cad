import { afterEach, describe, expect, it } from 'vitest';
import { formatsBuilt, kcadInProcess } from '../../io/testFormats';
import { snapshotSampleDocument } from '../../model/snapshotSample';
import { ApiFailure, type CloudApi } from './api';
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

async function setup(opts: { base?: string; revisions?: number; canWrite?: boolean; api?: (a: CloudApi) => CloudApi; part?: number } = {}) {
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
    ...(opts.part ? { part: () => opts.part } : {}),
  });
  made.push(file);
  file.state.subscribe((s) => states.push(s));
  return { doc, server, file, warnings, told, states };
}

const point = (x: number) => ({ kind: 'point' as const, layerId: 'cizim', p: { x, y: 4420200 }, attrs: {} });
/** Lets the answers already on their way arrive. */
const tick = () => new Promise((r) => setTimeout(r, 0));

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

  it('bytes that arrived but whose answer was lost are not sent twice: the upload is asked (docs/adr/0040)', async () => {
    const { doc, server, file } = await setup();
    server.files.loseNextSendAnswer = true;
    doc.add(point(486501));
    expect(await file.save()).toBe('saved');
    expect([server.files.sends, server.files.revisions.length, file.base.value]).toEqual([1, 2, '2']);
  });

  it('a server that cannot say (no upload route): the second send is refused, and the next Kaydet writes the revision', async () => {
    const { doc, server, file } = await setup();
    server.files.loseNextSendAnswer = true;
    server.files.noUploadState = true;
    doc.add(point(486501));
    expect(await file.save()).toBe('failed');
    expect([file.state.value, file.error.value, doc.dirty.value, server.files.revisions.length]).toEqual(['error', expect.stringMatching(/zaten alındı/), true, 1]);
    expect(await file.save()).toBe('saved');
    expect([server.files.revisions.length, file.base.value, doc.dirty.value]).toEqual([2, '2', false]);
  });

  it('a large drawing goes in parts (docs/adr/0045): a part cut on its way goes on from what arrived, one revision is written', async () => {
    const { doc, server, file, states } = await setup({ part: 256 });
    server.files.cutNextSend = 0;
    // The second part is cut on its way; nothing of it is kept, the upload is asked and the part goes again.
    let parts = 0;
    const send = server.files.sendUpload.bind(server.files);
    server.files.sendUpload = (...a: Parameters<typeof send>) => {
      if (++parts === 2) server.files.cutNextSend = 1;
      return send(...a);
    };
    doc.add(point(486501));
    expect(await file.save()).toBe('saved');
    const offsets = server.files.offsets;
    expect([offsets.slice(0, 3), offsets.every((o) => o !== null && o % 256 === 0), server.files.revisions.length]).toEqual([[0, 256, 256], true, 2]);
    expect(states.slice(states.indexOf('encoding'))).toEqual(['encoding', 'uploading', 'verifying', 'saved']);
    // The revision holds the drawing, byte for byte what was sent.
    const back = await server.files.decode!(server.files.revisions[1].bytes);
    expect(back.entities).toHaveLength(doc.size);
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
    // The refusal names only the number: who saved it and when are asked after it.
    await tick();
    expect(file.newer.value).toEqual({ revision: '2', by: 'Mehmet Demir', at: '2026-09-26T11:00:00Z' });
    // Until the user chooses (a copy, a local file, the newest revision: each leaves this session), Kaydet does not try again over it.
    expect(await file.save()).toBe('conflict');
    expect(server.files.commits).toBe(0);
  });

  it('someone else’s revision while the project is open is said and offered, never loaded by itself', async () => {
    const { doc, server, file, told } = await setup();
    const before = doc.size;
    const e = await server.files.commitAs(await bytesOf(snapshotSampleDocument()), 'Mehmet Demir');
    file.receive([e]);
    await tick();
    const newer = { revision: '2', by: 'Mehmet Demir', at: '2026-09-26T11:00:00Z' };
    expect(file.newer.value).toEqual(newer);
    expect(file.state.value).toBe('outdated');
    expect(told.newer).toEqual([newer]);
    expect(doc.size).toBe(before);
    // Heard again (the same revision, another event of it): said once.
    file.receive([e]);
    await tick();
    expect(told.newer).toHaveLength(1);
  });

  it('a newer revision over unsaved work: the state says it; Kaydet uploads nothing and goes to the choice (docs/specs/file-revisions.md)', async () => {
    const { doc, server, file, states } = await setup();
    doc.add(point(486501));
    file.receive([await server.files.commitAs(await bytesOf(snapshotSampleDocument()))]);
    await tick();
    expect([file.state.value, file.revisions.dirty, file.newer.value?.revision]).toEqual(['outdated', true, '2']);
    expect(await file.save()).toBe('conflict');
    // The server would refuse it: nothing was encoded, uploaded or committed.
    expect([server.files.sends, server.files.commits, states.includes('encoding')]).toEqual([0, 0, false]);
    expect([file.state.value, file.conflict.value, file.newer.value?.by, doc.dirty.value]).toEqual(['conflict', { expected: '1', actual: '2' }, 'Mehmet Demir', true]);
  });

  it('a late answer never lowers the newer revision known', async () => {
    const gate: { open: (() => void) | null } = { open: null };
    const { server, file } = await setup({
      api: (a) =>
        Object.assign(Object.create(a) as CloudApi, {
          fileRevisions: async (...args: Parameters<CloudApi['fileRevisions']>) => {
            const answer = await a.fileRevisions(...args);
            // The first question's answer (revision 2) comes back last.
            if (!gate.open) await new Promise<void>((r) => (gate.open = r));
            return answer;
          },
        }),
    });
    file.receive([await server.files.commitAs(await bytesOf(snapshotSampleDocument()))]);
    await tick();
    file.receive([await server.files.commitAs(await bytesOf(snapshotSampleDocument()), 'Zeynep Kaya', 'ucuncu')]);
    await tick();
    expect(file.newer.value?.revision).toBe('3');
    gate.open?.();
    await tick();
    expect(file.newer.value).toEqual({ revision: '3', by: 'Zeynep Kaya', at: '2026-09-26T11:00:00Z' });
  });

  it('a resync asks the project and its newest revision, follows from its cursor now, and never replaces the drawing', async () => {
    const { doc, server, file, told } = await setup();
    doc.add(point(486501));
    const before = { size: doc.size, revision: doc.revision };
    // Missed while away: someone else's revision.
    await server.files.commitAs(await bytesOf(snapshotSampleDocument()));
    expect(await file.resync()).toBe('follow');
    expect([file.cursor, file.newer.value?.revision, told.newer.length, told.asked, file.state.value]).toEqual([String(server.history.length), '2', 1, 1, 'outdated']);
    expect([doc.size, doc.revision, doc.dirty.value, file.base.value]).toEqual([before.size, before.revision, true, '1']);
  });

  it('a resync tries a passing failure again before it waits for later', async () => {
    let failures = 2;
    const { server, file } = await setup({
      api: (a) =>
        Object.assign(Object.create(a) as CloudApi, {
          project: async (...args: Parameters<CloudApi['project']>) => {
            if (failures-- > 0) throw new ApiFailure(503, { error: 'unavailable', message: 'Sunucu kısa süreliğine yanıt veremiyor.' }, 'Sunucu kısa süreliğine yanıt veremiyor.');
            return a.project(...args);
          },
        }),
    });
    expect([await file.resync(), file.cursor]).toEqual(['follow', String(server.history.length)]);
  });

  it('a resync that finds the project deleted, archived or out of reach ends it; no answer is tried again later', async () => {
    const deleted = await setup();
    deleted.server.deleteAs('biri');
    expect([await deleted.file.resync(), deleted.file.state.value, deleted.told.deleted]).toEqual(['ended', 'deleted', 1]);
    const archived = await setup();
    archived.server.archiveAs('biri');
    expect([await archived.file.resync(), archived.file.state.value, archived.told.archived]).toEqual(['ended', 'archived', 1]);
    const revoked = await setup();
    revoked.server.grant(null);
    expect([await revoked.file.resync(), revoked.file.state.value]).toEqual(['ended', 'revoked']);
    const away = await setup();
    away.server.offline = true;
    expect([await away.file.resync(), away.file.state.value, away.file.cursor]).toEqual(['retry', 'saved', String(away.server.history.length)]);
  });

  it('the event of this window’s own commit is not someone else’s', async () => {
    const { doc, server, file, told } = await setup();
    doc.add(point(486501));
    expect(await file.save()).toBe('saved');
    file.receive([server.history.at(-1)!]);
    await tick();
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
