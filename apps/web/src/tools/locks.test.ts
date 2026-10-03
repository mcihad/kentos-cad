import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../model/geometry';
import { constrainLocked, lockDeflected, lockDirection, lockPoint, parseLockText, squareCorner } from './locks';

/**
 * The digitizing locks (docs/adr/0166) through the WASM core and the web's lock text reader, against the independent
 * reference in fixtures/locks/v1/cases.json (scripts/fixtures/lock_cases.py: exact fractions, the angles with 50-digit
 * mpmath, no KentOS code), the cases the core runs natively in crates/shared/geometry-core/tests/locks.rs: points
 * within 1e-8 m, unit directions within 1e-14, none where the reference has none, lock text exactly.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Pt = [number, number] | null;
interface File {
  format: string;
  lockPoint: { name: string; o: Pt; c: Pt; length: number | null; u: Pt; both: boolean; expect: Pt }[];
  constrainLocked: {
    name: string;
    from: Pt;
    world: Pt;
    exact: boolean;
    ortho: boolean;
    polarStep: number | null;
    tol: number;
    length: number | null;
    u: Pt;
    both: boolean;
    expect: { point: Pt; tracking: { origin: Pt; angle: number } | null } | null;
  }[];
  direction: { name: string; angle: number; fromNorth: boolean; grads: boolean; expect: Pt }[];
  deflected: { name: string; prev: Pt; from: Pt; angle: number; fromNorth: boolean; grads: boolean; expect: Pt }[];
  squareCorner: { name: string; first: Pt; second: Pt; prev: Pt; last: Pt; expect: Pt }[];
  lockText: { text: string; expect: number | null }[];
}

const v = (p: Pt): Vec2 | null => (p ? { x: p[0], y: p[1] } : null);
const P = (p: Pt): Vec2 => v(p)!;

function same(name: string, got: Vec2 | null, expected: Pt, tol: number): void {
  if (expected === null) return void expect(got, name).toBeNull();
  expect(got, name).not.toBeNull();
  expect(Math.abs(got!.x - expected[0]), `${name}: x`).toBeLessThanOrEqual(tol);
  expect(Math.abs(got!.y - expected[1]), `${name}: y`).toBeLessThanOrEqual(tol);
}

describe('Sayısallaştırma kilitleri (docs/adr/0166)', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/locks/v1/cases.json', import.meta.url), 'utf8')) as File;

  it('reads the reference', () => expect(file.format).toBe('kentos.locks'));

  it('locks the point', () => {
    for (const c of file.lockPoint) same(c.name, lockPoint(P(c.o), P(c.c), c.length, v(c.u), c.both), c.expect, 1e-8);
  });

  it('constrains the cursor with the locks', () => {
    for (const c of file.constrainLocked) {
      const got = constrainLocked(v(c.from), P(c.world), c.exact, c.ortho, c.polarStep, c.tol, c.length, v(c.u), c.both);
      same(c.name, got?.point ?? null, c.expect?.point ?? null, 1e-8);
      if (!c.expect?.tracking) expect(got?.tracking ?? null, c.name).toBeNull();
      else {
        same(c.name, got!.tracking!.origin, c.expect.tracking.origin, 1e-8);
        expect(Math.abs(got!.tracking!.angle - c.expect.tracking.angle), c.name).toBeLessThanOrEqual(1e-9);
      }
    }
  });

  it('turns angles and deflections into directions', () => {
    for (const c of file.direction) same(c.name, lockDirection(c.angle, c.fromNorth, c.grads), c.expect, 1e-14);
    for (const c of file.deflected) same(c.name, lockDeflected(P(c.prev), P(c.from), c.angle, c.fromNorth, c.grads), c.expect, 1e-14);
  });

  it('finds Dik kapat’s corner', () => {
    for (const c of file.squareCorner) same(c.name, squareCorner(P(c.first), P(c.second), P(c.prev), P(c.last)), c.expect, 1e-8);
  });

  it('reads lock text as the core does', () => {
    for (const c of file.lockText) expect(parseLockText(c.text), JSON.stringify(c.text)).toBe(c.expect);
  });
});
