import { describe, expect, it } from 'vitest';
import { holeAdd, holeRemove, holeRing, reshapeBy } from './reshapeBy';

/**
 * Biçim değiştir and the hole operations (docs/adr/0173) through the WASM core, against the independent reference in
 * fixtures/reshape/v1/cases.json (scripts/fixtures/reshape_cases.py: rings and paths spliced with exact fractions, the
 * arc case in mpmath; no KentOS code), the cases the core runs natively in
 * crates/shared/geometry-core/tests/all/reshape.rs. Rings are compared as shapes: their start, their direction and
 * vertices on straight lines between straight edges do not matter; coordinates within 1e-9 m, bulges within 1e-12.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Vertex = [number, number, number];
type Json = Record<string, unknown>;

function ringOf(v: Json): Vertex[] {
  const pts = v.pts as { x: number; y: number }[];
  const bulges = v.bulges as number[] | undefined;
  return pts.map((p, i) => [p.x, p.y, bulges?.[i] ?? 0]);
}

/** A ring as a shape: repeated vertices and vertices on straight lines between straight edges dropped, counter-clockwise, from its least vertex. */
function canonical(input: Vertex[]): Vertex[] {
  let r = input.map((v) => [...v] as Vertex);
  const same = (a: Vertex, b: Vertex) => Math.abs(a[0] - b[0]) <= 1e-9 && Math.abs(a[1] - b[1]) <= 1e-9;
  for (;;) {
    const n = r.length;
    if (n < 3) break;
    let drop = -1;
    for (let i = 0; i < n; i++) {
      const [p, q, s] = [r[(i + n - 1) % n], r[i], r[(i + 1) % n]];
      if (same(p, q)) {
        p[2] = q[2];
        drop = i;
        break;
      }
      const cross = (q[0] - p[0]) * (s[1] - p[1]) - (q[1] - p[1]) * (s[0] - p[0]);
      const scale = Math.hypot(q[0] - p[0], q[1] - p[1]) * Math.hypot(s[0] - p[0], s[1] - p[1]);
      if (Math.abs(p[2]) <= 1e-12 && Math.abs(q[2]) <= 1e-12 && Math.abs(cross) <= 1e-12 * Math.max(scale, 1)) {
        drop = i;
        break;
      }
    }
    if (drop < 0) break;
    r.splice(drop, 1);
  }
  const n = r.length;
  let area = 0;
  for (let i = 0; i < n; i++) area += r[i][0] * r[(i + 1) % n][1] - r[(i + 1) % n][0] * r[i][1];
  if (area < 0) {
    // Reversed: edge i of the reversed ring is edge (n − 2 − i) of the old, turned.
    const pts = [...r].reverse();
    r = pts.map((p, i) => [p[0], p[1], -r[(2 * n - 2 - i) % n][2]] as Vertex);
  }
  let start = 0;
  for (let i = 1; i < n; i++) if (r[i][0] < r[start][0] || (r[i][0] === r[start][0] && r[i][1] < r[start][1])) start = i;
  return [...r.slice(start), ...r.slice(0, start)];
}

function ringDiffers(a: Vertex[], b: Vertex[], path: string): string | null {
  const [x, y] = [canonical(a), canonical(b)];
  if (x.length !== y.length) return `${path}: ${JSON.stringify(x)} ≠ ${JSON.stringify(y)}`;
  for (let i = 0; i < x.length; i++) {
    if (Math.abs(x[i][0] - y[i][0]) > 1e-9 || Math.abs(x[i][1] - y[i][1]) > 1e-9 || Math.abs(x[i][2] - y[i][2]) > 1e-12) return `${path}[${i}]: ${x[i]} ≠ ${y[i]}`;
  }
  return null;
}

function partsOf(e: Json): { outer: Vertex[]; holes: Vertex[][] }[] {
  const holes = (v: Json) => ((v.holes as Json[] | undefined) ?? []).map(ringOf);
  return [{ outer: ringOf(e), holes: holes(e) }, ...((e.parts as Json[] | undefined) ?? []).map((p) => ({ outer: ringOf(p), holes: holes(p) }))];
}

function areaDiffers(got: Json, want: Json, name: string): string | null {
  const g = partsOf(got);
  const w = (want.parts as Json[]).map((p) => ({ outer: ringOf(p), holes: (p.holes as Json[]).map(ringOf) }));
  if (g.length !== w.length) return `${name}: ${g.length} parça ≠ ${w.length} parça`;
  for (let k = 0; k < g.length; k++) {
    const d = ringDiffers(g[k].outer, w[k].outer, `${name}.parts[${k}]`);
    if (d) return d;
    if (g[k].holes.length !== w[k].holes.length) return `${name}.parts[${k}]: ${g[k].holes.length} delik ≠ ${w[k].holes.length} delik`;
    const left = [...g[k].holes];
    for (const h of w[k].holes) {
      const at = left.findIndex((x) => ringDiffers(x, h, '') === null);
      if (at < 0) return `${name}.parts[${k}]: ${JSON.stringify(canonical(h))} deliği yok`;
      left.splice(at, 1);
    }
  }
  return null;
}

function pathDiffers(got: Json, want: Json, name: string): string | null {
  const [g, w] = [ringOf(got), ringOf(want)];
  if (g.length !== w.length) return `${name}: ${JSON.stringify(g)} ≠ ${JSON.stringify(w)}`;
  for (let i = 0; i < g.length; i++) {
    if (Math.abs(g[i][0] - w[i][0]) > 1e-9 || Math.abs(g[i][1] - w[i][1]) > 1e-9 || Math.abs(g[i][2] - w[i][2]) > 1e-12) return `${name}[${i}]: ${g[i]} ≠ ${w[i]}`;
  }
  return null;
}

interface Case {
  name: string;
  op: 'reshape' | 'holeAdd' | 'holeRemove' | 'holeRing';
  shape: Json;
  sketch?: { x: number; y: number }[];
  ring?: Json;
  at?: { x: number; y: number };
  refusal?: string;
  expect?: Json;
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/reshape/v1/cases.json', import.meta.url), 'utf8')) as { format: string; cases: Case[] };

function run(c: Case): Json {
  const e = c.shape as never;
  switch (c.op) {
    case 'reshape':
      return reshapeBy(e, c.sketch!) as unknown as Json;
    case 'holeAdd':
      return holeAdd(e, c.ring as never) as unknown as Json;
    case 'holeRemove':
      return holeRemove(e, c.at!) as unknown as Json;
    case 'holeRing':
      return holeRing(e, c.at!) as unknown as Json;
  }
}

describe('Biçim değiştir ve delikler', () => {
  it('is the reference’s file', () => expect(file.format).toBe('kentos.reshape'));

  it('gives every case’s object or refusal', () => {
    expect(file.cases.length).toBeGreaterThanOrEqual(30);
    const off = file.cases.flatMap((c) => {
      const got = run(c);
      const refusal = got.refusal as { why: string } | undefined;
      if (c.refusal) return refusal?.why === c.refusal ? [] : [`${c.name}: ${c.refusal} reddi beklenirken ${JSON.stringify(got).slice(0, 120)}`];
      if (refusal) return [`${c.name}: beklenmeyen ret ${refusal.why}`];
      const done = got.done as Json;
      const want = c.expect!;
      const d = want.ring ? ringDiffers(ringOf(done), ringOf(want.ring as Json), c.name) : want.kind === 'polygon' ? areaDiffers(done, want, c.name) : pathDiffers(done, want, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });
});
