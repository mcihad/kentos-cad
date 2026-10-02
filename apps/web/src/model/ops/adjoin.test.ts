import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import type { Area, Ring } from '../geom/overlay';
import { adjoinAvoidAreas, adjoinFillAreas, adjoinWork, type Avoided } from './adjoin';

/**
 * Bitişik alan and the overlap control (docs/adr/0162) through the WASM core, against the independent reference in
 * fixtures/adjoin/v1/cases.json (scripts/fixtures/adjoin_cases.py: exact rationals, the arc cases by hand with 50-digit
 * mpmath, no KentOS code), the cases the core runs natively in crates/shared/geometry-core/tests/adjoin.rs: an input
 * vertex bit for bit, any other within 1e-9 m, bulges within 1e-12, and the neighbours a new area overlaps. The core's
 * order is its own: both sides are put in the reference's order first. The kept neighbours (`adjoinWork`) give what
 * the operations give.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Pt = [number, number];
interface Want {
  outer: { pts: Pt[]; bulges?: number[] };
  holes: { pts: Pt[]; bulges?: number[] }[];
  area: number;
}
interface File {
  format: string;
  avoid: { name: string; area: Area; neighbours: Area[][]; expect: { areas: Want[]; overlapped: number[] } }[];
  fill: { name: string; path: { pts: Vec2[]; bulges?: number[] }; neighbours: Area[][]; expect: { areas: Want[] } }[];
}

interface Flat {
  pts: Pt[];
  bulges: number[];
}

const before = (a: Pt, b: Pt) => a[0] - b[0] || a[1] - b[1];

/** A ring starting at its lowest vertex (x, then y), its way kept; a bulge stays with the edge leaving its vertex. */
function lowestFirst(r: Flat): Flat {
  let k = 0;
  r.pts.forEach((p, i) => {
    if (before(p, r.pts[k]) < 0) k = i;
  });
  return { pts: [...r.pts.slice(k), ...r.pts.slice(0, k)], bulges: [...r.bulges.slice(k), ...r.bulges.slice(0, k)] };
}

const ours = (r: Ring): Flat => lowestFirst({ pts: r.pts.map((p) => [p.x, p.y]), bulges: r.bulges ?? r.pts.map(() => 0) });
const theirs = (r: { pts: Pt[]; bulges?: number[] }): Flat => lowestFirst({ pts: r.pts, bulges: r.bulges ?? r.pts.map(() => 0) });

function sorted<T extends { outer: Flat; holes: Flat[] }>(list: T[]): T[] {
  for (const a of list) a.holes.sort((h, k) => before(h.pts[0], k.pts[0]));
  return list.sort((a, b) => before(a.outer.pts[0], b.outer.pts[0]) || before(a.outer.pts[1], b.outer.pts[1]));
}

function differ(got: Area[], want: Want[], inputs: Set<string>): string | null {
  const a = sorted(got.map((x) => ({ outer: ours(x.outer), holes: x.holes.map(ours) })));
  const b = sorted(want.map((x) => ({ outer: theirs(x.outer), holes: x.holes.map(theirs) })));
  if (a.length !== b.length) return `${a.length} alan ≠ ${b.length}: ${JSON.stringify(a.map((x) => x.outer.pts))}`;
  const ring = (g: Flat, w: Flat): string | null => {
    if (g.pts.length !== w.pts.length) return `${g.pts.length} köşe ≠ ${w.pts.length}: ${JSON.stringify(g.pts)}`;
    for (let i = 0; i < w.pts.length; i++) {
      const [p, q] = [g.pts[i], w.pts[i]];
      const exact = inputs.has(`${q[0]},${q[1]}`);
      const ok = exact ? p[0] === q[0] && p[1] === q[1] : Math.abs(p[0] - q[0]) <= 1e-9 && Math.abs(p[1] - q[1]) <= 1e-9;
      if (!ok) return `köşe ${i}: ${p} ≠ ${q}${exact ? ' (girdi köşesi, bit bit)' : ''}`;
      if (Math.abs(g.bulges[i] - w.bulges[i]) > 1e-12) return `kabarıklık ${i}: ${g.bulges[i]} ≠ ${w.bulges[i]}`;
    }
    return null;
  };
  for (let i = 0; i < a.length; i++) {
    const e = ring(a[i].outer, b[i].outer) ?? (a[i].holes.length !== b[i].holes.length ? `${a[i].holes.length} delik ≠ ${b[i].holes.length}` : null) ?? a[i].holes.map((h, k) => ring(h, b[i].holes[k])).find((x) => x) ?? null;
    if (e) return `alan ${i}: ${e}`;
  }
  return null;
}

function inputsOf(rings: { pts: Vec2[] }[], neighbours: Area[][]): Set<string> {
  const all = [...rings, ...neighbours.flatMap((obj) => obj.flatMap((a) => [a.outer, ...a.holes]))];
  return new Set(all.flatMap((r) => r.pts.map((p) => `${p.x},${p.y}`)));
}

/** A single-part neighbour as the entity the tools give the core (a polygon, its arcs and holes as they are). */
const entity = (a: Area): Entity => ({ kind: 'polygon', pts: a.outer.pts, ...(a.outer.bulges && { bulges: a.outer.bulges }), holes: a.holes }) as unknown as Entity;

describe('Bitişik alan ve çakışma denetimi', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/adjoin/v1/cases.json', import.meta.url), 'utf8')) as File;

  it('cuts every new area as the reference does', () => {
    expect(file.format).toBe('kentos.adjoin');
    expect(file.avoid.length).toBeGreaterThanOrEqual(40);
    for (const c of file.avoid) {
      const got: Avoided = adjoinAvoidAreas(c.area, c.neighbours);
      expect(got.overlapped, c.name).toEqual(c.expect.overlapped);
      expect(differ(got.areas, c.expect.areas, inputsOf([c.area.outer, ...c.area.holes], c.neighbours)), c.name).toBeNull();
    }
  });

  it('fills what the reference fills', () => {
    expect(file.fill.length).toBeGreaterThanOrEqual(40);
    for (const c of file.fill) {
      const got = adjoinFillAreas(c.path.pts, c.path.bulges ?? null, c.neighbours);
      expect(differ(got, c.expect.areas, inputsOf([c.path], c.neighbours)), c.name).toBeNull();
    }
  });

  it('gives through the kept neighbours what the operations give', () => {
    let checked = 0;
    for (const c of [...file.avoid, ...file.fill]) {
      if (c.neighbours.some((obj) => obj.length !== 1)) continue;
      const work = adjoinWork(c.neighbours.map((obj) => entity(obj[0])));
      try {
        if ('area' in c) expect(work.avoid(c.area), c.name).toEqual(adjoinAvoidAreas(c.area, c.neighbours));
        else expect(work.fill(c.path.pts, c.path.bulges ?? null), c.name).toEqual(adjoinFillAreas(c.path.pts, c.path.bulges ?? null, c.neighbours));
        expect(work.edgeCount, c.name).toBeGreaterThanOrEqual(c.neighbours.length ? 2 : 0);
      } finally {
        work.free();
      }
      checked++;
    }
    expect(checked).toBeGreaterThanOrEqual(80);
  });
});
