import type { EntityGeometry } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import { TAU } from '../../../model/geom/arc';
import type { EllipseGeom } from '../../../model/geom/ellipse';
import type { CornerGeom } from '../../../tools/constructions';
import { repeat, type CallSet, type Gen } from '../harness';

/** S5: the drawing, modify, corner and dimension tools' own constructions (docs/adr/0008). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const E = 486512.34;
const N = 4420118.9;
const XMODES = ['point', 'horizontal', 'vertical', 'angle', 'bisect'];

/** Angles in radians that cross 0/2π and hit exact quarters, plus random ones. */
function radians(g: Gen): number {
  return g.chance(0.3) ? g.pick([0, Math.PI / 2, Math.PI, 1.5 * Math.PI, TAU, -Math.PI / 2, 1e-10]) : g.num(-2 * TAU, 2 * TAU);
}

/** Angles in degrees: the axes, full turns and signs, and random ones. */
function degrees(g: Gen): number {
  return g.chance(0.3) ? g.pick([0, 90, 180, 270, 360, -90, 45, 30, 1e-10, -720]) : g.num(-720, 720);
}

function ellipse(g: Gen): EllipseGeom {
  const t0 = radians(g);
  return { c: g.pt(), major: g.chance(0.2) ? v(g.pick([10, -10, 0]), g.pick([0, 5])) : g.vec(40), ratio: g.chance(0.2) ? g.pick([1, 0.5]) : g.num(0.05, 1), t0, t1: g.chance(0.4) ? t0 : radians(g) };
}

function entity(g: Gen): EntityGeometry {
  return g.pick<() => EntityGeometry>([
    () => {
      const a = g.pt();
      return { kind: 'line', a, b: g.chance(0.1) ? a : g.pt() };
    },
    () => ({ kind: 'arc', c: g.pt(), r: g.num(0.5, 60), a0: radians(g), a1: radians(g) }),
    () => {
      const pts = g.pts(g.int(1, 5), 40);
      return { kind: 'polyline', pts, ...(g.chance(0.5) && { bulges: pts.map(() => (g.chance(0.5) ? 0 : g.num(-1.5, 1.5))) }) };
    },
    () => ({ kind: 'polygon', pts: g.ring(4, 30) }),
    () => ({ kind: 'circle', c: g.pt(), r: g.num(1, 30) }),
    () => ({ kind: 'point', p: g.pt() }),
  ])();
}

/** A corner as the corner tools find it: a random vertex and two unit sides. */
function corner(g: Gen): CornerGeom {
  const at = g.pt();
  const a1 = g.num(0, TAU);
  const phi = g.chance(0.2) ? g.pick([Math.PI / 2, Math.PI / 3, 2.5]) : g.num(0.05, Math.PI - 0.05);
  const a2 = a1 + (g.chance(0.5) ? phi : -phi);
  return { at, u1: v(Math.cos(a1), Math.sin(a1)), u2: v(Math.cos(a2), Math.sin(a2)), reach: g.num(0.5, 40), phi };
}

/** Two lines meeting (or not) near a corner, picked on either side. */
function twoLines(g: Gen): Vec2[] {
  const x = g.chance(0.4) ? g.gridPt(5, 3) : g.pt();
  const end = (deg: number, d: number) => ({ x: x.x + d * Math.cos((deg * Math.PI) / 180), y: x.y + d * Math.sin((deg * Math.PI) / 180) });
  const d1 = g.chance(0.3) ? g.pick([0, 90, 180, 45]) : g.num(0, 360);
  const d2 = g.chance(0.1) ? d1 : g.chance(0.3) ? g.pick([0, 90, 270, 135]) : g.num(0, 360);
  // Lines may stop short of the corner or run past it; picks fall on either side.
  const line = (deg: number) => [end(deg, g.num(-5, 3)), end(deg, g.num(5, 40))];
  const pick = (deg: number) => end(deg, g.num(-20, 30));
  const [a1, b1] = line(d1);
  const [a2, b2] = line(d2);
  return [a1, b1, pick(d1), a2, b2, pick(d2)];
}

function pickedEdge(g: Gen, c: Vec2): { a: Vec2; b: Vec2; at: Vec2 } {
  const deg = g.num(0, 360);
  const p = (d: number) => ({ x: c.x + d * Math.cos((deg * Math.PI) / 180), y: c.y + d * Math.sin((deg * Math.PI) / 180) });
  return { a: p(g.num(-10, 5)), b: p(g.num(8, 40)), at: g.chance(0.1) ? c : p(g.num(-10, 40)) };
}

