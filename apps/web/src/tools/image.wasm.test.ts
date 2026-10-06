import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/image/v1/cases.json?raw';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { imageClip, imagePlaced } from './constructions';

/**
 * Resim nesnesi (docs/adr/0192 §5) through the WASM core, on the independent reference's cases
 * (fixtures/image/v1/cases.json, scripts/fixtures/image_cases.py): Resim ekle's frame from two points, Resmi kırp's
 * boundary in the picture's own fractions. The core runs the same file natively
 * (crates/shared/geometry-core/tests/all/image.rs).
 */
const f = JSON.parse(text);

function near(got: number, want: number, tol: number, what: string): void {
  expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(tol);
}

describe('Resim nesnesi (fixtures/image/v1)', () => {
  it('places a picture by its corner and a second point', () => {
    for (const c of f.placements) {
      const got = imagePlaced(c.p as Vec2, c.q as Vec2, c.aspect);
      if (c.problem) {
        expect(got.problem, c.name).toBe(c.problem);
        continue;
      }
      for (const key of ['width', 'height', 'rotation'] as const) near(got.placed![key], c.placed[key], 1e-12, `${c.name}: ${key}`);
    }
  });

  it('cuts a boundary to the picture in its own fractions', () => {
    for (const c of f.clips) {
      const got = imageClip(c.shape as Entity, c.world as Vec2[]);
      if (c.problem) {
        expect(got.problem, c.name).toBe(c.problem);
        continue;
      }
      const tol = c.tolerance ?? 1e-15;
      expect(got.clip?.length, c.name).toBe(c.clip.length);
      got.clip!.forEach((p, i) => {
        near(p.x, c.clip[i].x, tol, c.name);
        near(p.y, c.clip[i].y, tol, c.name);
      });
    }
  });
});
