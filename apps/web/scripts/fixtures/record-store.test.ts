// Records the geometry store's answers on a fixed scene into
// fixtures/geometry/v1/store-v1.json (docs/adr/0008, S1): picking,
// snapping, boxes, labels and grips; the tool previews and totals (trim,
// extend, ghosts, stretch ghosts, selection totals, S1c); what the layer
// builders draw and the expressions' geometry values (S2). Rust reads the
// file natively (crates/shared/geometry-core/tests/store.rs), the app through the
// WASM build (src/wasm/store.wasm.test.ts).
// The committed answers were recorded from the TypeScript the store
// replaced; since S3c the recorder asks the core itself, so rewriting a
// case is a deliberate change of the frozen answers, to be read in the diff
// (robust predicates, a changed rule). Runs only on purpose:
//   GOLDEN_WRITE=1 npx vitest run scripts/fixtures/record-store.test.ts
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import type { Entity } from '../../src/model/entities';
import { mirror, rotation, scaling, translation, type Affine } from '../../src/model/geom/affine';
import type { Vec2 } from '../../src/model/geometry';
import { layerTable } from '../../src/viewport/picking';
import { DEFAULT_LABELS, labelRule } from '../../src/viewport/storeRecords';
import { Gen, TOLERANCE, toJson } from '../../src/wasm/calls/harness';
import { SCENE_SCALES, SCENE_TOLERANCES, sceneCursor, sceneDocument, sceneKinds, sceneRect } from '../../src/wasm/calls/sets/store-scene';
import { StoreScene, storeFileText, type StoreCase, type StoreFile } from '../../src/wasm/calls/storeCases';

/** Cursor positions and tool rounds recorded, and the largest answer kept (a window over the whole scene lists every edge). */
const CURSORS = 160;
const TOOL_ROUNDS = 80;
const DRAW_ROUNDS = 80;
const MAX_ANSWER = 6_000;

