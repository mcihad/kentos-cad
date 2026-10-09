import { bulgePathOutline } from '../../model/geom/bulge';
import type { Entity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import type { NetworkLine } from '../../model/networkAnswers';
import { networkInput } from '../../model/networkInput';
import { CoreStore, type CoreNetwork } from '../../wasm/core';
import { packEntities } from '../../wasm/pack';
import type { NetworkQuestion, NetworkReply, NetworkRequest, XY } from './protocol';

/**
 * The network worker's work (docs/adr/0209 §12), apart from its entry so tests drive it without a Worker: a network
 * is built from the objects the page sent (packed into a store of the worker's own, its expressions evaluated there:
 * model/networkInput.ts), kept, and asked; the store goes once it is built. Answers carry their paths ready to draw.
 */

type Post = (reply: NetworkReply, transfer?: Transferable[]) => void;

/** Paths as tools/preview.ts `strokePaths` reads them: `flags, n, x0, y0, …`, flags 0 open, 1 closed. */
class Paths {
  private readonly out: number[] = [];

  /** A way, its arcs drawn as short segments. */
  line(l: NetworkLine, closed = false): void {
    this.points(outline(l.pts.map(([x, y]) => ({ x, y })), l.bulges, closed), closed);
  }

  points(pts: readonly Vec2[], closed: boolean): void {
    this.out.push(closed ? 1 : 0, pts.length);
    for (const p of pts) this.out.push(p.x, p.y);
  }

  done(): Float64Array {
    return Float64Array.from(this.out);
  }
}

/** A path's points with its arcs as short segments; straight paths as they are (no call to the core). */
function outline(pts: readonly Vec2[], bulges: readonly number[], closed: boolean): readonly Vec2[] {
  return bulges.some((b) => b !== 0) ? bulgePathOutline(pts, bulges, closed) : pts;
}

/** A ring of a polygon geometry. */
interface Ring {
  pts: Vec2[];
  bulges?: number[];
}

/**
 * Filled areas as `band, rings, n0, x0, y0, …, n1, …` each: a service area's polygons with their parts and holes, drawn
 * even–odd.
 */
function fillsOf(areas: readonly { band: number; shape: { pts: Vec2[]; bulges?: number[]; holes?: Ring[]; parts?: (Ring & { holes?: Ring[] })[] } | null }[]): Float64Array {
  const out: number[] = [];
  for (const a of areas) {
    if (!a.shape) continue;
    const rings: Ring[] = [a.shape, ...(a.shape.holes ?? [])];
    for (const p of a.shape.parts ?? []) rings.push(p, ...(p.holes ?? []));
    out.push(a.band, rings.length);
    for (const r of rings) {
      const pts = outline(r.pts, r.bulges ?? [], true);
      out.push(pts.length);
      for (const p of pts) out.push(p.x, p.y);
    }
  }
  return Float64Array.from(out);
}

const NONE: Float64Array = new Float64Array(0);
const json = (pts: readonly XY[]): string => JSON.stringify(pts);

export class NetworkHost {
  private readonly nets = new Map<string, CoreNetwork>();

  handle(msg: NetworkRequest, post: Post): void {
    if (msg.type === 'drop') {
      this.nets.get(msg.network)?.free();
      this.nets.delete(msg.network);
      return;
    }
    try {
      if (msg.type === 'build') this.build(msg, post);
      else this.ask(msg.id, msg.network, msg.question, post);
    } catch (err) {
      post({ type: 'error', id: msg.id, message: (err as Error).message ?? String(err) });
    }
  }

  private build(msg: Extract<NetworkRequest, { type: 'build' }>, post: Post): void {
    const t0 = performance.now();
    const store = new CoreStore();
    try {
      const packed = packEntities(msg.entities);
      store.putPacked(packed.nums, packed.strings);
      const byLayer = new Map<string, Entity[]>();
      for (const e of msg.entities) {
        const list = byLayer.get(e.layerId);
        if (list) list.push(e);
        else byLayer.set(e.layerId, [e]);
      }
      const names = new Map(msg.layers);
      const input = networkInput(msg.def, { byLayer: (id) => byLayer.get(id) ?? [], layerName: (id) => names.get(id) ?? id }, store);
      const net = store.buildNetwork(JSON.stringify(msg.def), input.edgeIds, input.edgeValues, input.junctionIds, input.junctionValues);
      this.nets.get(msg.network)?.free();
      this.nets.set(msg.network, net);
      const summary = JSON.parse(net.summary());
      post({ type: 'built', id: msg.id, summary: { ...summary, problems: input.problems, ms: performance.now() - t0 } });
    } finally {
      store.dispose();
    }
  }

  private ask(id: number, network: string, q: NetworkQuestion, post: Post): void {
    const net = this.nets.get(network);
    if (!net) throw new Error('Ağ kurulmadan soruldu.');
    const answer = (text: string, paths: Float64Array = NONE, fills: Float64Array = NONE) => post({ type: 'answer', id, json: text, paths, fills }, [paths.buffer as ArrayBuffer, fills.buffer as ArrayBuffer].filter((b) => b.byteLength));
    switch (q.kind) {
      case 'locate':
        return answer(net.locate(q.at[0], q.at[1], q.reach));
      case 'route': {
        const text = net.route(json(q.stops), json(q.barriers), q.reach, q.cost, q.reorder);
        const r = JSON.parse(text) as { line?: NetworkLine };
        const paths = new Paths();
        if (r.line) paths.line(r.line);
        return answer(text, paths.done());
      }
      case 'treeFrom':
        return answer(JSON.stringify(net.treeFrom(q.at[0], q.at[1], q.reach, q.cost, json(q.barriers))));
      case 'pathTo': {
        const nums = net.pathTo(q.at[0], q.at[1], q.reach);
        if (!nums.length) return answer('null');
        const n = nums[1];
        const pts: Vec2[] = [];
        for (let i = 0; i < n; i++) pts.push({ x: nums[2 + 2 * i], y: nums[3 + 2 * i] });
        const bulges = Array.from(nums.subarray(2 + 2 * n));
        const paths = new Paths();
        paths.points(outline(pts, bulges, false), false);
        return answer(JSON.stringify({ cost: nums[0] }), paths.done());
      }
      case 'area': {
        const text = net.area(json(q.facilities), JSON.stringify(q.breaks), q.reach, q.cost, q.toward, q.separate, json(q.barriers), q.trim, q.rings, q.areas);
        const r = JSON.parse(text) as { lines?: { line: NetworkLine }[]; areas?: Parameters<typeof fillsOf>[0] };
        const paths = new Paths();
        for (const l of r.lines ?? []) paths.line(l.line);
        return answer(text, paths.done(), r.areas ? fillsOf(r.areas) : NONE);
      }
      case 'nearest': {
        const text = net.nearest(json(q.origins), json(q.targets), q.reach, q.k, q.cutoff, q.cost, q.reverse, json(q.barriers), q.paths);
        const r = JSON.parse(text) as unknown;
        const paths = new Paths();
        if (Array.isArray(r)) for (const row of r as { line: NetworkLine | null }[][]) for (const f of row) if (f.line) paths.line(f.line);
        return answer(text, paths.done());
      }
      case 'trace': {
        const text = net.trace(json(q.starts), json(q.barriers), q.reach, q.trace);
        const r = JSON.parse(text) as { lines?: NetworkLine[]; unfedLines?: NetworkLine[] };
        const paths = new Paths();
        for (const l of r.lines ?? []) paths.line(l);
        for (const l of r.unfedLines ?? []) paths.line(l);
        return answer(text, paths.done());
      }
      case 'check':
        return answer(net.check());
    }
  }
}
