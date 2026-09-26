import { describe, expect, it } from 'vitest';
import { isUuid } from '../core/uuid';
import { CadDocument, type DocumentContent } from './document';
import type { Entity, NewEntity } from './entities';
import { LayerStore } from './layers';
import { readSnapshot, toSnapshot } from './snapshot';

/**
 * Persistent object ids in the web document (docs/adr/0014, slice 1): every
 * object has one; an edit, undo and redo keep it; a copy gets a new one; the
 * uid → slot index follows every change.
 */

const makeDoc = () => new CadDocument({ name: 'K', layers: new LayerStore([{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }], 'a'), origin: { x: 0, y: 0 } });
const point = (x: number, layerId = 'a'): NewEntity => ({ kind: 'point', layerId, p: { x, y: 0 }, attrs: {} });
const v7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

/** Every object's id leads back to it and to nothing else; ids of objects no longer in the drawing lead nowhere. */
function indexProblem(doc: CadDocument, everSeen: Set<string>): string | null {
  const live = new Set<string>();
  for (const e of doc.all()) {
    const uid = e.uid!;
    if (!isUuid(uid)) return `${e.id}: ${uid} UUID değil`;
    if (live.has(uid)) return `${uid} iki nesnede`;
    live.add(uid);
    everSeen.add(uid);
    if (doc.slotOf(uid) !== e.id || doc.byUid(uid) !== e || doc.uidOf(e.id) !== uid) return `${uid} ${e.id} yuvasına götürmüyor`;
  }
  for (const uid of everSeen) if (!live.has(uid) && doc.byUid(uid)) return `${uid} çizimde olmayan bir nesneye götürüyor`;
  return null;
}

