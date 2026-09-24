// Move, copy and paste of 10 000 mixed objects (docs/adr/0008): the
// transform alone and the whole command (the transform and the document's
// undo step), through JSON (`transformEntities`, every object both ways, the
// path before the store answered packed) and through the geometry store
// (`PickIndex.transformEntities`; for a paste the objects' own store, built
// by the paste tool at its first frame or, pasting in place, for the call).
// Node and the WASM package, one process; p50 and p95 of repeated runs after
// a warm-up. Runs only on purpose:
//   TRANSFORM_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/transform.test.ts --silent=false --reporter=verbose
import { it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import type { Entity, EntityKind, NewEntity } from '../../src/model/entities';
import { translation, type Affine } from '../../src/model/geom/affine';
import { LayerStore } from '../../src/model/layers';
import { transformEntities, transformedFrom } from '../../src/model/ops/transform';
import { PickIndex } from '../../src/viewport/picking';
import { Gen } from '../../src/wasm/calls/harness';
import { entity } from '../../src/wasm/calls/sets/p5-entities';
import { CoreStore } from '../../src/wasm/core';
import { packEntities } from '../../src/wasm/pack';

const N = 10_000;
const RUNS = Number(process.env.TRANSFORM_RUNS ?? 40);
const WARM = 5;

/** A survey drawing's mix: parcels, lines and polylines first, a little of every other kind. */
const MIX: [EntityKind, number][] = [
  ['polygon', 3500],
  ['line', 2000],
  ['polyline', 1500],
  ['point', 1000],
  ['text', 800],
  ['circle', 200],
  ['arc', 200],
  ['dimension', 200],
  ['hatch', 200],
  ['spline', 200],
  ['ellipse', 100],
  ['xline', 50],
  ['ray', 50],
];

/** Starts an object in a TM zone (the generator picks the origin or a zone at random). */
function tmFrame(g: Gen): void {
  for (;;) {
    g.frame();
    if (g.pt().x > 1e5) return;
  }
}

function scene(): CadDocument {
  const g = new Gen(10_000);
  const list: NewEntity[] = [];
  for (const [kind, n] of MIX)
    for (let i = 0; i < n; i++) {
      tmFrame(g);
      const { id: _id, ...e } = entity(g, kind);
      list.push(e as NewEntity);
    }
  const doc = new CadDocument({ name: 'Ölçüm', layers: new LayerStore([{ id: 'parsel', name: 'Parsel' }, { id: 'bina', name: 'Bina' }, { id: 'taslak', name: 'Taslak' }], 'parsel'), origin: { x: 486000, y: 4420000 } });
  doc.load(list);
  return doc;
}

function stats(ms: number[]): string {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return `p50 ${q(0.5).toFixed(1).padStart(6)} ms, p95 ${q(0.95).toFixed(1).padStart(6)} ms (${s.length} koşu)`;
}

/** `run` WARM + RUNS times; `after` (untimed) undoes each run. */
function time(run: () => void, after = () => {}): number[] {
  const out: number[] = [];
  for (let i = 0; i < WARM + RUNS; i++) {
    const t = performance.now();
    run();
    const d = performance.now() - t;
    after();
    if (i >= WARM) out.push(d);
  }
  return out;
}

/** Ids 1…n and a store of the clipboard's objects under them (as the paste tool keeps one). */
const numbered = (n: number) => Float64Array.from({ length: n }, (_, i) => i + 1);
function storeOf(items: readonly NewEntity[]): CoreStore {
  const store = new CoreStore();
  const p = packEntities(items.map((e, i) => ({ ...e, id: i + 1 })));
  store.putPacked(p.nums, p.strings);
  return store;
}

it.runIf(!!process.env.TRANSFORM_BENCH)('measures move, copy and paste of 10 000 objects', () => {
  const doc = scene();
  const index = new PickIndex(doc);
  index.ids(); // the store in step with the drawing, as after the first pointer move
  const list = [...doc.all()];
  const m: Affine = translation(12.5, -7.25);
  const clipboard = list.map(({ id: _id, ...rest }) => structuredClone(rest) as NewEntity);
  const ids = numbered(clipboard.length);
  const rows: string[] = [];
  const row = (what: string, ms: number[]) => rows.push(`${what.padEnd(46)} ${stats(ms)}`);

  const viaJson = () => transformEntities(list, [m]);
  const viaStore = () => index.transformEntities(list, [m]);
  const pasteJson = () => transformEntities(clipboard.map((e) => ({ ...e, id: 0 }) as Entity), [m]).map(({ id: _id, ...e }) => e as NewEntity);
  const tool = storeOf(clipboard);
  const pasteTool = () => transformedFrom(clipboard, ids, 1, tool.transformPacked(ids, Float64Array.from(m)));
  const pasteInPlace = () => {
    const s = storeOf(clipboard);
    try {
      return transformedFrom(clipboard, ids, 1, s.transformPacked(ids, Float64Array.from(m)));
    } finally {
      s.dispose();
    }
  };

  // The transform alone: the selection's objects, one translation.
  row('taşı/kopyala: dönüşüm, JSON', time(viaJson));
  row('taşı/kopyala: dönüşüm, depodan paketli', time(viaStore));
  row('yapıştır: dönüşüm, JSON', time(pasteJson));
  row('yapıştır: dönüşüm, aracın deposundan', time(pasteTool));
  row('yapıştır: aracın deposunu kurmak (ilk kare)', time(() => storeOf(clipboard).dispose()));
  row('yerine yapıştır: depo kurulup dönüşüm', time(pasteInPlace));

  // The whole command: the transform and the document's undo step (undone, untimed, after each run).
  const move = (moved: Entity[]) => doc.transact('Taşı', () => moved.forEach((t) => doc.update(t.id, t)));
  const copy = (moved: Entity[]) =>
    doc.transact('Kopyala', () =>
      moved.forEach((t) => {
        const { id: _id, ...rest } = t;
        doc.add(rest as NewEntity);
      }),
    );
  const paste = (moved: NewEntity[]) =>
    doc.transact('Yapıştır', () => {
      for (const e of moved) doc.add(e);
    });
  const undo = () => {
    doc.undo();
    index.ids();
  };
  row('taşı: komut, JSON', time(() => move(viaJson()), undo));
  row('taşı: komut, depodan paketli', time(() => move(viaStore()), undo));
  row('kopyala: komut, JSON', time(() => copy(viaJson()), undo));
  row('kopyala: komut, depodan paketli', time(() => copy(viaStore()), undo));
  row('yapıştır: komut, JSON', time(() => paste(pasteJson()), undo));
  row('yapıştır: komut, aracın deposundan', time(() => paste(pasteTool()), undo));
  row('yerine yapıştır: komut, depo kurulup', time(() => paste(pasteInPlace()), undo));

  tool.dispose();
  console.log(`\n${N} nesne (${MIX.map(([k, n]) => `${k} ${n}`).join(', ')}), TM koordinatları\n${rows.join('\n')}`);
  index.dispose();
}, 900_000);
