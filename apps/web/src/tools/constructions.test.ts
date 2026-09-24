import { describe, expect, it } from 'vitest';
import { dist, type Vec2 } from '../model/geometry';
import { apply } from '../model/geom/affine';
import {
  alignTransform,
  alongRatio,
  calcPolar,
  centreBulge,
  chamferLine,
  circleOnDiameter,
  degDirection,
  directionAngle,
  donutRings,
  edgeArms,
  ellipseParamToward,
  ellipseRotationHalf,
  endTangent,
  filletArc,
  filletRadiusFor,
  linesCornerAt,
  midpoint,
  nearestOf,
  offsetAlong,
  polarArrayTransforms,
  pulledDistance,
  radialDimension,
  radialPoint,
  radiusBulge,
  regularPolygonRadius,
  rotationAngle,
  scaleFactor,
  textAngle,
  unitToward,
  vertexArms,
  vertexCorner,
  xlineDirection,
} from './constructions';

/** The tools' own constructions, from the Rust core (docs/adr/0008, S5). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const E = 486512.34;
const N = 4420118.9;

describe('point calculator', () => {
  it('takes the middle, a ratio of a line, and an angle in the project unit', () => {
    expect(midpoint(v(E, N), v(E + 10, N - 4))).toEqual(v(E + 5, N - 2));
    const p = alongRatio(v(0, 0), v(30, 0), 1, 3)!;
    expect(p.x).toBeCloseTo(10, 12);
    expect(alongRatio(v(1, 1), v(1, 1), 1, 3)).toBeNull();
    // 90 degrees and 100 grads are the same clockwise turn.
    const deg = calcPolar(v(E, N), v(E, N + 50), 90, 'deg', 25)!;
    const grad = calcPolar(v(E, N), v(E, N + 50), 100, 'grad', 25)!;
    expect(deg.x).toBeCloseTo(E + 25, 9);
    expect(grad.x).toBeCloseTo(deg.x, 9);
    expect(grad.y).toBeCloseTo(deg.y, 9);
  });
  it('picks the nearer solution, the first of equally near ones', () => {
    expect(nearestOf([v(0, 0), v(10, 0)], v(8, 1))).toEqual(v(10, 0));
    expect(nearestOf([v(-1, 0), v(1, 0)], v(0, 0))).toEqual(v(-1, 0));
    expect(nearestOf([], v(0, 0))).toBeNull();
  });
});

describe('drawing tool constructions', () => {
  it('turns directions into angles and back', () => {
    expect(directionAngle(v(E, N), v(E, N + 5))).toBeCloseTo(Math.PI / 2, 15);
    const d = degDirection(90);
    expect(d.x).toBeCloseTo(0, 15);
    expect(d.y).toBe(1);
    expect(unitToward(v(0, 0), v(3, 4))).toEqual(v(0.6, 0.8));
    expect(unitToward(v(E, N), v(E, N))).toBeNull();
    expect(offsetAlong(v(E, N), v(0.6, 0.8), 10)).toEqual(v(E + 6, N + 8));
  });
  it('keeps the bottom edge of a polygon with a typed radius horizontal', () => {
    for (const inscribed of [true, false]) {
      const ring = regularPolygonRadius(v(E, N), 6, 10, inscribed)!;
      expect(ring).toHaveLength(6);
      const low = [...ring].sort((a, b) => a.y - b.y).slice(0, 2);
      expect(low[0].y).toBeCloseTo(low[1].y, 8);
      // Inscribed: vertices on the circle; circumscribed: edge middles on it.
      const r = inscribed ? dist(v(E, N), ring[0]) : dist(v(E, N), midpoint(low[0], low[1]));
      expect(r).toBeCloseTo(10, 9);
    }
  });
  it('continues a line, an arc and a polyline at their ends', () => {
    expect(endTangent({ kind: 'line', a: v(0, 0), b: v(3, 4) })).toEqual({ p: v(3, 4), dir: v(0.6, 0.8) });
    expect(endTangent({ kind: 'line', a: v(1, 1), b: v(1, 1) })).toBeNull();
    const arc = endTangent({ kind: 'arc', c: v(0, 0), r: 10, a0: 0, a1: Math.PI / 2 })!;
    expect(arc.p.y).toBeCloseTo(10, 12);
    // Counter-clockwise at the top of the circle: heading west.
    expect(arc.dir.x).toBeCloseTo(-1, 15);
    const path = endTangent({ kind: 'polyline', pts: [v(0, 0), v(10, 0)], bulges: [1] })!;
    // A counter-clockwise half circle from west to east runs below the chord and ends heading north.
    expect(path.dir.x).toBeCloseTo(0, 15);
    expect(path.dir.y).toBeCloseTo(1, 15);
    expect(endTangent({ kind: 'circle', c: v(0, 0), r: 1 })).toBeNull();
  });
  it('builds circles and ellipses from their points', () => {
    expect(circleOnDiameter(v(E, N), v(E + 6, N + 8))).toEqual({ c: v(E + 3, N + 4), r: 5 });
    expect(ellipseParamToward({ c: v(0, 0), major: v(10, 0), ratio: 1, t0: 0, t1: 0 }, v(0, 5))).toBeCloseTo(Math.PI / 2, 15);
    // Seen tilted by 60°, the other axis is half the first.
    expect(ellipseRotationHalf(v(0, 0), v(20, 0), false, 60)).toBeCloseTo(5, 12);
    expect(ellipseRotationHalf(v(0, 0), v(20, 0), true, 60)).toBeCloseTo(10, 12);
  });
  it('gives construction lines their direction', () => {
    const b = xlineDirection('bisect', [v(0, 0), v(10, 0)], v(0, 10), 0)!;
    expect(b.x).toBeCloseTo(Math.SQRT1_2, 15);
    expect(b.y).toBeCloseTo(Math.SQRT1_2, 15);
    // Opposite arms: the bisector is square to them.
    expect(xlineDirection('bisect', [v(0, 0), v(10, 0)], v(-5, 0), 0)).toEqual(v(-0, 1));
    expect(xlineDirection('bisect', [v(0, 0)], v(1, 1), 0)).toBeNull();
    expect(xlineDirection('vertical', [], v(1, 1), 0)).toEqual(v(0, 1));
    expect(xlineDirection('point', [], v(1, 1), 0)).toBeNull();
  });
  it('shapes polyline arc segments by radius and by centre', () => {
    // A chord as long as the diameter is a half circle (bulge 1); the side follows the turn.
    expect(radiusBulge(v(0, 0), v(10, 0), 5, null)).toBeCloseTo(1, 12);
    expect(radiusBulge(v(0, 0), v(10, -3), 8, v(1, 0))!).toBeLessThan(0);
    expect(radiusBulge(v(0, 0), v(10, 3), 8, v(1, 0))!).toBeGreaterThan(0);
    expect(radiusBulge(v(0, 0), v(10, 0), 4, null)).toBeNull();
    expect(centreBulge(v(0, 0), v(10, 0), v(0, 10))).toBeCloseTo(Math.tan(Math.PI / 8), 15);
    expect(centreBulge(v(0, 0), v(10, 0), v(10, 0))).toBeNull();
    expect(radialPoint(v(E, N), 5, v(E + 30, N + 40))).toEqual(v(E + 3, N + 4));
    expect(radialPoint(v(E, N), 5, v(E, N))).toBeNull();
  });
  it('keeps text readable and makes donut rings', () => {
    expect(textAngle(v(0, 0), v(-10, 0))).toBeCloseTo(0, 12);
    expect(textAngle(v(0, 0), v(0, 10))).toBe(90);
    expect(textAngle(v(0, 0), v(0, -10))).toBe(90);
    const solid = donutRings(v(E, N), 0, 2);
    expect(solid.ring).toHaveLength(96);
    expect(solid.holes).toBeUndefined();
    const ring = donutRings(v(E, N), 1, 2);
    expect(dist(v(E, N), ring.holes![0][10])).toBeCloseTo(0.5, 9);
  });
});

describe('modify tool constructions', () => {
  it('rotates by a shown direction less the reference, scales by lengths', () => {
    expect(rotationAngle(v(E, N), v(E, N + 5), Math.PI / 2)).toBeCloseTo(0, 15);
    expect(scaleFactor(v(0, 0), v(3, 4), 2.5)).toBe(2);
  });
  it('shares a full turn out and puts the last copy of a partial fill on the end angle', () => {
    const full = polarArrayTransforms(v(0, 0), 4, 360, true, v(0, 0));
    expect(full).toHaveLength(3);
    expect(apply(full[0], v(10, 0)).y).toBeCloseTo(10, 12);
    const half = polarArrayTransforms(v(0, 0), 3, 180, true, v(0, 0));
    expect(apply(half[1], v(10, 0)).x).toBeCloseTo(-10, 12);
  });
  it('moves non-rotating polar copies by their offset from the centre, the same in a TM zone', () => {
    // Rotating TM-size coordinates and taking them apart again left the copies' places to the
    // last bits of sin and cos, magnified by 4.4 million (docs/adr/0008, S5).
    const near = polarArrayTransforms(v(0, 0), 11, 360, false, v(10.5, 3.25));
    const tm = polarArrayTransforms(v(486000, 4420000), 11, 360, false, v(486010.5, 4420003.25));
    expect(tm).toEqual(near);
  });
  it('aligns a source pair onto a destination pair, scaling on request', () => {
    expect(alignTransform([], false)).toBeNull();
    expect(alignTransform([v(E, N), v(E + 5, N + 2)], false)).toEqual([1, 0, 0, 1, 5, 2]);
    const pts = [v(0, 0), v(5, 2), v(10, 0), v(5, 22)];
    const m = alignTransform(pts, true)!;
    const s2 = apply(m, pts[2]);
    expect(s2.x).toBeCloseTo(5, 12);
    expect(s2.y).toBeCloseTo(22, 12);
    // Without scaling the source keeps its length along the destination direction.
    const k = apply(alignTransform(pts, false)!, pts[2]);
    expect(k.y).toBeCloseTo(12, 12);
    expect(alignTransform([v(0, 0), v(5, 2), v(0, 0), v(5, 22)], true)).toBeNull();
  });
});

describe('corner and dimension constructions', () => {
  const square = { at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 };

  it('finds a path corner between two straight segments', () => {
    const c = vertexCorner(v(E, N), v(E + 10, N), v(E + 10, N + 4), 0, 0)!;
    expect(c.at).toEqual(v(E + 10, N));
    expect(c.u1).toEqual(v(-1, 0));
    expect(c.u2).toEqual(v(0, 1));
    expect(c.reach).toBe(4);
    expect(c.phi).toBeCloseTo(Math.PI / 2, 15);
    // Beside an arc, on a straight run, turning back.
    expect(vertexCorner(v(0, 0), v(10, 0), v(10, 10), 0.5, 0)).toBeNull();
    expect(vertexCorner(v(0, 0), v(10, 0), v(20, 0), 0, 0)).toBeNull();
    expect(vertexCorner(v(0, 0), v(10, 0), v(0, 0), 0, 0)).toBeNull();
  });
  it('finds the corner of two lines, each keeping the side it was picked on', () => {
    const c = linesCornerAt(v(0, 0), v(10, 0), v(5, 0), v(12, 2), v(12, 20), v(12, 10))!;
    expect(c.at.x).toBeCloseTo(12, 12);
    // Picked west of the corner: the first line keeps its western side.
    expect(c.u1.x).toBe(-1);
    expect(c.u1.y).toBeCloseTo(0, 15);
    expect(c.u2).toEqual(v(0, 1));
    expect(c.reach).toBeCloseTo(12, 12);
    expect(linesCornerAt(v(0, 0), v(10, 0), v(5, 0), v(0, 1), v(10, 1), v(5, 1))).toBeNull();
  });
  it('rounds a pulled size to a step that suits the zoom and stops at the shorter side', () => {
    expect(pulledDistance(square, v(3.14159, 0.5), 0.037)).toBeCloseTo(3.14, 12);
    expect(pulledDistance(square, v(3.14159, 0), 0.37)).toBeCloseTo(3.1, 12);
    expect(pulledDistance(square, v(30, 1), 0.037)).toBe(10);
    expect(pulledDistance(square, null, 0.037)).toBe(0);
  });
  it('makes the fillet arc tangent to both sides, and the chamfer cut', () => {
    expect(filletRadiusFor(4, Math.PI / 2)).toBeCloseTo(4, 15);
    const arc = filletArc({ ...square, at: v(E, N) }, 2)!;
    expect(arc.r).toBe(2);
    expect(arc.c.x - E).toBeCloseTo(2, 9);
    expect(arc.c.y - N).toBeCloseTo(2, 9);
    expect(filletArc(square, 0)).toBeNull();
    expect(chamferLine(square, 2, 3)).toEqual({ a: v(2, 0), b: v(0, 3) });
    expect(chamferLine(square, 0, 3)).toBeNull();
  });
  it('places angular arms in the sector of the arc, and radial dimensions on the circle', () => {
    expect(vertexArms(v(0, 0), v(10, 0), v(0, 10), v(3, 3))).toEqual({ c: v(0, 0), a: v(10, 0), b: v(0, 10) });
    expect(vertexArms(v(0, 0), v(10, 0), v(0, 10), v(-3, -3))).toEqual({ c: v(0, 0), a: v(0, 10), b: v(10, 0) });
    const arms = edgeArms({ a: v(0, 0), b: v(10, 0), at: v(6, 0) }, { a: v(0, 0), b: v(0, 10), at: v(0, 4) }, v(3, 3))!;
    expect(arms.a).toEqual(v(6, 0));
    expect(arms.b).toEqual(v(0, 4));
    expect(edgeArms({ a: v(0, 0), b: v(10, 0), at: v(6, 0) }, { a: v(0, 1), b: v(10, 1), at: v(4, 1) }, v(3, 3))).toBeNull();
    expect(radialDimension(v(E, N), 5, v(E + 30, N + 40))).toEqual({ b: v(E + 3, N + 4), offset: 45 });
    expect(radialDimension(v(0, 0), 5, v(1, 1))).toEqual({ b: expect.any(Object), offset: 0 });
    expect(radialDimension(v(0, 0), 5, v(0, 0)).b).toEqual(v(5, 0));
  });
});
