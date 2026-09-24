import { crsBySrid, DEFAULT_SRID } from '../geo/crs';
import { CadDocument } from './document';
import type { NewEntity } from './entities';
import { signedArea, type Vec2 } from './geometry';
import { LayerStore } from './layers';
import { STANDARD_ACTIVE_LAYER, standardLayers } from './standardLayers';

/**
 * Procedurally generated cadastral sheet so the UI can be judged against
 * realistic density: blocks (ada), parcels, buildings, curbs, road axes,
 * contours, benchmarks and a sheet frame (coordinates in the default TM zone).
 */

const ORIGIN: Vec2 = { x: 486_780, y: 4_420_080 };
const ROT = (14 * Math.PI) / 180;
const MAHALLE = 'Kızılırmak';
const PAFTA = 'P-12';

function rng(seed: number) {
  return () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const toWorld = (p: Vec2): Vec2 => ({
  x: ORIGIN.x + p.x * Math.cos(ROT) - p.y * Math.sin(ROT),
  y: ORIGIN.y + p.x * Math.sin(ROT) + p.y * Math.cos(ROT),
});

const round3 = (v: number) => Math.round(v * 1000) / 1000;
const W = (p: Vec2) => {
  const w = toWorld(p);
  return { x: round3(w.x), y: round3(w.y) };
};

function roundedRect(x0: number, y0: number, x1: number, y1: number, r: number, seg = 6): Vec2[] {
  const corners = [
    { c: { x: x1 - r, y: y0 + r }, a0: -Math.PI / 2 },
    { c: { x: x1 - r, y: y1 - r }, a0: 0 },
    { c: { x: x0 + r, y: y1 - r }, a0: Math.PI / 2 },
    { c: { x: x0 + r, y: y0 + r }, a0: Math.PI },
  ];
  const pts: Vec2[] = [];
  for (const { c, a0 } of corners)
    for (let s = 0; s <= seg; s++) {
      const t = a0 + ((Math.PI / 2) * s) / seg;
      pts.push({ x: c.x + Math.cos(t) * r, y: c.y + Math.sin(t) * r });
    }
  return pts;
}

const fmtArea = (pts: Vec2[]) => Math.abs(signedArea(pts)).toFixed(2);

export function createSampleProject(srid = DEFAULT_SRID): CadDocument {
  const layers = new LayerStore(standardLayers(1000), STANDARD_ACTIVE_LAYER);
  const crs = crsBySrid(srid) ?? crsBySrid(DEFAULT_SRID)!;
  const doc = new CadDocument({
    name: 'Ornek_1244-1249_Ada.kcad',
    layers,
    origin: ORIGIN,
    settings: { srid: crs.srid, plotScale: 1000 },
  });

  const rand = rng(1245);
  const out: NewEntity[] = [];
  const blockW = 96;
  const blockH = 64;
  const colX = [-165, -54, 57];
  const rowY = [12, -76];
  let ada = 1244;

  for (const y0 of rowY) {
    for (const x0 of colX) {
      const x1 = x0 + blockW;
      const y1 = y0 + blockH;
      const adaNo = String(ada++);
      const ring = [
        { x: x0, y: y0 },
        { x: x1, y: y0 },
        { x: x1, y: y1 },
        { x: x0, y: y1 },
      ];
      out.push({
        kind: 'polygon',
        layerId: 'ada',
        pts: ring.map(W),
        label: adaNo,
        attrs: { Ada: adaNo, Mahalle: MAHALLE, Pafta: PAFTA, 'Yüzölçümü (m²)': fmtArea(ring) },
      });
      out.push({ kind: 'polygon', layerId: 'kaldirim', pts: roundedRect(x0 - 3, y0 - 3, x1 + 3, y1 + 3, 6).map(W), attrs: {} });

      // Parcel dividers: skewed so the sheet does not look machine-made.
      const n = 5 + Math.floor(rand() * 3);
      const top: number[] = [x0];
      const bot: number[] = [x0];
      for (let i = 1; i < n; i++) {
        const t = x0 + (blockW * i) / n + (rand() - 0.5) * 6;
        top.push(t + (rand() - 0.5) * 4);
        bot.push(t + (rand() - 0.5) * 4);
      }
      top.push(x1);
      bot.push(x1);
      const midY = top.map((_, i) => (i === 0 || i === n ? (y0 + y1) / 2 : (y0 + y1) / 2 + (rand() - 0.5) * 6));
      const mid = top.map((t, i) => ({ x: (t + bot[i]) / 2, y: midY[i] }));

      let parsel = 1;
      const addParcel = (ringLocal: Vec2[], building: Vec2[] | null) => {
        const no = String(parsel++);
        const nitelik = building ? 'Kargir ev ve arsası' : 'Arsa';
        out.push({
          kind: 'polygon',
          layerId: 'parsel',
          pts: ringLocal.map(W),
          label: no,
          attrs: {
            Ada: adaNo,
            Parsel: no,
            Mahalle: MAHALLE,
            Nitelik: nitelik,
            'Tapu alanı (m²)': fmtArea(ringLocal),
            Pafta: PAFTA,
          },
        });
        if (building) {
          out.push({
            kind: 'polygon',
            layerId: 'yapi',
            pts: building.map(W),
            attrs: {
              Ada: adaNo,
              Parsel: no,
              'Kat adedi': String(2 + Math.floor(rand() * 4)),
              Yapı: 'Betonarme',
              'Taban alanı (m²)': fmtArea(building),
            },
          });
        }
      };

      const makeBuilding = (xa: number, xb: number, front: number, back: number, dir: 1 | -1): Vec2[] | null => {
        if (rand() < 0.22) return null;
        const w = xb - xa - 6;
        if (w < 7) return null;
        const bw = Math.min(w, 9 + rand() * 8);
        const bx = xa + 3 + (w - bw) * rand();
        const depth = 9 + rand() * 7;
        const f = front - 5 * dir;
        const b = f - depth * dir;
        if ((b - back) * dir < 3) return null;
        return dir === 1
          ? [
              { x: bx, y: b },
              { x: bx + bw, y: b },
              { x: bx + bw, y: f },
              { x: bx, y: f },
            ]
          : [
              { x: bx, y: f },
              { x: bx + bw, y: f },
              { x: bx + bw, y: b },
              { x: bx, y: b },
            ];
      };

      const through = new Set<number>();
      for (let i = 0; i < n; i++) if (rand() < 0.14) through.add(i);

      for (let i = 0; i < n; i++) {
        const backTop = Math.max(mid[i].y, mid[i + 1].y);
        const xa = Math.max(top[i], mid[i].x);
        const xb = Math.min(top[i + 1], mid[i + 1].x);
        if (through.has(i)) {
          const ringLocal = [
            { x: bot[i], y: y0 },
            { x: bot[i + 1], y: y0 },
            mid[i + 1],
            { x: top[i + 1], y: y1 },
            { x: top[i], y: y1 },
            mid[i],
          ];
          addParcel(ringLocal, makeBuilding(xa, xb, y1, y0 + 8, 1));
        } else {
          addParcel([mid[i], mid[i + 1], { x: top[i + 1], y: y1 }, { x: top[i], y: y1 }], makeBuilding(xa, xb, y1, backTop, 1));
        }
      }
      for (let i = 0; i < n; i++) {
        if (through.has(i)) continue;
        const backBot = Math.min(mid[i].y, mid[i + 1].y);
        const xa = Math.max(bot[i], mid[i].x);
        const xb = Math.min(bot[i + 1], mid[i + 1].x);
        addParcel([{ x: bot[i], y: y0 }, { x: bot[i + 1], y: y0 }, mid[i + 1], mid[i]], makeBuilding(xa, xb, y0, backBot, -1));
      }
    }
  }

  // Road axes and street names.
  const verticalX = [-172.5, -61.5, 49.5, 160.5];
  const horizontalY = [83.5, 0, -83.5];
  const roadNames = ['1428. Sokak', 'Kızılırmak Caddesi', '1430. Sokak'];
  const sokakNames = ['1431. Sokak', '1432. Sokak', '1434. Sokak', '1436. Sokak'];
  horizontalY.forEach((y, i) => {
    out.push({ kind: 'polyline', layerId: 'yol-ekseni', pts: [W({ x: -205, y }), W({ x: 195, y })], attrs: { Ad: roadNames[i], Genişlik: y === 0 ? '24 m' : '15 m' } });
    out.push({ kind: 'text', layerId: 'yazi', p: W({ x: -115 + i * 40, y: y + 2.2 }), text: roadNames[i], height: y === 0 ? 4 : 3, rotation: 14, attrs: {} });
  });
  verticalX.forEach((x, i) => {
    out.push({ kind: 'polyline', layerId: 'yol-ekseni', pts: [W({ x, y: -115 }), W({ x, y: 115 })], attrs: { Ad: sokakNames[i], Genişlik: '15 m' } });
    out.push({ kind: 'text', layerId: 'yazi', p: W({ x: x - 2, y: 40 - i * 22 }), text: sokakNames[i], height: 3, rotation: 14 + 90, attrs: {} });
  });

  // Traverse points at intersections (on the curb).
  let pNo = 101;
  for (const y of horizontalY)
    for (const x of verticalX) {
      const name = `P.${pNo++}`;
      const z = 884 + (y + 120) / 18 + rand() * 0.4;
      out.push({ kind: 'point', layerId: 'poligon', p: W({ x: x + 5.5, y: y + 5.5 }), z: round3(z), label: name, attrs: { Ad: name, Tür: 'Poligon noktası', 'Z (m)': z.toFixed(3) } });
    }

  // Sheet frame, grid-aligned in TM coordinates.
  const fx0 = 486_540;
  const fy0 = 4_419_880;
  const fx1 = 487_020;
  const fy1 = 4_420_280;
  out.push({
    kind: 'polygon',
    layerId: 'pafta',
    pts: [
      { x: fx0, y: fy0 },
      { x: fx1, y: fy0 },
      { x: fx1, y: fy1 },
      { x: fx0, y: fy1 },
    ],
    label: PAFTA,
    attrs: { Pafta: PAFTA, Ölçek: '1:1000', 'Koordinat sistemi': crs.name },
  });
  for (let x = fx0 + 50; x < fx1; x += 50)
    for (let y = fy0 + 50; y < fy1; y += 50) {
      out.push({ kind: 'line', layerId: 'karelaj', a: { x: x - 2.5, y }, b: { x: x + 2.5, y }, attrs: {} });
      out.push({ kind: 'line', layerId: 'karelaj', a: { x, y: y - 2.5 }, b: { x, y: y + 2.5 }, attrs: {} });
    }

  // Contours (1 m interval, every fifth an index contour) clipped to the frame.
  const zAt = (x: number, y: number) => 880 + (y - fy0 + 10 - 14 * Math.sin(x / 70) - 6 * Math.sin(x / 23)) / 18;
  for (let k = 0; k < 24; k++) {
    const pts: Vec2[] = [];
    for (let x = fx0; x <= fx1; x += 6) {
      const y = fy0 - 10 + k * 18 + 14 * Math.sin(x / 70) + 6 * Math.sin(x / 23);
      if (y > fy0 && y < fy1) pts.push({ x: round3(x), y: round3(y) });
    }
    if (pts.length < 2) continue;
    const z = 880 + k;
    const index = z % 5 === 0;
    out.push({ kind: 'polyline', layerId: index ? 'ana-esyukselti' : 'esyukselti', pts, label: index ? String(z) : undefined, attrs: { 'Kot (m)': String(z) } });
  }

  // Spot heights.
  for (let i = 0; i < 46; i++) {
    const x = fx0 + 20 + rand() * (fx1 - fx0 - 40);
    const y = fy0 + 20 + rand() * (fy1 - fy0 - 40);
    const z = zAt(x, y) + (rand() - 0.5) * 0.3;
    out.push({ kind: 'point', layerId: 'kot', p: { x: round3(x), y: round3(y) }, z: round3(z), label: z.toFixed(2), attrs: { Tür: 'Kot noktası', 'Z (m)': z.toFixed(3) } });
  }

  doc.load(out);
  // The sheet is the demo; the symbol catalogue below it is reached by panning or "Tümünü göster".
  doc.homeView = { minX: fx0, minY: fy0, maxX: fx1, maxY: fy1 };
  doc.dirty.set(false);
  return doc;
}