export const S5_TOOLS: CallSet = {
  file: 'calls-s5-tools.json',
  named: [
    { name: 'doğrultu açısı, aynı nokta', fn: 'directionAngle', args: [v(E, N), v(E, N)] },
    { name: 'doğrultu açısı, batı', fn: 'directionAngle', args: [v(0, 0), v(-5, 0)] },
    { name: 'yarıçapla altıgen, içten', fn: 'regularPolygonRadius', args: [v(E, N), 6, 10, true] },
    { name: 'yarıçapla altıgen, dıştan', fn: 'regularPolygonRadius', args: [v(E, N), 6, 10, false] },
    { name: 'yarıçapla üçgen', fn: 'regularPolygonRadius', args: [v(0, 0), 3, 5, true] },
    { name: 'yarıçapla, yuvarlanan kenar sayısı', fn: 'regularPolygonRadius', args: [v(0, 0), 4.6, 5, true] },
    { name: 'yarıçapla, sıfır yarıçap', fn: 'regularPolygonRadius', args: [v(0, 0), 5, 0, true] },
    { name: 'devam: çizgi', fn: 'endTangent', args: [{ kind: 'line', a: v(E, N), b: v(E + 30, N + 40) }] },
    { name: 'devam: sıfır boy çizgi', fn: 'endTangent', args: [{ kind: 'line', a: v(1, 1), b: v(1, 1) }] },
    { name: 'devam: yay', fn: 'endTangent', args: [{ kind: 'arc', c: v(0, 0), r: 10, a0: 0, a1: Math.PI / 2 }] },
    { name: 'devam: yaylı çoklu çizgi', fn: 'endTangent', args: [{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)], bulges: [0, 0.5] }] },
    { name: 'devam: tek köşe', fn: 'endTangent', args: [{ kind: 'polyline', pts: [v(0, 0)] }] },
    { name: 'devam: kapalı alan yok', fn: 'endTangent', args: [{ kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10)] }] },
    { name: 'devam: daire yok', fn: 'endTangent', args: [{ kind: 'circle', c: v(0, 0), r: 3 }] },
    { name: 'yön 90°', fn: 'degDirection', args: [90] },
    { name: 'yön 360°', fn: 'degDirection', args: [360] },
    { name: 'yön −45°', fn: 'degDirection', args: [-45] },
    { name: 'çaptan daire, TM', fn: 'circleOnDiameter', args: [v(E, N), v(E + 30, N + 40)] },
    { name: 'çaptan daire, aynı nokta', fn: 'circleOnDiameter', args: [v(3, 3), v(3, 3)] },
    { name: 'elips parametresi, daire', fn: 'ellipseParamToward', args: [{ c: v(0, 0), major: v(10, 0), ratio: 1, t0: 0, t1: 0 }, v(0, 5)] },
    { name: 'elips parametresi, dönük', fn: 'ellipseParamToward', args: [{ c: v(E, N), major: v(6, 8), ratio: 0.5, t0: 0, t1: 0 }, v(E - 3, N + 4)] },
    { name: 'elips parametresi, merkezde', fn: 'ellipseParamToward', args: [{ c: v(0, 0), major: v(10, 0), ratio: 0.5, t0: 0, t1: 0 }, v(0, 0)] },
    { name: 'döndürme 0°', fn: 'ellipseRotationHalf', args: [v(0, 0), v(20, 0), false, 0] },
    { name: 'döndürme 60°, merkezden', fn: 'ellipseRotationHalf', args: [v(E, N), v(E + 6, N + 8), true, 60] },
    { name: 'birim yön, çakışık', fn: 'unitToward', args: [v(E, N), v(E, N)] },
    { name: 'birim yön, 1e-9', fn: 'unitToward', args: [v(0, 0), v(0, 1e-9)] },
    { name: 'yardımcı çizgi: yatay', fn: 'xlineDirection', args: ['horizontal', [], v(1, 1), 0] },
    { name: 'yardımcı çizgi: düşey', fn: 'xlineDirection', args: ['vertical', [], v(1, 1), 0] },
    { name: 'yardımcı çizgi: 30°', fn: 'xlineDirection', args: ['angle', [], v(1, 1), 30] },
    { name: 'yardımcı çizgi: nokta', fn: 'xlineDirection', args: ['point', [v(E, N)], v(E + 3, N + 4), 0] },
    { name: 'yardımcı çizgi: nokta yok', fn: 'xlineDirection', args: ['point', [], v(1, 1), 0] },
    { name: 'açıortay', fn: 'xlineDirection', args: ['bisect', [v(0, 0), v(10, 0)], v(0, 10), 0] },
    { name: 'açıortay: zıt kollar', fn: 'xlineDirection', args: ['bisect', [v(0, 0), v(10, 0)], v(-5, 0), 0] },
    { name: 'açıortay: kol eksik', fn: 'xlineDirection', args: ['bisect', [v(0, 0)], v(-5, 0), 0] },
    { name: 'açıortay: çakışık kol', fn: 'xlineDirection', args: ['bisect', [v(0, 0), v(0, 0)], v(-5, 0), 0] },
    { name: 'çember üzerinde, merkezde', fn: 'radialPoint', args: [v(E, N), 5, v(E, N)] },
    { name: 'çember üzerinde, TM', fn: 'radialPoint', args: [v(E, N), 5, v(E + 30, N - 40)] },
    { name: 'yarıçapla yay: kiriş uzun', fn: 'radiusBulge', args: [v(0, 0), v(10, 0), 4, null] },
    { name: 'yarıçapla yay: yarım daire', fn: 'radiusBulge', args: [v(0, 0), v(10, 0), 5, null] },
    { name: 'yarıçapla yay: sağa dönüş', fn: 'radiusBulge', args: [v(E, N), v(E + 10, N - 3), 8, v(1, 0)] },
    { name: 'yarıçapla yay: düz devam', fn: 'radiusBulge', args: [v(0, 0), v(10, 0), 8, v(1, 0)] },
    { name: 'merkezle yay: çeyrek', fn: 'centreBulge', args: [v(0, 0), v(10, 0), v(0, 10)] },
    { name: 'merkezle yay: sıfır', fn: 'centreBulge', args: [v(0, 0), v(10, 0), v(10, 0)] },
    { name: 'merkezle yay: neredeyse tam tur', fn: 'centreBulge', args: [v(E, N), v(E + 10, N), v(E + 10, N - 1e-3)] },
    { name: 'doğrultuda uzunluk, TM', fn: 'offsetAlong', args: [v(E, N), v(0.6, 0.8), 12.5] },
    { name: 'yazı açısı: sola bakan', fn: 'textAngle', args: [v(0, 0), v(-10, 1)] },
    { name: 'yazı açısı: tam yukarı', fn: 'textAngle', args: [v(0, 0), v(0, 10)] },
    { name: 'yazı açısı: tam aşağı', fn: 'textAngle', args: [v(0, 0), v(0, -10)] },
    { name: 'yazı açısı: batı', fn: 'textAngle', args: [v(E, N), v(E - 10, N)] },
    { name: 'halka: dolu', fn: 'donutRings', args: [v(E, N), 0, 1] },
    { name: 'halka: delikli', fn: 'donutRings', args: [v(0, 0), 0.5, 1] },
    { name: 'döndürme açısı, referanssız', fn: 'rotationAngle', args: [v(E, N), v(E - 3, N + 4), 0] },
    { name: 'döndürme açısı, referanslı', fn: 'rotationAngle', args: [v(0, 0), v(0, 5), Math.PI] },
    { name: 'ölçek: sıfır referans', fn: 'scaleFactor', args: [v(0, 0), v(3, 4), 0] },
    { name: 'ölçek: çakışık', fn: 'scaleFactor', args: [v(E, N), v(E, N), 2.5] },
    { name: 'kutupsal dizi: tam tur', fn: 'polarArrayTransforms', args: [v(E, N), 6, 360, true, v(E + 10, N)] },
    { name: 'kutupsal dizi: eksi tam tur', fn: 'polarArrayTransforms', args: [v(0, 0), 4, -360, true, v(1, 1)] },
    { name: 'kutupsal dizi: yarım, dönmeden', fn: 'polarArrayTransforms', args: [v(E, N), 5, 180, false, v(E + 10, N + 5)] },
    { name: 'kutupsal dizi: iki adet', fn: 'polarArrayTransforms', args: [v(0, 0), 2, 90, true, v(1, 0)] },
    { name: 'hizala: nokta yok', fn: 'alignTransform', args: [[], false] },
    { name: 'hizala: yalnız taşı', fn: 'alignTransform', args: [[v(E, N), v(E + 5, N + 2)], false] },
    { name: 'hizala: üç nokta', fn: 'alignTransform', args: [[v(0, 0), v(5, 2), v(1, 1)], true] },
    { name: 'hizala: döndür', fn: 'alignTransform', args: [[v(E, N), v(E + 5, N + 2), v(E + 10, N), v(E + 5, N + 12)], false] },
    { name: 'hizala: ölçekle', fn: 'alignTransform', args: [[v(0, 0), v(5, 2), v(10, 0), v(5, 22)], true] },
    { name: 'hizala: çakışık kaynak', fn: 'alignTransform', args: [[v(0, 0), v(5, 2), v(0, 0), v(5, 22)], true] },
    { name: 'köşe: dik, TM', fn: 'vertexCorner', args: [v(E, N), v(E + 10, N), v(E + 10, N + 10), 0, 0] },
    { name: 'köşe: kapanış köşesi', fn: 'vertexCorner', args: [v(0, 10), v(0, 0), v(10, 0), 0, 0] },
    { name: 'köşe: gelen kenar yay', fn: 'vertexCorner', args: [v(0, 0), v(10, 0), v(10, 10), 0.5, 0] },
    { name: 'köşe: giden kenar yay', fn: 'vertexCorner', args: [v(0, 0), v(10, 0), v(10, 10), 0, -1e-11] },
    { name: 'köşe: düz devam', fn: 'vertexCorner', args: [v(0, 0), v(10, 0), v(20, 0), 0, 0] },
    { name: 'köşe: geri dönüş', fn: 'vertexCorner', args: [v(0, 0), v(10, 0), v(0, 0), 0, 0] },
    { name: 'köşe: sıfır kenar', fn: 'vertexCorner', args: [v(10, 0), v(10, 0), v(10, 10), 0, 0] },
    { name: 'iki çizgi: dik', fn: 'linesCornerAt', args: [v(0, 0), v(10, 0), v(5, 0), v(12, 2), v(12, 20), v(12, 10)] },
    { name: 'iki çizgi: paralel', fn: 'linesCornerAt', args: [v(0, 0), v(10, 0), v(5, 0), v(0, 1), v(10, 1), v(5, 1)] },
    { name: 'iki çizgi: arkada seçim', fn: 'linesCornerAt', args: [v(0, 0), v(10, 0), v(15, 0), v(12, 2), v(12, 20), v(12, 10)] },
    { name: 'iki çizgi, TM', fn: 'linesCornerAt', args: [v(E, N), v(E + 10, N + 1), v(E + 5, N + 0.5), v(E + 11, N + 3), v(E + 13, N + 20), v(E + 12, N + 10)] },
    { name: 'çekilen boy: imleç yok', fn: 'pulledDistance', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, null, 0.37] },
    { name: 'çekilen boy: kenar boyunca', fn: 'pulledDistance', args: [{ at: v(E, N), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, v(E + 3.14159, N + 0.5), 0.037] },
    { name: 'çekilen boy: erişimden uzun', fn: 'pulledDistance', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, v(30, 1), 0.037] },
    { name: 'çekilen boy: çok yakın görünüm', fn: 'pulledDistance', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, v(3.14159, 0), 1e-5] },
    { name: 'çekilen boy: onluk sınırlar', fn: 'pulledDistance', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 1e4, phi: Math.PI / 2 }, v(1234.5678, 0), 10] },
    { name: 'çekilen boy: 0.1', fn: 'pulledDistance', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 1e4, phi: Math.PI / 2 }, v(1234.5678, 0), 0.1] },
    { name: 'çekilen boy: 1000', fn: 'pulledDistance', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 1e5, phi: Math.PI / 2 }, v(12345.678, 0), 1000] },
    { name: 'çekilen boy: geride', fn: 'pulledDistance', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, v(-3, -3), 0.037] },
    { name: 'yuvarlama yarıçapı: dik köşe', fn: 'filletRadiusFor', args: [4, Math.PI / 2] },
    { name: 'köşe yayı: dik', fn: 'filletArc', args: [{ at: v(E, N), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, 2] },
    { name: 'köşe yayı: saat yönü kenarlar', fn: 'filletArc', args: [{ at: v(0, 0), u1: v(0, 1), u2: v(1, 0), reach: 10, phi: Math.PI / 2 }, 2] },
    { name: 'köşe yayı: sıfır', fn: 'filletArc', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, 0] },
    { name: 'pah: ikisi', fn: 'chamferLine', args: [{ at: v(E, N), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, 2, 3] },
    { name: 'pah: sıfır', fn: 'chamferLine', args: [{ at: v(0, 0), u1: v(1, 0), u2: v(0, 1), reach: 10, phi: Math.PI / 2 }, 0, 3] },
    { name: 'köşeden açı: içte', fn: 'vertexArms', args: [v(0, 0), v(10, 0), v(0, 10), v(3, 3)] },
    { name: 'köşeden açı: dışta', fn: 'vertexArms', args: [v(E, N), v(E + 10, N), v(E, N + 10), v(E - 3, N - 3)] },
    { name: 'kenarlardan açı: dik', fn: 'edgeArms', args: [{ a: v(0, 0), b: v(10, 0), at: v(6, 0) }, { a: v(0, 0), b: v(0, 10), at: v(0, 4) }, v(3, 3)] },
    { name: 'kenarlardan açı: paralel', fn: 'edgeArms', args: [{ a: v(0, 0), b: v(10, 0), at: v(6, 0) }, { a: v(0, 1), b: v(10, 1), at: v(4, 1) }, v(3, 3)] },
    { name: 'kenarlardan açı: köşede tık', fn: 'edgeArms', args: [{ a: v(E, N), b: v(E + 10, N), at: v(E, N) }, { a: v(E, N), b: v(E, N + 10), at: v(E, N + 4) }, v(E - 2, N + 3)] },
    { name: 'yarıçap ölçüsü: merkezde', fn: 'radialDimension', args: [v(0, 0), 5, v(0, 0)] },
    { name: 'yarıçap ölçüsü: içte', fn: 'radialDimension', args: [v(E, N), 5, v(E + 1, N + 2)] },
    { name: 'yarıçap ölçüsü: dışta', fn: 'radialDimension', args: [v(E, N), 5, v(E + 30, N - 40)] },
    // The rectangle tool's corner style and the tangent circles' picked edge (docs/adr/0032).
    { name: 'köşeler: yuvarlanan dikdörtgen, TM', fn: 'cornersOfRing', args: [[v(E, N), v(E + 20, N), v(E + 20, N + 10), v(E, N + 10)], { radius: 2 }] },
    { name: 'köşeler: pahlı dikdörtgen', fn: 'cornersOfRing', args: [[v(0, 0), v(20, 0), v(20, 10), v(0, 10)], { d1: 1.5, d2: 1.5 }] },
    { name: 'köşeler: döndürülmüş, saat yönü', fn: 'cornersOfRing', args: [[v(0, 0), v(6, 8), v(14, 2), v(8, -6)], { radius: 1 }] },
    { name: 'köşeler: yarıçap kenara sığmaz', fn: 'cornersOfRing', args: [[v(0, 0), v(20, 0), v(20, 10), v(0, 10)], { radius: 6 }] },
    { name: 'köşeler: yarıçap kenarın yarısı', fn: 'cornersOfRing', args: [[v(0, 0), v(20, 0), v(20, 10), v(0, 10)], { radius: 5 }] },
    { name: 'köşeler: sıfır yarıçap', fn: 'cornersOfRing', args: [[v(0, 0), v(20, 0), v(20, 10), v(0, 10)], { radius: 0 }] },
    { name: 'köşeler: sıfır pah', fn: 'cornersOfRing', args: [[v(0, 0), v(20, 0), v(20, 10), v(0, 10)], { d1: 0, d2: 0 }] },
    { name: 'köşeler: düz devam eden köşe', fn: 'cornersOfRing', args: [[v(0, 0), v(10, 0), v(20, 0), v(20, 10), v(0, 10)], { radius: 1 }] },
    { name: 'köşeler: boş halka', fn: 'cornersOfRing', args: [[], { radius: 1 }] },
    { name: 'en yakın kenar: çoklu çizginin tıklanan parçası', fn: 'nearestEdge', args: [{ kind: 'polyline', pts: [v(E, N), v(E + 10, N), v(E + 10, N + 10)] }, v(E + 9, N + 6)] },
    { name: 'en yakın kenar: eşit uzaklıkta ilki', fn: 'nearestEdge', args: [{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] }, v(11, -1)] },
    { name: 'en yakın kenar: daire', fn: 'nearestEdge', args: [{ kind: 'circle', c: v(E, N), r: 5 }, v(E + 1, N + 1)] },
    { name: 'en yakın kenar: yay', fn: 'nearestEdge', args: [{ kind: 'arc', c: v(0, 0), r: 10, a0: 0, a1: Math.PI / 2 }, v(-3, -3)] },
    { name: 'en yakın kenar: kapalı alanın deliği', fn: 'nearestEdge', args: [{ kind: 'polygon', pts: [v(0, 0), v(20, 0), v(20, 20), v(0, 20)], holes: [{ pts: [v(8, 8), v(12, 8), v(12, 12), v(8, 12)] }] }, v(10, 9)] },
    { name: 'en yakın kenar: nokta', fn: 'nearestEdge', args: [{ kind: 'point', p: v(1, 1) }, v(0, 0)] },
    { name: 'noktadan geç: çizgi, TM', fn: 'offsetThroughDistance', args: [{ kind: 'line', a: v(E, N), b: v(E + 30, N + 40) }, v(E + 8, N - 6)] },
    { name: 'noktadan geç: daire içinde', fn: 'offsetThroughDistance', args: [{ kind: 'circle', c: v(0, 0), r: 5 }, v(1, 1)] },
    { name: 'noktadan geç: yaylı çoklu çizgi', fn: 'offsetThroughDistance', args: [{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)], bulges: [0, 0.5, 0] }, v(14, 5)] },
    { name: 'noktadan geç: delikli alanın deliğine yakın', fn: 'offsetThroughDistance', args: [{ kind: 'polygon', pts: [v(0, 0), v(20, 0), v(20, 20), v(0, 20)], holes: [{ pts: [v(8, 8), v(12, 8), v(12, 12), v(8, 12)] }] }, v(10, 9)] },
    { name: 'noktadan geç: kenarı olmayan nokta', fn: 'offsetThroughDistance', args: [{ kind: 'point', p: v(1, 1) }, v(0, 0)] },
    { name: 'deliğe yakın: delik', fn: 'nearHole', args: [{ kind: 'polygon', pts: [v(0, 0), v(20, 0), v(20, 20), v(0, 20)], holes: [{ pts: [v(8, 8), v(12, 8), v(12, 12), v(8, 12)] }] }, v(10, 9)] },
    { name: 'deliğe yakın: dış halka', fn: 'nearHole', args: [{ kind: 'polygon', pts: [v(0, 0), v(20, 0), v(20, 20), v(0, 20)], holes: [{ pts: [v(8, 8), v(12, 8), v(12, 12), v(8, 12)] }] }, v(1, 10)] },
    { name: 'deliğe yakın: eşit uzaklık dış halkanın', fn: 'nearHole', args: [{ kind: 'polygon', pts: [v(0, 0), v(20, 0), v(20, 20), v(0, 20)], holes: [{ pts: [v(8, 8), v(12, 8), v(12, 12), v(8, 12)] }] }, v(4, 10)] },
    { name: 'deliğe yakın: deliksiz alan', fn: 'nearHole', args: [{ kind: 'polygon', pts: [v(E, N), v(E + 20, N), v(E + 20, N + 20)] }, v(E + 10, N + 5)] },
    { name: 'deliğe yakın: çoklu çizgi', fn: 'nearHole', args: [{ kind: 'polyline', pts: [v(0, 0), v(20, 0)], holes: [{ pts: [v(8, 8), v(12, 8), v(12, 12)] }] }, v(10, 9)] },
    { name: 'köşe: iki çizginin ortak ucu, TM', fn: 'cornerNear', args: [[{ kind: 'line', a: v(E, N), b: v(E + 20, N) }, { kind: 'line', a: v(E, N + 15), b: v(E, N) }], v(E + 0.5, N + 0.25), 1.5, 0.25] },
    { name: 'köşe: uçlar yakın ama ayrı', fn: 'cornerNear', args: [[{ kind: 'line', a: v(0, 0), b: v(20, 0) }, { kind: 'line', a: v(0.2, 15), b: v(0.2, 0.1) }], v(0.3, 0.3), 1.5, 0.25] },
    { name: 'köşe: uçlar same’den uzak', fn: 'cornerNear', args: [[{ kind: 'line', a: v(0, 0), b: v(20, 0) }, { kind: 'line', a: v(1, 15), b: v(1, 0.5) }], v(0.5, 0.3), 1.5, 0.25] },
    { name: 'köşe: çoklu çizginin köşesi', fn: 'cornerNear', args: [[{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] }], v(9.5, 0.5), 1.5, 0.25] },
    { name: 'köşe: açık çoklu çizginin ucu değil', fn: 'cornerNear', args: [[{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)] }], v(0.2, 0.1), 1.5, 0.25] },
    { name: 'köşe: kapalı alanın ilk köşesi', fn: 'cornerNear', args: [[{ kind: 'polygon', pts: [v(0, 0), v(10, 0), v(10, 10), v(0, 10)] }], v(0.2, 0.1), 1.5, 0.25] },
    { name: 'köşe: yay yanındaki köşe değil', fn: 'cornerNear', args: [[{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 10)], bulges: [0.4, 0, 0] }], v(10, 0), 1.5, 0.25] },
    { name: 'köşe: düz devam köşe değil', fn: 'cornerNear', args: [[{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(20, 0)] }], v(10, 0), 1.5, 0.25] },
    { name: 'köşe: en yakını kazanır', fn: 'cornerNear', args: [[{ kind: 'polyline', pts: [v(0, 0), v(10, 0), v(10, 1)] }, { kind: 'polygon', pts: [v(10.8, 0), v(20, 0), v(20, 10)] }], v(10.6, 0.1), 1.5, 0.25] },
    { name: 'köşe: eşit uzaklıkta ilki', fn: 'cornerNear', args: [[{ kind: 'polyline', pts: [v(-5, 5), v(0, 0), v(-5, -5)] }, { kind: 'polyline', pts: [v(7, 5), v(2, 0), v(7, -5)] }], v(1, 0), 1.5, 0.25] },
    { name: 'köşe: daire köşe değil', fn: 'cornerNear', args: [[{ kind: 'circle', c: v(0, 0), r: 5 }], v(5, 0), 1.5, 0.25] },
    { name: 'köşe: aday yok', fn: 'cornerNear', args: [[], v(0, 0), 1.5, 0.25] },
    // The arrays of cad.entities.array (docs/adr/0047, part 2).
    { name: 'dizi: 2 × 3, satır satır', fn: 'gridArrayTransforms', args: [2, 3, 12.5, -4] },
    { name: 'dizi: tek sütun', fn: 'gridArrayTransforms', args: [3, 1, 0, 7.5] },
    { name: 'dizi: tek yer', fn: 'gridArrayTransforms', args: [1, 1, 5, 5] },
    { name: 'dizi: sıfır satır', fn: 'gridArrayTransforms', args: [0, 3, 5, 5] },
    { name: 'dizi: 10 000 yerden çok', fn: 'gridArrayTransforms', args: [101, 100, 5, 5] },
    { name: 'dizi: TM aralıklar, eksi', fn: 'gridArrayTransforms', args: [3, 2, -0.1, 1e-9] },
    { name: 'orta: çizgi ve daire, TM', fn: 'shapesMiddle', args: [[{ kind: 'line', a: v(E, N), b: v(E + 10, N) }, { kind: 'circle', c: v(E + 20, N + 10), r: 2.5 }], 'barlow'] },
    { name: 'orta: yazı yazı tipiyle ölçülür', fn: 'shapesMiddle', args: [[{ kind: 'text', p: v(E, N), text: 'Ada 101', height: 2, rotation: 30 }], 'courier-prime'] },
    { name: 'orta: yazı tipi yok, Barlow', fn: 'shapesMiddle', args: [[{ kind: 'text', p: v(E, N), text: 'Ada 101', height: 2, rotation: 30 }], null] },
    { name: 'orta: yaylı kapalı alan', fn: 'shapesMiddle', args: [[{ kind: 'polygon', pts: [v(0, 0), v(40, 0), v(40, 30)], bulges: [0.25, 0.5, -0.25] }], 'barlow'] },
    { name: 'orta: nesne yok', fn: 'shapesMiddle', args: [[], 'barlow'] },
    { name: 'dizi kur: ızgara', fn: 'arrayTransforms', args: ['grid', [2, 3, 12.5, -4], [], 'barlow'] },
    { name: 'dizi kur: ızgara, kesirli satır', fn: 'arrayTransforms', args: ['grid', [2.5, 3, 12.5, -4], [], 'barlow'] },
    { name: 'dizi kur: kutupsal, döner', fn: 'arrayTransforms', args: ['polar', [E, N, 6, 360, 1], [], 'barlow'] },
    { name: 'dizi kur: kutupsal, dönmeden, ortasıyla', fn: 'arrayTransforms', args: ['polar', [E, N, 5, 180, 0], [{ kind: 'line', a: v(E + 10, N), b: v(E + 20, N + 5) }], 'barlow'] },
    { name: 'dizi kur: kutupsal, dönmeden, nesne yok', fn: 'arrayTransforms', args: ['polar', [E, N, 5, 180, 0], [], 'barlow'] },
    { name: 'dizi kur: kutupsal, 1000 öğeden çok', fn: 'arrayTransforms', args: ['polar', [0, 0, 1001, 360, 1], [], 'barlow'] },
    { name: 'dizi kur: bilinmeyen tür', fn: 'arrayTransforms', args: ['spiral', [1, 2, 3, 4], [], 'barlow'] },
  ],
  random: (g, n) => [
    ...repeat(g, 'directionAngle', n, () => [g.pt(), g.pt()]),
    ...repeat(g, 'regularPolygonRadius', n, () => [g.pt(), g.pick([3, 4, 5, 6, 8, 12, 4.5, g.num(0, 12)]), g.pick([0, g.num(0.1, 60)]), g.chance(0.5)]),
    ...repeat(g, 'endTangent', n, () => [entity(g)]),
    ...repeat(g, 'degDirection', n, () => [degrees(g)]),
    ...repeat(g, 'circleOnDiameter', n, () => [g.pt(), g.pt()]),
    ...repeat(g, 'ellipseParamToward', n, () => {
      const e = ellipse(g);
      return [e, { x: e.c.x + g.num(-60, 60), y: e.c.y + g.num(-60, 60) }];
    }),
    ...repeat(g, 'ellipseRotationHalf', n, () => [g.pt(), g.pt(), g.chance(0.5), g.chance(0.2) ? g.pick([0, 45, 60, 89.4]) : g.num(0, 89.4)]),
    ...repeat(g, 'unitToward', n, () => {
      const a = g.pt();
      return [a, g.chance(0.1) ? a : g.pt()];
    }),
    ...repeat(g, 'xlineDirection', n, () => {
      const vertex = g.pt();
      const arm = () => (g.chance(0.1) ? vertex : g.pt());
      return [g.pick(XMODES), Array.from({ length: g.int(0, 2) }, (_, i) => (i === 0 ? vertex : arm())), g.chance(0.1) ? vertex : g.pt(), degrees(g)];
    }),
    ...repeat(g, 'radialPoint', n, () => {
      const c = g.pt();
      return [c, g.num(0, 50), g.chance(0.1) ? c : g.pt()];
    }),
    ...repeat(g, 'radiusBulge', n, () => {
      const last = g.pt();
      return [last, g.pt(), g.num(0.5, 120), g.chance(0.3) ? null : g.pick([v(1, 0), v(0, -1), g.vec(1)])];
    }),
    ...repeat(g, 'centreBulge', n, () => [g.pt(), g.pt(), g.pt()]),
    ...repeat(g, 'offsetAlong', n, () => [g.pt(), g.vec(1), g.num(-100, 100)]),
    ...repeat(g, 'textAngle', n, () => [g.pt(), g.pt()]),
    ...repeat(g, 'donutRings', n, () => [g.pt(), g.pick([0, g.num(0, 5)]), g.num(0.5, 20)]),
    ...repeat(g, 'rotationAngle', n, () => [g.pt(), g.pt(), g.chance(0.3) ? 0 : radians(g)]),
    ...repeat(g, 'scaleFactor', n, () => [g.pt(), g.pt(), g.num(0.1, 100)]),
    ...repeat(g, 'polarArrayTransforms', n, () => [g.pt(), g.int(2, 12), g.pick([360, -360, 180, 90, g.num(-360, 360)]), g.chance(0.5), g.pt()]),
    ...repeat(g, 'alignTransform', n, () => [Array.from({ length: g.int(0, 4) }, () => g.pt()), g.chance(0.5)]),
    ...repeat(g, 'vertexCorner', n, () => {
      const at = g.chance(0.3) ? g.gridPt(5, 3) : g.pt();
      const near = () => (g.chance(0.3) ? g.gridPt(5, 3) : g.pt());
      const bulge = () => (g.chance(0.7) ? 0 : g.pick([1e-13, -1e-11, g.num(-1, 1)]));
      return [g.chance(0.05) ? at : near(), at, near(), bulge(), bulge()];
    }),
    ...repeat(g, 'linesCornerAt', n, () => twoLines(g)),
    ...repeat(g, 'pulledDistance', n, () => {
      const c = corner(g);
      return [c, g.chance(0.1) ? null : { x: c.at.x + g.num(-50, 50), y: c.at.y + g.num(-50, 50) }, g.chance(0.2) ? g.pick([1e-4, 0.001, 0.01, 0.1, 1, 10, 100]) : 10 ** g.num(-4, 3)];
    }),
    ...repeat(g, 'filletRadiusFor', n, () => [g.num(0, 40), g.num(0.01, Math.PI - 0.01)]),
    ...repeat(g, 'filletArc', n, () => [corner(g), g.chance(0.1) ? 0 : g.num(0.1, 20)]),
    ...repeat(g, 'chamferLine', n, () => [corner(g), g.chance(0.1) ? 0 : g.num(0.1, 20), g.chance(0.1) ? 0 : g.num(0.1, 20)]),
    ...repeat(g, 'vertexArms', n, () => [g.pt(), g.pt(), g.pt(), g.pt()]),
    ...repeat(g, 'edgeArms', n, () => {
      const c = g.pt();
      return [pickedEdge(g, c), pickedEdge(g, c), { x: c.x + g.num(-30, 30), y: c.y + g.num(-30, 30) }];
    }),
    ...repeat(g, 'radialDimension', n, () => {
      const c = g.pt();
      return [c, g.num(0.5, 40), g.chance(0.1) ? c : g.pt()];
    }),
    // Appended: the calls above keep their random draws (docs/adr/0032).
    ...repeat(g, 'cornersOfRing', n, () => {
      // A rectangle at any angle and size (the rectangle tool's rings), sometimes another ring.
      const ring = g.chance(0.8) ? rotatedRect(g) : g.ring(g.int(3, 6), 30);
      const size = g.chance(0.1) ? g.pick([0, -1]) : g.num(0.05, 12);
      return [ring, g.chance(0.5) ? { radius: size } : { d1: size, d2: size }];
    }),
    ...repeat(g, 'nearestEdge', n, () => [entity(g), g.chance(0.3) ? g.gridPt(5, 3) : g.pt()]),
    // Appended: the calls above keep their random draws (docs/adr/0047).
    ...repeat(g, 'offsetThroughDistance', n, () => [entity(g), g.chance(0.3) ? g.gridPt(5, 3) : g.pt()]),
    ...repeat(g, 'nearHole', n, () => holed(g)),
    ...repeat(g, 'cornerNear', n, () => cornerCandidates(g)),
    // Appended: the calls above keep their random draws (docs/adr/0047, part 2).
    ...repeat(g, 'gridArrayTransforms', n, () => [g.int(0, 6), g.int(0, 6), g.chance(0.2) ? 0 : g.num(-50, 50), g.chance(0.2) ? 0 : g.num(-50, 50)]),
    ...repeat(g, 'shapesMiddle', n, () => [Array.from({ length: g.int(0, 4) }, () => entity(g)), g.pick(['barlow', 'arimo', 'courier-prime', null])]),
    ...repeat(g, 'arrayTransforms', n, () =>
      g.chance(0.4)
        ? ['grid', [g.int(0, 6), g.int(0, 6), g.num(-50, 50), g.num(-50, 50)], [], 'barlow']
        : ['polar', [g.pt().x, g.pt().y, g.int(1, 12), g.pick([360, -360, 180, 90, g.num(-360, 360)]), g.pick([0, 1])], Array.from({ length: g.int(0, 3) }, () => entity(g)), g.pick(['barlow', 'plex-mono'])],
    ),
  ],
};

