// Styled layer builds (docs/adr/0008 “Stil derleyicisi”): a layer's objects
// through the style engine into GPU batches, the way the viewport rebuilds
// a layer after an edit or a style change. Parcels with a rule renderer and
// area labels, boundaries with an MPYY marker line, plain lines in the
// simple look, points with categorized markers. Node and the WASM package,
// one process; p50 and p95 of repeated builds after a warm-up. Runs only on
// purpose:
//   STYLE_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/style.test.ts --disable-console-intercept
import { it } from 'vitest';
import type { NewEntity } from '../../src/model/entities';
import type { LayerStyle } from '../../src/model/layers';
import type { LibrarySymbol, Symbol } from '../../src/model/style';
import { buildStyledLayer } from '../../src/render/styledLayer';
import { layerDocument, PALETTE } from '../../src/style/cases';
import { SYSTEM_LIBRARY } from '../../src/style/system';
import { PickIndex } from '../../src/viewport/picking';

const RUNS = Number(process.env.STYLE_RUNS ?? 10);
const WARM = 2;
const X0 = 486000;
const Y0 = 4420000;

const system = new Map(SYSTEM_LIBRARY.items.filter((i): i is LibrarySymbol => i.kind === 'symbol').map((s) => [s.id, s.symbol]));
const library = { symbol: (id: string): Symbol | undefined => system.get(id), asset: () => undefined };

const NITELIK = ['Arsa', 'Bahçe', 'Tarla'];
const arsa: Symbol = {
  type: 'fill',
  layers: [
    { id: 'f', type: 'simpleFill', color: '#E8C07A80' },
    { id: 'o', type: 'simpleLine', color: 'ink', width: 0.3 },
    { id: 't', type: 'centroidMarker', marker: { type: 'marker', layers: [{ id: 'n', type: 'text', text: { expr: 'Parsel' }, size: 2.5 }, { id: 'a', type: 'text', text: { expr: "yuvarla($alan, 1) || ' m²'" }, size: 1.8, offset: [0, -3] }] } },
  ],
};

interface Scene {
  name: string;
  style: LayerStyle;
  entities: NewEntity[];
}

function parcels(n: number): Scene {
  const side = Math.round(Math.sqrt(n));
  const entities: NewEntity[] = [];
  for (let i = 0; i < n; i++) {
    const x = X0 + (i % side) * 20;
    const y = Y0 + Math.floor(i / side) * 20;
    entities.push({ kind: 'polygon', layerId: 'k', attrs: { Parsel: String(i + 1), Nitelik: NITELIK[i % 3] }, pts: [{ x, y }, { x: x + 20, y }, { x: x + 20, y: y + 20 }, { x, y: y + 20 }] });
  }
  const style: LayerStyle = {
    color: 'ink',
    lineType: 'continuous',
    lineWeight: 0.25,
    renderer: {
      type: 'rules',
      rules: [
        { id: 'a', label: 'Arsa', filter: "Nitelik = 'Arsa'", symbols: { fill: arsa } },
        { id: 'd', label: 'Diğer', isElse: true, symbols: { fill: { ref: 'mpyy.cdp.yerlesim.kentsel-meskun-alan' } } },
      ],
    },
  };
  return { name: `Parsel ×${n}`, style, entities };
}

function boundaries(n: number): Scene {
  const entities: NewEntity[] = [];
  for (let i = 0; i < n; i++) {
    const pts = Array.from({ length: 12 }, (_, k) => ({ x: X0 + (i % 50) * 120 + k * 9, y: Y0 + Math.floor(i / 50) * 60 + 8 * Math.sin(k + i) }));
    entities.push({ kind: 'polyline', layerId: 'k', attrs: {}, pts });
  }
  return { name: `Köy sınırı ×${n}`, style: { color: 'ink', lineType: 'continuous', lineWeight: 0.25, renderer: { type: 'single', symbols: { line: { ref: 'mpyy.nip.idari.koy-siniri' } } } }, entities };
}

function plainLines(n: number): Scene {
  const entities: NewEntity[] = [];
  for (let i = 0; i < n; i++) {
    const x = X0 + (i % 250) * 8;
    const y = Y0 + Math.floor(i / 250) * 8;
    entities.push({ kind: 'line', layerId: 'k', attrs: {}, a: { x, y }, b: { x: x + 6, y: y + 3 } });
  }
  return { name: `Çizgi ×${n} (yalın görünüş)`, style: { color: 'ink', lineType: 'dashed', lineWeight: 0.18 }, entities };
}

function points(n: number): Scene {
  const entities: NewEntity[] = [];
  for (let i = 0; i < n; i++) entities.push({ kind: 'point', layerId: 'k', attrs: { Kat: String(1 + (i % 5)) }, p: { x: X0 + (i % 200) * 10, y: Y0 + Math.floor(i / 200) * 10 } });
  const marker = (color: string): Symbol => ({ type: 'marker', layers: [{ id: 's', type: 'shape', shape: 'square', size: 2, fill: color, stroke: 'ink', strokeWidth: 0.2 }, { id: 't', type: 'text', text: { expr: 'Kat' }, size: 1.5, offset: [0, 2.5] }] });
  const categories = ['1', '2', '3', '4', '5'].map((value, k) => ({ value, label: value, symbols: { marker: marker(['#2E7D32', '#1565C0', '#F9A825', '#C62828', '#6A1B9A'][k]) } }));
  return { name: `Yapı noktası ×${n} (kategorili)`, style: { color: 'ink', lineType: 'continuous', lineWeight: 0.25, renderer: { type: 'categorized', expr: 'Kat', categories } }, entities };
}

function stats(ms: number[]): string {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return `p50 ${q(0.5).toFixed(1).padStart(7)} ms, p95 ${q(0.95).toFixed(1).padStart(7)} ms`;
}

it.runIf(!!process.env.STYLE_BENCH)('measures styled layer builds', () => {
  const rows: string[] = [];
  for (const scene of [parcels(10_000), boundaries(2_000), plainLines(50_000), points(20_000)]) {
    const doc = layerDocument(scene.name, scene.style, scene.entities);
    const index = new PickIndex(doc);
    const entities = [...doc.all()];
    const style = doc.layers.get('k')!.style;
    const ms: number[] = [];
    let batches = 0;
    let numbers = 0;
    for (let run = 0; run < WARM + RUNS; run++) {
      const t = performance.now();
      const layer = buildStyledLayer('k', entities, style, { origin: { x: X0, y: Y0 }, palette: PALETTE, plotScale: 1000, library, layerName: () => scene.name, geometry: index });
      if (run >= WARM) ms.push(performance.now() - t);
      const styled = layer.styled ?? [];
      batches = styled.length;
      numbers = styled.reduce((s, b) => s + (b.kind === 'stroke' ? b.segments.length : b.kind === 'fill' ? b.positions.length : b.instances.length), 0);
    }
    rows.push(`${scene.name.padEnd(34)} ${stats(ms)}   ${batches} topluluk, ${numbers} sayı`);
  }
  console.log(`\nKatman kurma (${RUNS} koşu)\n${rows.join('\n')}`);
}, 1_800_000);
