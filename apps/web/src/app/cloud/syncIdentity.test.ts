import { afterEach, describe, expect, it } from 'vitest';
import type { Entity as ContractEntity } from '../../contracts/generated/Entity';
import type { Entity } from '../../model/entities';
import { applyImport, type LayerTarget } from '../../io/apply';
import { MemoryDraftStore } from './drafts';
import { disposeAll, pt, reopen, setup, uidsOf, wire, xOf } from './syncTesting';

/**
 * One identity from the drawing to the server and back (ADR 0014 slice 3,
 * docs/adr/0026): an object's persistent id is its id on the server. A
 * retried command writes once, under the same ids; an undone deletion comes
 * back under the same id, above every version the id had; the same object
 * brought back twice is still one object.
 */

afterEach(disposeAll);

const moved = (e: Entity, x: number) => wire({ ...e, p: { x, y: 4420210 } } as Entity);

describe('one identity from the drawing to the server', () => {
  it('sends every object under its persistent id, and opening again gives the server’s ids back', async () => {
    const { doc, server, sync } = setup();
    const made = doc.addMany([pt(1), pt(2), pt(3)]);
    await sync.flush();
    const ids = made.map((e) => e.uid).sort();
    expect([...server.store.keys()].sort()).toEqual(ids);
    // The command named them by the same ids; the entity itself carries none (the contract's shape).
    expect(server.history[0].features.map((f) => f.id).sort()).toEqual(ids);
    expect([...server.store.values()].every((f) => !('uid' in f.entity))).toBe(true);
    const again = await reopen(server);
    expect(uidsOf(again.doc)).toEqual(ids);
    expect(again.records.map((r) => r.id).sort()).toEqual(ids);
  });

  it('a retried command after a lost answer writes once, under the objects’ own ids', async () => {
    const { doc, server, sync } = setup();
    const made = doc.addMany([pt(1), pt(2), pt(3)]);
    server.loseNextAnswer = true;
    expect(await sync.flush()).toBe(false);
    expect([sync.state.value, server.commits]).toEqual(['offline_pending', 1]);
    expect(await sync.flush()).toBe(true);
    expect([server.commits, server.replays, server.store.size]).toEqual([1, 1, 3]);
    expect([...server.store.keys()].sort()).toEqual(made.map((e) => e.uid).sort());
    // An update whose answer is lost: the same version comes once.
    doc.update(made[0].id, { p: { x: 9, y: 4420210 } });
    server.loseNextAnswer = true;
    await sync.flush();
    expect(await sync.flush()).toBe(true);
    expect([server.commits, server.replays, server.store.get(made[0].uid)?.version]).toEqual([2, 2, 2]);
    expect(sync.versionOf(made[0].uid)).toBe('2');
  });

  it('an undone deletion comes back under the same id and slot, above every version it had', async () => {
    const { doc, server, sync } = setup();
    const e = doc.add(pt(1));
    await sync.flush();
    doc.update(e.id, { p: { x: 2, y: 4420210 } });
    await sync.flush();
    expect(server.store.get(e.uid)?.version).toBe(2);
    doc.remove([e.id]);
    await sync.flush();
    expect(server.store.has(e.uid)).toBe(false);
    doc.undo();
    expect(doc.slotOf(e.uid)).toBe(e.id);
    expect(await sync.flush()).toBe(true);
    // Created again: its version is that commit's data revision, above the 2 it had.
    expect(server.store.get(e.uid)).toMatchObject({ version: 4, entity: { p: { x: 2 } } });
    expect(server.history.at(-1)!.features).toEqual([{ id: e.uid, op: 'create', version: '4' }]);
    // Redo deletes the same id again, undo brings it back once more.
    doc.redo();
    await sync.flush();
    doc.undo();
    await sync.flush();
    expect([server.store.size, server.store.get(e.uid)?.version]).toEqual([1, 6]);
  });

  it('an edit based on a version from before a deletion and a return is a conflict, not an overwrite', async () => {
    const drafts = new MemoryDraftStore();
    const a = setup();
    const x = a.doc.add(pt(1));
    await a.sync.flush();
    a.doc.update(x.id, { p: { x: 2, y: 4420210 } });
    await a.sync.flush();
    // B opens it at version 2 and edits it with the server away: the edit waits in B's device draft.
    const opened = await reopen(a.server);
    const b = setup({ doc: opened.doc, server: a.server, records: opened.records, drafts });
    a.server.offline = true;
    b.doc.update(b.doc.slotOf(x.uid)!, { p: { x: 99, y: 4420210 } });
    await b.sync.flush();
    await b.sync.keepDraft();
    b.sync.dispose();
    a.server.offline = false;
    // A deletes it, brings it back and edits it once: with versions starting again at 1 it would be at 2 again.
    a.doc.remove([x.id]);
    await a.sync.flush();
    a.doc.undo();
    await a.sync.flush();
    a.doc.update(x.id, { p: { x: 3, y: 4420210 } });
    await a.sync.flush();
    // B opens it again: its draft, based on version 2, is a conflict; nothing of A's is overwritten.
    const later = await reopen(a.server);
    const b2 = setup({ doc: later.doc, server: a.server, records: later.records, drafts });
    await b2.sync.restore(await drafts.get('u1/t/p'));
    expect(await b2.sync.flush()).toBe(false);
    expect(a.server.store.get(x.uid)?.entity).toMatchObject({ p: { x: 3 } });
    expect(b2.sync.conflicts.value).toMatchObject([{ featureId: x.uid, reason: 'changed', actual: '5' }]);
  });

  it('an object someone else brought back cannot come a second time through my undo', async () => {
    const { doc, server, sync } = setup();
    const x = doc.add(pt(1));
    await sync.flush();
    doc.remove([x.id]);
    await sync.flush();
    // Another editor, who had changed it, keeps theirs: the same object is created again.
    const back = server.commitAs('baska', [{ op: 'create', id: x.uid, entity: moved(x, 7) }]);
    await sync.receive([back]);
    expect(xOf(doc.byUid(x.uid))).toBe(7);
    // My undo of the deletion is gone with it: one object, one id.
    expect(doc.canUndo.value).toBe(false);
    expect(doc.undo()).toBeNull();
    expect([doc.size, uidsOf(doc)]).toEqual([1, [x.uid]]);
    expect(await sync.flush()).toBe(true);
    expect(server.store.size).toBe(1);
  });

  it('the same object brought back twice is still one: “exists”, then mine over theirs or theirs taken', async () => {
    for (const choice of ['mine', 'server'] as const) {
      const { doc, server, sync } = setup();
      const x = doc.add(pt(1));
      await sync.flush();
      doc.remove([x.id]);
      await sync.flush();
      // Before this editor hears of it, another one brings the object back.
      server.commitAs('baska', [{ op: 'create', id: x.uid, entity: moved(x, 7) }]);
      const theirs = server.store.get(x.uid)!.version;
      doc.undo();
      expect(await sync.flush()).toBe(false);
      expect(sync.conflicts.value).toMatchObject([{ featureId: x.uid, reason: 'exists', actual: String(theirs) }]);
      await sync.resolve(choice);
      expect(await sync.flush()).toBe(true);
      // Never a second object or a new id.
      expect([server.store.size, [...server.store.keys()], uidsOf(doc)]).toEqual([1, [x.uid], [x.uid]]);
      if (choice === 'mine') expect(server.store.get(x.uid)).toMatchObject({ version: theirs + 1, entity: { p: { x: 1 } } });
      else expect([server.store.get(x.uid)?.version, xOf(doc.byUid(x.uid)), doc.slotOf(x.uid)]).toEqual([theirs, 7, x.id]);
      disposeAll();
    }
  });

  it('importing into an open cloud project gives the new objects new ids, never a second copy', async () => {
    const first = setup();
    first.doc.addMany([pt(1), pt(2)]);
    await first.sync.flush();
    const { doc, records } = await reopen(first.server);
    const { server, sync } = setup({ doc, server: first.server, records });
    // A reader's objects that carry the drawing's own ids (a file made from this project, say).
    const same = [...doc.all()].map((e) => ({ ...wire(e), uid: e.uid, id: 0 }) as unknown as ContractEntity);
    const plan = { label: 'İçe aktar', layers: new Map<string, LayerTarget>([['cizim', { kind: 'existing', id: 'cizim' }]]) };
    const r = applyImport(doc, same, plan);
    expect(r.ok).toBe(true);
    const ids = uidsOf(doc);
    expect(ids).toHaveLength(4);
    expect(await sync.flush()).toBe(true);
    expect([...server.store.keys()].sort()).toEqual(ids);
  });
});
