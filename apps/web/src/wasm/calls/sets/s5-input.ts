import type { Vec2 } from '../../../model/geometry';
import { repeat, type CallSet, type Gen } from '../harness';
import { trackAngles, type TrackHit } from '../../../viewport/objectTracking';

/** S5: typed point input, the ortho and polar cursor, object tracking, the point calculator's arithmetic (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const E = 486512.34;
const N = 4420118.9;
const ORTHO = [0, 90, 180, 270];
const STEPS = [15, 30, 45, 90, 7.5, 22.5, 5, 10, 11.25, 1];

/** Typed angles in degrees: the axes, full turns and signs, and random ones. */
function degrees(g: Gen): number {
  return g.chance(0.3) ? g.pick([0, 90, 180, 270, 360, -90, 45, 30, 1e-10, -720]) : g.num(-720, 720);
}

/** A point `d` away from `o` at `deg` degrees (the generator's own trigonometry: both sides get the same numbers). */
function at(o: Vec2, deg: number, d: number): Vec2 {
  const r = (deg * Math.PI) / 180;
  return { x: o.x + d * Math.cos(r), y: o.y + d * Math.sin(r) };
}

/** A cursor near a polar ray from `from`: a multiple of the step, a degree or two off. */
function nearRay(g: Gen, from: Vec2, step: number): Vec2 {
  return at(from, g.int(-24, 24) * step + g.num(-2, 2), g.num(0.5, 80));
}

/**
 * Acquired tracking points and a cursor near one of their alignments or a
 * crossing. Polar steps take random points: three lines through one point
 * would make the pick among equally near crossings depend on the last bit
 * of sin and cos. Orthogonal alignments are exact, so grid points are fine.
 */
function trackCase(g: Gen): [Vec2, Vec2[], Vec2 | null, number[], number] {
  const polar = g.chance(0.4) ? g.pick(STEPS) : null;
  const angles = trackAngles(polar);
  const n = g.int(1, 3);
  const acquired = Array.from({ length: n }, () => (polar === null && g.chance(0.5) ? g.gridPt(5, 4) : g.pt(60)));
  const from = g.chance(0.4) ? (polar === null && g.chance(0.5) ? g.gridPt(5, 4) : g.pt(60)) : null;
  const tol = g.pick([0.5, 1, 3, 10]);
  const a = g.pick(acquired);
  const b = g.pick(from ? [...acquired, from] : acquired);
  const jitter = () => g.num(-tol, tol);
  const p = g.pick([
    () => at(a, g.pick(angles), g.num(1, 90)),
    () => ({ x: a.x + jitter(), y: b.y + jitter() }),
    () => ({ x: b.x + jitter(), y: a.y + jitter() }),
    () => g.pt(80),
  ])();
  const q = g.chance(0.7) ? { x: p.x + jitter() / 2, y: p.y + jitter() / 2 } : p;
  return [q, acquired, from, angles, tol];
}

function hit(g: Gen): TrackHit {
  const line = () => ({ origin: g.pt(), angle: g.chance(0.5) ? g.pick([0, 90, 180, 270, 45, 135]) : g.num(0, 360) });
  return { point: g.pt(), lines: g.chance(0.8) ? [line()] : [line(), line()] };
}