describe('persistent object ids', () => {
  it('gives every new object a UUIDv7 that edits, undo and redo keep', () => {
    const doc = makeDoc();
    const p = doc.add(point(1));
    expect(p.uid).toMatch(v7);
    doc.update(p.id, { p: { x: 2, y: 0 }, uid: 'başka' } as Partial<Entity>);
    doc.updateMany([{ id: p.id, layerId: 'b' }]);
    expect(doc.get(p.id)).toMatchObject({ p: { x: 2, y: 0 }, layerId: 'b', uid: p.uid });
    doc.undo();
    doc.undo();
    expect(doc.get(p.id)).toMatchObject({ p: { x: 1, y: 0 }, uid: p.uid });
    doc.redo();
    expect(doc.byUid(p.uid)).toMatchObject({ p: { x: 2, y: 0 } });
    // Undoing a delete brings the same object back: the same slot and id.
    doc.remove([p.id]);
    expect(doc.byUid(p.uid)).toBeUndefined();
    doc.undo();
    expect(doc.byUid(p.uid)?.id).toBe(p.id);
    doc.redo();
    expect(doc.slotOf(p.uid)).toBeUndefined();
    doc.undo();
    expect(doc.uidOf(p.id)).toBe(p.uid);
  });

  it('makes a copy a new object with a new id, even when it carries the original’s', () => {
    const doc = makeDoc();
    const [a, b] = doc.addMany([point(1), point(2)]);
    expect(a.uid).not.toBe(b.uid);
    // Spread from the originals (as the copy, array and paste tools build them): slot and id are the document's.
    const copies = doc.addMany([a, b].map((e) => ({ ...e, p: { x: 10, y: 0 } }) as NewEntity), 'Kopyala');
    expect(copies.map((e) => e.uid)).not.toContain(a.uid);
    expect(copies.map((e) => e.uid)).not.toContain(b.uid);
    expect(new Set(copies.map((e) => e.uid)).size).toBe(2);
    expect(doc.add({ ...a } as NewEntity).uid).not.toBe(a.uid);
    expect(doc.byUid(a.uid)?.id).toBe(a.id);
    // A cut is a delete, the paste new objects: the cut one's id does not come back with them.
    doc.remove([a.id]);
    const pasted = doc.add({ ...a, id: undefined, uid: undefined } as unknown as NewEntity);
    expect(pasted.uid).not.toBe(a.uid);
    // Bulk loads are new objects too.
    doc.load([point(5)]);
    expect([...doc.all()].at(-1)!.uid).toMatch(v7);
  });

  it('records a multi-edit as one undo step, every object keeping its id', () => {
    const doc = makeDoc();
    const list = doc.addMany([point(1), point(2), point(3)]);
    const uids = list.map((e) => e.uid);
    expect(doc.updateMany(list.map((e, i) => ({ id: e.id, p: { x: 100 + i, y: 5 } })), 'Taşı')).toBe(3);
    expect([...doc.all()].map((e) => e.uid)).toEqual(uids);
    expect(doc.undo()).toBe('Taşı');
    expect([...doc.all()].map((e) => (e.kind === 'point' ? e.p.x : NaN))).toEqual([1, 2, 3]);
    expect([...doc.all()].map((e) => e.uid)).toEqual(uids);
    expect(doc.undo()).toBe('Ekle');
    expect(doc.canUndo.value).toBe(false);
    doc.redo();
    doc.redo();
    expect([...doc.all()].map((e) => e.uid)).toEqual(uids);
  });

  it('replaces an object’s content in place: same slot and id, nothing merged, undone to the old kind', () => {
    const doc = makeDoc();
    const c = doc.add({ kind: 'circle', layerId: 'a', c: { x: 0, y: 0 }, r: 5, attrs: { K: '1' }, symbol: 'mpyy:x' });
    doc.replace(c.id, { kind: 'arc', layerId: 'a', c: { x: 0, y: 0 }, r: 5, a0: 0, a1: 1, attrs: {} }, 'Buda');
    expect(doc.get(c.id)).toEqual({ kind: 'arc', layerId: 'a', c: { x: 0, y: 0 }, r: 5, a0: 0, a1: 1, attrs: {}, id: c.id, uid: c.uid });
    expect(doc.undo()).toBe('Buda');
    expect(doc.get(c.id)).toEqual(c);
    doc.redo();
    expect(doc.byUid(c.uid)?.kind).toBe('arc');
    doc.replace(9999, point(1));
    expect(doc.size).toBe(1);
  });

  it('keeps the uid → slot index right through edits, undo, redo, rollbacks, groups, other editors and a new drawing', () => {
    let seed = 20260925;
    const rnd = (n: number) => (seed = (seed * 16807) % 2147483647) % n;
    const doc = makeDoc();
    const everSeen = new Set<string>();
    /** Each slot's id: a slot never changes its id while the drawing lasts. */
    let slots = new Map<number, string>();
    const any = () => {
      const all = [...doc.all()];
      return all.length ? all[rnd(all.length)] : null;
    };
    for (let step = 0; step < 2000; step++) {
      const e = any();
      switch (rnd(12)) {
        case 0:
          doc.add(point(rnd(100), rnd(2) ? 'a' : 'b'));
          break;
        case 1:
          if (e) doc.addMany([e, e].map((x) => ({ ...x, p: { x: rnd(50), y: 1 } }) as NewEntity));
          break;
        case 2:
          if (e) doc.update(e.id, { layerId: rnd(2) ? 'a' : 'b' });
          break;
        case 3:
          if (e) doc.updateMany([{ id: e.id, attrs: { N: String(step) } }, { id: e.id, layerId: 'b' }]);
          break;
        case 4:
          if (e) doc.replace(e.id, e.kind === 'point' ? { kind: 'circle', layerId: e.layerId, c: e.p, r: 1, attrs: {} } : point(rnd(9)));
          break;
        case 5:
          if (e) doc.remove([e.id, any()!.id]);
          break;
        case 6:
          doc.undo();
          break;
        case 7:
          doc.redo();
          break;
        case 8:
          expect(() =>
            doc.transact('Yarım', () => {
              doc.add(point(1));
              if (e) doc.update(e.id, { layerId: 'b' });
              if (e) doc.remove([e.id]);
              throw new Error('bilerek');
            }),
          ).toThrow('bilerek');
          break;
        case 9: {
          const g = doc.beginGroup('Model');
          doc.add(point(2));
          if (e) doc.remove([e.id]);
          if (rnd(2)) g.cancel();
          else g.end();
          break;
        }
        case 10: {
          // Another editor: a new object, a change, now and then a removal, and now and then an object
          // deleted here that they bring back under its id, in a new slot (the cloud's ids, docs/adr/0026).
          const live = new Set([...doc.all()].map((x) => x.uid));
          const gone = [...everSeen].filter((u) => !live.has(u));
          const back = gone.length && !rnd(3) ? [{ ...(point(rnd(30)) as Entity), id: doc.allocateId(), uid: gone[rnd(gone.length)] }] : [];
          doc.applyExternal({ put: [{ ...(point(rnd(30)) as Entity), id: doc.allocateId() }, ...back, ...(e ? [{ ...e, uid: undefined, layerId: 'a' } as Entity] : [])], remove: rnd(3) ? [] : [any()?.id ?? 0] });
          break;
        }
        default: {
          if (rnd(4)) break;
          // Another drawing: some objects bring ids, the others get new ones.
          const read = readSnapshot(JSON.stringify(toSnapshot(doc)));
          if (!read.ok) throw new Error(read.error);
          const kept = [...doc.all()];
          read.content.entities.forEach((x, i) => {
            if (i % 2) x.uid = kept[i].uid;
          });
          doc.replaceWith(read.content);
          slots = new Map();
        }
      }
      let problem = indexProblem(doc, everSeen);
      for (const x of doc.all()) {
        const was = slots.get(x.id);
        if (was !== undefined && was !== x.uid) problem ??= `${x.id} yuvasının kimliği değişti`;
        slots.set(x.id, x.uid!);
      }
      expect(problem, `adım ${step}`).toBeNull();
    }
    expect(everSeen.size).toBeGreaterThan(500);
  });

  it('takes the ids a new drawing brings, gives one to an object without, and refuses bad or repeated ids before anything changes', () => {
    const doc = makeDoc();
    const mine = doc.add(point(1));
    const content = (entities: Entity[]): DocumentContent => ({ name: 'Yeni', settings: doc.settings.toJSON(), origin: { x: 0, y: 0 }, homeView: null, layers: [{ id: 'a', name: 'A' }], activeLayer: 'a', entities, styles: { items: [], categories: [] } });
    const given = '4a5259a1-97f7-4742-88ce-b747287025ed';
    expect(() => doc.replaceWith(content([{ ...(point(1) as Entity), id: 1, uid: 'P-1' }]))).toThrow('UUID değil');
    expect(() =>
      doc.replaceWith(
        content([
          { ...(point(1) as Entity), id: 1, uid: given },
          { ...(point(2) as Entity), id: 2, uid: given },
        ]),
      ),
    ).toThrow('iki nesnede');
    // Refused: the open drawing is as it was.
    expect([...doc.all()]).toEqual([mine]);
    expect(doc.name.value).toBe('K');
    doc.replaceWith(content([{ ...(point(1) as Entity), id: 4, uid: given }, { ...(point(2) as Entity), id: 7 }]));
    expect(doc.byUid(given)?.id).toBe(4);
    expect(doc.uidOf(7)).toMatch(v7);
    expect(doc.byUid(mine.uid)).toBeUndefined();
  });

  it('writes no persistent id into a v1 file and takes none from one', () => {
    const doc = makeDoc();
    const p = doc.add(point(1));
    const snap = toSnapshot(doc);
    expect('uid' in snap.entities[0]).toBe(false);
    // A v1 file that carries a uid anyway: the reader leaves it out, the drawing gives its own.
    const text = JSON.stringify({ ...snap, entities: [{ ...snap.entities[0], uid: p.uid }] });
    const read = readSnapshot(text);
    expect(read.ok && 'uid' in read.content.entities[0]).toBe(false);
    const other = makeDoc();
    if (read.ok) other.replaceWith(read.content);
    expect(other.uidOf(p.id)).not.toBe(p.uid);
  });
});
