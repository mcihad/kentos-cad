import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/network/v1/cases.json?raw';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import { CadDocument } from '../model/document';
import { entityArea, type Entity, type EntityGeometry } from '../model/entities';
import { LayerStore } from '../model/layers';
import type { NetworkCheck, NetworkFound, NetworkNearest, NetworkRoute, NetworkServiceArea, NetworkTrace } from '../model/networkAnswers';
import type { NetworkReply, NetworkRequest, XY } from '../io/network/protocol';
import { NetworkHost } from '../io/network/handle';
import { NetworkService, type NetworkWorkerLike } from './networks';

/**
 * Ağ analizi on the web (docs/adr/0209 §3–§9, §12) against the independent reference's cases
 * (fixtures/network/v1/cases.json, scripts/fixtures/network_cases.py), the whole way the page asks: each scene drawn as
 * a drawing (edges on `yol`, their direction and costs as attributes, a closed edge by the network's expression;
 * each junction on a layer of its own with its role, a closed valve by its layer's expression), its network built from the drawing's
 * objects (model/networkInput.ts) in the worker's host through the WASM core, and every question asked as the tools
 * and the İşlemler tools ask it. The core runs the same file natively (crates/shared/geometry-core/tests/all/network.rs);
 * Python through its JSON (python/tests/test_network.py).
 */
const file = JSON.parse(text);

function near(got: number, want: number, what: string): void {
  expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(1e-9 * Math.max(Math.abs(want), 1));
}

/** A fake worker over the host: counts what is built. */
function fakeWorker(builds: string[]): NetworkWorkerLike {
  const host = new NetworkHost();
  const w: NetworkWorkerLike = {
    onmessage: null,
    onerror: null,
    terminate: () => {},
    postMessage(m: NetworkRequest) {
      if (m.type === 'build') builds.push(m.network);
      queueMicrotask(() => host.handle(m, (reply: NetworkReply) => w.onmessage?.({ data: reply })));
    },
  };
  return w;
}

/** A scene as a drawing and its network; fixture ids → the drawing's. */
function drawing(scene: Record<string, any>) {
  // A layer for each junction, in the scene's order: the junctions go to the core in it, each with its layer's role.
  const own = (scene.junctions ?? []).map((j: { id: number }) => ({ id: `j${j.id}`, name: `Düğüm ${j.id}` }));
  const doc = new CadDocument({
    name: scene.name,
    layers: new LayerStore([{ id: 'yol', name: 'Yol' }, ...own, { id: 'diger', name: 'Diğer' }], 'diger'),
    origin: { x: 500000, y: 4400000 },
  });
  const rules = scene.rules;
  const field = rules.direction.kind === 'field' ? rules.direction.field : null;
  const edges = new Map<number, number>();
  for (const e of scene.edges) {
    const attrs: Record<string, string> = {};
    if (field && e.direction !== null) attrs[field] = e.direction;
    (rules.costs ?? []).forEach((c: { field: string }, i: number) => {
      if (e.costs[i] !== null && e.costs[i] !== undefined) attrs[c.field] = e.costs[i];
    });
    if (e.closed) attrs.kapali = 'evet';
    const pts = e.pts.map(([x, y]: number[]) => ({ x, y }));
    const made = doc.add({ kind: 'polyline', layerId: 'yol', attrs, pts, ...(e.bulges && { bulges: e.bulges }) } as never);
    edges.set(e.id, made.id);
  }
  const junctions = new Map<number, number>();
  for (const j of scene.junctions ?? []) {
    const made = doc.add({ kind: 'point', layerId: `j${j.id}`, attrs: j.closed ? { durum: 'kapalı' } : {}, p: { x: j.p[0], y: j.p[1] } } as never);
    junctions.set(j.id, made.id);
  }
  const def = {
    id: 'ag-1',
    name: scene.name,
    kind: 'road',
    edges: [{ layer: 'yol' }],
    junctions: (scene.junctions ?? []).map((j: { id: number; role: string }) => ({ layer: `j${j.id}`, role: j.role, closed: "durum = 'kapalı'" })),
    connect: rules.connect,
    tolerance: rules.tolerance,
    direction: rules.direction,
    ...(rules.costs && { costs: rules.costs }),
    closed: "kapali = 'evet'",
  } as NetworkDef;
  doc.settings.assign({ networks: [def] });
  return { doc, edges, junctions };
}

