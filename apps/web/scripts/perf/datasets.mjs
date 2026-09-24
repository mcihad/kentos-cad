// Data sets of docs/adr/0005 for the interaction harness (interaction.mjs).
// Each generator runs inside the page (it is sent as source text), so it is
// self-contained: its own seeded PRNG (mulberry32, as in
// src/model/sampleProject.ts), no imports, plain entity objects with ids.
// Changing a generator changes every measurement: take a new baseline then.

/**
 * parsel-50k: blocks (ada) of 2 × 5 parcels on the TM36 grid around
 * 486 000 / 4 420 000, rotated 12°, coordinates in mm like survey data.
 * Neighbours share their vertices exactly. About 12 % of the parcels have
 * a curved street front (bulge), about 5 % a hole, about 60 % a building.
 */
export function parcelData(p) {
  let seed = p.seed;
  const rand = () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const PW = 18; // frontage
  const PD = 28; // depth
  const ROAD = 12;
  const N = 5;
  const BW = N * PW;
  const BH = 2 * PD;
  const w = p.blocksX * (BW + ROAD) - ROAD;
  const h = p.blocksY * (BH + ROAD) - ROAD;
  const rot = (p.rotation * Math.PI) / 180;
  const ca = Math.cos(rot);
  const sa = Math.sin(rot);
  const mm = (v) => Math.round(v * 1000) / 1000;
  const W = (q) => {
    const x = q.x - w / 2;
    const y = q.y - h / 2;
    return { x: mm(p.center.x + x * ca - y * sa), y: mm(p.center.y + x * sa + y * ca) };
  };
  const shoelace = (ring) => {
    let s = 0;
    for (let i = 0; i < ring.length; i++) {
      const u = ring[i];
      const v = ring[(i + 1) % ring.length];
      s += u.x * v.y - v.x * u.y;
    }
    return s / 2;
  };
  // Circular segment a bulge adds on a chord (positive on a counter-clockwise ring: outwards).
  const segment = (c, b) => {
    const th = 4 * Math.atan(Math.abs(b));
    const r = c / (2 * Math.sin(th / 2));
    return Math.sign(b) * ((r * r) / 2) * (th - Math.sin(th));
  };
  const rect = (x0, y0, x1, y1, ccw) =>
    ccw
      ? [{ x: x0, y: y0 }, { x: x1, y: y0 }, { x: x1, y: y1 }, { x: x0, y: y1 }]
      : [{ x: x0, y: y0 }, { x: x0, y: y1 }, { x: x1, y: y1 }, { x: x1, y: y0 }];
  const entities = [];
  let id = 0;
  const count = { parcels: 0, buildings: 0, bulged: 0, holed: 0, edges: 0 };
  for (let by = 0; by < p.blocksY; by++)
    for (let bx = 0; bx < p.blocksX; bx++) {
      const ox = bx * (BW + ROAD);
      const oy = by * (BH + ROAD);
      const ada = String(101 + by * p.blocksX + bx);
      const pafta = `P-${1 + Math.floor(bx / 12)}-${1 + Math.floor(by / 12)}`;
      const top = [0];
      const bot = [0];
      const midY = [PD];
      for (let i = 1; i < N; i++) {
        const t = i * PW + (rand() - 0.5) * 5;
        top.push(t + (rand() - 0.5) * 3);
        bot.push(t + (rand() - 0.5) * 3);
        midY.push(PD + (rand() - 0.5) * 4);
      }
      top.push(BW);
      bot.push(BW);
      midY.push(PD);
      const mid = top.map((t, i) => ({ x: (t + bot[i]) / 2, y: midY[i] }));
      let no = 1;
      for (const north of [false, true])
        for (let i = 0; i < N; i++) {
          // Counter-clockwise; the street front is edge 0 (south row) or 2 (north row).
          const ring = (north ? [mid[i], mid[i + 1], { x: top[i + 1], y: BH }, { x: top[i], y: BH }] : [{ x: bot[i], y: 0 }, { x: bot[i + 1], y: 0 }, mid[i + 1], mid[i]]).map((q) => ({
            x: q.x + ox,
            y: q.y + oy,
          }));
          let area = shoelace(ring);
          let bulges;
          if (rand() < 0.12) {
            const front = north ? 2 : 0;
            const b = 0.06 + rand() * 0.12;
            bulges = [0, 0, 0, 0];
            bulges[front] = b;
            const u = ring[front];
            const v = ring[front + 1];
            area += segment(Math.hypot(v.x - u.x, v.y - u.y), b);
            count.bulged++;
          }
          // The part of the parcel clear of its side boundaries, 5 m back from the street.
          const xa = Math.max(north ? top[i] : bot[i], mid[i].x) + 3;
          const xb = Math.min(north ? top[i + 1] : bot[i + 1], mid[i + 1].x) - 3;
          const face = north ? BH - 5 : 5;
          const back = north ? Math.max(midY[i], midY[i + 1]) + 3 : Math.min(midY[i], midY[i + 1]) - 3;
          const across = xb - xa;
          const deep = Math.abs(back - face);
          let building = null;
          let hole = null;
          if (rand() < 0.62 && across > 7 && deep > 8) {
            const bw = Math.min(across, 8 + rand() * 6);
            const bd = Math.min(deep, 9 + rand() * 7);
            const x0 = xa + (across - bw) * rand();
            building = rect(ox + x0, oy + (north ? face - bd : face), ox + x0 + bw, oy + (north ? face : face + bd), true);
          } else if (rand() < 0.12 && across > 6 && deep > 6) {
            const s = 3 + rand() * 2;
            const cx = ox + (xa + xb) / 2;
            const cy = oy + (face + back) / 2;
            hole = rect(cx - s / 2, cy - s / 2, cx + s / 2, cy + s / 2, false);
            area -= s * s;
            count.holed++;
          }
          const parsel = String(no++);
          const parcel = { id: ++id, kind: 'polygon', layerId: 'parsel', pts: ring.map(W), label: parsel, attrs: { Ada: ada, Parsel: parsel, Mahalle: p.mahalle, Nitelik: building ? 'Kargir ev ve arsası' : 'Arsa', 'Tapu alanı (m²)': area.toFixed(2), Pafta: pafta } };
          if (bulges) parcel.bulges = bulges;
          if (hole) parcel.holes = [{ pts: hole.map(W) }];
          entities.push(parcel);
          count.parcels++;
          count.edges += hole ? 8 : 4;
          if (building) {
            entities.push({
              id: ++id,
              kind: 'polygon',
              layerId: 'yapi',
              pts: building.map(W),
              attrs: { Ada: ada, Parsel: parsel, 'Kat adedi': String(2 + Math.floor(rand() * 4)), Yapı: 'Betonarme', 'Taban alanı (m²)': shoelace(building).toFixed(2) },
            });
            count.buildings++;
            count.edges += 4;
          }
        }
    }
  const layers = [
    { id: 'taslak', name: 'Taslak', style: { color: 'ink' } },
    {
      id: 'g-kadastro',
      name: 'Kadastro',
      children: [
        { id: 'parsel', name: 'Parsel sınırı', style: { color: 'fg-dim', lineWeight: 0.18, label: { placement: 'center', size: 9, grow: 1.2, maxSize: 14, minFeaturePx: 26 } } },
        { id: 'yapi', name: 'Yapı', style: { color: '#7FB2E5', lineWeight: 0.25, fill: '#7FB2E52E' } },
      ],
    },
  ];
  return { entities, layers, bigLayer: 'parsel', count };
}

