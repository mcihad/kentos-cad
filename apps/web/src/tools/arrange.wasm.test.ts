import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/arrange/v1/cases.json?raw';
import type { Bounds } from '../model/geometry';
import { arrangeAt, arrangeMoves, arrangeUnion } from '../model/ops/arrange';

/**
 * Hizala ve dağıt (docs/adr/0194 §2) through the WASM core, on the independent reference's cases
 * (fixtures/arrange/v1/cases.json, scripts/fixtures/arrange_cases.py): each box's displacement bit for bit, a
 * reference box's side or middle, the selection's box. The core runs the same file natively
 * (crates/shared/geometry-core/tests/all/arrange.rs).
 */
const f = JSON.parse(text);
const bounds = (b: number[]): Bounds => ({ minX: b[0], minY: b[1], maxX: b[2], maxY: b[3] });
/** Bit for bit, but a zero is a zero. */
const same = (got: number, want: number, what: string) => expect(got === want, `${what}: ${got} ≠ ${want}`).toBe(true);

describe('Hizala ve dağıt (fixtures/arrange/v1)', () => {
  it('moves each box as the reference says', () => {
    for (const c of f.cases) {
      const got = arrangeMoves(c.boxes.map(bounds), c.mode, c.at);
      expect(got.length, c.name).toBe(c.moves.length);
      got.forEach((d, i) => {
        same(d.x, c.moves[i][0], `${c.name} #${i} east`);
        same(d.y, c.moves[i][1], `${c.name} #${i} north`);
      });
    }
  });

  it('gives a reference box its side or middle, and the selection its box', () => {
    for (const r of f.references) same(arrangeAt(bounds(r.box), r.mode), r.at, `${r.mode} ${r.box}`);
    for (const u of f.unions) expect(arrangeUnion(u.boxes.map(bounds))).toEqual(bounds(u.box));
    expect(arrangeUnion([])).toBeNull();
  });
});
