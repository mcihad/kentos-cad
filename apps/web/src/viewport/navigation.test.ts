import { describe, expect, it } from 'vitest';
import { CoreStore } from '../wasm/core';
import { cards, fit, nextSide, toWorld, viewFrame, type Card, type Side } from './navigation';

/**
 * Genel bakış and Büyüteç (docs/adr/0181) against `fixtures/navigation/v1/cases.json`, which
 * scripts/fixtures/navigation_cases.py writes from the ADR: the cards' places, the overview's fit, and its
 * picture from the WASM store pixel for pixel (the desktop plays the same cases natively).
 */

interface CardCase {
  name: string;
  area: [number, number];
  header: number;
  cbs: boolean;
  overview: boolean;
  side: Side;
  pointer: [number, number];
  layout: { overview: Card | null; right: Card; left: Card };
  next: Side;
}
interface FitCase {
  name: string;
  extent: [number, number, number, number];
  size: [number, number];
  center: [number, number];
  metresPerPixel: number;
  viewPx: [number, number];
  fit: [number, number, number];
  frame: [number, number, number, number];
  cross: boolean;
  presses: { at: [number, number]; world: [number, number] }[];
}
interface PictureCase {
  name: string;
  size: [number, number];
  dpr: number;
  layers: string[];
  objects: { layer: string; shape: Record<string, unknown> }[];
  extent: [number, number, number, number];
  fit: [number, number, number];
  width: number;
  height: number;
  rows: string[];
}
interface File {
  cards: CardCase[];
  fits: FitCase[];
  pictures: PictureCase[];
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/navigation/v1/cases.json', import.meta.url), 'utf8')) as File;

describe('Genel bakış and Büyüteç cards (ADR 0181 §4)', () => {
  for (const c of file.cards)
    it(c.name, () => {
      const laid = cards({ width: c.area[0], height: c.area[1] }, c.header, c.cbs, c.overview);
      expect(laid).toEqual(c.layout);
      expect(nextSide(laid, c.side, { x: c.pointer[0], y: c.pointer[1] })).toBe(c.next);
    });
});

describe('Genel bakış fit (ADR 0181 §3)', () => {
  for (const c of file.fits)
    it(c.name, () => {
      const size = { width: c.size[0], height: c.size[1] };
      const f = fit({ minX: c.extent[0], minY: c.extent[1], maxX: c.extent[2], maxY: c.extent[3] }, size);
      expect([f.cx, f.cy, f.k]).toEqual(c.fit);
      const v = viewFrame(f, size, { x: c.center[0], y: c.center[1] }, c.metresPerPixel, { width: c.viewPx[0], height: c.viewPx[1] });
      expect(v.frame).toEqual(c.frame);
      expect(v.cross).toBe(c.cross);
      for (const p of c.presses) {
        const w = toWorld(f, size, p.at[0], p.at[1]);
        expect([w.x, w.y]).toEqual(p.world);
      }
    });
});

/** A case's object as the document's entity. */
function entity(i: number, o: PictureCase['objects'][number]): Record<string, unknown> {
  const s = o.shape as Record<string, never>;
  const p = (v: [number, number]) => ({ x: v[0], y: v[1] });
  const base = { id: i + 1, layerId: o.layer, kind: s.kind, attrs: {} };
  switch (s.kind as string) {
    case 'point':
      return { ...base, p: p(s.p) };
    case 'line':
      return { ...base, a: p(s.a), b: p(s.b) };
    case 'polyline':
      return { ...base, pts: (s.points as [number, number][]).map(p) };
    case 'polygon': {
      const rings = s.rings as [number, number][][];
      return { ...base, pts: rings[0].map(p), ...(rings.length > 1 ? { holes: rings.slice(1).map((r) => ({ pts: r.map(p) })) } : {}) };
    }
    case 'circle':
      return { ...base, c: p(s.c), r: s.r };
    case 'arc':
      return { ...base, c: p(s.c), r: s.r, a0: s.a0, a1: s.a1 };
  }
  throw new Error(`a case's kind: ${String(s.kind)}`);
}

const COLORS = ['#E5484D', '#5FBF77', '#4D96FF'];

describe('Genel bakış picture from the WASM store (ADR 0181 §3)', () => {
  for (const c of file.pictures)
    it(c.name, () => {
      const s = new CoreStore();
      try {
        s.put(JSON.stringify(c.objects.map((o, i) => entity(i, o))));
        expect(Array.from(s.overviewExtent())).toEqual(c.extent);
        const colors = Object.fromEntries(c.layers.map((l, i) => [l, COLORS[i]]));
        const px = s.overviewPicture(c.size[0], c.size[1], c.dpr, JSON.stringify(colors));
        expect(px.length).toBe(c.width * c.height * 4);
        const rgb = COLORS.map((h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)).join());
        const rows: string[] = [];
        for (let j = 0; j < c.height; j++) {
          let row = '';
          for (let i = 0; i < c.width; i++) {
            const at = (j * c.width + i) * 4;
            if (px[at + 3] === 0) {
              row += '.';
              continue;
            }
            const letter = 'ABCDEFGH'[rgb.indexOf([px[at], px[at + 1], px[at + 2]].join())];
            row += px[at + 3] === 255 ? letter : letter.toLowerCase();
          }
          rows.push(row);
        }
        expect(rows).toEqual(c.rows);
      } finally {
        s.dispose();
      }
    });
});
