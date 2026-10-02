import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../model/geometry';
import { CoreStore } from '../wasm/core';
import { extensionRecords, readExtensions, SNAP_BITS, type Extension } from './picking';

/**
 * The snap additions (docs/adr/0163) through the WASM core, against the independent reference in
 * fixtures/snap/v1/cases.json (scripts/fixtures/snap_cases.py: exact rationals, the arcs with 50-digit mpmath, no KentOS
 * code), the cases the core runs natively in crates/shared/geometry-core/tests/snap.rs: the kind and the object exactly,
 * the point within 1e-9 m (a grid node bit for bit), and the acquisitions within 1e-12.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Pt = [number, number];
interface Case {
  name: string;
  entities: unknown[];
  layers: unknown[];
  p: Pt;
  tol: number;
  kinds: string[];
  from: Pt | null;
  extras: { extensions?: number[][]; parallels?: Pt[]; draft?: { pts: Pt[]; bulges?: number[] }; grid?: Pt };
  expect: { kind: string; point: Pt; id: number } | null;
}
interface File {
  format: string;
  kinds: string[];
  snap: Case[];
  extensionsAt: { name: string; entities: unknown[]; id: number; at: Pt; expect: number[] }[];
  directionAt: { name: string; entities: unknown[]; p: Pt; tol: number; expect: Pt | null }[];
}

function store(entities: unknown[], layers?: unknown[]): CoreStore {
  const s = new CoreStore();
  s.put(JSON.stringify(entities));
  if (layers) s.setLayers(JSON.stringify(layers));
  return s;
}

const close = (a: number, b: number, tol: number) => Math.abs(a - b) <= tol;

describe('Kenet ekleri', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/snap/v1/cases.json', import.meta.url), 'utf8')) as File;

  it('names the kinds as the core numbers them', () => {
    expect(file.format).toBe('kentos.snap');
    expect([...SNAP_BITS]).toEqual(file.kinds);
  });

  it('snaps as the reference does', () => {
    expect(file.snap.length).toBeGreaterThanOrEqual(35);
    for (const c of file.snap) {
      const s = store(c.entities, c.layers);
      const mask = c.kinds.reduce((m, k) => m | (1 << file.kinds.indexOf(k)), 0);
      const from = c.from && { x: c.from[0], y: c.from[1] };
      const x = c.extras;
      const r = s.snapEx(
        c.p[0],
        c.p[1],
        c.tol,
        mask,
        from,
        Float64Array.from((x.extensions ?? []).flat()),
        Float64Array.from((x.parallels ?? []).flat()),
        Float64Array.from((x.draft?.pts ?? []).flat()),
        Float64Array.from(x.draft?.bulges ?? []),
        x.grid?.[0] ?? 0,
        x.grid?.[1] ?? 0,
      );
      if (!c.expect) {
        expect(r.length, c.name).toBe(0);
        continue;
      }
      expect(r.length, c.name).toBe(4);
      const kind = file.kinds[r[0]];
      expect(kind, c.name).toBe(c.expect.kind);
      expect(r[3], c.name).toBe(c.expect.id);
      const tol = kind === 'grid' ? 0 : 1e-9;
      expect(close(r[1], c.expect.point[0], tol) && close(r[2], c.expect.point[1], tol), `${c.name}: ${r[1]}, ${r[2]}`).toBe(true);
    }
  });

  it('acquires extensions and directions as the reference does', () => {
    for (const c of file.extensionsAt) {
      const got = readExtensions(store(c.entities).extensionsAt(c.id, c.at[0], c.at[1]));
      const want: Extension[] = readExtensions(c.expect);
      expect(got.length, c.name).toBe(want.length);
      // Back to records: compared number by number.
      const [g, w] = [extensionRecords(got), extensionRecords(want)];
      g.forEach((v, i) => expect(close(v, w[i], 1e-12), `${c.name} [${i}]: ${v} ≠ ${w[i]}`).toBe(true));
    }
    for (const c of file.directionAt) {
      const u = store(c.entities).directionAt(c.p[0], c.p[1], c.tol);
      if (!c.expect) {
        expect(u.length, c.name).toBe(0);
        continue;
      }
      const want: Vec2 = { x: c.expect[0], y: c.expect[1] };
      expect(close(u[0], want.x, 1e-12) && close(u[1], want.y, 1e-12), c.name).toBe(true);
    }
  });
});
