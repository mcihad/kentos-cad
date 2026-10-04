import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import { polygonize, type PolyLabel, type PolyResult } from './polygonize';

/**
 * Toplu alan (docs/adr/0151) through the WASM core, against the independent reference in
 * fixtures/polygonize/v1/cases.json (scripts/fixtures/polygonize_cases.py, exact rationals, no KentOS code), the
 * cases the core runs natively in crates/shared/geometry-core/tests/all/polygonize.rs: regions (rings, holes, labels,
 * the input area each repeats), labels on a boundary and free ends; points within 1e-8 m. Both sides are put in the
 * reference's order first (the core's is smallest region first).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Pt = [number, number];
interface Expected {
  regions: { outer: Pt[]; holes: Pt[][]; labels: number[]; existing: number | null }[];
  onBoundary: number[];
  freeEnds: Pt[];
}
interface Case {
  name: string;
  lines: Entity[];
  labels: PolyLabel[];
  islands: boolean;
  expected: Expected;
}

const less = (a: Pt, b: Pt) => a[0] - b[0] || a[1] - b[1];
const lowestFirst = (ring: Pt[]): Pt[] => {
  let k = 0;
  ring.forEach((p, i) => less(p, ring[k]) < 0 && (k = i));
  return [...ring.slice(k), ...ring.slice(0, k)];
};
const close = (a: Pt[], b: Pt[]) => a.length === b.length && a.every((p, i) => Math.abs(p[0] - b[i][0]) <= 1e-8 && Math.abs(p[1] - b[i][1]) <= 1e-8);

function ordered(regions: Expected['regions']): Expected['regions'] {
  return regions
    .map((r) => ({ ...r, outer: lowestFirst(r.outer), holes: r.holes.map(lowestFirst).sort((a, b) => less(a[0], b[0])) }))
    .sort((a, b) => less(a.outer[0], b.outer[0]) || less(a.outer[1], b.outer[1]));
}

function ours(r: PolyResult): Expected {
  const pts = (ring: { pts: { x: number; y: number }[] }): Pt[] => ring.pts.map((p) => [p.x, p.y]);
  return {
    regions: r.regions.map((g) => ({ outer: pts(g.area.outer), holes: g.area.holes.map(pts), labels: g.labels, existing: g.existing ?? null })),
    onBoundary: r.onBoundary,
    freeEnds: r.freeEnds.map((p): Pt => [p.x, p.y]).sort(less),
  };
}

function differ(got: Expected, want: Expected): string | null {
  const a = ordered(got.regions);
  const b = ordered(want.regions);
  if (a.length !== b.length) return `${a.length} bölge ≠ ${b.length} bölge`;
  for (let i = 0; i < a.length; i++) {
    if (!close(a[i].outer, b[i].outer)) return `bölge ${i}: dış halka ${JSON.stringify(a[i].outer)} ≠ ${JSON.stringify(b[i].outer)}`;
    if (a[i].holes.length !== b[i].holes.length || !a[i].holes.every((h, k) => close(h, b[i].holes[k]))) return `bölge ${i}: delikler`;
    if (JSON.stringify(a[i].labels) !== JSON.stringify(b[i].labels)) return `bölge ${i}: etiketler ${JSON.stringify(a[i].labels)} ≠ ${JSON.stringify(b[i].labels)}`;
    if (a[i].existing !== b[i].existing) return `bölge ${i}: var olan alan ${a[i].existing} ≠ ${b[i].existing}`;
  }
  if (JSON.stringify(got.onBoundary) !== JSON.stringify(want.onBoundary)) return `sınırdakiler ${JSON.stringify(got.onBoundary)} ≠ ${JSON.stringify(want.onBoundary)}`;
  if (!close(got.freeEnds, want.freeEnds)) return `boşta uçlar ${JSON.stringify(got.freeEnds)} ≠ ${JSON.stringify(want.freeEnds)}`;
  return null;
}

describe('Toplu alan', () => {
  it('finds every case as the reference finds it', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/polygonize/v1/cases.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.polygonize-fixtures', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(50);
    const off = file.cases.flatMap((c) => {
      const d = differ(ours(polygonize(c.lines, c.labels, c.islands)), c.expected);
      return d ? [`${c.name}: ${d}`] : [];
    });
    expect(off).toEqual([]);
  });
});
