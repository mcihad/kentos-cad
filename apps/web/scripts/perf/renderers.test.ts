// Ek işleyiciler on the web (docs/adr/0213 §6): the desktop's scenes (crates/native/style/tests/all/renderer_timing.rs)
// built as the viewport builds a layer (the page's call, the style core through WASM, the GPU's batches): 10 000
// districts in Sürekli renk and in pies, 100 000 dots in 1 000 districts, a heat map and clusters of 100 000 points,
// 10 000 parcels' outside; the view 1440 × 900 px over 2 km. Node and the WASM package, one process; p50 and p95 of
// repeated builds after a warm-up, against 2.5 times the desktop's budget. Runs only on purpose, with the shipped WASM
// (docs/adr/0170 §3; remove its stamp afterwards so the next test run builds the tests' own again):
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm
//   RENDERER_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/renderers.test.ts --disable-console-intercept
//   rm apps/web/src/wasm/pkg/.stamp
import { it } from 'vitest';
import type { Bounds } from '../../src/model/geometry';
import type { NewEntity } from '../../src/model/entities';
import type { LayerStyle } from '../../src/model/layers';
import type { LayerRenderer } from '../../src/model/style';
import { buildStyledLayer } from '../../src/render/styledLayer';
import { layerDocument, PALETTE } from '../../src/style/cases';
import { PickIndex } from '../../src/viewport/picking';

const RUNS = Number(process.env.RENDERER_RUNS ?? 10);
const WARM = 2;
const X0 = 487000;
const Y0 = 4420000;
/** The view: 1440 × 900 px over 2 000 m. */
const PX_PER_M = 1440 / 2000;
const VIEW: Bounds = { minX: X0, minY: Y0, maxX: X0 + 2000, maxY: Y0 + 1250 };

const grown = (b: Bounds, k: number): Bounds => {
  const [w, h] = [(b.maxX - b.minX) * k, (b.maxY - b.minY) * k];
  return { minX: b.minX - w, minY: b.minY - h, maxX: b.maxX + w, maxY: b.maxY + h };
};

/** `n` districts on a grid, `w` metres each, numbers that vary. */
function districts(n: number, w: number): NewEntity[] {
  const side = Math.ceil(Math.sqrt(n));
  return Array.from({ length: n }, (_, i): NewEntity => {
    const [x, y] = [X0 + (i % side) * w, Y0 + Math.floor(i / side) * w];
    const s = w * 0.96;
    const attrs = {
      Nüfus: String((i * 7919) % 40_000),
      Konut: String(((i * 31) % 100) + 1),
      Ticaret: String(((i * 17) % 60) + 1),
      Yeşil: String(((i * 13) % 30) + 1),
      Erkek: '500',
      Kadın: '500',
    };
    return { kind: 'polygon', layerId: 'k', attrs, pts: [{ x, y }, { x: x + s, y }, { x: x + s, y: y + s }, { x, y: y + s }] };
  });
}

/** `n` points over 2 km × 1.25 km in 40 clumps (a fixed generator, mulberry32). */
function points(n: number): NewEntity[] {
  let a = 0x9e3779b9;
  const next = () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const centres = Array.from({ length: 40 }, () => [next() * 2000, next() * 1250]);
  return Array.from({ length: n }, (_, i): NewEntity => {
    const [cx, cy] = centres[i % centres.length];
    const r = next() * 120;
    const t = next() * 2 * Math.PI;
    return { kind: 'point', layerId: 'k', attrs: { Önem: String((i % 5) + 1) }, p: { x: X0 + cx + r * Math.cos(t), y: Y0 + cy + r * Math.sin(t) } };
  });
}

const styleOf = (renderer: LayerRenderer): LayerStyle => ({ color: '#4E79A7', lineType: 'continuous', lineWeight: 0.25, renderer });

interface Scene {
  name: string;
  entities: NewEntity[];
  renderer: LayerRenderer;
  /** The desktop's budget, ms; the web's is 2.5 times it. */
  budget: number;
  clip?: Bounds;
  frame?: boolean;
}

function stats(ms: number[]): [number, number] {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return [q(0.5), q(0.95)];
}