/** A closed area with holes (or without), and a point among its rings: the vertex tool's click. */
function holed(g: Gen): unknown[] {
  const c = g.pt();
  const box = (r: number, at: Vec2) => [v(at.x - r, at.y - r), v(at.x + r, at.y - r), v(at.x + r, at.y + r), v(at.x - r, at.y + r)];
  const outer = box(g.num(20, 40), c);
  const holes = Array.from({ length: g.chance(0.2) ? 0 : g.int(1, 2) }, () => ({ pts: box(g.num(1, 6), { x: c.x + g.num(-10, 10), y: c.y + g.num(-10, 10) }) }));
  const kind = g.chance(0.1) ? 'polyline' : 'polygon';
  return [{ kind, pts: outer, ...(holes.length && { holes }), ...(g.chance(0.2) && { bulges: outer.map(() => (g.chance(0.5) ? 0 : g.num(-0.5, 0.5))) }) }, { x: c.x + g.num(-45, 45), y: c.y + g.num(-45, 45) }];
}

/**
 * Candidates as the corner tools gather them around the cursor: lines that
 * meet end to end (or nearly), paths with a vertex near it, now and then an
 * object that has no corners; the cursor, the reach and how near two ends
 * must be to meet (12 and 2 pixels at some zoom).
 */
