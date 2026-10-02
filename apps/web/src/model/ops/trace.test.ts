import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import { joinChain, traceGraph, traceNearest, tracePath, type ChainFound, type ChainObject, type Traced } from './trace';

/**
 * İzle and Zincir (docs/adr/0161) through the WASM core, against the independent reference in fixtures/trace/v1/trace.json
 * (scripts/fixtures/trace_cases.py, 50-digit mpmath, no KentOS code), the cases the core runs natively in
 * crates/shared/geometry-core/src/ops/trace.rs: corners within 1e-9 m (the given ones bit for bit), bulges within 1e-12,
 * lengths within 1e-9 m; where two ways are as short, either.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Way {
  pts: Vec2[];
  bulges: number[];
  exact: number[];
}

interface File {
  format: string;
  paths: { name: string; lines: Entity[]; a: Vec2; b: Vec2; expected: { ways: Way[]; length: number } | null }[];
  nearest: { name: string; lines: Entity[]; p: Vec2; reach: number; expected: Vec2 | null; exact: boolean }[];
  chains: { name: string; objects: ChainObject[]; seed: number; tol: number; expected: ChainFound }[];
}

const fits = (got: Traced, w: Way) =>
  got.pts.length === w.pts.length &&
  w.pts.every((p, i) => (w.exact.includes(i) ? p.x === got.pts[i].x && p.y === got.pts[i].y : Math.hypot(p.x - got.pts[i].x, p.y - got.pts[i].y) <= 1e-9)) &&
  got.bulges.length === w.bulges.length &&
  w.bulges.every((b, i) => Math.abs(b - got.bulges[i]) <= 1e-12);

describe('İzle ve Zincir', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/trace/v1/trace.json', import.meta.url), 'utf8')) as File;

  it('traces every way as the reference does, through the op and through the kept graph alike', () => {
    expect(file.format).toBe('kentos.trace');
    expect(file.paths.length).toBeGreaterThanOrEqual(15);
    for (const c of file.paths) {
      const got = tracePath(c.lines, c.a, c.b);
      const graph = traceGraph(c.lines);
      try {
        expect(graph.path(c.a, c.b), c.name).toEqual(got);
      } finally {
        graph.free();
      }
      if (!c.expected) {
        expect(got, c.name).toBeNull();
        continue;
      }
      expect(got, c.name).not.toBeNull();
      expect(c.expected.ways.some((w) => fits(got!, w)), `${c.name}: ${JSON.stringify(got)}`).toBe(true);
      expect(Math.abs(got!.length - c.expected.length), c.name).toBeLessThanOrEqual(1e-9);
    }
  });

  it('finds every nearest point as the reference does (a vertex bit for bit)', () => {
    expect(file.nearest.length).toBeGreaterThanOrEqual(5);
    for (const c of file.nearest) {
      const got = traceNearest(c.lines, c.p, c.reach);
      const graph = traceGraph(c.lines);
      try {
        expect(graph.nearest(c.p, c.reach), c.name).toEqual(got);
      } finally {
        graph.free();
      }
      if (!c.expected || c.exact) expect(got, c.name).toEqual(c.expected);
      else expect(Math.hypot(got!.x - c.expected.x, got!.y - c.expected.y), c.name).toBeLessThanOrEqual(1e-9);
    }
  });

  it('walks every chain as the reference does', () => {
    expect(file.chains.length).toBeGreaterThanOrEqual(7);
    for (const c of file.chains) expect(joinChain(c.objects, c.seed, c.tol), c.name).toEqual(c.expected);
  });
});
