import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Entity } from '../../model/entities';
import { MemoryDraftStore } from './drafts';
import { disposeAll, layers, pt, reopen, setup, storedDraft, wire, xOf } from './syncTesting';

afterEach(disposeAll);

describe('cloud autosave', () => {
  it('sends an edit once and says saved only after the answer', async () => {
    const { doc, server, sync } = setup();
    const e = doc.add(pt(486512));
    expect([sync.state.value, sync.pending.value, doc.dirty.value]).toEqual(['pending', 1, true]);
    expect(await sync.flush()).toBe(true);
    expect([sync.state.value, sync.pending.value, doc.dirty.value]).toEqual(['saved', 0, false]);
    // The object's persistent id is its id on the server (ADR 0014 slice 3).
    expect([...server.store.keys()]).toEqual([e.uid]);
    expect(server.store.get(e.uid)).toMatchObject({ version: 1, entity: { kind: 'point', p: { x: 486512 } } });
    doc.update(e.id, { p: { x: 486513, y: 4420210 } });
    await sync.flush();
    expect(server.store.get(e.uid)?.version).toBe(2);
    // Undo back to what the server has: nothing to send.
    doc.update(e.id, { p: { x: 486514, y: 4420210 } });
    doc.undo();
    const commits = server.commits;
    await sync.flush();
    expect(server.commits).toBe(commits);
    expect(sync.state.value).toBe('saved');
    // Deleting and undoing the delete creates it again under the same id.
    doc.remove([e.id]);
    await sync.flush();
    expect(server.store.has(e.uid)).toBe(false);
    doc.undo();
    await sync.flush();
    expect(server.store.get(e.uid)?.entity.kind).toBe('point');
    expect(doc.get(e.id)?.uid).toBe(e.uid);
  });

  it('keeps unsent changes through a dead network and never commits twice', async () => {
    const { doc, server, sync, drafts } = setup();
    const e = doc.add(pt(1));
    server.offline = true;
    expect(await sync.flush()).toBe(false);
    expect(sync.state.value).toBe('offline_pending');
    const kept = await storedDraft(drafts);
    expect(Object.keys(kept!.changes)).toEqual([e.uid]);
    expect(kept!.inflight).toBeDefined();
    server.offline = false;
    // The answer of the retry is lost too: the commit happened, the client does not know.
    server.loseNextAnswer = true;
    expect(await sync.flush()).toBe(false);
    expect(server.commits).toBe(1);
    expect(await sync.flush()).toBe(true);
    expect([server.commits, server.replays, server.store.size, sync.state.value]).toEqual([1, 1, 1, 'saved']);
    expect(await drafts.get('u1/t/p')).toBeNull();
  });

  it('stops at a conflict and lets the user take the server copy or keep theirs', async () => {
    const { doc, server, sync } = setup();
    const e = doc.add(pt(10));
    await sync.flush();
    const id = e.uid;
    // Another editor moves it; we move it too, without having seen theirs.
    server.commitAs('baska', [{ op: 'update', id, entity: wire({ ...e, p: { x: 20, y: 4420210 } } as Entity) }]);
    doc.update(e.id, { p: { x: 30, y: 4420210 } });
    expect(await sync.flush()).toBe(false);
    expect(sync.state.value).toBe('conflict');
    expect(sync.conflicts.value).toMatchObject([{ featureId: id, reason: 'changed', actual: '2' }]);
    // Nothing is sent while the conflict stands.
    doc.add(pt(99));
    expect(await sync.flush()).toBe(false);
    await sync.resolve('server');
    expect(xOf(doc.get(e.id))).toBe(20);
    expect(doc.get(e.id)?.uid).toBe(id);
    expect(sync.state.value).toBe('saved');
    expect(server.store.size).toBe(2);
    // Again, now keeping ours: it goes over the server's newer version.
    server.commitAs('baska', [{ op: 'update', id, entity: wire({ ...e, p: { x: 40, y: 4420210 } } as Entity) }]);
    doc.update(e.id, { p: { x: 50, y: 4420210 } });
    await sync.flush();
    await sync.resolve('mine');
    expect(server.store.get(id)).toMatchObject({ version: 4, entity: { p: { x: 50 } } });
  });

  it('applies other editors’ changes without an undo step or an unsaved mark', async () => {
    const { doc, server, sync } = setup();
    const mine = doc.add(pt(1));
    await sync.flush();
    const id = mine.uid;
    const theirs = crypto.randomUUID();
    const events = [
      server.commitAs('baska', [{ op: 'create', id: theirs, entity: wire({ ...pt(7), id: 0 } as Entity) }]),
      server.commitAs('baska', [{ op: 'update', id, entity: wire({ ...mine, p: { x: 2, y: 4420210 } } as Entity) }]),
    ];
    // Our own commit's event comes too and is skipped.
    await sync.receive([...server.history.slice(0, 1), ...events]);
    expect(doc.size).toBe(2);
    expect(xOf(doc.get(mine.id))).toBe(2);
    // The new object's persistent id is the server's id for it.
    expect(xOf(doc.byUid(theirs))).toBe(7);
    expect(doc.dirty.value).toBe(false);
    expect(doc.canUndo.value).toBe(false);
    expect(sync.cursor).toBe(events[1].seq);
    await sync.receive([server.commitAs('baska', [{ op: 'delete', id: theirs }])]);
    expect(doc.size).toBe(1);
    expect(sync.state.value).toBe('saved');
    // A change arriving for an object with unsent edits is a conflict, not an overwrite.
    doc.update(mine.id, { p: { x: 3, y: 4420210 } });
    await sync.receive([server.commitAs('baska', [{ op: 'update', id, entity: wire({ ...mine, p: { x: 4, y: 4420210 } } as Entity) }])]);
    expect(sync.conflicts.value[0]).toMatchObject({ featureId: id, reason: 'remote', actual: '3' });
    expect(xOf(doc.get(mine.id))).toBe(3);
  });

  it('brings back a device draft after a reload and sends its lost command with the same key', async () => {
    const drafts = new MemoryDraftStore();
    const first = setup({ drafts });
    const a = first.doc.add(pt(1));
    await first.sync.flush();
    first.doc.update(a.id, { p: { x: 5, y: 4420210 } });
    const b = first.doc.add(pt(6));
    first.server.loseNextAnswer = true;
    await first.sync.flush();
    expect(first.server.commits).toBe(2);
    // "Reload": a fresh drawing from the server, then the device draft on top.
    first.sync.dispose();
    const { doc, records } = await reopen(first.server);
    const again = setup({ drafts, doc, server: first.server, records });
    expect(await again.sync.restore(await drafts.get('u1/t/p'))).toBe(true);
    // The lost command was answered from the server's log: nothing is committed twice.
    expect([first.server.commits, first.server.replays]).toEqual([2, 1]);
    expect(await again.sync.flush()).toBe(true);
    expect(first.server.commits).toBe(2);
    expect(first.server.store.get(a.uid)?.entity).toMatchObject({ p: { x: 5 } });
    // The same two objects, under the ids they had before the reload.
    expect([...first.server.store.keys()].sort()).toEqual([a.uid, b.uid].sort());
    expect([doc.size, xOf(doc.byUid(b.uid))]).toEqual([2, 6]);
  });

  it('an edit made after reopening wins over an older device draft', async () => {
    const drafts = new MemoryDraftStore();
    const first = setup({ drafts });
    const a = first.doc.add(pt(1));
    await first.sync.flush();
    first.server.offline = true;
    first.doc.update(a.id, { p: { x: 5, y: 4420210 } });
    await first.sync.flush();
    first.sync.dispose();
    first.server.offline = false;
    // Reopen, and edit the same object before the draft comes back in.
    const { doc, records } = await reopen(first.server);
    const again = setup({ drafts, doc, server: first.server, records });
    const slot = doc.slotOf(a.uid)!;
    doc.update(slot, { p: { x: 9, y: 4420210 } });
    await again.sync.restore(await drafts.get('u1/t/p'));
    expect(xOf(doc.get(slot))).toBe(9);
    await again.sync.flush();
    expect(first.server.store.get(a.uid)?.entity).toMatchObject({ p: { x: 9 } });
  });

  it('sends metadata with its version, and keeps it local without the right to change it', async () => {
    const { doc, server, sync } = setup();
    doc.setLayerStyle('parsel', { color: '#ff0000' });
    doc.name.set('Ada 102');
    await sync.flush();
    expect(server.metaVersion).toBe(2);
    expect(server.meta.name).toBe('Ada 102');
    const editor = setup({ canEditMeta: false });
    editor.doc.setLayerStyle('parsel', { color: '#00ff00' });
    expect(editor.sync.pending.value).toBe(0);
    expect(editor.warnings[0]).toMatch(/yalnız bu cihazda/);
    await editor.sync.flush();
    expect(editor.server.metaVersion).toBe(1);
    expect(editor.sync.state.value).toBe('saved');
  });

  it('a project left while a command is on its way never touches the next drawing', async () => {
    const { doc, server, sync, drafts } = setup();
    doc.add(pt(1));
    let release!: () => void;
    server.gate = new Promise((r) => (release = r));
    const sending = sync.flush();
    await vi.waitFor(() => expect(server.waiting).toBe(1));
    doc.add(pt(2));
    // As CloudSession.leave does: what is unsent goes to the device, then the sync stops and another drawing comes in.
    await sync.keepDraft();
    sync.dispose();
    const other = (id: number, x: number): Entity => ({ id, kind: 'point', layerId: 'cizim', p: { x, y: 4420210 }, attrs: {} });
    doc.replaceWith({ name: 'Başka', settings: doc.settings.toJSON(), origin: doc.origin, homeView: null, layers: [...layers().tree], activeLayer: 'cizim', entities: [other(1, 50), other(2, 60)], styles: { items: [], categories: [] } });
    release();
    expect(await sending).toBe(false);
    // The command on its way was committed; nothing of the other drawing was sent after it.
    expect([server.commits, server.store.size]).toEqual([1, 1]);
    expect([...server.store.values()][0].entity).toMatchObject({ p: { x: 1 } });
    // The device draft still holds both changes and the command, for the next time the project opens.
    const draft = await storedDraft(drafts);
    expect([Object.keys(draft!.changes).length, !!draft!.inflight]).toEqual([2, true]);
    await sync.receive([server.commitAs('baska', [{ op: 'create', id: crypto.randomUUID(), entity: wire(other(0, 70)) }])]);
    expect(doc.size).toBe(2);
  });

  it('another editor deletes the project: nothing more is sent, edits stay on the device', async () => {
    const { doc, server, sync, drafts, told } = setup();
    doc.add(pt(1));
    await sync.flush();
    // Their last change and the deletion arrive together: the deletion ends it, nothing is fetched.
    const theirs = server.commitAs('baska', [{ op: 'create', id: crypto.randomUUID(), entity: wire({ ...pt(7), id: 0 } as Entity) }]);
    const gone = server.deleteAs('yonetici');
    await sync.receive([theirs, gone]);
    expect([sync.state.value, told.deleted, sync.cursor, doc.size]).toEqual(['deleted', 1, gone.seq, 1]);
    const commits = server.commits;
    const later = doc.add(pt(2));
    expect(await sync.flush()).toBe(false);
    expect([server.commits, sync.state.value, sync.pending.value]).toEqual([commits, 'deleted', 1]);
    await sync.keepDraft();
    expect((await storedDraft(drafts))!.changes[later.uid].entity).toMatchObject({ p: { x: 2 } });
    // Hearing it again changes nothing.
    await sync.receive([gone]);
    expect(told.deleted).toBe(1);
  });

  it('a command refused because the project was deleted stops sending the same way', async () => {
    const { doc, server, sync, drafts, told, warnings } = setup();
    doc.add(pt(1));
    server.deleted = true;
    expect(await sync.flush()).toBe(false);
    expect([sync.state.value, told.deleted, server.commits, warnings.length]).toEqual(['deleted', 1, 0, 0]);
    const draft = await storedDraft(drafts);
    expect([Object.keys(draft!.changes).length, draft!.inflight]).toEqual([1, undefined]);
  });

  it('refuses objects from the server that break the drawing’s rules', async () => {
    const { doc, server, sync, warnings } = setup();
    await sync.receive([server.commitAs('baska', [{ op: 'create', id: crypto.randomUUID(), entity: wire({ ...pt(1, 'yok-boyle-katman'), id: 0 } as Entity) }])]);
    expect(doc.size).toBe(0);
    expect(warnings[0]).toMatch(/okunamadı/);
  });
});

