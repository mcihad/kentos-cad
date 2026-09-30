import { describe, expect, it } from 'vitest';
import type { BlockDefinition as ContractBlock } from '../contracts/generated/BlockDefinition';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
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

/** A block id as the reader numbers them (1, 2, …). */
const readerId = (n: number) => `00000000-0000-0000-0000-${n.toString(16).padStart(12, '0')}`;
const insert = (layerId: string, n: number): ContractEntity => ({ kind: 'insert', id: 0, layerId, attrs: {}, block: readerId(n), p: { x: 5, y: 5 }, scale: 1, rotation: 0 });
/**
 * A reader's block: a line on the block's layer (a DXF's 0), one on a source layer the import may not bring
 * and, when given, an insert of another; its objects numbered as the reader numbers them.
 */
const block = (n: number, name: string, inner?: number): ContractBlock => ({
  id: readerId(n),
  name,
  base: { x: 0, y: 0 },
  entities: [line(''), line('DETAY'), ...(inner ? [insert('0', inner)] : [])].map((e, k) => ({ ...e, id: k + 1 })),
});
const onto = { label: 'DXF: plan.dxf', layers: new Map<string, LayerTarget>([['0', { kind: 'existing', id: 'a' }]]) };

/** The file's blocks with its objects (docs/adr/0144 §5), in the desktop's cases (apps/desktop/src/exchange/apply.rs). */
describe('applyImport with blocks', () => {
  it('adds the blocks with the objects in one step, each under a new id and a name the drawing does not have', () => {
    const doc = makeDoc();
    doc.addBlock({ id: '018f0000-0000-7000-8000-000000000009', name: 'KAPI', base: { x: 0, y: 0 }, entities: [{ ...line('0'), id: 1 } as unknown as Entity] });
    const r = applyImport(doc, [insert('0', 2), line('0')], onto, [block(1, 'No'), block(2, 'Kapı', 1)]);
    if (!r.ok) throw new Error(r.error);
    expect(r.blocks).toBe(2);
    expect(r.renamed).toEqual([['Kapı', 'Kapı (2)']]);
    expect(doc.blocks.value.map((b) => b.name)).toEqual(['KAPI', 'No', 'Kapı (2)']);
    const [, no, kapi] = doc.blocks.value;
    expect([no.id, kapi.id]).not.toContain(readerId(1));
    const placed = doc.get(r.ids[0]);
    expect(placed?.kind === 'insert' && placed.block).toBe(kapi.id);
    const inner = kapi.entities[2];
    expect(inner.kind === 'insert' && inner.block).toBe(no.id);
    // The objects' layers: the block's stays none, an unbrought one none, a brought one the drawing's.
    expect(kapi.entities.map((e) => e.layerId)).toEqual(['', '', 'a']);
    // One step: undo takes the objects and the definitions.
    expect(doc.undo()).toBe('DXF: plan.dxf');
    expect(doc.size).toBe(0);
    expect(doc.blocks.value.map((b) => b.name)).toEqual(['KAPI']);
    expect(doc.redo()).toBe('DXF: plan.dxf');
    expect(doc.blocks.value).toHaveLength(3);
  });

  it('refuses an insert of a block the file did not bring, and a block with a number that is not finite, changing nothing', () => {
    const doc = makeDoc();
    const stray = applyImport(doc, [line('0'), insert('0', 1), insert('0', 7)], onto, [block(1, 'No')]);
    expect(!stray.ok && stray.error).toContain('(İçe aktarılan nesne 3 (insert) › blok: 00000000-0000-0000-0000-000000000007 çizimde tanımlı değil)');
    const far = { ...block(2, 'Uzak'), base: { x: null, y: 0 } } as unknown as ContractBlock;
    const bad = applyImport(doc, [line('0')], onto, [block(1, 'No'), far]);
    expect(!bad.ok && bad.error).toContain('Dosyadan okunan bloklar çizime uymuyor (Blok 2 (“Uzak”) › taban noktası.x: sonlu bir sayı olmalı)');
    expect(doc.size).toBe(0);
    expect(doc.blocks.value).toEqual([]);
    expect(doc.canUndo.value).toBe(false);
  });
});
