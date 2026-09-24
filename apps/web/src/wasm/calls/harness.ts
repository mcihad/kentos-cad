import type { Vec2 } from '../../model/geometry';

/**
 * Test support for the Rust core's calls (docs/adr/0008). A call set lists
 * named edge cases (from the unit tests: parallel, coincident, zero length,
 * 0/2π, TM coordinates) and a seeded generator for random ones. The
 * recorder (scripts/fixtures/record-calls.test.ts) freezes a set into
 * fixtures/geometry/v1/calls-*.json, which Rust (tests/calls.rs) and the
 * WASM build (src/wasm/calls.wasm.test.ts) check. The frozen answers came
 * from the TypeScript each operation was ported from, compared side by
 * side with the core in 20 000 random calls per operation before that
 * TypeScript was deleted (S3c).
 */

export interface Tolerance {
  abs: number;
  rel: number;
}

/** The golden tolerance for projected metres (fixtures/geometry/v1). */
export const TOLERANCE: Tolerance = { abs: 1e-9, rel: 1e-14 };

export interface Call {
  name: string;
  /** The operation's name in the core's call table (the TypeScript function's it was ported from). */
  fn: string;
  args: unknown[];
}

export interface CallSet {
  /** Fixture file name under fixtures/geometry/v1. */
  file: string;
  /**
   * A wider bound for an operation whose result is sensitive to the last bit
   * of sin/cos (V8 and libm differ there, docs/adr/0008), with the reason;
   * the recorder writes it into every case of that operation.
   */
  tolerance?: Record<string, Tolerance & { why: string }>;
  named: Call[];
  /** `n` random calls per operation. */
  random(g: Gen, n: number): Call[];
}

export interface CallFile {
  format: 'kentos.geometry-calls';
  version: 1;
  tolerance: Tolerance;
  crs: { kind: 'projected'; unit: 'metre'; note: string };
  cases: (Call & { expect: unknown; tol?: Tolerance & { why: string } })[];
}

/** Deterministic random numbers (mulberry32): the same seed gives the same calls everywhere. */
export class Gen {
  private s: number;
  /** Where this call's points lie: near the origin or in a TM zone (see `frame`). */
  private origin: Vec2 = { x: 0, y: 0 };
  constructor(seed: number) {
    this.s = seed >>> 0;
  }
  /**
   * Starts a call: its points share one place, near the origin or in a
   * TUREF TM zone. A shape spanning 4 400 km from one to the other is not a
   * drawing, and would only magnify last-bit differences (docs/adr/0008).
   */
  frame(): void {
    this.origin = this.chance(0.3) ? { x: 486000, y: 4420000 } : { x: 0, y: 0 };
  }
  next(): number {
    this.s = (this.s + 0x6d2b79f5) >>> 0;
    let t = this.s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }
  num(lo: number, hi: number): number {
    return lo + (hi - lo) * this.next();
  }
  int(lo: number, hi: number): number {
    return Math.floor(this.num(lo, hi + 1));
  }
  pick<T>(items: readonly T[]): T {
    return items[Math.floor(this.next() * items.length)];
  }
  chance(p: number): boolean {
    return this.next() < p;
  }
  /** A point of this call's frame. */
  pt(scale = 100): Vec2 {
    return { x: this.origin.x + this.num(-scale, scale), y: this.origin.y + this.num(-scale, scale) };
  }
  /** A vector (a direction or an axis), never a position. */
  vec(scale = 100): Vec2 {
    return { x: this.num(-scale, scale), y: this.num(-scale, scale) };
  }
  /** A point of this call's frame snapped to a coarse grid, so coincidences and parallels happen. */
  gridPt(step = 1, cells = 6): Vec2 {
    return { x: this.origin.x + this.int(-cells, cells) * step, y: this.origin.y + this.int(-cells, cells) * step };
  }
  pts(n: number, scale = 100): Vec2[] {
    const base = this.pt(scale);
    return Array.from({ length: n }, () => ({ x: base.x + this.num(-scale, scale), y: base.y + this.num(-scale, scale) }));
  }
  /** A simple (star-shaped) ring around a centre, counter-clockwise unless `cw`. */
  ring(n: number, r = 50, cw = false): Vec2[] {
    const c = this.pt(1000);
    const out = Array.from({ length: n }, (_, i) => {
      const a = ((cw ? -1 : 1) * 2 * Math.PI * (i + this.num(0.1, 0.9))) / n;
      const d = r * this.num(0.4, 1);
      return { x: c.x + d * Math.cos(a), y: c.y + d * Math.sin(a) };
    });
    return out;
  }
}

const SPECIAL = (v: number) => (Number.isNaN(v) ? '#NaN' : v === Infinity ? '#Inf' : v === -Infinity ? '#-Inf' : v);

/**
 * A value as JSON keeps it: undefined properties dropped, NaN and ±∞ as the
 * core writes them ("#NaN", "#Inf", "#-Inf"), undefined itself as null.
 */
export function toJson(v: unknown): unknown {
  if (v === undefined) return null;
  const text = JSON.stringify(v, (_k, x: unknown) => (typeof x === 'number' && !Number.isFinite(x) ? SPECIAL(x) : x));
  return text === undefined ? null : JSON.parse(text);
}

/** Numbers within the tolerance; strings, booleans, nulls, array lengths and object keys exactly. */
export function sameResult(actual: unknown, expected: unknown, tol: Tolerance, path = ''): string | null {
  if (typeof expected === 'number' && typeof actual === 'number') {
    const d = Math.abs(actual - expected);
    return d <= tol.abs + tol.rel * Math.max(Math.abs(actual), Math.abs(expected)) ? null : `${path}: ${actual} ≠ ${expected} (fark ${d})`;
  }
  if (Array.isArray(expected) && Array.isArray(actual)) {
    if (actual.length !== expected.length) return `${path}: ${actual.length} öğe ≠ ${expected.length} öğe`;
    for (let i = 0; i < expected.length; i++) {
      const r = sameResult(actual[i], expected[i], tol, `${path}[${i}]`);
      if (r) return r;
    }
    return null;
  }
  if (expected && typeof expected === 'object' && actual && typeof actual === 'object' && !Array.isArray(expected) && !Array.isArray(actual)) {
    const a = actual as Record<string, unknown>;
    const e = expected as Record<string, unknown>;
    const keys = new Set([...Object.keys(a), ...Object.keys(e)]);
    for (const k of keys) {
      if (!(k in a)) return `${path}.${k}: eksik`;
      if (!(k in e)) return `${path}.${k}: fazla (${JSON.stringify(a[k])})`;
      const r = sameResult(a[k], e[k], tol, `${path}.${k}`);
      if (r) return r;
    }
    return null;
  }
  return actual === expected ? null : `${path}: ${JSON.stringify(actual)} ≠ ${JSON.stringify(expected)}`;
}

/** The calls of a set: its named cases, then `n` random ones per operation from a fixed seed. */
export function callsOf(set: CallSet, n: number, seed = 20260924): Call[] {
  return [...set.named, ...set.random(new Gen(seed), n)];
}

/** `n` random calls of one operation, named by their index; each call gets its own frame. */
export function repeat(g: Gen, fn: string, n: number, args: () => unknown[]): Call[] {
  return Array.from({ length: n }, (_, i) => {
    g.frame();
    return { name: `rastgele ${i + 1}`, fn, args: args() };
  });
}