/**
 * hat-1m: contour-like polylines, all on one layer: `lines` lines of
 * `segments` segments each (20 m apart along, 5 m apart across, 10 km wide),
 * waves that never cross; every fifth is an index contour with a label.
 */
export function contourData(p) {
  let seed = p.seed;
  const rand = () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const mm = (v) => Math.round(v * 1000) / 1000;
  const x0 = p.center.x - p.width / 2;
  const y0 = p.center.y - (p.lines * p.spacing) / 2;
  const step = p.width / p.segments;
  const entities = [];
  for (let k = 0; k < p.lines; k++) {
    const amp = 60 + 20 * Math.sin(k / 150);
    const phase = k / 400;
    const pts = [];
    for (let j = 0; j <= p.segments; j++) {
      const u = j * step + (j > 0 && j < p.segments ? (rand() - 0.5) * 4 : 0);
      const y = y0 + k * p.spacing + amp * Math.sin(u / 700 + phase) + 12 * Math.sin(u / 97 + k / 300) + (rand() - 0.5);
      pts.push({ x: mm(x0 + u), y: mm(y) });
    }
    const z = p.baseZ + k;
    const line = { id: k + 1, kind: 'polyline', layerId: 'esyukselti', pts, attrs: { 'Kot (m)': String(z) } };
    if (z % 5 === 0) line.label = String(z);
    entities.push(line);
  }
  const layers = [
    { id: 'taslak', name: 'Taslak', style: { color: 'ink' } },
    { id: 'g-topo', name: 'Topografya', children: [{ id: 'esyukselti', name: 'Eşyükselti', style: { color: '#A87C54', lineWeight: 0.13, label: { placement: 'along', size: 10, minScale: 1.6 } } }] },
  ];
  return { entities, layers, bigLayer: 'esyukselti', count: { lines: p.lines, edges: p.lines * p.segments } };
}

const CENTER = { x: 486_000, y: 4_420_000 };

/**
 * The data sets of docs/adr/0005 by name. `scale` shrinks them for a quick
 * look (0.02 ≈ 1 000 parcels, 40 lines); a baseline is taken at 1.
 */
export function datasets(scale = 1) {
  return {
    'parsel-50k': {
      title: '~50 000 parsel (bir kısmı yaylı, bir kısmı delikli) ve içlerinde yapılar, TM36',
      build: parcelData,
      trimMoves: 120,
      params: { seed: 5256, blocksX: Math.max(2, Math.round(72 * Math.sqrt(scale))), blocksY: Math.max(2, Math.round(70 * Math.sqrt(scale))), center: CENTER, rotation: 12, mahalle: 'Kızılırmak' },
    },
    'hat-1m': {
      title: '1 000 000 doğru parçası: 2 000 eşyükselti benzeri çoklu çizgi × 500 parça, TM36',
      build: contourData,
      trimMoves: 40,
      params: { seed: 1000, lines: Math.max(10, Math.round(2000 * scale)), segments: 500, center: CENTER, width: 10_000, spacing: 5, baseZ: 800 },
    },
  };
}
