import type { Entity } from '../../model/entities';
import { ObjectStore, type CornerWalk } from '../../processing/geometry';
import { measuredAt } from '../../model/expression/expressionLib';
import { DrawnReader } from '../../style/geometry';
import { CoreStore } from '../core';
import type { Tolerance } from './harness';

/**
 * The store fixtures (fixtures/geometry/v1/store-*.json): a fixed scene
 * (objects, layer table, default label rules) and the answers of the
 * geometry store's queries on it, by name. The WASM test
 * (src/wasm/store.wasm.test.ts) and the recorders
 * (scripts/fixtures/record-store*.test.ts) ask the store the same way;
 * Rust reads the same files natively (crates/shared/geometry-core/tests/store.rs).
 */

export interface StoreCase {
  name: string;
  op: string;
  args: unknown[];
  expect: unknown;
}

export interface StoreFile {
  format: 'kentos.geometry-store';
  version: 1;
  tolerance: Tolerance;
  crs?: { kind: 'projected'; unit: 'metre'; note: string };
  layers: unknown[];
  labelDefaults: unknown;
  entities: { id: number }[];
  cases: StoreCase[];
}

const KINDS = ['endpoint', 'midpoint', 'center', 'node', 'quadrant', 'intersection', 'perpendicular', 'tangent', 'nearest'];

type Rect = { minX: number; minY: number; maxX: number; maxY: number };

function edges(f: Float64Array): unknown[] {
  const out: unknown[] = [];
  for (let i = 0; i < f.length; ) {
    if (f[i] === 0) {
      out.push({ kind: 'seg', a: { x: f[i + 1], y: f[i + 2] }, b: { x: f[i + 3], y: f[i + 4] } });
      i += 5;
    } else {
      out.push({ kind: 'arc', c: { x: f[i + 1], y: f[i + 2] }, r: f[i + 3], a0: f[i + 4], sweep: f[i + 5] });
      i += 6;
    }
  }
  return out;
}

/** The scene of a store file in the geometry store (and, for processing's queries, in a run's own store). */
export class StoreScene {
  readonly store = new CoreStore();
  readonly objects: ObjectStore;
  private readonly byId: Map<number, unknown>;

  constructor(file: Pick<StoreFile, 'entities' | 'layers' | 'labelDefaults'>) {
    this.store.put(JSON.stringify(file.entities));
    this.store.setLayers(JSON.stringify(file.layers));
    this.store.setLabelDefaults(JSON.stringify(file.labelDefaults));
    this.byId = new Map(file.entities.map((e) => [e.id, e]));
    this.objects = new ObjectStore(file.entities as unknown as Entity[]);
  }

  /** The answer of one query, in the file's form. */
  answer(op: string, a: unknown[]): unknown {
    const s = this.store;
    const n = (i: number) => a[i] as number;
    const r = a[0] as Rect;
    const except = (a[1] as number | null) ?? undefined;
    const ids = (i: number) => Float64Array.from(a[i] as number[]);
    const tool = (f: typeof s.trimPreview) => {
      const p = a[1] as { x: number; y: number };
      const v = a[2] as Rect;
      const chosen = a[3] as number[] | null;
      return f.call(s, JSON.stringify(this.byId.get(n(0))), p.x, p.y, n(0), chosen && Float64Array.from(chosen), v.minX, v.minY, v.maxX, v.maxY);
    };
    switch (op) {
      case 'hit':
        return s.hit(n(0), n(1), n(2)) ?? null;
      case 'hitEdge': {
        const h = s.hitEdge(n(0), n(1), n(2));
        return h.length ? h[0] : null;
      }
      case 'snap': {
        let mask = 0;
        for (const k of a[3] as string[]) mask |= 1 << KINDS.indexOf(k);
        const h = s.snap(n(0), n(1), n(2), mask, a[4] as { x: number; y: number } | null);
        return h.length ? { kind: KINDS[h[0]], point: { x: h[1], y: h[2] }, entityId: h[3] } : null;
      }
      case 'enclosing': {
        const e = s.enclosing(n(0), n(1));
        if (!e.length) return null;
        const ring = [];
        for (let i = 1; i + 1 < e.length; i += 2) ring.push({ x: e[i], y: e[i + 1] });
        return { id: e[0], ring };
      }
      case 'inRect':
        return Array.from(s.inRect(r.minX, r.minY, r.maxX, r.maxY, a[1] as boolean));
      case 'overlapping':
        return Array.from(s.overlapping(r.minX, r.minY, r.maxX, r.maxY, except));
      case 'edgesIn':
        return edges(s.edgesIn(r.minX, r.minY, r.maxX, r.maxY, except));
      case 'labels':
        return Array.from(s.labels(r.minX, r.minY, r.maxX, r.maxY, n(1), a[2] as number | null));
      case 'grips':
        return Array.from(s.grips(ids(0)));
      case 'trim':
        return tool(s.trimPreview);
      case 'extend':
        return tool(s.extendPreview);
      case 'ghosts':
        return Array.from(s.transformOutlines(ids(0), Float64Array.from((a[1] as number[][]).flat()), n(2)));
      case 'stretchGhosts': {
        const w = a[1] as Rect;
        return Array.from(s.stretchOutlines(ids(0), w.minX, w.minY, w.maxX, w.maxY, n(2), n(3)));
      }
      case 'measure':
        return Array.from(s.measure(ids(0)));
      case 'drawn': {
        const list = (a[0] as number[]).map((id) => this.byId.get(id) as Entity);
        const reader = new DrawnReader(s.drawn(ids(0), a[1] as boolean, a[2] as Rect | null));
        return list.map((e) => reader.read(e));
      }
      case 'measures': {
        const values = s.measures(ids(0));
        return (a[0] as number[]).map((_, i) => measuredAt(values, i));
      }
      case 'inBox':
        return this.objects.inBox(r);
      case 'numberCorners':
        return this.objects.numberCorners(a[0] as number[], a[1] as CornerWalk, a[2] as { x: number; y: number }[]);
      case 'edgeLengths':
        return this.objects.edgeLengths(a[0] as number[], n(1), n(2), a[3] as 'outside' | 'inside', a[4] as boolean);
    }
    throw new Error(`bilinmeyen sorgu: ${op}`);
  }

  dispose(): void {
    this.store.dispose();
    this.objects.dispose();
  }
}

/** A store file as the recorders write it: one object and one case per line (small, readable diffs). */
export function storeFileText(file: StoreFile): string {
  const { entities, cases, ...head } = file;
  return `${JSON.stringify(head, null, 2).slice(0, -2)},\n  "entities": [\n${entities.map((e) => `    ${JSON.stringify(e)}`).join(',\n')}\n  ],\n  "cases": [\n${cases.map((c) => `    ${JSON.stringify(c)}`).join(',\n')}\n  ]\n}\n`;
}
