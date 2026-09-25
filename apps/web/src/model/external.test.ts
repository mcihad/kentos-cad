import { describe, expect, it } from 'vitest';
import { isUuid } from '../core/uuid';
import { CadDocument } from './document';
import type { Entity } from './entities';
import { LayerStore } from './layers';

/** Changes other editors saved (applyExternal) and what cloud sync learns from `touched`. */

const doc = () => new CadDocument({ name: 'P', layers: new LayerStore([{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }], 'a'), origin: { x: 0, y: 0 } });
const point = (x: number) => ({ kind: 'point' as const, layerId: 'a', p: { x, y: 0 }, attrs: {} });

describe('changes from elsewhere', () => {
  it('reports which objects each change touched, undo and rollback included', () => {
    const d = doc();
    const seen: { ids: number[]; external: boolean }[] = [];
    d.events.on('touched', (e) => seen.push({ ids: e.ids, external: e.external }));
    const p = d.add(point(1));
    d.update(p.id, { p: { x: 2, y: 0 } });
    d.undo();
    expect(() => d.transact('bozuk', () => (d.add(point(9)), d.remove([p.id]), fail()))).toThrow();
    // add, update, undo, then the failed transaction's add and remove and its rollback (one step, both objects).
    expect(seen.map((s) => s.ids)).toEqual([[p.id], [p.id], [p.id], [p.id + 1], [p.id], [p.id, p.id + 1]]);
    expect(seen.every((s) => !s.external)).toBe(true);
    let styles = false;
    d.events.on('touched', (e) => (styles ||= e.layerStyles));
    d.setLayerStyle('a', { color: '#ff0000' });
    expect(styles).toBe(true);
  });

  it('applies another editor’s objects without an undo step or an unsaved mark', () => {
    const d = doc();
    const mine = d.add(point(1));
    d.markSaved(d.revision);
    const external: boolean[] = [];
    d.events.on('touched', (e) => external.push(e.external));
    const theirs: Entity = { ...point(5), id: d.allocateId() };
    d.applyExternal({ put: [theirs, { ...mine, p: { x: 7, y: 0 }, uid: undefined } as Entity] });
    expect(d.size).toBe(2);
    // A new object gets a persistent id; one already here keeps its own, whatever arrives (ADR 0014).
    expect(isUuid(d.uidOf(theirs.id))).toBe(true);
    expect(d.get(theirs.id)).toEqual({ ...theirs, uid: d.uidOf(theirs.id) });
    expect(d.get(mine.id)).toMatchObject({ p: { x: 7, y: 0 }, uid: mine.uid });
    expect(d.byUid(mine.uid)?.id).toBe(mine.id);
    expect(d.dirty.value).toBe(false);
    expect(external).toEqual([true]);
    // Undo cannot revert the object someone else changed: its step is gone.
    expect(d.canUndo.value).toBe(false);
    d.applyExternal({ remove: [theirs.id] });
    expect(d.get(theirs.id)).toBeUndefined();
    // New local objects never reuse an id that arrived from outside.
    expect(d.add(point(3)).id).toBeGreaterThan(theirs.id);
  });

  it('keeps undo for objects nobody else touched', () => {
    const d = doc();
    const a = d.add(point(1));
    const b = d.add(point(2));
    d.applyExternal({ put: [{ ...a, p: { x: 10, y: 0 } } as Entity] });
    expect(d.canUndo.value).toBe(true);
    expect(d.undo()).toBe('Ekle');
    expect(d.get(b.id)).toBeUndefined();
    expect(d.get(a.id)).toBeDefined();
    expect(d.canUndo.value).toBe(false);
  });

  it('takes another editor’s metadata quietly, and waits while an edit is open', () => {
    const d = doc();
    d.applyExternal({ meta: { name: 'Yeni ad', settings: { ...d.settings.toJSON(), plotScale: 500 }, activeLayer: 'b' } });
    expect([d.name.value, d.settings.toJSON().plotScale, d.layers.active.value, d.dirty.value]).toEqual(['Yeni ad', 500, 'b', false]);
    d.applyExternal({ meta: { layers: [{ id: 'c', name: 'C' }], activeLayer: 'c' } });
    expect(d.layers.leaves().map((l) => l.id)).toEqual(['c']);
    expect(d.dirty.value).toBe(false);
    const g = d.beginGroup('model');
    expect(() => d.applyExternal({ remove: [1] })).toThrow();
    g.end();
  });
});

function fail(): never {
  throw new Error('bilerek');
}
