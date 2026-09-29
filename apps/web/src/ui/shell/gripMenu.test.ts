import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../../model/entities';
import { entityGrips } from '../../model/ops/grips';
import { at, pt, toolHarness } from '../../tools/toolHarness';
import type { MenuItem } from '../widgets/PopupMenu';
import { gripItems } from './gripMenu';

/**
 * The right-button menu of a grip (docs/adr/0074) on an area of several parts (docs/adr/0143): the actions are the
 * grip's own part's, its heading counts in that part, the other parts stay, and every elevation follows by the
 * command's rules (docs/adr/0142).
 */
const square = (x: number, y: number, side: number) => [pt(x, y), pt(x + side, y), pt(x + side, y + side), pt(x, y + side)];

function scene(multi = true) {
  const h = toolHarness();
  const area = h.add({
    kind: 'polygon',
    pts: square(0, 0, 100),
    zs: [1, 2, 3, 4],
    holes: [{ pts: square(20, 20, 10), zs: [10, 11, 12, 13] }],
    ...(multi ? { parts: [{ pts: square(200, 0, 100), zs: [5, 6, 7, 8], holes: [{ pts: square(240, 40, 20), zs: [20, 21, 22, 23] }] }] } : {}),
  }) as PolylineEntity;
  const state = { index: 0 };
  Object.assign(h.ctx.view, { gripAt: () => ({ id: area.id, index: state.index }) });
  const menu = (index: number) => {
    state.index = index;
    return gripItems(h.ctx, at(0, 0).screen);
  };
  const run = (items: MenuItem[], label: string) => {
    const item = items.find((i) => i.label === label);
    if (!item?.run) throw new Error(`“${label}” yok: ${items.map((i) => i.label).join(', ')}`);
    item.run();
  };
  const now = () => h.doc.get(area.id) as PolylineEntity;
  return { h, area, menu, run, now };
}

const labels = (items: MenuItem[]) => items.filter((i) => i.kind !== 'separator').map((i) => i.label);

describe('the grip menu on the second part of an area', () => {
  it('a corner grip: the heading counts in its part, and it removes the corner from that part alone', () => {
    const { h, area, menu, run, now } = scene();
    // Grips run part after part: 12 for the first part (4 corners, 4 mid grips, 4 hole corners), then the second's.
    expect(entityGrips(area)[13]).toEqual(pt(300, 0));
    const items = menu(13);
    expect(labels(items)).toEqual(['Köşe 2', 'Köşeyi sil']);
    run(items, 'Köşeyi sil');
    expect(now().parts![0].pts).toEqual([pt(200, 0), pt(300, 100), pt(200, 100)]);
    expect(now().parts![0].zs).toEqual([5, 7, 8]);
    expect(now().parts![0].holes).toEqual(area.parts![0].holes);
    expect(now().pts).toEqual(area.pts);
    expect(now().holes).toEqual(area.holes);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(h.said()).toEqual(['Köşe sil: tamam.']);
    expect(h.doc.undo()).toBe('Köşe sil');
  });

  it('a mid grip: the edge of its part, where a corner is added on its middle', () => {
    const { area, menu, run, now } = scene();
    expect(entityGrips(area)[16]).toEqual(pt(250, 0));
    const items = menu(16);
    expect(labels(items)).toEqual(['Kenar 1', 'Ortasına köşe ekle', 'Yaya dönüştür']);
    run(items, 'Ortasına köşe ekle');
    expect(now().parts![0].pts).toEqual([pt(200, 0), pt(250, 0), ...square(200, 0, 100).slice(1)]);
    expect(now().parts![0].zs).toEqual([5, 5.5, 6, 7, 8]);
    expect(now().pts).toEqual(area.pts);
    expect(now().zs).toEqual([1, 2, 3, 4]);
  });

  it('a mid grip: the edge turns into an arc and back into a straight edge, in its part only', () => {
    const { area, menu, run, now } = scene();
    run(menu(17), 'Yaya dönüştür');
    // The edge from (300, 0) to (300, 100), the second of the part.
    const bulges = now().parts![0].bulges!;
    expect(bulges).toHaveLength(4);
    expect(bulges[1]).not.toBe(0);
    expect([bulges[0], bulges[2], bulges[3]]).toEqual([0, 0, 0]);
    expect(now().bulges).toBeUndefined();
    expect(now().parts![0].zs).toEqual([5, 6, 7, 8]);
    expect(labels(menu(17))).toEqual(['Kenar 2', 'Ortasına köşe ekle', 'Düz kenar yap']);
    run(menu(17), 'Düz kenar yap');
    expect(now().parts![0].bulges).toBeUndefined();
    expect(now().parts![0].pts).toEqual(area.parts![0].pts);
    expect(now().parts![0].holes).toEqual(area.parts![0].holes);
  });

  it('the corner of a hole of a part is moved by dragging: no menu', () => {
    const { menu } = scene();
    // The part's hole corners are its grips 8 to 11, after its 4 corners and 4 mid grips.
    expect(menu(20)).toEqual([]);
    expect(menu(23)).toEqual([]);
  });

  it('a corner grip of the first part still edits the first part, the parts after it whole', () => {
    const { menu, run, now, area } = scene();
    expect(labels(menu(0))).toEqual(['Köşe 1', 'Köşeyi sil']);
    run(menu(0), 'Köşeyi sil');
    expect(now().pts).toEqual(area.pts.slice(1));
    expect(now().zs).toEqual([2, 3, 4]);
    expect(now().parts).toEqual(area.parts);
    expect(now().holes).toEqual(area.holes);
  });
});

describe('the grip menu on an area of one part', () => {
  it('removes a corner of an area that has elevations: they follow the corners that stay (this refused: too many elevations)', () => {
    const { h, menu, run, now } = scene(false);
    run(menu(1), 'Köşeyi sil');
    expect(h.said()).toEqual(['Köşe sil: tamam.']);
    expect(now().pts).toEqual([pt(0, 0), pt(100, 100), pt(0, 100)]);
    expect(now().zs).toEqual([1, 3, 4]);
    expect(now().holes![0].zs).toEqual([10, 11, 12, 13]);
    expect(now().parts).toBeUndefined();
  });
});
