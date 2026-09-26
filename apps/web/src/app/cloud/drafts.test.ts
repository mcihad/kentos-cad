import { afterEach, describe, expect, it } from 'vitest';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { Entity } from '../../model/entities';
import { DRAFT_VERSION, MemoryDraftStore, readDraft } from './drafts';
import { disposeAll, pt, reopen, serverFor, newDoc, setup, storedDraft, uidsOf, wire, xOf } from './syncTesting';

/**
 * Device drafts (IndexedDB `kentos.cloud/drafts`) across the change that made
 * an object's persistent id its server id (ADR 0014 slice 3, docs/adr/0026).
 * The old drafts below have exactly the shape `ProjectSync.draft()` wrote
 * before it (commit b7a154c): no `version`; each change keyed by the id the
 * object had on the server, or by the v4 id that sync made up for a new
 * object; entities as the drawing held them minus the persistent id (the
 * slot left in as `id`); `meta` and `inflight` present but undefined when
 * there were none; the command on its way built by the same code. They
 * carry over to the same ids: nothing lost, nothing made twice.
 */

afterEach(disposeAll);

/** A project on the fake server with three points, as an earlier upload left it (v4 ids). */
function project() {
  const server = serverFor(newDoc());
  const [s1, s2, s3] = [crypto.randomUUID(), crypto.randomUUID(), crypto.randomUUID()];
  server.commitAs('yukleme', [
    { op: 'create', id: s1, entity: wire({ ...pt(1), id: 1 } as Entity) },
    { op: 'create', id: s2, entity: wire({ ...pt(2), id: 2 } as Entity) },
    { op: 'create', id: s3, entity: wire({ ...pt(3), id: 3 } as Entity) },
  ]);
  return { server, s1, s2, s3 };
}

/**
 * What the old code kept after these edits in a session: S1 moved, S2
 * deleted, a new point drawn (the old tracker named it with a fresh v4 id),
 * and the command carrying the move and the new point on its way when the
 * tab closed (its answer never seen).
 */
function oldDraft(s1: string, s2: string, n: string) {
  const moved = { kind: 'point', layerId: 'cizim', p: { x: 11, y: 4420210 }, attrs: {}, id: 1 };
  const drawn = { kind: 'point', layerId: 'parsel', p: { x: 40, y: 4420210 }, attrs: {}, id: 4 };
  const inflight: CommandEnvelope = {
    commandName: 'project.changes',
    version: 1,
    tenantId: 't',
    projectId: 'p',
    requestId: `web-${crypto.randomUUID()}`,
    idempotencyKey: crypto.randomUUID(),
    expectedVersions: { [s1]: '1' },
    input: {
      features: [
        { op: 'update', id: s1, entity: structuredClone(moved) },
        { op: 'create', id: n, entity: structuredClone(drawn) },
      ],
    },
  };
  return {
    userId: 'u1',
    changes: {
      [s1]: { base: '1', entity: moved },
      [s2]: { base: '1', entity: null },
      [n]: { base: null, entity: drawn },
    },
    meta: undefined,
    inflight,
    updated: 1727300000000,
  };
}

