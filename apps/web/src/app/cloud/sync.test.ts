import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Entity as ContractEntity } from '../../contracts/generated/Entity';
import { CadDocument } from '../../model/document';
import type { Entity } from '../../model/entities';
import { LayerStore } from '../../model/layers';
import { MemoryDraftStore } from './drafts';
import { FakeServer } from './fakeServer';
import { ProjectSync, type SyncOptions } from './sync';

const layers = () => new LayerStore([{ id: 'cizim', name: 'Çizim' }, { id: 'parsel', name: 'Parsel' }], 'cizim');
const newDoc = () => new CadDocument({ name: 'Ada 101', layers: layers(), origin: { x: 486500, y: 4420200 } });
const pt = (x: number, layerId = 'cizim') => ({ kind: 'point' as const, layerId, p: { x, y: 4420210 }, attrs: {} });
const wire = (e: Entity): ContractEntity => structuredClone(e);

let open: ProjectSync[] = [];
afterEach(() => {
  for (const s of open) s.dispose();
  open = [];
});

type Records = SyncOptions['records'];

function setup(opts: { canEditMeta?: boolean; drafts?: MemoryDraftStore; doc?: CadDocument; server?: FakeServer; records?: Records } = {}) {
  const doc = opts.doc ?? newDoc();
  const server =
    opts.server ??
    new FakeServer({ name: doc.name.value, settings: doc.settings.toJSON(), layers: [...doc.layers.tree], activeLayer: 'cizim', styles: structuredClone({ items: [...doc.styles.value.items], categories: [...doc.styles.value.categories] }), origin: doc.origin });
  const warnings: string[] = [];
  const drafts = opts.drafts ?? new MemoryDraftStore();
  const records = opts.records ?? [];
  const told = { deleted: 0 };
  const o: SyncOptions = {
    doc,
    api: server,
    drafts,
    draftKey: 'u1/t/p',
    userId: 'u1',
    tenantId: 't',
    projectId: 'p',
    canEditMeta: opts.canEditMeta ?? true,
    metaVersion: String(server.metaVersion),
    cursor: String(server.history.length),
    records,
    warn: (t) => warnings.push(t),
    onDeleted: () => told.deleted++,
    debounceMs: 60_000,
    maxDelayMs: 60_000,
  };
  const sync = new ProjectSync(o);
  open.push(sync);
  return { doc, server, sync, warnings, drafts, told };
}