it.runIf(!!process.env.RENDERER_BENCH)(
  'measures the thematic renderers on the web',
  () => {
    const areas = districts(10_000, 20);
    const pts = points(100_000);
    const scenes: Scene[] = [
      {
        name: 'Sürekli renk, 10 000 alan',
        entities: areas,
        budget: 30,
        renderer: {
          type: 'unclassed',
          expr: 'Nüfus',
          min: 0,
          max: 40000,
          ramp: ['#FFF5B8', '#FDB863', '#E66101', '#A50F15'],
          symbols: { fill: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#000000' }, { id: 'l', type: 'simpleLine', color: '#FFFFFF', width: 0.3 }] } },
        },
      },
      {
        name: 'Pasta grafik, 10 000 alan (çerçeveli)',
        entities: areas,
        budget: 70,
        renderer: {
          type: 'chart',
          kind: 'pie',
          size: 6,
          unit: 'mm',
          fields: [
            { expr: 'Konut', color: '#E15759' },
            { expr: 'Ticaret', color: '#4E79A7' },
            { expr: 'Yeşil', color: '#59A14F' },
          ],
          outline: { color: '#FFFFFF', width: 0.2 },
        },
      },
      {
        name: 'Nokta yoğunluğu, 1 000 alanda 100 000 nokta',
        entities: districts(1_000, 60),
        budget: 60,
        renderer: { type: 'dotDensity', dotValue: 10, dotSize: 0.5, unit: 'mm', seed: 1, fields: [{ expr: 'Erkek', color: '#4E79A7' }, { expr: 'Kadın', color: '#E15759' }] },
      },
      {
        name: 'Isı haritası, 100 000 nokta (20 px, kalite 2)',
        entities: pts,
        budget: 40,
        clip: grown(VIEW, 0.5),
        frame: true,
        renderer: { type: 'heatmap', radius: 20, unit: 'px', quality: 2, ramp: ['#2B83BA00', '#2B83BA', '#ABDDA4', '#FFFFBF', '#FDAE61', '#D7191C'] },
      },
      { name: 'Kümeleme, 100 000 nokta (40 px)', entities: pts, budget: 30, clip: grown(VIEW, 3), frame: true, renderer: { type: 'cluster', distance: 40, unit: 'px' } },
      {
        name: 'Ters alan, 10 000 parsel',
        entities: areas,
        budget: 40,
        clip: grown(VIEW, 3),
        renderer: { type: 'inverted', symbols: { fill: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#FFFFFFB3' }, { id: 'l', type: 'simpleLine', color: '#8E4EC6', width: 0.6 }] } } },
      },
    ];
    const rows: string[] = [];
    for (const scene of scenes) {
      const doc = layerDocument(scene.name, styleOf(scene.renderer), scene.entities);
      const index = new PickIndex(doc);
      const entities = [...doc.all()];
      const style = doc.layers.get('k')!.style;
      const ms: number[] = [];
      let batches = 0;
      for (let run = 0; run < WARM + RUNS; run++) {
        const t = performance.now();
        const layer = buildStyledLayer('k', entities, style, {
          origin: { x: X0, y: Y0 },
          palette: PALETTE,
          plotScale: 5000,
          library: { symbol: () => undefined, asset: () => undefined },
          layerName: () => scene.name,
          geometry: index,
          ...(scene.clip && { clip: scene.clip }),
          ...(scene.frame && { frame: { pxPerM: PX_PER_M, picture: `heat:k:${run}` } }),
        });
        if (run >= WARM) ms.push(performance.now() - t);
        batches = layer.styled?.length ?? 0;
      }
      const [p50, p95] = stats(ms);
      const budget = 2.5 * scene.budget;
      rows.push(`${scene.name.padEnd(48)} p50 ${p50.toFixed(1).padStart(7)} ms, p95 ${p95.toFixed(1).padStart(7)} ms   bütçe ${String(budget).padStart(5)} ms  ${p50 <= budget ? '✓' : '✗'}   ${batches} topluluk`);
    }
    console.log(`\nEk işleyiciler, web (${RUNS} koşu; bütçe masaüstününkinin 2,5 katı)\n${rows.join('\n')}`);
  },
  1_800_000,
);
