// Etiket motoru on the web (docs/adr/0212 §6): the engine alone through the WASM store on the scenes the desktop's
// crates/shared/geometry-core/tests/all/label_engine.rs times (its `label_scenes`, built the same way), and the
// texts of 100 000 parcels' labels (two classes with expressions) worked out as the geometry store takes them after a
// labelling change. The web's budgets are 2.5 times the native ones. Node and the WASM package, one process; the
// best and the median of repeated runs after a warm-up. Runs only on purpose, with the shipped WASM (docs/adr/0170
// §3; remove its stamp afterwards so the next test run builds the tests' own again):
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm
//   LABEL_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/labels.test.ts --disable-console-intercept
//   rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import type { NewEntity } from '../../src/model/entities';
import { layerTexts, objectTexts } from '../../src/model/labelTexts';
import type { LayerStyle } from '../../src/model/layers';
import { layerDocument } from '../../src/style/cases';
import { LABEL, LABEL_STRIDE } from '../../src/viewport/storeRecords';
import { CoreStore } from '../../src/wasm/core';

const RUNS = Number(process.env.LABEL_RUNS ?? 7);
const X0 = 487000;
const Y0 = 4420000;
/** The web's budget over the native one (docs/adr/0212 §6). */
const WEB = 2.5;

interface Scene {
  name: string;
  objects: unknown[];
  layers: unknown[];
  texts: [number, string, number][];
  window: [number, number, number, number];
  scale: number;
  budget: number;
}

/** The scenes, as `label_scenes` builds them. */
function scenes(): Scene[] {
  const out: Scene[] = [];
  const points = (step: number, jx: number, jy: number) => {
    const objects: unknown[] = [];
    const texts: [number, string, number][] = [];
    for (let i = 0; i < 2000; i++) {
      const x = X0 + (i % 50) * step + ((i * 7) % 3) * jx;
      const y = Y0 + Math.floor(i / 50) * step + ((i * 11) % 5) * jy;
      objects.push({ id: i + 1, layerId: 'nokta', kind: 'point', p: { x, y } });
      texts.push([i + 1, `P.${1000 + i}`, NaN]);
    }
    return { objects, texts };
  };
  const pointLayer = [{ id: 'nokta', rank: 0, point: 6, label: { placement: 'beside', size: 10, point: 'around' } }];
  out.push({ name: '2 000 nokta adı (yoğun, 20 px)', ...points(5, 0.7, 0.4), layers: pointLayer, window: [X0 - 10, Y0 - 10, X0 + 260, Y0 + 210], scale: 4, budget: 8 });
  out.push({ name: '2 000 nokta adı (40 px)', ...points(10, 1.4, 0.8), layers: pointLayer, window: [X0 - 10, Y0 - 10, X0 + 510, Y0 + 410], scale: 4, budget: 5 });

  const parcels = (side: number) => {
    const objects: unknown[] = [];
    const texts: [number, string, number][] = [];
    for (let i = 0; i < side * side; i++) {
      const x = X0 + (i % side) * 20;
      const y = Y0 + Math.floor(i / side) * 20;
      objects.push({ id: i + 1, layerId: 'parsel', kind: 'polygon', pts: [{ x, y }, { x: x + 18, y }, { x: x + 18, y: y + 18 }, { x, y: y + 18 }] });
      texts.push([i + 1, `${i + 1}`, NaN]);
    }
    return { objects, texts };
  };
  const parcelLayer = [{ id: 'parsel', rank: 0, label: { placement: 'center', size: 10, area: 'parcel', inside: true, minFeaturePx: 26 } }];
  out.push({ name: '10 000 parsel (parsel kipi)', ...parcels(100), layers: parcelLayer, window: [X0 - 10, Y0 - 10, X0 + 2010, Y0 + 2010], scale: 1.6, budget: 30 });

  const objects: unknown[] = [];
  const texts: [number, string, number][] = [];
  for (let hill = 0; hill < 25; hill++) {
    const [cx, cy] = [X0 + 225 + (hill % 5) * 450, Y0 + 225 + Math.floor(hill / 5) * 450];
    for (let ring = 0; ring < 20; ring++) {
      const id = hill * 20 + ring + 1;
      const r = 210 - ring * 10;
      const pts = [];
      for (let k = 0; k <= 72; k++) {
        const a = (Math.PI * 2 * (k % 72)) / 72;
        const wobble = 1 + 0.06 * Math.sin(3 * a + hill);
        pts.push({ x: cx + r * wobble * Math.cos(a), y: cy + r * wobble * Math.sin(a) });
      }
      objects.push({ id, layerId: 'esyukselti', kind: 'polyline', pts });
      const z = 1000 + ring * 5;
      texts.push([id, `${z}`, z]);
    }
  }
  out.push({
    name: "500 eş yükselti (25 tepe, 400 px'te bir)",
    objects,
    texts,
    layers: [{ id: 'esyukselti', rank: 0, label: { placement: 'along', size: 9, line: 'contour', repeat: 400 } }],
    window: [X0 - 10, Y0 - 10, X0 + 2260, Y0 + 2260],
    scale: 0.8,
    budget: 15,
  });

  out.push({ name: '100 000 parsellik genel bakış', ...parcels(316), layers: parcelLayer, window: [X0 - 50, Y0 - 50, X0 + 6370, Y0 + 6370], scale: 0.15, budget: 10 });
  return out;
}

