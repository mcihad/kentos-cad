import { describe, expect, it } from 'vitest';
import { edgematchApply, edgematchLinks, type EdgeEnd, type EdgeMeet, type EdgeMember, type EdgeMethod, type EdgeSettings } from './edgematch';
import type { PathElevations } from './warp';
import type { Vec2 } from '../geometry';

/**
 * Kenar eşleme (docs/adr/0159) through the WASM core, against the independent reference in
 * fixtures/fit/v1/edgematch.json (scripts/fixtures/edgematch_cases.py, mpmath at 50 digits, no KentOS code), the cases
 * the core runs natively in crates/shared/geometry-core/src/ops/edgematch.rs.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Shape = { kind: 'line'; a: Vec2; b: Vec2 } | { kind: 'polyline'; pts: Vec2[]; bulges?: number[] } | { kind: string };

interface Expected {
  links: { source: number; sourceEnd: EdgeEnd; adjacent: number; adjacentEnd: EdgeEnd; gap: number; angle: number; score: number }[];
  unmatched: { source: number; end: EdgeEnd }[];
  junctions: number;
  others: number;
}

interface Applied {
  sources: (Shape | null)[];
  sourceZs: (PathElevations | null)[];
  adjacent: (Shape | null)[];
  adjacentZs: (PathElevations | null)[];
  refused: number[];
}

interface File {
  format: string;
  tolerance: { metres: number; degrees: number; score: number };
  cases: {
    name: string;
    sources: EdgeMember<Shape>[];
    adjacent: EdgeMember<Shape>[];
    settings: EdgeSettings<Shape>;
    expected: Expected;
    apply: { meet: EdgeMeet; method: EdgeMethod; use: number[]; expected: Applied }[];
  }[];
}

/** Two shapes the same within `metres`: kind, points, bulges (within 1e-12). */
function same(a: Shape, b: Shape, metres: number): boolean {
  const close = (p: Vec2, q: Vec2) => Math.hypot(p.x - q.x, p.y - q.y) <= metres;
  if (a.kind === 'line' && b.kind === 'line' && 'a' in a && 'a' in b) return close(a.a, b.a) && close(a.b, b.b);
  if (a.kind === 'polyline' && b.kind === 'polyline' && 'pts' in a && 'pts' in b) {
    const [ga, gb] = [a.bulges ?? [], b.bulges ?? []];
    return a.pts.length === b.pts.length && a.pts.every((p, i) => close(p, b.pts[i])) && ga.length === gb.length && ga.every((v, i) => Math.abs(v - gb[i]) <= 1e-12);
  }
  return false;
}

describe('Kenar eşleme', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/fit/v1/edgematch.json', import.meta.url), 'utf8')) as File;

  it('finds and writes every case as the reference does', () => {
    expect(file.format).toBe('kentos.fit-edgematch');
    expect(file.cases.length).toBeGreaterThanOrEqual(10);
    const { metres, degrees, score } = file.tolerance;
    const off = file.cases.flatMap((c) => {
      const out: string[] = [];
      const found = edgematchLinks(c.sources, c.adjacent, c.settings);
      const w = c.expected;
      if (found.links.length !== w.links.length) return [`${c.name}: ${found.links.length} links ≠ ${w.links.length}`];
      found.links.forEach((l, k) => {
        const e = w.links[k];
        const ok =
          l.source === e.source &&
          l.sourceEnd === e.sourceEnd &&
          l.adjacent === e.adjacent &&
          l.adjacentEnd === e.adjacentEnd &&
          Math.abs(l.gap - e.gap) <= metres &&
          Math.abs(l.angle - e.angle) <= degrees &&
          Math.abs(l.score - e.score) <= score;
        if (!ok) out.push(`${c.name}: link ${k}: ${JSON.stringify(l)} ≠ ${JSON.stringify(e)}`);
      });
      if (JSON.stringify(found.unmatched) !== JSON.stringify(w.unmatched)) out.push(`${c.name}: unmatched ${JSON.stringify(found.unmatched)}`);
      if (found.junctions !== w.junctions || found.others !== w.others) out.push(`${c.name}: junctions ${found.junctions}, others ${found.others}`);
      for (const a of c.apply) {
        const label = `${c.name} (${a.meet}, ${a.method})`;
        const got = edgematchApply(
          c.sources,
          c.adjacent,
          a.use.map((i) => found.links[i]),
          a.meet,
          a.method,
          c.settings.border ?? null,
        );
        if ('error' in got) {
          out.push(`${label}: ${got.error}`);
          continue;
        }
        for (const [side, shapes, zs, wantShapes, wantZs] of [
          ['source', got.sources, got.sourceZs, a.expected.sources, a.expected.sourceZs],
          ['adjacent', got.adjacent, got.adjacentZs, a.expected.adjacent, a.expected.adjacentZs],
        ] as const) {
          shapes.forEach((s, i) => {
            const want = wantShapes[i];
            if (s === null || want === null) {
              if (s !== want) out.push(`${label}: ${side} ${i}: ${JSON.stringify(s)} ≠ ${JSON.stringify(want)}`);
              return;
            }
            if (!same(s, want, metres)) out.push(`${label}: ${side} ${i}: ${JSON.stringify(s)}`);
            if (JSON.stringify(zs[i]) !== JSON.stringify(wantZs[i])) out.push(`${label}: ${side} ${i} elevations ${JSON.stringify(zs[i])}`);
          });
        }
        const refused = got.refused.map((k) => a.use[k]);
        if (JSON.stringify(refused) !== JSON.stringify(a.expected.refused)) out.push(`${label}: refused ${JSON.stringify(refused)}`);
      }
      return out;
    });
    expect(off).toEqual([]);
  });
});