describe('Ağ analizi on the web (fixtures/network/v1)', () => {
  it('builds each scene from the drawing and answers as the independent reference', async () => {
    for (const c of file.cases) {
      const scene = c.scene;
      const name = scene.name as string;
      const { doc, edges, junctions } = drawing(scene);
      const builds: string[] = [];
      const nets = new NetworkService(doc, () => fakeWorker(builds));
      const summary = await nets.ready('ag-1');
      expect(summary.nodes, `${name}: nodes`).toBe(c.graph.nodes.length);
      expect(summary.pieces, `${name}: pieces`).toBe(c.graph.pieces.length);
      expect(summary.problems, name).toEqual([]);
      const at = (k: string): XY => [c.places[k].x, c.places[k].y];
      const reach = 5;
      const edgeId = (i: number) => edges.get(scene.edges[i].id)!;
      const junctionId = (i: number) => junctions.get(scene.junctions[i].id)!;
      for (const q of c.questions) {
        const title = `${name}: ${q.title}`;
        const want = q.expect;
        const barriers = (q.barriers ?? []).map(at);
        const cost = q.cost ?? 0;
        if (q.kind === 'route') {
          const { value: r, paths } = await nets.ask<NetworkRoute>('ag-1', { kind: 'route', stops: q.stops.map(at), barriers, reach, cost, reorder: q.reorder ?? 'none' });
          if (want.error) {
            expect('error' in r && r.error, title).toBe(want.error);
            if (want.error === 'unreachable') expect('between' in r && r.between, title).toEqual(want.between);
            continue;
          }
          if ('error' in r) throw new Error(`${title}: ${r.error}`);
          expect(r.order, title).toEqual(want.order);
          near(r.cost, want.cost, title);
          want.totals.forEach((t: number | null, i: number) => (t === null ? expect(r.totals[i], title).toBeNull() : near(r.totals[i]!, t, `${title}: cost ${i}`)));
          expect(paths.length, `${title}: drawn`).toBeGreaterThan(4);
        } else if (q.kind === 'area') {
          const ask = (rings: boolean) =>
            nets.ask<NetworkServiceArea>('ag-1', {
              kind: 'area',
              facilities: q.facilities.map(at),
              breaks: q.breaks,
              reach,
              cost,
              toward: !!q.toward,
              separate: !!q.separate,
              barriers,
              trim: q.trim,
              rings,
              areas: true,
            });
          const [{ value: discs, fills }, { value: rings }] = await Promise.all([ask(false), ask(true)]);
          if ('error' in discs || 'error' in rings) throw new Error(`${title}: refused`);
          expect(discs.lines.length, `${title}: lines`).toBe(want.lines.length);
          const key = (f: number | null | undefined) => f ?? -1;
          const got = discs.lines.map((l) => [key(l.facility), l.band]).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
          const wanted = want.lines.map((l: { facility: number | null; band: number }) => [key(l.facility), l.band]).sort((a: number[], b: number[]) => a[0] - b[0] || a[1] - b[1]);
          expect(got, title).toEqual(wanted);
          expect(discs.areas.length, `${title}: areas`).toBe(want.areas.length);
          discs.areas.forEach((a, i) => {
            const w = want.areas[i];
            expect([key(a.facility), a.band], title).toEqual([key(w.facility), w.band]);
            const area = (g: EntityGeometry | null | undefined) => entityArea({ ...(g as object), id: 0, layerId: '', attrs: {} } as Entity)!;
            const disc = area(a.shape);
            const ring = area(rings.areas[i].shape);
            expect(Math.abs(disc - w.disc), `${title}: disc ${disc} ≠ ${w.disc}`).toBeLessThanOrEqual(1e-5 * w.disc);
            expect(Math.abs(ring - w.ring), `${title}: ring ${ring} ≠ ${w.ring}`).toBeLessThanOrEqual(1e-5 * w.ring);
          });
          expect(fills.length, `${title}: filled`).toBeGreaterThan(0);
        } else if (q.kind === 'closest' || q.kind === 'matrix') {
          const paths = q.kind === 'closest';
          const { value: rows } = await nets.ask<NetworkNearest>('ag-1', {
            kind: 'nearest',
            origins: q.origins.map(at),
            targets: q.targets.map(at),
            reach,
            k: q.k ?? -1,
            cutoff: q.cutoff ?? NaN,
            cost,
            reverse: !!q.reverse,
            barriers,
            paths,
          });
          if (!Array.isArray(rows)) throw new Error(`${title}: refused`);
          expect(rows.length, title).toBe(want.length);
          rows.forEach((row, i) => {
            expect(row.map((f: NetworkFound) => f.target), title).toEqual(want[i].map((w: { target: number }) => w.target));
            row.forEach((f: NetworkFound, k: number) => near(f.cost, want[i][k].cost, title));
            if (paths) for (const f of row) expect(f.line, `${title}: a way`).not.toBeNull();
          });
        } else if (q.kind === 'trace') {
          const { value: t } = await nets.ask<NetworkTrace>('ag-1', { kind: 'trace', starts: q.starts.map(at), barriers, reach, trace: q.trace });
          if ('error' in t) throw new Error(`${title}: ${t.error}`);
          near(t.length, want.length, title);
          expect(t.objects, `${title}: objects`).toEqual([...new Set<number>(want.edges.map(edgeId))]);
          if (q.trace === 'isolation') {
            expect(t.valves.map((v) => v.id), `${title}: valves`).toEqual(want.valves.map(junctionId));
            if (want.unfed) {
              expect(t.unfed, `${title}: unfed`).toEqual([...new Set<number>(want.unfedEdges.map(edgeId))]);
              near(t.unfedLength, want.unfedLength, title);
            }
          }
        } else if (q.kind === 'check') {
          const { value: k } = await nets.ask<NetworkCheck>('ag-1', { kind: 'check' });
          expect([k.nodes, k.pieces, k.deadEnds], title).toEqual([want.nodes, want.pieces, want.deadEnds]);
          near(k.length, want.length, title);
          expect(k.parts.length, title).toBe(want.parts.length);
          k.parts.forEach((p, i) => {
            expect(p[0], title).toBe(want.parts[i][0]);
            near(p[1], want.parts[i][1], title);
          });
          expect(k.problems.map((p) => p.kind), title).toEqual(want.problems.map((p: { kind: string }) => p.kind));
          k.problems.forEach((p, i) => {
            const w = want.problems[i];
            near(p.x, w.at[0], title);
            near(p.y, w.at[1], title);
            // Problem ids are the objects' (fixture edges and junctions by their fixture ids).
            expect(p.ids, title).toEqual(w.ids.map((id: number) => edges.get(id) ?? junctions.get(id) ?? id));
          });
        } else throw new Error(`unknown question ${q.kind}`);
      }
      // Asked again and again, the network was built once.
      expect(builds, name).toEqual(['ag-1']);
      nets.dispose();
    }
  });

  it('builds again only when the network’s layers, the tree or its definition change', async () => {
    const { doc } = drawing(file.cases[0].scene);
    const builds: string[] = [];
    const nets = new NetworkService(doc, () => fakeWorker(builds));
    await nets.ready('ag-1');
    // Another layer's object: not the network's.
    doc.add({ kind: 'point', layerId: 'diger', attrs: {}, p: { x: 500000, y: 4400000 } } as never);
    await nets.ready('ag-1');
    expect(builds).toEqual(['ag-1']);
    // An edge's attribute: built again.
    const first = doc.byLayer('yol')[0];
    doc.update(first.id, { attrs: { ...first.attrs, yon: 'N' } });
    await nets.ready('ag-1');
    expect(builds).toEqual(['ag-1', 'ag-1']);
    // The definition: built again.
    doc.settings.assign({ networks: [{ ...doc.settings.networks.value[0], tolerance: 0.02 }] });
    await nets.ready('ag-1');
    expect(builds.length).toBe(3);
    // Asked twice at once while it must be built: one build.
    doc.update(first.id, { attrs: { ...first.attrs, yon: 'FT' } });
    await Promise.all([nets.ready('ag-1'), nets.ready('ag-1')]);
    expect(builds.length).toBe(4);
    // A network taken away cannot be asked.
    doc.settings.assign({ networks: [] });
    await expect(nets.ready('ag-1')).rejects.toThrow(/projede yok/);
    nets.dispose();
  });

  it('asks the way to the cursor one at a time, the newest waiting', async () => {
    const c = file.cases[0];
    const { doc } = drawing(c.scene);
    const nets = new NetworkService(doc, () => fakeWorker([]));
    const a: XY = [c.places.a.x, c.places.a.y];
    const { value: started } = await nets.ask<boolean>('ag-1', { kind: 'treeFrom', at: a, reach: 5, cost: 0, barriers: [] });
    expect(started).toBe(true);
    const to = (x: number): XY => [x, 4400000];
    const [one, two, three] = await Promise.all([nets.pathTo('ag-1', to(500150), 5), nets.pathTo('ag-1', to(500160), 5), nets.pathTo('ag-1', to(500170), 5)]);
    // The second waited and was overtaken by the third.
    expect(two).toBeNull();
    near(one!.value!.cost, 100, 'first');
    near(three!.value!.cost, 120, 'third');
    expect(three!.paths[0]).toBe(0);
    // Far from the network: an answer without a way.
    const far = await nets.pathTo('ag-1', [400000, 4300000], 5);
    expect(far!.value).toBeNull();
    expect(nets.busy.value).toBe(false);
    nets.dispose();
  });
});
