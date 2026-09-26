import { afterEach, describe, expect, it } from 'vitest';
import type { Checkpoint } from '../../contracts/generated/Checkpoint';
import { formatsBuilt } from '../../io/testFormats';
import { snapshotSampleDocument } from '../../model/snapshotSample';
import { HistoryPanel } from '../../ui/cloud/historyPanel';
import { bytesOf, cloudSetup } from './cloudTesting';
import { ROLE_PERMISSIONS } from './fakeServer';
import { loadHistory, whyNotCreate, whyNotDelete, whyNotDownload, whyNotTake } from './history';

/**
 * A project's history in the web (docs/adr/0034, 0038; TODOS.md SYNC-11,
 * CLOUD-07): who may name, remove, download and restore a checkpoint; the
 * history as the catalog asks for it; checkpoints made, removed and
 * restored as new projects; and the list asked again when the project's
 * events say it changed, for the open project and for another one.
 */

const ALL = ['project.read', 'feature.write', 'project.edit', 'project.delete', 'project.comment', 'project.download', 'project.history', 'project.share'] as const;
const made: { session: { detach(): void } }[] = [];
afterEach(() => {
  for (const s of made.splice(0)) s.session.detach();
});
function setup() {
  const s = cloudSetup();
  made.push(s);
  return s;
}

const checkpoint = (createdBy: string): Checkpoint => ({ id: 'c1', name: 'Teslim', kind: 'snapshot', revision: '4', size: '10', sha256: '0'.repeat(64), createdBy, createdByName: 'X', createdAt: '2026-09-26T12:00:00Z' });

describe('who may do what with a project’s history (docs/adr/0034)', () => {
  it('removing a checkpoint: its maker (who may write) or someone with project.edit; never in the archive', () => {
    const editor = ROLE_PERMISSIONS.editor;
    expect(whyNotDelete(checkpoint('u1'), 'u1', editor, false)).toBeNull();
    expect(whyNotDelete(checkpoint('u2'), 'u1', editor, false)).toMatch(/yalnız onu oluşturan ya da projeyi yöneten \(project\.edit\)/);
    expect(whyNotDelete(checkpoint('u2'), 'u1', ROLE_PERMISSIONS.manager, false)).toBeNull();
    // A viewer who made one while an editor may not remove it now: the server asks feature.write first.
    expect(whyNotDelete(checkpoint('u1'), 'u1', ROLE_PERMISSIONS.viewer, false)).toMatch(/feature\.write/);
    expect(whyNotDelete(checkpoint('u1'), 'u1', [...ALL], true)).toMatch(/Arşivlenmiş/);
  });

  it('naming one needs feature.write and, in a file project, a saved revision; taking one needs history and download', () => {
    expect(whyNotCreate(ROLE_PERMISSIONS.viewer, false, 'database', null)).toMatch(/feature\.write/);
    expect(whyNotCreate(ROLE_PERMISSIONS.editor, false, 'file', { revisions: [] })).toMatch(/henüz kaydedilmiş revizyonu yok/);
    expect(whyNotCreate(ROLE_PERMISSIONS.editor, false, 'database', null)).toBeNull();
    expect(whyNotTake(['project.read', 'project.history'], 'indirme')).toMatch(/project\.download/);
    expect(whyNotTake(ROLE_PERMISSIONS.viewer, 'indirme')).toBeNull();
    // A revision is a file project's content: downloading it needs project.download, not the history.
    expect([whyNotDownload(['project.read', 'project.download']), whyNotDownload(['project.read', 'project.history'])]).toEqual([null, expect.stringMatching(/project\.download/)]);
  });
});

