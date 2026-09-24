import { describe, expect, it } from 'vitest';
import { CadDocument, type DocumentContent } from './document';
import type { Entity, NewEntity } from './entities';
import { LayerStore } from './layers';

/**
 * CadDocument keeps each layer's objects as the drawing changes (byLayer,
 * countByLayer). Whatever happens, they must be what a walk of the whole
 * drawing finds: the same objects in the same (document) order.
 */

const LAYERS = ['a', 'b', 'c'];
const makeDoc = () => new CadDocument({ name: 'K', layers: new LayerStore(LAYERS.map((id) => ({ id, name: id.toUpperCase() })), 'a'), origin: { x: 0, y: 0 } });
const point = (layerId: string, x: number): NewEntity => ({ kind: 'point', layerId, p: { x, y: 0 }, attrs: {} });

/** What byLayer answered before the index: a walk of every object. */
const walked = (doc: CadDocument, layerId: string) => [...doc.all()].filter((e) => e.layerId === layerId);

function walkedCounts(doc: CadDocument): [string, number][] {
  const m = new Map<string, number>();
  for (const e of doc.all()) m.set(e.layerId, (m.get(e.layerId) ?? 0) + 1);
  return [...m].sort();
}

/** Checks every layer and the counts against the walk; returns each layer's ids. */
function indexed(doc: CadDocument): Record<string, number[]> {
  const out: Record<string, number[]> = {};
  for (const id of LAYERS) {
    const list = doc.byLayer(id);
    const walk = walked(doc, id);
    expect(list).toHaveLength(walk.length);
    list.forEach((e, i) => expect(e).toBe(walk[i]));
    out[id] = list.map((e) => e.id);
  }
  expect([...doc.countByLayer()].sort()).toEqual(walkedCounts(doc));
  return out;
}

