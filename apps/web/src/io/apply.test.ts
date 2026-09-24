import { describe, expect, it } from 'vitest';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { applyImport, layerNamed, type LayerTarget } from './apply';

/**
 * Putting a reader's objects into the drawing (io/apply.ts): checked like a
 * .kcad file first, new layers made, then one undo step and one change event
 * however many objects there are.
 */

const makeDoc = () =>
  new CadDocument({
    name: 't',
    layers: new LayerStore(
      [
        { id: 'a', name: 'Noktalar' },
        { id: 'k', name: 'Kilitli', locked: true },
      ],
      'a',
    ),
    origin: { x: 0, y: 0 },
  });

const point = (layerId: string, x: number, y: number): ContractEntity => ({ kind: 'point', id: 0, layerId, attrs: { Ad: `P${x}` }, label: `P${x}`, p: { x, y } });
const line = (layerId: string): ContractEntity => ({ kind: 'line', id: 0, layerId, attrs: {}, a: { x: 0, y: 0 }, b: { x: 1, y: 1 } });

describe('applyImport', () => {
  it('adds every object as one undo step with one change event, and merges by layer name', () => {
    const doc = makeDoc();
    const events: number[] = [];
    doc.events.on('touched', (e) => events.push(e.ids.length));
    const objects = Array.from({ length: 1000 }, (_, i) => point('0', 452000 + i, 4412000 + i / 3));
    const plan = { label: 'Koordinat listesi: a.ncn', layers: new Map<string, LayerTarget>([['0', { kind: 'existing', id: layerNamed(doc, 'NOKTALAR')!.id }]]) };
    const r = applyImport(doc, objects, plan);
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.ids).toHaveLength(1000);
    expect(doc.size).toBe(1000);
    expect(events).toEqual([1000]);
    // Coordinates exactly as read; ids are the document's.
    const first = doc.get(r.ids[0])!;
    expect(first.kind === 'point' && first.p).toEqual({ x: 452000, y: 4412000 });
    expect(new Set(r.ids).size).toBe(1000);
    expect(doc.undo()).toBe('Koordinat listesi: a.ncn');
    expect(doc.size).toBe(0);
    expect(doc.redo()).toBe('Koordinat listesi: a.ncn');
    expect(doc.size).toBe(1000);
  });

  it('makes new layers in the named group and leaves out layers that are not chosen', () => {
    const doc = makeDoc();
    const targets = new Map<string, LayerTarget>([
      ['PARSEL', { kind: 'new', name: 'PARSEL', style: { color: '#FF0000' }, visible: true, locked: false }],
      ['YOL', { kind: 'new', name: 'YOL', style: {}, visible: false, locked: true }],
    ]);
    const r = applyImport(doc, [line('PARSEL'), line('YOL'), line('DEFPOINTS')], { label: 'DXF: plan.dxf', layers: targets, group: 'plan.dxf' });
    expect(r.ok && r.created).toEqual(['PARSEL', 'YOL']);
    const group = doc.layers.tree.find((n) => n.name === 'plan.dxf')!;
    expect(group.type).toBe('group');
    expect(group.children.map((c) => [c.name, c.visible, c.locked, c.style.color])).toEqual([
      ['PARSEL', true, false, '#FF0000'],
      ['YOL', false, true, 'fg'],
    ]);
    expect(doc.size).toBe(2);
    // A second import finds the group and its layers again.
    const again = applyImport(doc, [line('PARSEL')], { label: 'DXF: plan.dxf', layers: new Map([['PARSEL', { kind: 'existing', id: layerNamed(doc, 'parsel')!.id }]]), group: 'plan.dxf' });
    expect(again.ok && again.created).toEqual([]);
    expect(doc.layers.tree.filter((n) => n.name === 'plan.dxf')).toHaveLength(1);
  });

  it('refuses a locked target and unusable objects without changing anything', () => {
    const doc = makeDoc();
    const locked = applyImport(doc, [point('0', 1, 2)], { label: 'x', layers: new Map([['0', { kind: 'existing', id: 'k' }]]) });
    expect(locked).toEqual({ ok: false, error: expect.stringContaining('“Kilitli” katmanı kilitli') });
    const bad = { ...point('0', 1, 2), p: { x: null, y: 2 } } as unknown as ContractEntity;
    const before = doc.layers.leaves().length;
    const r = applyImport(doc, [point('0', 3, 4), bad], { label: 'x', layers: new Map([['0', { kind: 'new', name: 'Yeni', style: {}, visible: true, locked: false }]]) });
    expect(r.ok).toBe(false);
    expect(!r.ok && r.error).toContain('İçe aktarılan nesne 2 (point) › p.x: sonlu bir sayı olmalı');
    expect(doc.size).toBe(0);
    expect(doc.layers.leaves()).toHaveLength(before);
    expect(doc.canUndo.value).toBe(false);
  });

  it('waits for a running model instead of joining its undo step', () => {
    const doc = makeDoc();
    const plan = { label: 'DXF: plan.dxf', layers: new Map<string, LayerTarget>([['0', { kind: 'new', name: 'Yeni', style: {}, visible: true, locked: false }]]) };
    const group = doc.beginGroup('Model');
    const refused = applyImport(doc, [point('0', 1, 2)], plan);
    expect(refused).toEqual({ ok: false, error: expect.stringContaining('hâlâ çalışıyor') });
    // Nothing changed: no object, no new layer; the model's cancel has nothing of the import to take back.
    expect(doc.size).toBe(0);
    expect(doc.layers.leaves().map((l) => l.name)).toEqual(['Noktalar', 'Kilitli']);
    group.cancel();
    const r = applyImport(doc, [point('0', 1, 2)], plan);
    expect(r.ok).toBe(true);
    expect(doc.undo()).toBe('DXF: plan.dxf');
  });
});