describe('device drafts written before persistent ids were the server’s', () => {
  for (const committed of [true, false]) {
    it(`carry over to the same ids, nothing lost or made twice (the command on its way ${committed ? 'was committed, its answer lost' : 'never arrived'})`, async () => {
      const { server, s1, s2, s3 } = project();
      const n = crypto.randomUUID();
      const old = oldDraft(s1, s2, n);
      if (committed) await server.command(structuredClone(old.inflight));
      const commits = server.commits;
      const drafts = new MemoryDraftStore();
      await drafts.put('u1/t/p', old);
      const { doc, records } = await reopen(server);
      const { sync, warnings } = setup({ doc, server, records, drafts });
      expect(await sync.restore(await drafts.get('u1/t/p'))).toBe(true);
      // The command went again with its own key: answered from the log, or committed now; once either way.
      expect([server.commits - commits, server.replays]).toEqual(committed ? [0, 1] : [1, 0]);
      // The new point is in the drawing under the id the old sync gave it; the move is there; the deletion waits.
      expect([xOf(doc.byUid(n)), xOf(doc.byUid(s1)), doc.byUid(s2)]).toEqual([40, 11, undefined]);
      expect(sync.pending.value).toBe(1);
      expect(await sync.flush()).toBe(true);
      expect([...server.store.keys()].sort()).toEqual([s1, s3, n].sort());
      expect(uidsOf(doc)).toEqual([s1, s3, n].sort());
      expect(server.store.get(n)?.entity).toMatchObject({ layerId: 'parsel', p: { x: 40 } });
      expect([sync.state.value, warnings]).toEqual(['saved', []]);
      // Everything sent: the draft is gone, and no copy was kept aside (it was read in full).
      expect(drafts.keys()).toEqual([]);
    });
  }

  it('an old draft that must wait for the server is kept in the current format, with the same key and ids', async () => {
    const { server, s1, s2 } = project();
    const n = crypto.randomUUID();
    const old = oldDraft(s1, s2, n);
    const drafts = new MemoryDraftStore();
    await drafts.put('u1/t/p', old);
    const { doc, records } = await reopen(server);
    const { sync } = setup({ doc, server, records, drafts });
    server.offline = true;
    await sync.restore(await drafts.get('u1/t/p'));
    expect(sync.state.value).toBe('offline_pending');
    const waiting = await storedDraft(drafts);
    expect(waiting).toEqual({ ...old, version: DRAFT_VERSION });
    expect(waiting!.inflight!.idempotencyKey).toBe(old.inflight.idempotencyKey);
    expect(server.commits).toBe(0);
  });

  it('what is left of an old draft is written again in the current format, under the same ids', async () => {
    const { server, s1, s2 } = project();
    const n = crypto.randomUUID();
    const drafts = new MemoryDraftStore();
    await drafts.put('u1/t/p', oldDraft(s1, s2, n));
    const { doc, records } = await reopen(server);
    const { sync } = setup({ doc, server, records, drafts });
    await sync.restore(await drafts.get('u1/t/p'));
    // The deletion waits; the new point, now on the server (created at revision 2), is moved with the server away.
    server.offline = true;
    doc.update(doc.slotOf(n)!, { p: { x: 41, y: 4420210 } });
    await sync.keepDraft();
    const now = await storedDraft(drafts);
    expect(now!.version).toBe(DRAFT_VERSION);
    expect(Object.keys(now!.changes).sort()).toEqual([s2, n].sort());
    expect(now!.changes[n]).toEqual({ base: '2', entity: expect.objectContaining({ layerId: 'parsel', p: { x: 41, y: 4420210 } }) });
    expect(now!.changes[s2]).toEqual({ base: '1', entity: null });
  });

  it('reads the old format and the current one, and refuses what is not a draft', () => {
    const [a, b] = [crypto.randomUUID(), crypto.randomUUID()];
    const old = readDraft({ userId: 'u1', changes: { [a.toUpperCase()]: { base: '3', entity: null }, [b]: { base: null, entity: { kind: 'point' } } }, meta: undefined, inflight: undefined, updated: 5 })!;
    expect(old.upgraded).toBe(true);
    expect(old.problems).toEqual([]);
    expect(old.draft).toEqual({ version: DRAFT_VERSION, userId: 'u1', changes: { [a]: { base: '3', entity: null }, [b]: { base: null, entity: { kind: 'point' } } }, meta: undefined, inflight: undefined, updated: 5 });
    const current = readDraft(old.draft)!;
    expect([current.upgraded, current.problems, current.draft]).toEqual([false, [], old.draft]);
    // A key that is no object id at all never reached the server: the object goes there as a new one.
    const odd = readDraft({ userId: 'u1', changes: { 'yerel-7': { base: '2', entity: { kind: 'point' } }, 'yerel-8': { base: '2', entity: null } }, updated: 1 }, () => a)!;
    expect(odd.draft.changes).toEqual({ [a]: { base: null, entity: { kind: 'point' } } });
    expect(odd.problems).toHaveLength(2);
    for (const bad of [null, 7, 'taslak', {}, { userId: 'u1' }, { userId: 'u1', changes: [] }, { version: 3, userId: 'u1', changes: {} }]) expect(readDraft(bad)).toBeNull();
    const broken = readDraft({ userId: 'u1', changes: { [a]: { base: 1, entity: null } }, meta: 'x', inflight: { idempotencyKey: 5 }, updated: 1 })!;
    expect([Object.keys(broken.draft.changes), broken.problems.length]).toEqual([[], 3]);
  });

  it('a draft that cannot be read in full is kept aside as it was before anything replaces it', async () => {
    const drafts = new MemoryDraftStore();
    const { doc, server, sync, warnings } = setup({ drafts });
    const e = doc.add(pt(1));
    await sync.flush();
    const raw = { userId: 'u1', changes: { [e.uid]: { base: '1', entity: wire({ ...e, p: { x: 5, y: 4420210 } } as Entity) }, bozuk: { base: 7 } }, updated: 2 };
    await drafts.put('u1/t/p', raw);
    const { doc: again, records } = await reopen(server);
    const second = setup({ drafts, doc: again, server, records });
    expect(await second.sync.restore(await drafts.get('u1/t/p'))).toBe(true);
    const aside = drafts.keys().filter((k) => k.startsWith('u1/t/p#unreadable-'));
    expect(aside).toHaveLength(1);
    expect(await drafts.get(aside[0])).toEqual(raw);
    expect(second.warnings.at(-1)).toMatch(/bir kısmı okunamadı.*ayrıca saklandı/);
    // What could be read came back and is sent.
    expect(await second.sync.flush()).toBe(true);
    expect(server.store.get(e.uid)?.entity).toMatchObject({ p: { x: 5 } });
    // Not a draft at all, or another account's: kept aside, nothing restored.
    for (const other of ['bozuk', { userId: 'u2', changes: {}, updated: 1 }]) {
      const store = new MemoryDraftStore();
      const s = setup({ drafts: store, doc: (await reopen(server)).doc, server });
      expect(await s.sync.restore(other)).toBe(false);
      expect(store.keys().filter((k) => k.includes('#unreadable-'))).toHaveLength(1);
    }
    expect(warnings).toEqual([]);
  });

  it('a change the drawing cannot take stays in the device draft, unsent, and the user is told', async () => {
    const drafts = new MemoryDraftStore();
    const { server } = setup({ drafts });
    const lost = crypto.randomUUID();
    const kept = crypto.randomUUID();
    await drafts.put('u1/t/p', {
      version: DRAFT_VERSION,
      userId: 'u1',
      changes: {
        [lost]: { base: null, entity: { ...wire({ ...pt(1), id: 1 } as Entity), layerId: 'silinen-katman' } },
        [kept]: { base: null, entity: wire({ ...pt(2), id: 2 } as Entity) },
      },
      updated: 1,
    });
    const { doc, records } = await reopen(server);
    const { sync, warnings } = setup({ drafts, doc, server, records });
    await sync.restore(await drafts.get('u1/t/p'));
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toMatch(new RegExp(`${lost}.*silinen-katman.*saklanıyor, gönderilmedi`));
    expect(await sync.flush()).toBe(true);
    expect([...server.store.keys()]).toEqual([kept]);
    // Saved: yet the draft keeps the change the drawing could not take, for the next opening.
    const left = await storedDraft(drafts);
    expect(Object.keys(left!.changes)).toEqual([lost]);
    expect(xOf(doc.byUid(kept))).toBe(2);
  });
});