export const S5_INPUT: CallSet = {
  file: 'calls-s5-input.json',
  named: [
    { name: 'göreli, TM', fn: 'relativePoint', args: [v(E, N), 12.5, -3.25] },
    { name: 'göreli, sıfır', fn: 'relativePoint', args: [v(100, 200), 0, 0] },
    { name: 'kutupsal @10<90', fn: 'polarOffset', args: [v(100, 200), 10, 90] },
    { name: 'kutupsal, doğu', fn: 'polarOffset', args: [v(E, N), 25, 0] },
    { name: 'kutupsal, tam tur', fn: 'polarOffset', args: [v(E, N), 25, 360] },
    { name: 'kutupsal, eksi açı', fn: 'polarOffset', args: [v(0, 0), 10, -45] },
    { name: 'kutupsal, sıfır mesafe', fn: 'polarOffset', args: [v(E, N), 0, 33] },
    { name: 'kutupsal, eksi mesafe', fn: 'polarOffset', args: [v(E, N), -7.5, 120] },
    { name: 'imleç yönünde', fn: 'towardPoint', args: [v(100, 200), v(110, 200), 5] },
    { name: 'imleç yönünde, TM', fn: 'towardPoint', args: [v(E, N), v(E + 30, N + 40), 12.5] },
    { name: 'imleç son noktada', fn: 'towardPoint', args: [v(E, N), v(E, N), 5] },
    { name: 'imleç tam 1e-9 uzakta', fn: 'towardPoint', args: [v(0, 0), v(1e-9, 0), 5] },
    { name: 'imleç yönünde, eksi', fn: 'towardPoint', args: [v(0, 0), v(3, 4), -10] },
    { name: 'son nokta yok', fn: 'constrainCursor', args: [null, v(5, 7), false, true, 15, 1] },
    { name: 'kenet varken olduğu gibi', fn: 'constrainCursor', args: [v(0, 0), v(5, 7), true, true, 15, 1] },
    { name: 'orto: yatay', fn: 'constrainCursor', args: [v(E, N), v(E + 10, N + 3), false, true, null, 1] },
    { name: 'orto: düşey', fn: 'constrainCursor', args: [v(E, N), v(E + 3, N - 10), false, true, 15, 1] },
    { name: 'orto: eşit', fn: 'constrainCursor', args: [v(0, 0), v(4, 4), false, true, null, 1] },
    { name: 'kutupsal kapalı', fn: 'constrainCursor', args: [v(0, 0), v(10, 0.2), false, false, null, 1] },
    { name: 'kutupsal 15°: kilit', fn: 'constrainCursor', args: [v(0, 0), v(10, 0.2), false, false, 15, 1] },
    { name: 'kutupsal 45°: kilit, TM', fn: 'constrainCursor', args: [v(E, N), v(E + 10, N + 10.3), false, false, 45, 0.5] },
    { name: 'kutupsal: uzak', fn: 'constrainCursor', args: [v(0, 0), v(10, 3), false, false, 90, 1] },
    { name: 'kutupsal: 360° → 0°', fn: 'constrainCursor', args: [v(0, 0), v(10, -0.1), false, false, 30, 1] },
    { name: 'kutupsal: eksi açı', fn: 'constrainCursor', args: [v(0, 0), v(0.1, -10), false, false, 90, 1] },
    { name: 'kutupsal: imleç başlangıçta', fn: 'constrainCursor', args: [v(3, 3), v(3, 3), false, false, 15, 1] },
    { name: 'orta nokta, TM', fn: 'midpoint', args: [v(E, N), v(E + 31.3, N - 12.9)] },
    { name: 'orta nokta, aynı', fn: 'midpoint', args: [v(1, 2), v(1, 2)] },
    { name: 'hat üzerinde 1/3', fn: 'alongRatio', args: [v(E, N), v(E + 30, N + 40), 1, 3] },
    { name: 'hat üzerinde 5/2', fn: 'alongRatio', args: [v(0, 0), v(10, 0), 5, 2] },
    { name: 'hat üzerinde, çakışık', fn: 'alongRatio', args: [v(1, 1), v(1, 1), 1, 2] },
    { name: 'hat üzerinde, sıfır payda', fn: 'alongRatio', args: [v(0, 0), v(10, 0), 1, 0] },
    { name: 'açı-mesafe, 90 derece', fn: 'calcPolar', args: [v(E, N), v(E, N + 50), 90, 'deg', 25] },
    { name: 'açı-mesafe, 100 grad', fn: 'calcPolar', args: [v(E, N), v(E, N + 50), 100, 'grad', 25] },
    { name: 'açı-mesafe, çakışık', fn: 'calcPolar', args: [v(0, 0), v(0, 0), 100, 'grad', 25] },
    { name: 'en yakın: yok', fn: 'nearestOf', args: [[], v(0, 0)] },
    { name: 'en yakın: eşitte ilki', fn: 'nearestOf', args: [[v(-1, 0), v(1, 0)], v(0, 0)] },
    { name: 'en yakın: ikinci', fn: 'nearestOf', args: [[v(E, N), v(E + 10, N)], v(E + 7, N + 1)] },
    { name: 'hizalar: kutupsal yok', fn: 'trackAngles', args: [null] },
    { name: 'hizalar: 45°', fn: 'trackAngles', args: [45] },
    { name: 'hizalar: 7°', fn: 'trackAngles', args: [7] },
    { name: 'hizalar: 7.5°', fn: 'trackAngles', args: [7.5] },
    { name: 'hizalar: 1°', fn: 'trackAngles', args: [1] },
    { name: 'hizalar: 0°', fn: 'trackAngles', args: [0] },
    { name: 'hizalar: eksi adım', fn: 'trackAngles', args: [-15] },
    { name: 'hizalar: 400°', fn: 'trackAngles', args: [400] },
    { name: 'düşeye kilit', fn: 'trackPoint', args: [v(100.3, 250), [v(100, 0)], null, ORTHO, 1] },
    { name: 'hizanın dışında', fn: 'trackPoint', args: [v(105, 250), [v(100, 0)], null, ORTHO, 1] },
    { name: 'iki hizanın kesişimi', fn: 'trackPoint', args: [v(10.4, 49.7), [v(10, 0), v(80, 50)], null, ORTHO, 1] },
    { name: 'son noktayla kesişim', fn: 'trackPoint', args: [v(0.2, 29.6), [v(50, 30)], v(0, 0), ORTHO, 1] },
    { name: 'yalnız son noktadan izlenmez', fn: 'trackPoint', args: [v(0.2, 29.6), [v(50, 90)], v(0, 0), ORTHO, 1] },
    { name: 'kutupsal hizalar', fn: 'trackPoint', args: [v(10.2, 9.9), [v(0, 0)], null, trackAngles(45), 1] },
    { name: 'TM kesişimi', fn: 'trackPoint', args: [v(E + 0.3, N + 40.2), [v(E, N), v(E + 70, N + 40)], null, ORTHO, 1] },
    { name: 'izleme noktası yok', fn: 'trackPoint', args: [v(1, 1), [], v(0, 0), ORTHO, 1] },
    { name: 'hiza boyunca mesafe', fn: 'alongTrack', args: [{ point: v(100, 250), lines: [{ origin: v(100, 0), angle: 90 }] }, 12.5] },
    { name: 'hiza boyunca, 45°', fn: 'alongTrack', args: [{ point: v(7, 7), lines: [{ origin: v(E, N), angle: 45 }] }, 10] },
    { name: 'kesişimde mesafe yok', fn: 'alongTrack', args: [{ point: v(10, 50), lines: [{ origin: v(10, 0), angle: 90 }, { origin: v(80, 50), angle: 180 }] }, 5] },
  ],
  random: (g, n) => [
    ...repeat(g, 'relativePoint', n, () => [g.pt(), g.num(-100, 100), g.num(-100, 100)]),
    ...repeat(g, 'polarOffset', n, () => [g.pt(), g.num(-100, 100), degrees(g)]),
    ...repeat(g, 'towardPoint', n, () => {
      const last = g.pt();
      return [last, g.chance(0.1) ? last : g.pt(), g.num(-60, 60)];
    }),
    ...repeat(g, 'constrainCursor', n, () => {
      const from = g.chance(0.1) ? null : g.pt();
      const step = g.chance(0.2) ? null : g.pick(STEPS);
      const world = from && step !== null && g.chance(0.7) ? nearRay(g, from, step) : g.pt();
      return [from, world, g.chance(0.1), g.chance(0.3), step, g.pick([0.5, 1, 3, 10])];
    }),
    ...repeat(g, 'midpoint', n, () => [g.pt(), g.pt()]),
    ...repeat(g, 'alongRatio', n, () => {
      const a = g.pt();
      return [a, g.chance(0.1) ? a : g.pt(), g.chance(0.5) ? g.int(0, 5) : g.num(0, 5), g.chance(0.5) ? g.int(1, 6) : g.num(0.1, 6)];
    }),
    ...repeat(g, 'calcPolar', n, () => [g.pt(), g.pt(), g.num(-400, 400), g.pick(['deg', 'grad']), g.num(-100, 100)]),
    ...repeat(g, 'nearestOf', n, () => {
      const pts = Array.from({ length: g.int(0, 4) }, () => g.pt());
      if (pts.length && g.chance(0.2)) pts.push(pts[0]);
      return [pts, g.pt()];
    }),
    ...repeat(g, 'trackAngles', n, () => [g.chance(0.2) ? null : g.pick([...STEPS, 7, 0, -15, 360, 400, 2.5, 3])]),
    ...repeat(g, 'trackPoint', n, () => trackCase(g)),
    ...repeat(g, 'alongTrack', n, () => [hit(g), g.num(-100, 100)]),
  ],
};
