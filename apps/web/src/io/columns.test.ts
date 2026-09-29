import { describe, expect, it } from 'vitest';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import { ColumnsReader, KINDS, byCodePoint, packDrawing, unpackSnapshot, type DrawingHead, type PageEntity } from './columns';
import { KcadError, difference, transferables } from './kcad';

/**
 * The page's side of the typed boundary (io/columns.ts, docs/adr/0030): what
 * it packs reads back to the same objects, every float bit for bit, in the
 * layout the Rust side reads (io/kcad.wasm.test.ts holds the two to each
 * other on the fixtures); what the contract does not know is counted, and a
 * value of the wrong kind stops the save with its place.
 */

const head: DrawingHead = {
  format: 'kentos.document',
  version: 2,
  name: 'Sınama',
  settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000 },
  origin: { x: 500000, y: 4400000 },
  layers: [{ id: '0', name: '0', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
  activeLayer: '0',
  styles: { items: [], categories: [] },
};

let n = 0;
const uid = () => `0192f5a0-7c3e-7000-8000-${(++n).toString(16).padStart(12, '0')}`;
const base = (layerId = '0') => ({ id: n + 1, uid: uid(), layerId, attrs: {} as Record<string, string> });
const P = (x: number, y: number) => ({ x, y });

/** One object of every kind, with every optional field somewhere. */
function everyKind(): PageEntity[] {
  return [
    { ...base(), kind: 'point', p: P(-0, 1.5), z: 12.25, label: 'P1', color: '#FF0000', symbol: 'nirengi' },
    { ...base('yol'), kind: 'line', a: P(1, 2), b: P(3, 4), lineWeight: 0.35 },
    { ...base(), kind: 'polyline', pts: [P(0, 0), P(1, 1), P(2, 0)], bulges: [0.5, -0] },
    { ...base(), kind: 'polygon', pts: [P(0, 0), P(10, 0), P(10, 10)], bulges: [0, 0, 0.25], holes: [{ pts: [P(1, 1), P(2, 1), P(2, 2)] }, { pts: [P(3, 3), P(4, 3), P(4, 4)], bulges: [0.1] }] },
    { ...base(), kind: 'circle', c: P(5, 5), r: 2, lineWeight: 0 },
    { ...base(), kind: 'arc', c: P(5, 5), r: 2, a0: 0, a1: Math.PI },
    { ...base(), kind: 'ellipse', c: P(1, 1), major: P(3, 0), ratio: 0.5, t0: 0, t1: 2 * Math.PI },
    { ...base(), kind: 'spline', pts: [P(0, 0), P(1, 2), P(3, 1)], closed: true },
    { ...base(), kind: 'xline', p: P(0, 0), dir: P(1, 0) },
    { ...base(), kind: 'ray', p: P(0, 0), dir: P(0, 1) },
    { ...base(), kind: 'text', p: P(2, 2), text: 'Ada 101 🏠', height: 2.5, rotation: -0 },
    { ...base(), kind: 'dimension', a: P(0, 0), b: P(10, 0), offset: 2, height: 1.5, text: '10,00', style: 'linear', angle: 90, c: P(5, 5) },
    { ...base(), kind: 'dimension', a: P(0, 0), b: P(10, 0), offset: 2, height: 1.5 },
    { ...base(), kind: 'hatch', ring: [P(0, 0), P(5, 0), P(5, 5)], holes: [[P(1, 1), P(2, 1), P(2, 2)]], pattern: { type: 'cross', angle: 45, spacing: 1 } },
    ...elevated(),
  ] as PageEntity[];
}

/** Vertex elevations (docs/adr/0142): every place they may be, −0 and null (a vertex without one, not 0) among them. */
function elevated(): PageEntity[] {
  return [
    { ...base(), kind: 'line', a: P(1, 2), b: P(3, 4), za: -0, zb: -25.125, lineWeight: 0.5 },
    { ...base(), kind: 'line', a: P(1, 2), b: P(3, 4), zb: 1e-300 },
    { ...base(), kind: 'polyline', pts: [P(0, 0), P(1, 1), P(2, 0)], zs: [101.5, null, -0] },
    { ...base(), kind: 'polyline', pts: [P(0, 0), P(1, 1), P(2, 0)], zs: [null, null, null] },
    {
      ...base(),
      kind: 'polygon',
      pts: [P(0, 0), P(10, 0), P(10, 10), P(0, 10)],
      bulges: [0, 0.25],
      zs: [50, 50.5, null, 5e-324],
      holes: [{ pts: [P(1, 1), P(2, 1), P(2, 2)], bulges: [0.1], zs: [null, 3, 1.7976931348623157e308] }, { pts: [P(3, 3), P(4, 3), P(4, 4)] }],
    },
    // Only a hole has them.
    { ...base(), kind: 'polygon', pts: [P(0, 0), P(10, 0), P(10, 10)], holes: [{ pts: [P(1, 1), P(2, 1), P(2, 2)], zs: [1, 2, 3] }] },
  ] as PageEntity[];
}

describe('the page packs a drawing into typed columns', () => {
  it('reads back every kind and every optional field, −0 kept, slots renumbered', () => {
    const list = everyKind();
    list[0].attrs = { Ada: '101', Parsel: '7', İlçe: 'Çankaya', '10': 'on', '2': 'iki' };
    const { drawing, dropped } = packDrawing(head, list);
    expect(dropped).toEqual({});
    const back = unpackSnapshot(drawing);
    const want: DocumentSnapshotV2 = { ...head, entities: list.map(({ uid: _u, ...e }) => e as DocumentSnapshotV2['entities'][number]), uids: list.map((e) => e.uid!) };
    expect(difference(back, want)).toBeNull();
    expect(back.entities.map((e) => e.id)).toEqual(list.map((_, i) => i + 1));
    const point = back.entities[0];
    expect(point.kind === 'point' && Object.is(point.p.x, -0)).toBe(true);
    expect(point.attrs).toEqual(list[0].attrs);
    // Attributes go in UTF-8 order, as the contract's map (not JavaScript's, which puts "2" and "10" first):
    // after the layer table and the point's colour, label and symbol come the keys and values.
    const c = drawing.columns;
    const all = new TextDecoder('utf-16le').decode(new Uint8Array(c.text.buffer, c.text.byteOffset, c.text.byteLength));
    const texts: string[] = [];
    for (let i = 0, at = 0; i < c.textLengths.length; at += c.textLengths[i++]) texts.push(all.slice(at, at + c.textLengths[i]));
    expect(texts.slice(0, 15)).toEqual(['0', 'yol', '#FF0000', 'P1', 'nirengi', '10', 'on', '2', 'iki', 'Ada', '101', 'Parsel', '7', 'İlçe', 'Çankaya']);
    // The layer table lists layers in the order objects first use them.
    const r = new ColumnsReader(drawing.columns);
    expect(r.count).toBe(list.length);
    expect(KINDS[drawing.columns.kinds[1]]).toBe('line');
  });

  it('carries vertex elevations, a vertex without one as NaN and never as 0, −0 kept', () => {
    const list = elevated();
    const { drawing, dropped } = packDrawing(head, list);
    expect(dropped).toEqual({});
    const back = unpackSnapshot(drawing);
    const want: DocumentSnapshotV2 = { ...head, entities: list.map(({ uid: _u, ...e }) => e as DocumentSnapshotV2['entities'][number]), uids: list.map((e) => e.uid!) };
    // `difference` compares every number with Object.is: null is not 0, −0 is not 0.
    expect(difference(back, want)).toBeNull();
    const [line, half, path, none, holed, hole] = back.entities;
    expect(line.kind === 'line' && Object.is(line.za, -0) && line.zb === -25.125 && line.lineWeight === 0.5).toBe(true);
    expect(half.kind === 'line' && half.za === undefined && half.zb === 1e-300 && !('za' in half)).toBe(true);
    expect(path.kind === 'polyline' && path.zs?.[0] === 101.5 && path.zs[1] === null && Object.is(path.zs[2], -0)).toBe(true);
    expect(none.kind === 'polyline' && none.zs).toEqual([null, null, null]);
    if (holed.kind !== 'polygon' || hole.kind !== 'polygon') throw new Error('polygons');
    expect(holed.zs).toEqual([50, 50.5, null, 5e-324]);
    expect(holed.holes?.[0].zs).toEqual([null, 3, 1.7976931348623157e308]);
    expect(holed.holes?.[1].zs).toBeUndefined();
    expect(hole.zs).toBeUndefined();
    expect(hole.holes?.[0].zs).toEqual([1, 2, 3]);

    // The columns: a list is its length and its numbers; NaN only where a vertex has none.
    const c = drawing.columns;
    // One of the path's list, the three of the all-null list, one each of the polygon's list and its hole's.
    expect([...c.floats].filter(Number.isNaN)).toHaveLength(1 + 3 + 1 + 1);
    const r = new ColumnsReader(c);
    expect(r.count).toBe(list.length);
  });

  it('counts elevations where the contract has none instead of writing them', () => {
    const list = everyKind();
    (list[2] as unknown as Record<string, unknown>).za = 5; // a polyline has no ends
    (list[13] as unknown as Record<string, unknown>).zs = [1, 2, 3]; // a hatch has no vertex elevations
    (list[0] as unknown as Record<string, unknown>).zs = [1]; // nor a point
    const { drawing, dropped } = packDrawing(head, list);
    expect(dropped).toEqual({ 'polyline.za': 1, 'hatch.zs': 1, 'point.zs': 1 });
    const back = unpackSnapshot(drawing);
    expect('za' in back.entities[2] || 'zs' in back.entities[13] || 'zs' in back.entities[0]).toBe(false);
    // A hole's own unknown field is still dropped, its elevations are not.
    const holed = elevated()[4] as unknown as { holes: Record<string, unknown>[] };
    holed.holes[0].note = 'bilinmeyen';
    expect(packDrawing(head, [holed as unknown as PageEntity]).dropped).toEqual({ 'polygon.holes.note': 1 });
  });

  it('orders text as UTF-8 bytes do, beyond U+FFFF too', () => {
    const words = ['b', 'a', 'ab', '10', '2', 'İ', 'z', '￿', '𐀀', '', 'Ç'];
    const utf8 = (s: string) => [...new TextEncoder().encode(s)];
    const bytes = (a: string, b: string) => {
      const x = utf8(a);
      const y = utf8(b);
      for (let i = 0; i < Math.min(x.length, y.length); i++) if (x[i] !== y[i]) return x[i] - y[i];
      return x.length - y.length;
    };
    expect([...words].sort(byCodePoint)).toEqual([...words].sort(bytes));
  });

  it('counts what the contract does not know instead of writing it', () => {
    const list = everyKind();
    (list[2] as unknown as Record<string, unknown>).note = 'bilinmeyen';
    (list[2] as unknown as Record<string, unknown>).holes = [{ pts: [P(0, 0), P(1, 1), P(2, 2)] }];
    ((list[3] as unknown as { pts: Record<string, unknown>[] }).pts[1] as Record<string, unknown>).w = 1;
    const { drawing, dropped } = packDrawing({ ...head, extra: true } as DrawingHead, list);
    expect(dropped).toEqual({ 'polyline.note': 1, 'polyline.holes': 1, 'polygon.pts.w': 1, 'belge.extra': 1 });
    const back = unpackSnapshot(drawing);
    expect('note' in back.entities[2] || 'holes' in back.entities[2]).toBe(false);
  });

  it('stops at a value the file cannot hold, saying where', () => {
    const packs = (change: (list: PageEntity[]) => void) => {
      const list = everyKind();
      change(list);
      try {
        packDrawing(head, list);
      } catch (e) {
        return `${(e as KcadError).code}: ${(e as Error).message}`;
      }
      return null;
    };
    expect(packs((l) => ((l[3] as { pts: { x: number }[] }).pts[2].x = Number.NaN))).toMatch(/^non_finite: .*entities\/3\/pts\/2\/x: sayı NaN ya da sonsuz/);
    expect(packs((l) => ((l[3] as unknown as { holes: { pts: { y: unknown }[] }[] }).holes[1].pts[0].y = '4'))).toMatch(/^wrong_type: .*entities\/3\/holes\/pts\/0\/y: sayı olmalı/);
    expect(packs((l) => ((l[4] as { r: number }).r = Infinity))).toMatch(/entities\/4\/r: sayı NaN/);
    expect(packs((l) => (l[5].uid = '0192F5A0-7C3E-7000-8000-000000000001'))).toMatch(/^bad_value: .*UUID değil/);
    expect(packs((l) => ((l[10] as { text: unknown }).text = 12))).toMatch(/entities\/10\/text: metin olmalı/);
    expect(packs((l) => ((l[11] as { style: string }).style = 'eğik'))).toMatch(/ölçü türü bilinmiyor/);
    expect(packs((l) => ((l[0] as { kind: string }).kind = 'blok'))).toMatch(/^unknown_kind: /);
  });

  it('stops at an elevation the file cannot hold, saying where', () => {
    const packs = (change: (list: PageEntity[]) => void) => {
      const list = everyKind();
      change(list);
      try {
        packDrawing(head, list);
      } catch (e) {
        return `${(e as KcadError).code}: ${(e as Error).message}`;
      }
      return null;
    };
    const at = (kind: string) => everyKind().findIndex((e) => e.kind === kind && ('zs' in e || 'za' in e || 'zb' in e));
    const line = at('line');
    const path = at('polyline');
    const polygon = at('polygon');
    // One elevation per vertex, in the path and in its hole.
    expect(packs((l) => (l[path] as unknown as { zs: unknown[] }).zs.pop())).toMatch(new RegExp(`^bad_value: .*entities/${path}/zs: 2 kot var ama 3 köşe var`));
    expect(packs((l) => (l[polygon] as unknown as { zs: unknown[] }).zs.push(1))).toMatch(new RegExp(`^bad_value: .*entities/${polygon}/zs: 5 kot var ama 4 köşe var`));
    expect(packs((l) => (l[polygon] as unknown as { holes: { zs: unknown[] }[] }).holes[0].zs.pop())).toMatch(new RegExp(`^bad_value: .*entities/${polygon}/holes/0/zs: 2 kot var ama 3 köşe var`));
    // A finite number or null: not NaN, an infinity, text, or a missing entry.
    expect(packs((l) => ((l[path] as unknown as { zs: unknown[] }).zs[1] = Number.NaN))).toMatch(new RegExp(`^non_finite: .*entities/${path}/zs/1: sayı NaN ya da sonsuz`));
    expect(packs((l) => ((l[polygon] as unknown as { holes: { zs: unknown[] }[] }).holes[0].zs[2] = Infinity))).toMatch(new RegExp(`^non_finite: .*entities/${polygon}/holes/0/zs/2:`));
    expect(packs((l) => ((l[path] as unknown as { zs: unknown[] }).zs[0] = '101,5'))).toMatch(new RegExp(`^wrong_type: .*entities/${path}/zs/0: kot sayı ya da null olmalı`));
    expect(packs((l) => ((l[path] as unknown as { zs: unknown[] }).zs[0] = undefined))).toMatch(/^wrong_type: /);
    expect(packs((l) => ((l[path] as unknown as { zs: unknown }).zs = 'kotlar'))).toMatch(new RegExp(`^wrong_type: .*entities/${path}/zs: kot listesi olmalı`));
    // A line's end is a number, not null: an end without an elevation has none.
    expect(packs((l) => ((l[line] as unknown as { za: unknown }).za = Number.NaN))).toMatch(new RegExp(`^non_finite: .*entities/${line}/za: sayı NaN`));
    expect(packs((l) => ((l[line] as unknown as { zb: unknown }).zb = null))).toMatch(new RegExp(`^wrong_type: .*entities/${line}/zb: sayı olmalı`));
  });

  it('gives its buffers to the worker once each', () => {
    const { drawing } = packDrawing(head, everyKind());
    const buffers = transferables(drawing.columns);
    expect(new Set(buffers).size).toBe(buffers.length);
    expect(buffers.length).toBeGreaterThanOrEqual(1);
  });

  it('refuses columns that do not hold together', () => {
    const { drawing } = packDrawing(head, everyKind());
    const short = { ...drawing, columns: { ...drawing.columns, floats: drawing.columns.floats.subarray(0, 10) } };
    expect(() => unpackSnapshot(short)).toThrow(KcadError);
    const extra = { ...drawing, columns: { ...drawing.columns, ints: new Uint32Array([...drawing.columns.ints, 0]) } };
    expect(() => unpackSnapshot(extra)).toThrow(/fazladan/);
    // An elevation list running past the numbers: the drawing's last object is one with elevations, so cut the last numbers.
    const cut = { ...drawing, columns: { ...drawing.columns, floats: drawing.columns.floats.subarray(0, drawing.columns.floats.length - 2) } };
    expect(() => unpackSnapshot(cut)).toThrow(/sayılardan uzun/);
  });

  it('packs a large drawing without a string or an object per vertex', () => {
    const list: PageEntity[] = [];
    for (let i = 0; i < 20_000; i++)
      list.push({ ...base(), kind: 'polygon', attrs: { Ada: String(100 + (i >> 6)), Parsel: String(i & 63) }, pts: Array.from({ length: 20 }, (_, k) => P(486000 + i + Math.cos(k), 4420000 + Math.sin(k))) } as PageEntity);
    const t = performance.now();
    const { drawing } = packDrawing(head, list);
    const ms = performance.now() - t;
    expect(drawing.columns.floats.length).toBe(20_000 * 40);
    // A generous bound for a loaded test machine: the point is no per-vertex allocation (seconds before).
    expect(ms).toBeLessThan(1500);
  });
});
