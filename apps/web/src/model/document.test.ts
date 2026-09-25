import { describe, expect, it } from 'vitest';
import { Formatter } from '../app/format';
import { CadDocument } from './document';
import type { Entity, NewEntity } from './entities';
import { LayerStore } from './layers';

/** Persistent ids 1, 2, 3… as UUIDs: two drawings built the same way get the same ones. */
const counting = () => {
  let n = 0;
  return () => `00000000-0000-7000-8000-${(++n).toString(16).padStart(12, '0')}`;
};

const makeDoc = () =>
  new CadDocument({
    name: 't.kcad',
    layers: new LayerStore([{ id: 'g', name: 'G', children: [{ id: 'a', name: 'A' }] }], 'a'),
    origin: { x: 0, y: 0 },
    newUid: counting(),
  });

describe('CadDocument history', () => {
  it('undoes and redoes an add', () => {
    const doc = makeDoc();
    const e = doc.add({ kind: 'point', layerId: 'a', p: { x: 1, y: 2 }, attrs: {} });
    expect(doc.size).toBe(1);
    doc.undo();
    expect(doc.get(e.id)).toBeUndefined();
    doc.redo();
    expect(doc.get(e.id)).toBeDefined();
  });
  it('groups a transaction into one undo step', () => {
    const doc = makeDoc();
    doc.transact('iki', () => {
      doc.add({ kind: 'point', layerId: 'a', p: { x: 0, y: 0 }, attrs: {} });
      doc.add({ kind: 'point', layerId: 'a', p: { x: 1, y: 1 }, attrs: {} });
    });
    expect(doc.size).toBe(2);
    expect(doc.undo()).toBe('iki');
    expect(doc.size).toBe(0);
  });
  it('groups several transactions across awaits into one undo step, or cancels them', async () => {
    const doc = new CadDocument({ name: 't', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: { x: 0, y: 0 } });
    const line = (x: number) => ({ kind: 'line' as const, a: { x, y: 0 }, b: { x: x + 1, y: 0 }, layerId: 'a', attrs: {} });
    const g = doc.beginGroup('Model');
    doc.transact('Adım 1', () => doc.add(line(0)));
    await Promise.resolve();
    const second = doc.transact('Adım 2', () => doc.add(line(1)));
    doc.update(second.id, { attrs: { K: '1' } });
    expect(doc.canUndo.value).toBe(false);
    g.end();
    expect(doc.size).toBe(2);
    expect(doc.undo()).toBe('Model');
    expect(doc.size).toBe(0);
    expect(doc.canUndo.value).toBe(false);
    doc.redo();
    expect(doc.get(second.id)!.attrs.K).toBe('1');
    const c = doc.beginGroup('İptal');
    doc.add(line(5));
    doc.remove([second.id]);
    c.cancel();
    expect(doc.size).toBe(2);
    expect(doc.get(second.id)!.attrs.K).toBe('1');
    expect(doc.undo()).toBe('Model');
  });
  it('reverts a failed transaction completely and records nothing', () => {
    const doc = makeDoc();
    const kept = doc.add({ kind: 'point', layerId: 'a', p: { x: 9, y: 9 }, attrs: { N: '1' } });
    doc.dirty.set(false);
    const changed: number[] = [];
    doc.events.on('changed', () => changed.push(doc.size));
    expect(() =>
      doc.transact('Yarım', () => {
        doc.add({ kind: 'point', layerId: 'a', p: { x: 0, y: 0 }, attrs: {} });
        doc.update(kept.id, { attrs: { N: '2' } });
        doc.remove([kept.id]);
        throw new Error('kural ihlali');
      }),
    ).toThrow('kural ihlali');
    // The document is exactly as before, and the views were told about the revert.
    expect(doc.size).toBe(1);
    expect(doc.get(kept.id)!.attrs.N).toBe('1');
    expect(changed.at(-1)).toBe(1);
    // No undo step, no redo loss, not dirty.
    expect(doc.dirty.value).toBe(false);
    expect(doc.undo()).toBe('Ekle');
    expect(doc.size).toBe(0);
    // Ids are not reused after a revert.
    doc.redo();
    const next = doc.add({ kind: 'point', layerId: 'a', p: { x: 1, y: 1 }, attrs: {} });
    expect(next.id).toBeGreaterThan(kept.id + 1);
  });
  it('treats a nested transaction as a savepoint', () => {
    const doc = makeDoc();
    const p = (x: number) => ({ kind: 'point' as const, layerId: 'a', p: { x, y: 0 }, attrs: {} });
    // The inner failure is caught inside: only the inner edits go, the outer step commits.
    doc.transact('Dış', () => {
      doc.add(p(0));
      try {
        doc.transact('İç', () => {
          doc.add(p(1));
          throw new Error('iç');
        });
      } catch {
        // handled by the caller
      }
      doc.add(p(2));
    });
    expect([...doc.all()].map((e) => (e.kind === 'point' ? e.p.x : -1))).toEqual([0, 2]);
    expect(doc.undo()).toBe('Dış');
    expect(doc.size).toBe(0);
    // An inner failure that reaches the outer transaction reverts all of it.
    expect(() =>
      doc.transact('Dış', () => {
        doc.add(p(0));
        doc.transact('İç', () => {
          doc.add(p(1));
          throw new Error('iç');
        });
      }),
    ).toThrow('iç');
    expect(doc.size).toBe(0);
    expect(doc.canUndo.value).toBe(false);
  });
  it('keeps a failed transaction inside a group out of the group', () => {
    const doc = makeDoc();
    const p = (x: number) => ({ kind: 'point' as const, layerId: 'a', p: { x, y: 0 }, attrs: {} });
    const g = doc.beginGroup('Model');
    doc.transact('Adım 1', () => doc.add(p(0)));
    expect(() =>
      doc.transact('Adım 2', () => {
        doc.add(p(1));
        throw new Error('adım');
      }),
    ).toThrow();
    g.end();
    expect(doc.size).toBe(1);
    expect(doc.undo()).toBe('Model');
    expect(doc.size).toBe(0);
    // Cancelling a group leaves undo and redo as they were and does not mark the project dirty.
    doc.dirty.set(false);
    const c = doc.beginGroup('İptal');
    doc.add(p(5));
    c.cancel();
    expect(doc.size).toBe(0);
    expect(doc.canUndo.value).toBe(false);
    expect(doc.canRedo.value).toBe(true);
    expect(doc.dirty.value).toBe(false);
  });
  it('changes many objects as one change and one undo step, as update after update would', () => {
    const p = (x: number) => ({ kind: 'point' as const, layerId: 'a', p: { x, y: 0 }, attrs: {} });
    const ring = { kind: 'polygon' as const, layerId: 'a', pts: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 0, y: 4 }], holes: [{ pts: [{ x: 1, y: 1 }, { x: 2, y: 1 }, { x: 1, y: 2 }] }], attrs: {} };
    // The same drawing twice: one takes the patches one by one, the other all at once.
    const [one, many] = [makeDoc(), makeDoc()];
    for (const d of [one, many]) d.addMany([p(1), p(2), ring as unknown as NewEntity]);
    const patches: (Partial<Entity> & { id: number })[] = [
      { id: 1, p: { x: 10, y: 0 } },
      { id: 2, attrs: { N: '1' } },
      { id: 99, attrs: { N: 'yok' } },
      // Twice: the second patch changes what the first made.
      { id: 1, attrs: { K: '2' } },
      // A polygon opened into a polyline loses its holes.
      { id: 3, kind: 'polyline' } as Partial<Entity> & { id: number },
    ];
    one.transact('Tek tek', () => patches.forEach(({ id, ...patch }) => one.update(id, patch)));
    const heard: string[] = [];
    many.events.on('changed', (c) => heard.push(`changed ${[...c.layerIds].join(',')}`));
    many.events.on('attrs', (a) => heard.push(`attrs ${a.ids.join(',')}`));
    many.events.on('touched', (t) => heard.push(`touched ${t.ids.join(',')}`));
    expect(many.updateMany(patches, 'Taşı')).toBe(4);
    // The second patch of object 1 changes only its attributes, as a second update would.
    expect(heard).toEqual(['changed a', 'attrs 2,1', 'touched 1,2,1,3']);
    expect([...many.all()]).toEqual([...one.all()]);
    expect('holes' in many.get(3)!).toBe(false);
    expect(many.get(1)).toMatchObject({ p: { x: 10, y: 0 }, attrs: { K: '2' } });
    // One undo step back to the start, one redo to the end.
    const after = structuredClone([...many.all()]);
    expect(many.undo()).toBe('Taşı');
    expect(many.get(1)).toMatchObject({ p: { x: 1, y: 0 }, attrs: {} });
    expect(many.get(3)!.kind).toBe('polygon');
    expect('holes' in many.get(3)!).toBe(true);
    many.redo();
    expect([...many.all()]).toEqual(after);
    expect(many.undo()).toBe('Taşı');
    expect(many.undo()).toBe('Ekle');
    expect(many.size).toBe(0);
    // Nothing to change: no step, no event.
    heard.length = 0;
    expect(many.updateMany([{ id: 42, attrs: {} }])).toBe(0);
    expect(heard).toEqual([]);
  });
  it('joins the open transaction with many changes, and a failure reverts them', () => {
    const doc = makeDoc();
    const [a, b] = doc.addMany([
      { kind: 'point', layerId: 'a', p: { x: 1, y: 0 }, attrs: {} },
      { kind: 'point', layerId: 'a', p: { x: 2, y: 0 }, attrs: {} },
    ]);
    doc.dirty.set(false);
    expect(() =>
      doc.transact('Yarım', () => {
        doc.updateMany([{ id: a.id, p: { x: 5, y: 5 } }]);
        doc.addMany([{ kind: 'point', layerId: 'a', p: { x: 3, y: 0 }, attrs: {} }]);
        doc.remove([b.id]);
        throw new Error('kural ihlali');
      }),
    ).toThrow('kural ihlali');
    expect([...doc.all()]).toEqual([a, b]);
    expect(doc.dirty.value).toBe(false);
    doc.transact('Taşı ve sil', () => {
      doc.updateMany([{ id: a.id, p: { x: 5, y: 5 } }]);
      doc.remove([b.id]);
    });
    expect(doc.undo()).toBe('Taşı ve sil');
    expect([...doc.all()]).toEqual([a, b]);
  });
  it('removes many objects as one change; unknown and repeated ids are skipped', () => {
    const doc = makeDoc();
    const ids = doc.addMany([1, 2, 3].map((x) => ({ kind: 'point', layerId: 'a', p: { x, y: 0 }, attrs: {} }) as NewEntity)).map((e) => e.id);
    const touched: number[][] = [];
    doc.events.on('touched', (t) => touched.push(t.ids));
    doc.remove([ids[0], 77, ids[2], ids[0]]);
    expect(touched).toEqual([[ids[0], ids[2]]]);
    expect([...doc.all()].map((e) => e.id)).toEqual([ids[1]]);
    expect(doc.undo()).toBe('Sil');
    expect(doc.size).toBe(3);
    doc.remove([55]);
    expect(doc.undo()).toBe('Ekle');
  });
  it('marks the project dirty when project settings change', () => {
    const doc = makeDoc();
    expect(doc.dirty.value).toBe(false);
    doc.settings.assign({ areaUnit: 'donum' });
    expect(doc.dirty.value).toBe(true);
  });
  it('rejects unknown SRIDs instead of guessing', () => {
    expect(() => makeDoc().settings.assign({ srid: 1 })).toThrow();
  });
});

