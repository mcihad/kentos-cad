import { describe, expect, it } from 'vitest';
import type { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import { mirror, rotation, scaling, translation, type Affine } from '../model/geom/affine';
import { transformEntity } from '../model/ops/transform';
import { createSampleProject } from '../model/sampleProject';
import { Gen, toJson } from '../wasm/calls/harness';
import { SCENE_LAYER_IDS, SCENE_SCALES, SCENE_TOLERANCES, sceneCursor, sceneDocument, sceneEntity, sceneKinds, sceneRect } from '../wasm/calls/sets/store-scene';
import { PickIndex } from './picking';

/**
 * The geometry store follows the document (docs/adr/0008, S1): through
 * random edits (adds, moves, removals, undo and redo, transactions,
 * attribute changes, another editor's changes, layer visibility and locks,
 * reloads) a store kept in step by the document's events must answer every
 * query exactly like a store built afresh from the document, and keep the
 * document's order. The answers themselves are held to the frozen fixtures
 * (fixtures/geometry/v1/store-*.json), recorded from the TypeScript the
 * store replaced before it was deleted (S3c).
 */

const env = (globalThis as unknown as { process?: { env: Record<string, string | undefined> } }).process?.env ?? {};
const ROUNDS = Number(env.STORE_ROUNDS ?? 150);

/** Every query at a random cursor, box, view, selection and tool target: the first answer that differs. */
function compare(g: Gen, doc: CadDocument, live: PickIndex, fresh: PickIndex): string | null {
  const all = [...doc.all()];
  const ids = all.map((e) => e.id);
  const p = sceneCursor(g, doc);
  const tol = g.pick(SCENE_TOLERANCES);
  const kinds = sceneKinds(g);
  const from = g.chance(0.5) ? sceneCursor(g, doc) : null;
  const r = sceneRect(g, p);
  const except = ids.length && g.chance(0.5) ? g.pick(ids) : undefined;
  const unlocked = (e: Entity) => !doc.layers.isLocked(e.layerId);
  const view = sceneRect(g, p, [20, 200, 5000]);
  const scale = g.pick(SCENE_SCALES);
  const editing = ids.length && g.chance(0.2) ? g.pick(ids) : null;
  const selected = [...ids.filter(() => g.chance(0.05)), ...(g.chance(0.2) ? [999_999] : [])];
  const selectedEntities = selected.map((id) => doc.get(id)).filter((e): e is Entity => !!e);
  const c = sceneCursor(g, doc);
  const affines: Affine[] = Array.from({ length: g.int(1, 3) }, () => g.pick([translation(g.num(-50, 50), g.num(-50, 50)), rotation(g.num(-3, 3), c), scaling(g.num(0.2, 3), c), mirror(c, sceneCursor(g, doc))]));
  const w = sceneRect(g, p, [5, 50]);
  const dx = g.num(-10, 10);
  const dy = g.num(-10, 10);
  const chosen = g.chance(0.3) ? new Set(ids.filter(() => g.chance(0.1))) : null;
  const list = g.chance(0.2) ? ids : ids.filter(() => g.chance(0.1));
  const oriented = g.chance(0.5);
  const clip = g.chance(0.6) ? sceneRect(g, sceneCursor(g, doc), [20, 200, 5000]) : undefined;
  const target = all.length ? g.pick(all) : null;
  const enc = (x: { entity: Entity; ring: unknown } | null) => x && { id: x.entity.id, ring: x.ring };
  const queries: [string, (s: PickIndex) => unknown][] = [
    ['hit', (s) => s.hit(p, tol)?.id ?? null],
    ['hitEdge', (s) => s.hitEdge(p, tol)?.id ?? null],
    ['hitEdge (kilitsiz)', (s) => s.hitEdge(p, tol, unlocked)?.id ?? null],
    ['snap', (s) => s.snap(p, tol, kinds, from)],
    ['inRect (pencere)', (s) => s.inRect(r, false)],
    ['inRect (kesişim)', (s) => s.inRect(r, true)],
    ['enclosing', (s) => enc(s.enclosing(p))],
    ['overlapping', (s) => s.overlapping(r, except).map((e) => e.id)],
    ['edgesIn', (s) => s.edgesIn(r, except)],
    ['labels', (s) => Array.from(s.labels(view, scale, editing))],
    ['grips', (s) => s.grips(selected)],
    ['ghosts', (s) => Array.from(s.ghosts(selected, affines, 400))],
    ['transformEntities', (s) => s.transformEntities(selectedEntities, affines)],
    ['stretchGhosts', (s) => Array.from(s.stretchGhosts(selected, w, dx, dy))],
    ['measure', (s) => s.measure(selected)],
    ['extent', (s) => [s.extent(selected), s.extent()]],
    ['inBox', (s) => s.inBox(view)],
    ['drawn', (s) => Array.from(s.drawn(list, oriented, clip))],
    ['measures', (s) => Array.from(s.measures(list))],
    ...(target
      ? ([
          ['trim', (s) => s.trim(target, p, view, chosen)],
          ['extend', (s) => s.extend(target, p, view, chosen)],
        ] as [string, (s: PickIndex) => unknown][])
      : []),
  ];
  for (const [what, q] of queries) {
    const got = JSON.stringify(toJson(q(live)));
    const want = JSON.stringify(toJson(q(fresh)));
    if (got !== want) return `${what}: ${got.slice(0, 300)} ≠ ${want.slice(0, 300)}\n    imleç ${JSON.stringify(p)}, tol ${tol}`;
  }
  return null;
}

/** A random edit through the document's API (or its layers). */
function edit(g: Gen, doc: CadDocument): void {
  const ids = [...doc.all()].map((e) => e.id);
  const some = () => (ids.length ? g.pick(ids) : -1);
  const move = (id: number) => {
    const e = doc.get(id);
    if (e) doc.update(id, transformEntity(e, translation(g.num(-5, 5), g.num(-5, 5))));
  };
  switch (g.int(0, 10)) {
    case 0:
      doc.add(sceneEntity(g));
      break;
    case 1:
      move(some());
      break;
    case 2:
      doc.remove(ids.filter(() => g.chance(0.05)));
      break;
    case 3:
      doc.undo();
      break;
    case 4:
      doc.redo();
      break;
    case 5:
      doc.transact('Depo', () => {
        for (let i = g.int(1, 6); i > 0; i--) {
          const k = g.int(0, 2);
          if (k === 0) doc.add(sceneEntity(g));
          else if (k === 1) move(some());
          else doc.remove([some()]);
        }
      });
      break;
    case 6: {
      // The scene's layer nodes (groups too), or the sample project's own layers.
      const nodes = SCENE_LAYER_IDS.filter((id) => doc.layers.get(id));
      const l = nodes.length ? g.pick(nodes) : g.pick(doc.layers.leaves()).id;
      if (g.chance(0.5)) doc.layers.toggleVisible(l);
      else doc.layers.toggleLocked(l);
      break;
    }
    case 7: {
      const put: Entity[] = [];
      for (const id of ids.filter(() => g.chance(0.02))) {
        const e = doc.get(id);
        if (e) put.push(transformEntity(e, translation(1, -1)));
      }
      put.push({ ...sceneEntity(g), id: doc.allocateId() } as Entity);
      doc.applyExternal({ put, remove: ids.filter(() => g.chance(0.02)) });
      break;
    }
    case 8:
      doc.load(Array.from({ length: g.int(1, 5) }, () => sceneEntity(g)));
      break;
    case 9: {
      const id = some();
      if (doc.get(id)) doc.update(id, { attrs: { Not: String(g.int(0, 9)) } });
      break;
    }
    default:
      doc.remove([some()]);
  }
}

function follow(doc: CadDocument, g: Gen): string[] {
  const live = new PickIndex(doc);
  const failures: string[] = [];
  for (let i = 0; i < ROUNDS && failures.length < 5; i++) {
    if (g.chance(0.6)) edit(g, doc);
    const order = [...doc.all()].map((e) => e.id);
    const got = live.ids();
    if (JSON.stringify(got) !== JSON.stringify(order)) {
      failures.push(`sıra ${i}. turda ayrıldı: ${got.length} / ${order.length} nesne`);
      break;
    }
    const fresh = new PickIndex(doc);
    const r = compare(g, doc, live, fresh);
    fresh.dispose();
    if (r) failures.push(`${i}. tur, ${r}`);
  }
  live.dispose();
  return failures;
}

describe('the geometry store follows the document', () => {
  it('answers like a store built afresh on the sample project, through edits', () => {
    expect(follow(createSampleProject(), new Gen(20260924)).join('\n')).toBe('');
  }, 120_000);

  it('answers like a store built afresh on random objects and layers, through edits', () => {
    const g = new Gen(81);
    expect(follow(sceneDocument(g, 400), g).join('\n')).toBe('');
  }, 120_000);
});