describe('cloud autosave', () => {
  it('sends an edit once and says saved only after the answer', async () => {
    const { doc, server, sync } = setup();
    const e = doc.add(pt(486512));
    expect([sync.state.value, sync.pending.value, doc.dirty.value]).toEqual(['pending', 1, true]);
    expect(await sync.flush()).toBe(true);
    expect([sync.state.value, sync.pending.value, doc.dirty.value]).toEqual(['saved', 0, false]);
    const id = sync.featureOf(e.id)!;
    expect(server.store.get(id)).toMatchObject({ version: 1, entity: { kind: 'point', p: { x: 486512 } } });
    doc.update(e.id, { p: { x: 486513, y: 4420210 } });
    await sync.flush();
    expect(server.store.get(id)?.version).toBe(2);
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
    expect(server.store.has(id)).toBe(false);
    doc.undo();
    await sync.flush();
    expect(server.store.get(id)?.entity.kind).toBe('point');
  });

  it('keeps unsent changes through a dead network and never commits twice', async () => {
    const { doc, server, sync, drafts } = setup();
    doc.add(pt(1));
    server.offline = true;
    expect(await sync.flush()).toBe(false);
    expect(sync.state.value).toBe('offline_pending');
    const kept = await drafts.get('u1/t/p');
    expect(Object.keys(kept!.changes)).toHaveLength(1);
    expect(kept!.inflight).toBeDefined();
    server.offline = false;
    // The answer of the retry is lost too: the commit happened, the client does not know.
    server.loseNextAnswer = true;
    expect(await sync.flush()).toBe(false);
    expect(server.commits).toBe(1);
    expect(await sync.flush()).toBe(true);
    expect([server.commits, server.store.size, sync.state.value]).toEqual([1, 1, 'saved']);
    expect(await drafts.get('u1/t/p')).toBeNull();
  });

  it('stops at a conflict and lets the user take the server copy or keep theirs', async () => {
    const { doc, server, sync } = setup();
    const e = doc.add(pt(10));
    await sync.flush();
    const id = sync.featureOf(e.id)!;
    // Another editor moves it; we move it too, without having seen theirs.
    server.commitAs('baska', [{ op: 'update', id, entity: wire({ ...e, p: { x: 20, y: 4420210 } } as Entity) }]);
    doc.update(e.id, { p: { x: 30, y: 4420210 } });
    expect(await sync.flush()).toBe(false);
    expect(sync.state.value).toBe('conflict');
    expect(sync.conflicts.value).toMatchObject([{ featureId: id, localId: e.id, reason: 'changed', actual: '2' }]);
    // Nothing is sent while the conflict stands.
    doc.add(pt(99));
    expect(await sync.flush()).toBe(false);
    await sync.resolve('server');
    expect((doc.get(e.id) as { p: { x: number } }).p.x).toBe(20);
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
    const id = sync.featureOf(mine.id)!;
    const theirs = crypto.randomUUID();
    const events = [
      server.commitAs('baska', [{ op: 'create', id: theirs, entity: wire({ ...pt(7), id: 0 } as Entity) }]),
      server.commitAs('baska', [{ op: 'update', id, entity: wire({ ...mine, p: { x: 2, y: 4420210 } } as Entity) }]),
    ];
    // Our own commit's event comes too and is skipped.
    await sync.receive([...server.history.slice(0, 1), ...events]);
    expect(doc.size).toBe(2);
    expect((doc.get(mine.id) as { p: { x: number } }).p.x).toBe(2);
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
    expect((doc.get(mine.id) as { p: { x: number } }).p.x).toBe(3);
  });

  it('brings back a device draft after a reload and sends its lost command with the same key', async () => {
    const drafts = new MemoryDraftStore();
    const first = setup({ drafts });
    const a = first.doc.add(pt(1));
    await first.sync.flush();
    const id = first.sync.featureOf(a.id)!;
    first.doc.update(a.id, { p: { x: 5, y: 4420210 } });
    first.doc.add(pt(6));
    first.server.loseNextAnswer = true;
    await first.sync.flush();
    expect(first.server.commits).toBe(2);
    // "Reload": a fresh drawing from the server, then the device draft on top.
    first.sync.dispose();
    const doc = newDoc();
    const records: Records = [...first.server.store].map(([featureId, f]) => {
      const localId = doc.allocateId();
      doc.applyExternal({ put: [{ ...(f.entity as Entity), id: localId }] });
      return { localId, featureId, version: String(f.version) };
    });
    const again = setup({ drafts, doc, server: first.server, records });
    const draft = await drafts.get('u1/t/p');
    await again.sync.restore(draft!);
    // The lost command was answered from the server's log: nothing is committed twice.
    expect(first.server.commits).toBe(2);
    expect(await again.sync.flush()).toBe(true);
    expect(first.server.commits).toBe(2);
    expect(first.server.store.get(id)?.entity).toMatchObject({ p: { x: 5 } });
    expect(first.server.store.size).toBe(2);
  });

  it('an edit made after reopening wins over an older device draft', async () => {
    const drafts = new MemoryDraftStore();
    const first = setup({ drafts });
    const a = first.doc.add(pt(1));
    await first.sync.flush();
    const id = first.sync.featureOf(a.id)!;
    first.server.offline = true;
    first.doc.update(a.id, { p: { x: 5, y: 4420210 } });
    await first.sync.flush();
    first.sync.dispose();
    first.server.offline = false;
    // Reopen, and edit the same object before the draft comes back in.
    const doc = newDoc();
    const localId = doc.allocateId();
    doc.applyExternal({ put: [{ ...(first.server.store.get(id)!.entity as Entity), id: localId }] });
    const again = setup({ drafts, doc, server: first.server, records: [{ localId, featureId: id, version: '1' }] });
    doc.update(localId, { p: { x: 9, y: 4420210 } });
    await again.sync.restore((await drafts.get('u1/t/p'))!);
    expect((doc.get(localId) as { p: { x: number } }).p.x).toBe(9);
    await again.sync.flush();
    expect(first.server.store.get(id)?.entity).toMatchObject({ p: { x: 9 } });
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
    const draft = await drafts.get('u1/t/p');
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
    doc.add(pt(2));
    expect(await sync.flush()).toBe(false);
    expect([server.commits, sync.state.value, sync.pending.value]).toEqual([commits, 'deleted', 1]);
    await sync.keepDraft();
    expect(Object.values((await drafts.get('u1/t/p'))!.changes)[0].entity).toMatchObject({ p: { x: 2 } });
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
    const draft = await drafts.get('u1/t/p');
    expect([Object.keys(draft!.changes).length, draft!.inflight]).toEqual([1, undefined]);
  });

  it('refuses objects from the server that break the drawing’s rules', async () => {
    const { doc, server, sync, warnings } = setup();
    await sync.receive([server.commitAs('baska', [{ op: 'create', id: crypto.randomUUID(), entity: wire({ ...pt(1, 'yok-boyle-katman'), id: 0 } as Entity) }])]);
    expect(doc.size).toBe(0);
    expect(warnings[0]).toMatch(/okunamadı/);
  });
});