function row(name: string, placed: number, runs: number[], budget: number): void {
  const s = [...runs].sort((a, b) => a - b);
  const p50 = s[Math.floor(s.length / 2)];
  const web = budget * WEB;
  console.log(`${name.padEnd(42)} ${String(placed).padStart(6)} etiket  en iyi ${s[0].toFixed(3).padStart(8)} ms  p50 ${p50.toFixed(3).padStart(8)} ms   bütçe ${String(web).padStart(5)} ms  ${p50 <= web ? '✓' : '✗'}`);
}

it.runIf(!!process.env.LABEL_BENCH)('measures the label engine on the web', () => {
  console.log('');
  for (const sc of scenes()) {
    const s = new CoreStore();
    s.put(JSON.stringify(sc.objects));
    s.setLabelLayers(JSON.stringify(sc.layers));
    const from = Uint32Array.from({ length: sc.texts.length + 1 }, (_, i) => i);
    s.setObjectLabels(
      Float64Array.from(sc.texts.map(([id]) => id)),
      from,
      new Uint16Array(sc.texts.length),
      sc.texts.map(([, t]) => t).join(''),
      Uint32Array.from(sc.texts.map(([, t]) => t.length)),
      Float64Array.from(sc.texts.map(([, , z]) => z)),
    );
    let placed = 0;
    const runs: number[] = [];
    for (let k = 0; k < RUNS + 1; k++) {
      const t = performance.now();
      const records = s.placeLabels(...sc.window, sc.scale, '[]', 0);
      const ms = performance.now() - t;
      placed = 0;
      for (let i = 1; i < records.length; i += LABEL_STRIDE) if (records[i] === LABEL.placed) placed++;
      if (k > 0) runs.push(ms);
    }
    row(sc.name, placed, runs, sc.budget);
  }

  // The texts: 100 000 parcels, the number by an expression and the owner's name where there is one.
  const side = 316;
  const entities: NewEntity[] = [];
  for (let i = 0; i < side * side; i++) {
    const x = X0 + (i % side) * 20;
    const y = Y0 + Math.floor(i / side) * 20;
    entities.push({
      kind: 'polygon',
      layerId: 'k',
      attrs: { Ada: `${100 + Math.floor(i / 300)}`, Parsel: `${1 + (i % 300)}`, Malik: i % 3 === 0 ? 'Ayşe Demir' : '' },
      pts: [{ x, y }, { x: x + 18, y }, { x: x + 18, y: y + 18 }, { x, y: y + 18 }],
    });
  }
  const label = (text: string) => ({ placement: 'center' as const, size: 10, text });
  const style: LayerStyle = {
    color: '#3E63DD',
    lineType: 'continuous',
    lineWeight: 0.25,
    labels: {
      mode: 'rules',
      classes: [
        { name: 'No', style: label("Ada || '/' || Parsel") },
        { name: 'Malik', when: "Malik <> ''", style: label('upper(Malik)') },
      ],
    },
  };
  const doc = layerDocument('Parsel', style, entities);
  const store = new CoreStore();
  const list = doc.byLayer('k');
  const node = doc.layers.get('k');
  const runs: number[] = [];
  let count = 0;
  for (let k = 0; k < 6; k++) {
    const t = performance.now();
    const packed = objectTexts(list, layerTexts(node), () => 'Parsel', store);
    store.setObjectLabels(packed.ids, packed.from, packed.classes, packed.texts, packed.lens, packed.zs);
    const ms = performance.now() - t;
    count = packed.ids.length;
    if (k > 0) runs.push(ms);
  }
  expect(count).toBe(list.length);
  row(`Etiket metinleri, ${list.length} parsel, iki sınıf`, list.length, runs, 80);
});