it.runIf(!!process.env.GOLDEN_WRITE)('records the geometry store into the store fixture', () => {
  const g = new Gen(2609);
  const doc = sceneDocument(g, 300);
  // A hidden group and a locked leaf on top of the scene's own states.
  doc.layers.toggleVisible('g2');
  doc.layers.toggleLocked('g1');
  const asked: Omit<StoreCase, 'expect'>[] = [];
  const add = (name: string, op: string, args: unknown[]) => asked.push({ name, op, args: toJson(args) as unknown[] });
  for (let i = 1; i <= CURSORS; i++) {
    const p = sceneCursor(g, doc);
    const tol = g.pick(SCENE_TOLERANCES);
    const kinds = sceneKinds(g);
    const from = g.chance(0.5) ? sceneCursor(g, doc) : null;
    const r = sceneRect(g, p, [0.1, 3, 20, 200]);
    const ids = [...doc.all()].map((e) => e.id);
    const except = g.chance(0.5) ? g.pick(ids) : null;
    add(`imleç ${i}`, 'hit', [p.x, p.y, tol]);
    add(`imleç ${i}`, 'hitEdge', [p.x, p.y, tol]);
    add(`imleç ${i}`, 'snap', [p.x, p.y, tol, [...kinds], from]);
    add(`imleç ${i}`, 'enclosing', [p.x, p.y]);
    const crossing = g.chance(0.5);
    add(`kutu ${i}`, 'inRect', [r, crossing]);
    add(`kutu ${i}`, 'overlapping', [r, except]);
    const small = sceneRect(g, p, [0.1, 3, 20]);
    add(`kutu ${i}`, 'edgesIn', [small, except]);
    const view = sceneRect(g, p, [20, 200]);
    const scale = g.pick(SCENE_SCALES);
    const editing = g.chance(0.2) ? g.pick(ids) : null;
    add(`görünüm ${i}`, 'labels', [view, scale, editing]);
    const selected = ids.filter(() => g.chance(0.02));
    add(`seçim ${i}`, 'grips', [selected]);
  }
  // Tool previews and totals (S1c) after the queries, so the cases above stay as recorded.
  const all = [...doc.all()];
  const ids = all.map((e) => e.id);
  /** Mostly an object the tool accepts, so most answers are cuts and reaches rather than refusals. */
  const target = (kinds: string[]) => {
    const of = all.filter((e) => kinds.includes(e.kind));
    return g.chance(0.9) && of.length ? g.pick(of) : g.pick(all);
  };
  /** A pick near one of the object's vertices or anchors. */
  const near = (e: Entity): Vec2 => {
    const at = 'pts' in e && e.pts.length ? g.pick(e.pts) : e.kind === 'line' ? g.pick([e.a, e.b]) : 'c' in e && e.c ? e.c : sceneCursor(g, doc);
    return { x: at.x + g.num(-0.5, 0.5), y: at.y + g.num(-0.5, 0.5) };
  };
  for (let i = 1; i <= TOOL_ROUNDS; i++) {
    const chosen = g.chance(0.3) ? ids.filter(() => g.chance(0.1)) : null;
    const cut = target(['line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'xline', 'ray']);
    const cutAt = near(cut);
    const view = sceneRect(g, cutAt, [20, 200, 5000]);
    add(`buda ${i}`, 'trim', [cut.id, cutAt, view, chosen]);
    const grow = target(['line', 'polyline', 'arc', 'ellipse']);
    const growAt = near(grow);
    add(`uzat ${i}`, 'extend', [grow.id, growAt, view, chosen]);
    const c = sceneCursor(g, doc);
    const affines: Affine[] = Array.from({ length: g.int(1, 2) }, () => g.pick([translation(g.num(-50, 50), g.num(-50, 50)), rotation(g.num(-3, 3), c), scaling(g.num(0.2, 3), c), mirror(c, sceneCursor(g, doc))]));
    const selected = [...Array.from({ length: g.int(0, 3) }, () => g.pick(ids)), ...(g.chance(0.2) ? [999_999] : [])];
    const limit = g.pick([0, 1, 400]);
    add(`hayalet ${i}`, 'ghosts', [selected, affines, limit]);
    // The stretch window around a vertex of one of the selected objects, so it catches some.
    const first = doc.get(selected[0] ?? 0);
    const at = first ? near(first) : cutAt;
    const reach = g.pick([5, 50]);
    const w = { minX: at.x - g.num(0, reach), minY: at.y - g.num(0, reach), maxX: at.x + g.num(0, reach), maxY: at.y + g.num(0, reach) };
    const dx = g.num(-10, 10);
    const dy = g.num(-10, 10);
    add(`esnet ${i}`, 'stretchGhosts', [selected, w, dx, dy]);
    add(`toplam ${i}`, 'measure', [selected]);
  }
  // Drawn geometry and the expressions' geometry values (S2), last, so the cases above stay as recorded.
  for (let i = 1; i <= DRAW_ROUNDS; i++) {
    const list = Array.from({ length: g.int(1, 3) }, () => g.pick(all));
    const oriented = g.chance(0.5);
    const clip = g.chance(0.6) ? sceneRect(g, sceneCursor(g, doc), [20, 200, 5000]) : undefined;
    const ids = list.map((e) => e.id);
    add(`çizim ${i}`, 'drawn', [ids, oriented, clip ?? null]);
    add(`değerler ${i}`, 'measures', [ids]);
  }
  const head = {
    format: 'kentos.geometry-store' as const,
    version: 1 as const,
    tolerance: TOLERANCE,
    crs: { kind: 'projected' as const, unit: 'metre' as const, note: 'Koordinatlar metre cinsinden bir projeksiyon düzlemindedir; tolerans bu birim içindir (fixtures/geometry/v1/cases.json ile aynı).' },
    layers: layerTable(doc.layers),
    labelDefaults: Object.fromEntries(Object.entries(DEFAULT_LABELS).map(([kind, st]) => [kind, labelRule(st)])),
    entities: [...doc.all()],
  };
  // Asked of a store built from the file itself, as the tests read it; a very large answer is not kept.
  const scene = new StoreScene(JSON.parse(JSON.stringify(head)) as StoreFile);
  const cases = asked.flatMap((c) => {
    const expect = toJson(scene.answer(c.op, c.args));
    return JSON.stringify(expect).length <= MAX_ANSWER ? [{ ...c, expect }] : [];
  });
  scene.dispose();
  writeFileSync(new URL('../../../../fixtures/geometry/v1/store-v1.json', import.meta.url), storeFileText({ ...head, cases }));
});