describe('LayerStore', () => {
  it('changes a layer style as one undo step, restoring it exactly', () => {
    const doc = makeDoc();
    const before = structuredClone(doc.layers.get('a')!.style);
    doc.setLayerStyle('a', { color: '#FF0000', renderer: { type: 'single', symbols: { fill: { ref: 'temel.alan.dolu' } } } });
    expect(doc.layers.get('a')!.style.color).toBe('#FF0000');
    expect(doc.canUndo.value).toBe(true);
    expect(doc.undo()).toBe('Katman stili');
    expect(doc.layers.get('a')!.style).toEqual(before);
    doc.redo();
    expect(doc.layers.get('a')!.style.renderer?.type).toBe('single');
    // Taking the renderer away removes the key; an unchanged style records nothing.
    doc.setLayerStyle('a', { renderer: undefined });
    expect('renderer' in doc.layers.get('a')!.style).toBe(false);
    const steps = doc.canUndo.value;
    doc.setLayerStyle('a', { color: '#FF0000' });
    doc.undo();
    expect(doc.layers.get('a')!.style.renderer?.type).toBe('single');
    expect(steps).toBe(true);
  });
  it('inherits visibility and lock from groups', () => {
    const doc = makeDoc();
    doc.layers.setVisible('g', false);
    expect(doc.layers.isVisible('a')).toBe(false);
    doc.layers.toggleLocked('g');
    expect(doc.layers.isLocked('a')).toBe(true);
  });
  it('tells when a group opens or closes, without making it an edit', () => {
    // The Layers panel rebuilds its rows on this event only; the tree's shape is otherwise unchanged.
    const doc = makeDoc();
    const seen: string[] = [];
    doc.layers.events.on('expanded', ({ id }) => seen.push(id));
    doc.layers.events.on('structure', () => seen.push('structure'));
    doc.layers.setExpanded('g', false);
    doc.layers.setExpanded('g', false);
    doc.layers.setExpanded('g', true);
    expect(seen).toEqual(['g', 'g']);
    expect(doc.layers.get('g')!.expanded).toBe(true);
    expect(doc.dirty.value).toBe(false);
  });
});

describe('Formatter', () => {
  it('follows project unit settings', () => {
    const doc = makeDoc();
    const f = new Formatter(doc.settings);
    expect(f.area(12997.304)).toBe('12997.30 m²');
    doc.settings.assign({ areaUnit: 'donum', areaDecimals: 3 });
    expect(f.area(12997.304)).toBe('12.997 dönüm');
    doc.settings.assign({ angleUnit: 'deg' });
    expect(f.bearing(100)).toBe('90.0000°');
    expect(f.point({ x: 1.23456, y: 2 })).toBe('Y 1.235  X 2.000');
  });
});
