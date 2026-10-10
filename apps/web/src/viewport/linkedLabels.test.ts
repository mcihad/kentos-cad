import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import type { Vec2 } from '../model/geometry';
import { LayerStore } from '../model/layers';
import { PickIndex } from './picking';
import { LABEL, LABEL_STRIDE } from './storeRecords';

/**
 * An object whose label a text writes shows none of its own (docs/adr/0175 §4): the text is its label. The geometry
 * store hears it from the document (`textLabelled`, `linksVersion`) before it answers, through the text's adding,
 * undo and redo, the link's breaking and the object's removal. The desktop's is in
 * crates/native/interaction/tests/all/spatial.rs.
 */
const v = (x: number, y: number): Vec2 => ({ x, y });
const square = (x: number, y: number) => [v(x, y), v(x + 10, y), v(x + 10, y + 10), v(x, y + 10)];
const view = { minX: -50, minY: -50, maxX: 150, maxY: 150 };

/** The ids of the objects whose own label (not a text's) the store places at 3 px/m. */
function labelled(index: PickIndex): number[] {
  const { records } = index.labels(view, 3, null);
  const out: number[] = [];
  for (let i = 0; i < records.length; i += LABEL_STRIDE) if (records[i + 1] === LABEL.placed) out.push(records[i]);
  return out;
}

describe('an object whose label a text writes', () => {
  it('shows none of its own while the link holds', () => {
    const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A', style: { color: 'ink', lineType: 'continuous', lineWeight: 0.25, label: { placement: 'center', size: 10 } } }], 'a'), origin: v(0, 0) });
    const [first, second] = doc.addMany([
      { layerId: 'a', attrs: {}, kind: 'polygon', pts: square(0, 0), label: '12' },
      { layerId: 'a', attrs: {}, kind: 'polygon', pts: square(20, 0), label: '13' },
    ]);
    const index = new PickIndex(doc);
    expect(labelled(index)).toEqual([first.id, second.id]);
    const text = doc.add({ layerId: 'a', attrs: {}, kind: 'text', p: v(5, 5), text: '12', height: 2, rotation: 0, align: 'middleCenter', labelOf: first.uid, labelScale: 1000 });
    expect(labelled(index)).toEqual([second.id]);
    expect(doc.undo()).toBe('Ekle');
    expect(labelled(index)).toEqual([first.id, second.id]);
    expect(doc.redo()).toBe('Ekle');
    expect(labelled(index)).toEqual([second.id]);
    // The text moved by hand loses its link: the object's own label is back.
    doc.update(text.id, { p: v(6, 6) });
    expect(labelled(index)).toEqual([first.id, second.id]);
    expect(doc.undo()).toBe('Değiştir');
    expect(labelled(index)).toEqual([second.id]);
    // The object removed takes its text with it.
    doc.remove([first.id]);
    expect(doc.get(text.id)).toBeUndefined();
    expect(labelled(index)).toEqual([second.id]);
    expect(doc.undo()).toBe('Sil');
    expect(labelled(index)).toEqual([second.id]);
    index.dispose();
  });
});