function cornerCandidates(g: Gen): unknown[] {
  const x = g.chance(0.4) ? g.gridPt(5, 3) : g.pt();
  const px = g.pick([0.01, 0.05, 0.125, 0.5, g.num(0.005, 1)]);
  const tol = 12 * px;
  const same = 2 * px;
  const near = (d: number) => ({ x: x.x + g.num(-d, d), y: x.y + g.num(-d, d) });
  const away = () => {
    const a = g.num(0, 2 * Math.PI);
    const d = g.num(1, 40);
    return { x: x.x + d * Math.cos(a), y: x.y + d * Math.sin(a) };
  };
  const candidates = Array.from({ length: g.int(0, 5) }, (): EntityGeometry => {
    const kind = g.pick(['line', 'line', 'line', 'polyline', 'polygon', 'circle']);
    // An end at the corner, exactly or within a few pixels.
    const end = () => (g.chance(0.5) ? x : near(g.pick([same / 2, same * 3, tol])));
    if (kind === 'line') return g.chance(0.5) ? { kind, a: end(), b: away() } : { kind, a: away(), b: end() };
    if (kind === 'circle') return { kind, c: away(), r: g.num(1, 20) };
    const pts = [away(), end(), away(), ...(g.chance(0.5) ? [away()] : [])];
    if (g.chance(0.3)) pts.reverse();
    return { kind, pts, ...(g.chance(0.2) && { bulges: pts.map(() => (g.chance(0.6) ? 0 : g.num(-0.5, 0.5))) }) } as EntityGeometry;
  });
  return [candidates, near(tol * 1.5), tol, same];
}

/** A rectangle as the rectangle tool gives it: four corners, any rotation, either turning. */
function rotatedRect(g: Gen): Vec2[] {
  const a = g.pt();
  const t = g.chance(0.3) ? g.pick([0, Math.PI / 2, Math.PI / 6]) : g.num(0, TAU);
  const [w, h] = [g.num(0.5, 40), g.num(0.5, 40)];
  const u = v(Math.cos(t), Math.sin(t));
  const side = g.chance(0.5) ? 1 : -1;
  const n = v(-u.y * side, u.x * side);
  return [a, v(a.x + u.x * w, a.y + u.y * w), v(a.x + u.x * w + n.x * h, a.y + u.y * w + n.y * h), v(a.x + n.x * h, a.y + n.y * h)];
}
