import { describe, expect, it } from 'vitest';
import type { Entity } from '../../model/entities';
import { LABEL, LABEL_STRIDE } from '../../viewport/storeRecords';
import { framedSpots, insideFrame, labelAnchor } from './frameLabels';

/**
 * A map frame writes a label whose anchor is inside its content and cuts it
 * at the frame; a label whose anchor is outside is not written (both
 * platforms, 3 Ekim). The frame: 100 × 50 mm of paper at 1/1000 round
 * (1000, 2000), so 100 × 50 m of ground, turned with the view.
 */

const frame = (rotation = 0) => ({ clip: { left: 0, top: 0, width: 100_000, height: 50_000 }, view: { center: { x: 1000, y: 2000 }, scale: 1000, rotation } }) as Parameters<typeof insideFrame>[0];

/** A label record: id, what, x, y and the rest. */
const record = (id: number, what: number, x: number, y: number, a = 0, b = 0): number[] => [id, what, x, y, a, b, 0, 0, 0];

describe('the labels a map frame writes', () => {
  it('knows its content on the ground, turned with the view', () => {
    const flat = insideFrame(frame());
    expect(flat(1049, 2024)).toBe(true);
    expect(flat(951, 1976)).toBe(true);
    expect(flat(1051, 2000)).toBe(false);
    expect(flat(1000, 2026)).toBe(false);
    // A quarter turn: the paper's width runs north–south on the ground.
    const turned = insideFrame(frame(90_000));
    expect(turned(1000, 2049)).toBe(true);
    expect(turned(1049, 2000)).toBe(false);
    expect(turned(1024, 2000)).toBe(true);
    // Turned 30° clockwise on the paper: a point just in and one just out along the frame's own axes
    // (u to the paper's right, v down it; north on the ground is up the paper turned 30° to the right).
    const t = (30 * Math.PI) / 180;
    const thirty = insideFrame(frame(30_000));
    const at = (u: number, v: number): [number, number] => [1000 + u * Math.cos(t) + v * Math.sin(t), 2000 + u * Math.sin(t) - v * Math.cos(t)];
    // The paper's top edge, 25 m up it, is north turned 30° to the left on the ground: 26 m that way is out,
    // 26 m due north is in (it is only 22.5 m up the paper).
    expect(thirty(1000 - 26 * Math.sin(t), 2000 + 26 * Math.cos(t))).toBe(false);
    expect(thirty(1000, 2026)).toBe(true);
    expect(thirty(...at(49.9, 24.9))).toBe(true);
    expect(thirty(...at(50.1, 0))).toBe(false);
    expect(thirty(...at(0, 25.1))).toBe(false);
  });

  it('writes a label by its anchor: a text by its own point, an object’s label by the middle the engine placed it at', () => {
    const objects = new Map<number, Entity>([
      [1, { id: 1, kind: 'polygon', layerId: 'parsel', pts: [], label: '7' } as unknown as Entity],
      [2, { id: 2, kind: 'polygon', layerId: 'parsel', pts: [], label: '1244 ada' } as unknown as Entity],
      // Right-aligned at the frame's east edge: its baseline starts outside, its point is inside.
      [3, { id: 3, kind: 'text', layerId: 'yazi', p: { x: 1049, y: 2000 }, text: 'Kızılırmak Caddesi', height: 3, rotation: 0, align: 'right' } as unknown as Entity],
      [4, { id: 4, kind: 'polyline', layerId: 'yol', pts: [], label: '1428. Sokak' } as unknown as Entity],
      [5, { id: 5, kind: 'polyline', layerId: 'yol', pts: [], label: '1434. Sokak' } as unknown as Entity],
      [6, { id: 6, kind: 'point', layerId: 'nokta', p: { x: 1000, y: 2030 }, label: '898.97' } as unknown as Entity],
    ]);
    // An engine's label: its frame (middle, angle, size, class, state), then its line or letters.
    const placed = (id: number, x: number, y: number, parts: number[][]): number[] => [id, LABEL.placed, x, y, 0, 20, 10, 0, 0, ...parts.flat()];
    const line = (id: number, x: number, y: number) => [id, LABEL.placedLine, x, y, 0, 10, 0, 20, 0];
    const letter = (id: number, x: number, y: number, i: number) => [id, LABEL.placedLetter, x, y, 0, 10, 0, i, 5];
    const spots = Float64Array.from([
      ...placed(1, 1010, 2010, [line(1, 1010, 2010)]),
      // Its letters would reach in; its middle is 3 m outside.
      ...placed(2, 947, 2000, [line(2, 947, 2000)]),
      ...record(3, LABEL.text, 1060, 2000),
      // A curved name from (930, 1990) to (980, 1990): it starts outside, its letters' middle (955) is inside.
      ...placed(4, 955, 1990, [letter(4, 930, 1990, 0), letter(4, 955, 1990, 1), letter(4, 980, 1990, 2)]),
      ...placed(5, 1060, 2010, [line(5, 1060, 2010)]),
      ...placed(6, 1000, 2030, [line(6, 1000, 2030)]),
    ]);
    const out = framedSpots(spots, (id) => objects.get(id), insideFrame(frame()));
    const frames = Array.from({ length: out.length / LABEL_STRIDE }, (_, i) => [out[i * LABEL_STRIDE], out[i * LABEL_STRIDE + 1]]).filter(([, what]) => what !== LABEL.placedLine && what !== LABEL.placedLetter);
    expect(frames.map(([id]) => id)).toEqual([1, 3, 4]);
    // A label goes whole: object 4's three letters with its frame, the one outside too.
    expect(out.length / LABEL_STRIDE).toBe(2 + 1 + 4);
    expect(labelAnchor(spots, 4 * LABEL_STRIDE, objects.get(3))).toEqual([1049, 2000]);
    expect(labelAnchor(spots, 5 * LABEL_STRIDE, objects.get(4))).toEqual([955, 1990]);
    // A record is kept whole; the frame's layer list still decides first.
    expect(Array.from(out.subarray(0, LABEL_STRIDE))).toEqual(placed(1, 1010, 2010, []));
    const parcelsOnly = framedSpots(spots, (id) => objects.get(id), insideFrame(frame()), (e) => e.layerId === 'parsel');
    expect(parcelsOnly.length / LABEL_STRIDE).toBe(2);
  });
});