describe.skipIf(!formatsBuilt)('checkpoints and restores in the web (docs/adr/0034, 0038)', () => {
  it('lists a database project’s checkpoints; without project.history it does not ask', async () => {
    const { server } = setup();
    server.files.snapshotBytes = await bytesOf(snapshotSampleDocument());
    server.files.addCheckpointBy('u2', 'Belediyeye teslim');
    const data = await loadHistory(server, 't', 'p', 'database', ROLE_PERMISSIONS.viewer);
    expect([data.revisions, data.checkpoints?.map((c) => c.name)]).toEqual([null, ['Belediyeye teslim']]);
    const blind = await loadHistory(server, 't', 'p', 'database', ['project.read']);
    expect(blind.checkpoints).toBeNull();
  });

  it('names a file project’s revision (the newest unless one is picked), removes it, and restores points as new projects', async () => {
    const { server, session } = setup();
    server.files.storage = 'file';
    const bytes = await bytesOf(snapshotSampleDocument());
    await server.files.commitAs(bytes);
    await server.files.commitAs(bytes);
    const ref = { tenantId: 't', projectId: 'p', name: server.summary().name };
    const newest = await session.lifecycle.createCheckpoint(ref, { name: '  Teslim  ', note: ' belediye ' });
    const older = await session.lifecycle.createCheckpoint(ref, { name: 'İlk', fileRevision: '1' });
    expect([newest.checkpoint.name, newest.checkpoint.note, newest.checkpoint.kind, newest.checkpoint.revision, older.checkpoint.revision]).toEqual(['Teslim', 'belediye', 'revision', '2', '1']);
    const data = await loadHistory(server, 't', 'p', 'file', ROLE_PERMISSIONS.editor);
    expect([data.revisions?.current, data.checkpoints?.map((c) => c.name)]).toEqual(['2', ['İlk', 'Teslim']]);
    // Restored as new projects, the source unchanged; a file project's point is a file project.
    const fromPoint = await session.lifecycle.restoreCheckpoint(ref, { checkpointId: newest.checkpoint.id });
    const fromRevision = await session.lifecycle.restoreCheckpoint(ref, { fileRevision: '1' }, { name: 'Eski hâl' });
    expect([fromPoint.project.name, fromPoint.project.storage, fromRevision.project.name, server.files.revisions.length]).toEqual([`${ref.name} (Teslim)`, 'file', 'Eski hâl', 2]);
    // Removed by its maker: the revision it named stays.
    const gone = await session.lifecycle.deleteCheckpoint(ref, newest.checkpoint.id);
    expect([gone.removed, server.files.checkpoints.length, server.files.revisions.length]).toEqual([true, 1, 2]);
    // Someone else's, as an editor: refused.
    const theirs = server.files.addCheckpointBy('u2', 'Onların');
    server.role = 'editor';
    await expect(session.lifecycle.deleteCheckpoint(ref, theirs.id)).rejects.toMatchObject({ code: 'forbidden' });
  });

  it('a project.checkpoint event asks for the list again while it shows: the open project’s, and another one’s', async () => {
    const { server, session, sockets, ctx } = setup();
    server.files.snapshotBytes = await bytesOf(snapshotSampleDocument());
    const summary = server.summary();
    let painted = 0;
    const panel = new HistoryPanel(ctx, { paint: () => painted++, download: () => {}, opened: () => {} });
    // Another project (not open): the panel subscribes to it on a channel of its own.
    panel.show(summary);
    for (let i = 0; i < 50 && typeof panel.state !== 'object'; i++) await new Promise((r) => setTimeout(r, 5));
    expect(panel.state).toMatchObject({ checkpoints: [] });
    await new Promise((r) => setTimeout(r, 5));
    const socket = sockets.made.at(-1)!;
    expect([socket.o.projectId, socket.started]).toEqual(['p', true]);
    // Someone names a checkpoint: its event arrives, the list is asked again and shows it.
    server.files.addCheckpointBy('u2', 'Onların');
    socket.o.onEvents([{ seq: String(server.history.length + 1), dataRevision: '0', kind: 'project.checkpoint', requestId: 'baskasi', features: [], meta: false }]);
    for (let i = 0; i < 100 && (panel.state as { checkpoints: unknown[] }).checkpoints.length === 0; i++) await new Promise((r) => setTimeout(r, 10));
    expect((panel.state as { checkpoints: Checkpoint[] }).checkpoints.map((c) => c.name)).toEqual(['Onların']);
    panel.hide();
    expect(socket.started).toBe(false);
    // The open project: its own channel's events, through the session.
    expect(await session.upload('t', 'Ada', undefined, {})).toBe(true);
    panel.show(server.summary());
    for (let i = 0; i < 50 && typeof panel.state !== 'object'; i++) await new Promise((r) => setTimeout(r, 5));
    const before = painted;
    const e = { seq: String(server.history.length + 1), dataRevision: '1', kind: 'project.checkpoint', requestId: 'biri', features: [], meta: false };
    sockets.push([e]);
    for (let i = 0; i < 100 && painted === before; i++) await new Promise((r) => setTimeout(r, 10));
    expect(painted).toBeGreaterThan(before);
    panel.hide();
  });
});