describe('cloud autosave when the account’s access changes (docs/adr/0015, TODOS.md CLOUD-13)', () => {
  it('access taken away: nothing more is sent, the drawing and its edits stay on the device', async () => {
    const { doc, server, sync, drafts, told } = setup();
    const a = doc.add(pt(1));
    await sync.flush();
    server.revoked = true;
    doc.update(a.id, { p: { x: 2, y: 4420210 } });
    expect(await sync.flush()).toBe(false);
    expect([sync.state.value, told.revoked, server.commits, doc.size]).toEqual(['revoked', [''], 1, 1]);
    // Refused before anything was written: the command stays in the draft with its key, for when access comes back.
    let kept = await storedDraft(drafts);
    expect([Object.keys(kept!.changes), !!kept!.inflight]).toEqual([[a.uid], true]);
    // Later edits are kept on the device too, and nothing is tried again.
    doc.add(pt(3));
    await sync.keepDraft();
    kept = await storedDraft(drafts);
    expect(Object.keys(kept!.changes)).toHaveLength(2);
    expect(await sync.flush()).toBe(false);
    expect(server.commits).toBe(1);
    // Told once; events heard after it change nothing.
    sync.markRevoked();
    await sync.receive([server.grant('editor')]);
    expect([told.revoked.length, told.asked, sync.state.value]).toEqual([1, 0, 'revoked']);
  });

  it('a changed grant is heard as a question: the session asks what this account may do now', async () => {
    const { server, sync, told } = setup();
    await sync.receive([server.grant('viewer')]);
    expect(told.asked).toBe(1);
    expect(sync.cursor).toBe(server.history.at(-1)!.seq);
  });

  it('a lowered role holds the edits on the device, and a raised one sends them', async () => {
    const { doc, server, sync, drafts } = setup();
    const a = doc.add(pt(1));
    await sync.flush();
    server.role = 'viewer';
    expect(sync.setAccess(false, false)).toBe('held');
    expect(sync.state.value).toBe('readonly');
    doc.update(a.id, { p: { x: 5, y: 4420210 } });
    expect(await sync.flush()).toBe(false);
    expect([server.commits, sync.pending.value, sync.state.value]).toEqual([1, 1, 'readonly']);
    await sync.keepDraft();
    expect((await storedDraft(drafts))!.changes[a.uid].entity).toMatchObject({ p: { x: 5 } });
    // The same role again changes nothing.
    expect(sync.setAccess(false, false)).toBe('same');
    server.role = 'editor';
    expect(sync.setAccess(true, false)).toBe('resumed');
    await vi.waitFor(() => expect(sync.state.value).toBe('saved'));
    expect(server.store.get(a.uid)).toMatchObject({ version: 2, entity: { p: { x: 5 } } });
  });

  it('a refusal for a missing right says so and asks again; once lowered, it is not an error', async () => {
    const { doc, server, sync, told, warnings } = setup();
    server.role = 'viewer';
    doc.add(pt(1));
    expect(await sync.flush()).toBe(false);
    expect([sync.state.value, told.asked]).toEqual(['error', 1]);
    expect(warnings.at(-1)).toMatch(/feature\.write/);
    // The session applies the lower role: the edit is held, not in error.
    expect(sync.setAccess(false, false)).toBe('held');
    expect([sync.state.value, sync.pending.value]).toEqual(['readonly', 1]);
  });

  it('a viewer made an editor does not send what it drew while it could only view', async () => {
    const { doc, server, sync, drafts } = setup({ canWrite: false });
    doc.add(pt(1));
    expect(sync.state.value).toBe('readonly');
    await sync.keepDraft();
    expect(await drafts.get('u1/t/p')).toBeNull();
    server.role = 'editor';
    // Those edits were never kept: opening the project again starts clean.
    expect(sync.setAccess(true, false)).toBe('reopen');
    expect(await sync.flush()).toBe(false);
    expect(server.commits).toBe(0);
    // With nothing drawn meanwhile, a viewer made an editor saves at once.
    const clean = setup({ canWrite: false, server });
    expect(clean.sync.setAccess(true, false)).toBe('resumed');
    clean.doc.add(pt(2));
    expect(await clean.sync.flush()).toBe(true);
    expect(server.commits).toBe(1);
  });
});
