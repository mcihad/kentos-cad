import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { Clipboard } from './clipboard';

describe('clipboard', () => {
  it('keeps copies without slot or persistent id, so every paste makes new objects (ADR 0014)', () => {
    const doc = new CadDocument({ name: 'K', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: { x: 0, y: 0 } });
    const [a, b] = doc.addMany([
      { kind: 'point', layerId: 'a', p: { x: 1, y: 2 }, attrs: { Ad: 'P1' } },
      { kind: 'line', layerId: 'a', a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, attrs: {} },
    ]);
    const clip = new Clipboard();
    clip.set([a, b], { minX: 0, minY: 0, maxX: 3, maxY: 4 });
    // Cut: the originals go; two pastes bring four new objects, none with an id seen before.
    doc.remove([a.id, b.id]);
    const first = doc.addMany(clip.get().items, 'Yapıştır');
    const second = doc.addMany(clip.get().items, 'Yapıştır');
    const items = clip.get().items;
    expect(items.every((e) => !('id' in e) && !('uid' in e))).toBe(true);
    const uids = [...first, ...second].map((e) => e.uid);
    expect(new Set(uids).size).toBe(4);
    expect(uids).not.toContain(a.uid);
    expect(uids).not.toContain(b.uid);
    expect(first[0]).toMatchObject({ kind: 'point', p: { x: 1, y: 2 }, attrs: { Ad: 'P1' } });
    // Undoing the cut brings the originals back with their own ids.
    doc.undo();
    doc.undo();
    doc.undo();
    expect(doc.byUid(a.uid)?.id).toBe(a.id);
    expect(doc.byUid(b.uid)?.id).toBe(b.id);
  });
});