describe('objects per layer', () => {
  it('keeps each layer in document order through add, update and remove', () => {
    const doc = makeDoc();
    const [p1, p2, p3] = [doc.add(point('a', 1)), doc.add(point('b', 2)), doc.add(point('a', 3))];
    expect(indexed(doc)).toEqual({ a: [p1.id, p3.id], b: [p2.id], c: [] });
    doc.update(p1.id, { attrs: { N: '1' } });
    doc.update(p3.id, { p: { x: 30, y: 0 } });
    // The edited objects themselves, in their places.
    expect(doc.byLayer('a')).toEqual([doc.get(p1.id), doc.get(p3.id)]);
    expect(doc.byLayer('a')[0].attrs.N).toBe('1');
    doc.remove([p1.id]);
    expect(indexed(doc)).toEqual({ a: [p3.id], b: [p2.id], c: [] });
    expect(doc.countByLayer().has('c')).toBe(false);
    // A fresh array each time: callers may keep or change it.
    expect(doc.byLayer('a')).not.toBe(doc.byLayer('a'));
  });

  it('keeps an object moved to another layer at its place in the document', () => {
    const doc = makeDoc();
    const [x1, x2, x3] = [doc.add(point('a', 1)), doc.add(point('b', 2)), doc.add(point('b', 3))];
    doc.update(x3.id, { layerId: 'a' });
    doc.update(x1.id, { layerId: 'b' });
    // x1 was drawn first: it leads its new layer rather than joining at the end.
    expect(indexed(doc)).toEqual({ a: [x3.id], b: [x1.id, x2.id], c: [] });
    doc.undo();
    expect(indexed(doc)).toEqual({ a: [x1.id, x3.id], b: [x2.id], c: [] });
    doc.redo();
    // Moved again while nobody asked in between.
    doc.update(x2.id, { layerId: 'c' });
    doc.update(x3.id, { layerId: 'c' });
    expect(indexed(doc)).toEqual({ a: [], b: [x1.id], c: [x2.id, x3.id] });
  });

  it('puts a removed object back where the document does: at the end', () => {
    const doc = makeDoc();
    const ids = [1, 2, 3].map((x) => doc.add(point('a', x)).id);
    doc.remove([ids[0]]);
    doc.undo();
    expect(indexed(doc)).toEqual({ a: [ids[1], ids[2], ids[0]], b: [], c: [] });
  });

  it('follows a failed transaction and a cancelled group back', () => {
    const doc = makeDoc();
    const [k1, k2] = [doc.add(point('a', 1)), doc.add(point('b', 2))];
    const before = indexed(doc);
    expect(() =>
      doc.transact('Yarım', () => {
        doc.add(point('c', 9));
        doc.update(k1.id, { layerId: 'c' });
        doc.remove([k2.id]);
        throw new Error('kural ihlali');
      }),
    ).toThrow('kural ihlali');
    expect(indexed(doc)).toEqual(before);
    const g = doc.beginGroup('Model');
    doc.transact('Adım', () => doc.update(k2.id, { layerId: 'a' }));
    doc.add(point('c', 5));
    g.cancel();
    expect(indexed(doc)).toEqual(before);
  });

  it('follows load, replaceWith and changes from another editor', () => {
    const doc = makeDoc();
    doc.load([point('b', 1), point('a', 2), point('b', 3)]);
    expect(indexed(doc)).toEqual({ a: [2], b: [1, 3], c: [] });
    const content: DocumentContent = {
      name: 'K2',
      settings: doc.settings.toJSON(),
      origin: { x: 0, y: 0 },
      homeView: null,
      layers: LAYERS.map((id) => ({ id, name: id.toUpperCase() })),
      activeLayer: 'a',
      entities: [7, 4, 9].map((id, i) => ({ ...point(i === 1 ? 'a' : 'c', id), id }) as Entity),
      styles: { items: [], categories: [] },
    };
    doc.replaceWith(content);
    expect(indexed(doc)).toEqual({ a: [4], b: [], c: [7, 9] });
    // Another editor moves 4 (drawn between 7 and 9), adds one and removes 9.
    const theirs = { ...point('b', 8), id: doc.allocateId() } as Entity;
    doc.applyExternal({ put: [{ ...doc.get(4)!, layerId: 'c' } as Entity, theirs], remove: [9] });
    expect(indexed(doc)).toEqual({ a: [], b: [theirs.id], c: [7, 4] });
  });

  it('answers what a walk of the drawing finds after any mix of edits', () => {
    let seed = 20260924;
    const rnd = (n: number) => (seed = (seed * 16807) % 2147483647) % n;
    const doc = makeDoc();
    const anyId = () => {
      const all = [...doc.all()];
      return all.length ? all[rnd(all.length)].id : null;
    };
    for (let step = 0; step < 800; step++) {
      const layer = LAYERS[rnd(LAYERS.length)];
      const id = anyId();
      const e = id === null ? undefined : doc.get(id);
      switch (rnd(12)) {
        case 0:
        case 1:
          doc.add(point(layer, step));
          break;
        case 2:
        case 3:
          if (e) doc.update(e.id, { layerId: layer });
          break;
        case 4:
          if (e) doc.update(e.id, { attrs: { N: String(step) } });
          break;
        case 5:
          if (e) doc.remove([e.id]);
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
              doc.add(point(layer, step));
              if (e) doc.update(e.id, { layerId: LAYERS[rnd(LAYERS.length)] });
              throw new Error('geri');
            }),
          ).toThrow('geri');
          break;
        case 9:
          doc.transact('Taşı', () => {
            for (let k = 0; k < 4; k++) {
              const j = anyId();
              if (j !== null) doc.update(j, { layerId: LAYERS[rnd(LAYERS.length)] });
            }
          });
          break;
        case 10: {
          const put: Entity[] = [{ ...point(layer, -step), id: doc.allocateId() } as Entity];
          if (e) put.push({ ...e, layerId: LAYERS[rnd(LAYERS.length)] } as Entity);
          const other = anyId();
          doc.applyExternal({ put, remove: other !== null && other !== e?.id ? [other] : [] });
          break;
        }
        case 11:
          if (rnd(8) === 0) doc.load([point(layer, step), point(LAYERS[rnd(LAYERS.length)], step)]);
          break;
      }
      // Asking only now and then lets several moves pile up before the next read.
      if (rnd(3) === 0) indexed(doc);
    }
    indexed(doc);
    expect(doc.size).toBeGreaterThan(20);
  });
});
