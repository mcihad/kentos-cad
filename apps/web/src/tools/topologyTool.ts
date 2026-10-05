import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { RingGeometry } from '../contracts/generated/RingGeometry';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity, type EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Edge } from '../model/geom/intersect';
import { entityEdges } from '../model/ops/edges';
import { topologyArcPath, topologyClean, topologyPathArc, type TopoObject, type TopoPath, type TopoResult, type TopoWorks } from '../model/ops/topology';
import { elevatedPaths } from '../product/elevation';
import type { ViewTransform } from '../viewport/Camera';
import { ringMark } from './constructPreview';
import { parseLength } from './coordinateInput';
import { uidOf, writeEdit } from './editCommand';
import { MAX_GHOSTS } from './modifyTools';
import { drawTag, strokeGeometry, strokePath } from './preview';
import { markVertices } from './reshapePreview';
import type { Tool, ToolPointer } from './Tool';

/**
 * Topolojik temizlik (docs/adr/0148): line work and area outlines put right within a tolerance the user gives,
 * shown first and written in one step. The finding is the shared core's (`topologyClean`); the desktop's tool is
 * `kentos_interaction::topology`, and both play `fixtures/interaction/v1/topology.json`.
 *
 * - Scope, taken when it starts (§2): the selection, else every object on a visible, unlocked layer. The other
 *   objects on visible layers are supports: never moved, their vertices and edges are where the others go. Points
 *   never move; circles, ellipses and curves are edges only; texts, dimensions, hatches, blocks, leaders and
 *   construction lines take no part; a hidden layer takes none.
 * - The tolerance (metres, at least 1 µm) and the four works are kept for the session (§3): a number typed is the
 *   tolerance (T asks for it), U, K, Z and B turn Uçlar, Köşeler, Uzat and Buda on or off.
 * - Every change is marked where it lands at a fixed size on the screen, the old outlines dashed under the new
 *   ones (§9). Enter, Uygula or a quick right click writes them through `cad.entities.edit` (operation
 *   `topology`), each geometry with its elevations as the core carried them (§6), and leaves; Esc leaves.
 */

const LABEL = 'Topolojik temizlik';
/** The tolerance before any is typed, metres (docs/adr/0148 §3). */
export const FIRST_TOLERANCE = 0.01;
export const FIRST_WORKS: Readonly<TopoWorks> = { ends: true, vertices: false, extend: true, trim: true };
/** The smallest tolerance, metres: the core's “the same place”. */
const LEAST = 1e-6;
/** The changes marked at most. */
const MAX_MARKS = 2000;

/** The kinds the cleanup corrects; the others it takes are supports only (docs/adr/0148 §2). */
const CORRECTED = new Set<Entity['kind']>(['line', 'polyline', 'arc', 'polygon']);
const TAKES_PART = new Set<Entity['kind']>([...CORRECTED, 'point', 'circle', 'ellipse', 'spline']);

/** The letters that turn the works on and off. */
const WORK_KEYS: Record<string, keyof TopoWorks> = { U: 'ends', K: 'vertices', Z: 'extend', B: 'trim' };

/** What the cleanup would write for the drawing as it is, and what it was worked out from. */
interface Plan {
  key: string;
  result: TopoResult;
  /** The objects that change and the geometry each takes, in the drawing's order. */
  edits: { e: Entity; geometry: EditGeometry }[];
  supports: number;
}

export class TopologyTool implements Tool {
  readonly id = 'topology';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  /** The tolerance and the works, kept for the session (docs/adr/0148 §3). */
  static tolerance = FIRST_TOLERANCE;
  static works: TopoWorks = { ...FIRST_WORKS };
  private readonly ctx: AppContext;
  /** The objects it corrects, by id, taken when it starts. */
  private targets = new Set<number>();
  private whole = true;
  /** T: the tolerance is being typed. */
  private typing = false;
  private plan: Plan | null = null;
  /** The finding last said, so a change that finds the same says nothing again. */
  private said = '';
  /** The cursor's world point: the tag beside it says the finding. */
  private hover: Vec2 | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    this.typing = false;
    this.plan = null;
    this.said = '';
    if (!this.takeScope()) return void queueMicrotask(() => this.ctx.tools.exit());
    const plan = this.current();
    this.tell(`${this.whole ? 'bütün çizim: ' : 'seçili '}${this.targets.size} nesne, ${plan.supports} dayanak`);
    this.refresh();
  }

  /**
   * Says the finding when it changed, so the history and the status bar keep the current one; when the tool starts,
   * with its scope and what to do.
   */
  private tell(scope?: string): void {
    const { log, format } = this.ctx;
    const plan = this.current();
    const text = plan.edits.length ? finding(plan.result, format) : `${format.length(TopologyTool.tolerance)} toleransla düzeltilecek bir şey yok`;
    if (!scope && text === this.said) return;
    this.said = text;
    if (plan.edits.length) log.info(`${LABEL}: ${text}.${scope ? ` Enter ile uygulayın (${scope}).` : ''}`);
    else log.warn(`${LABEL}: ${text}${scope ? `; daha büyük bir tolerans yazın (${scope})` : ''}.`);
  }

  /** The objects to correct (docs/adr/0148 §2), what is left out said; false when there are none. */
  private takeScope(): boolean {
    const { doc, log, selection } = this.ctx;
    this.whole = selection.size === 0;
    const chosen = this.whole ? [...doc.all()] : [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
    const seen = chosen.filter((e) => doc.layers.isVisible(e.layerId));
    const correctable = seen.filter((e) => CORRECTED.has(e.kind));
    const open = correctable.filter((e) => !doc.layers.isLocked(e.layerId));
    if (!this.whole) {
      const locked = correctable.length - open.length;
      if (locked) log.warn(`${locked} nesne kilitli katmanda olduğu için düzeltilmez; dayanak olarak kalır.`);
      const out = seen.filter((e) => !TAKES_PART.has(e.kind)).length;
      if (out) log.warn(`${out} nesne topolojik temizliğe katılmaz: yazı, ölçü, tarama, blok, kılavuz ve yardımcı çizgiler girmez.`);
    }
    this.targets = new Set(open.map((e) => e.id));
    if (!open.length) log.warn(`${LABEL}: düzeltilecek çizgi, çoklu çizgi, yay ya da alan yok.`);
    return open.length > 0;
  }

  /** What the plan was worked out from: the drawing's revision, the tolerance and the works. */
  private key(): string {
    const w = TopologyTool.works;
    return `${this.ctx.doc.revision}|${TopologyTool.tolerance}|${+w.ends}${+w.vertices}${+w.extend}${+w.trim}`;
  }

  /** The plan for the drawing as it is, kept until the drawing, the tolerance or the works change. */
  private current(): Plan {
    const key = this.key();
    if (this.plan?.key === key) return this.plan;
    const { doc } = this.ctx;
    const objects: TopoObject[] = [];
    const taken: Entity[] = [];
    // Every object on a visible layer that takes part, in the drawing's order (ties go by it).
    for (const e of doc.all()) {
      if (!doc.layers.isVisible(e.layerId)) continue;
      const o = topoObject(e, !this.targets.has(e.id) || doc.layers.isLocked(e.layerId));
      if (!o) continue;
      objects.push(o);
      taken.push(e);
    }
    const result = topologyClean(objects, TopologyTool.tolerance, TopologyTool.works);
    const edits = result.changed.flatMap(({ object, paths }) => {
      if (samePaths(objects[object].paths, paths)) return [];
      const geometry = geometryOf(taken[object], paths);
      return geometry ? [{ e: taken[object], geometry }] : [];
    });
    this.plan = { key, result, edits, supports: objects.filter((o) => o.fixed).length };
    return this.plan;
  }

  private refresh(): void {
    const { format } = this.ctx;
    const tolerance = format.length(TopologyTool.tolerance);
    if (this.typing) this.prompt.set(`${LABEL}: toleransı yazın, ${format.lengthUnitName} (Enter: ${tolerance})`);
    else {
      const plan = this.current();
      const w = TopologyTool.works;
      const on = (b: boolean) => (b ? 'açık' : 'kapalı');
      const step = plan.edits.length ? finding(plan.result, format) : 'düzeltilecek bir şey yok';
      this.prompt.set(`${LABEL}: ${step} [Tolerans (T): ${tolerance} / Uçlar (U): ${on(w.ends)} / Köşeler (K): ${on(w.vertices)} / Uzat (Z): ${on(w.extend)} / Buda (B): ${on(w.trim)} / Uygula (Enter)]`);
    }
    this.ctx.view.requestOverlay();
  }

  /** The tag follows the cursor; a drawing changed under the tool (an undo) is worked out again. */
  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    if (this.plan && this.plan.key !== this.key()) {
      this.refresh();
      this.tell();
    }
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    if (!this.typing) {
      const work = WORK_KEYS[key];
      if (work) {
        TopologyTool.works = { ...TopologyTool.works, [work]: !TopologyTool.works[work] };
        this.refresh();
        this.tell();
        return true;
      }
      if (key === 'T') {
        this.typing = true;
        this.refresh();
        return true;
      }
    }
    const n = /[,;@<]/.test(text) ? null : parseLength(this.ctx.format, text);
    if (n === null) return false;
    if (!(Number.isFinite(n) && n >= LEAST)) {
      this.ctx.log.warn(`Tolerans en az ${this.ctx.format.plain(LEAST)} ${this.ctx.format.lengthUnitLabel} olmalı.`);
      return true;
    }
    TopologyTool.tolerance = n;
    this.typing = false;
    this.refresh();
    this.tell();
    return true;
  }

  /** Esc while the tolerance is typed goes back to the finding; otherwise the tool leaves. */
  cancel(): boolean {
    if (!this.typing) return false;
    this.typing = false;
    this.refresh();
    return true;
  }

  /** Enter: the tolerance kept while it is typed; otherwise the changes written in one step, and the tool leaves. */
  confirm(): void {
    if (this.typing) {
      this.typing = false;
      return this.refresh();
    }
    const { log, format } = this.ctx;
    const plan = this.current();
    if (!plan.edits.length) {
      log.warn(`${LABEL}: düzeltilecek bir şey yok; hiçbir şey değişmedi.`);
      return this.ctx.tools.exit();
    }
    const changes: EntityEdit[] = plan.edits.map(({ e, geometry }) => ({ kind: 'update', uid: uidOf(this.ctx, e), geometry }));
    if (!writeEdit(this.ctx, 'topology', changes)) return;
    log.success(`${LABEL}: ${done(plan.result)}; ${plan.edits.length} nesne değişti, en büyük kayma ${format.length(plan.result.maxShift)}.`);
    this.ctx.tools.exit();
  }

  /**
   * Each changed object's old outline dashed in the danger colour under its new one in the accent; at every change a
   * mark of fixed size: a ring for a vertex joined or an end moved onto a line (a small ring where it was and a fine
   * dashed line from there), a green ring and the piece added for an end extended, a struck ring where an end is cut.
   */
  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const plan = this.plan;
    if (!plan) return;
    const pal = this.ctx.view.palette;
    for (const { e, geometry } of plan.edits.slice(0, MAX_GHOSTS)) {
      strokeGeometry(g, view, entityGeometry(e), { color: pal.danger, dash: [5, 3], width: 1.5 });
      strokeGeometry(g, view, geometry as unknown as EntityGeometry, { color: pal.accent, width: 2 });
    }
    for (const c of plan.result.changes.slice(0, MAX_MARKS)) {
      if (c.kind === 'extended') {
        strokePath(g, view, [c.from, c.to], { color: pal.snap, width: 2.5 });
        ringMark(g, view, c.to, pal.snap, 5);
      } else if (c.kind === 'trimmed') markVertices(g, view, [c.to], pal.danger, true);
      else {
        // Where the vertex was (a small ring in the danger colour) and the way to where it goes: seen close up.
        strokePath(g, view, [c.from, c.to], { color: pal.accent, dash: [3, 2], width: 1 });
        ringMark(g, view, c.from, pal.danger, 2.5);
        ringMark(g, view, c.to, pal.accent, 5);
      }
    }
    if (this.hover) drawTag(g, view.worldToScreen(this.hover), tagLines(plan, this.ctx.format), pal.accent, pal.labelHalo);
  }
}

/** The counts and their words, now and once written (docs/adr/0148 §9). */
const COUNTS = [
  ['ends', 'uç birleşir', 'uç birleşti'],
  ['vertices', 'köşe birleşir', 'köşe birleşti'],
  ['extended', 'uç uzar', 'uç uzadı'],
  ['trimmed', 'uç kısalır', 'uç kısaldı'],
  ['edges', 'uç kenara taşınır', 'uç kenara taşındı'],
] as const;

/** `4 uç birleşir`, `1 uç uzar` …: the counts that are not naught; `past` once written. */
const counted = (r: TopoResult, past: boolean): string[] => COUNTS.filter(([k]) => r.counts[k] > 0).map(([k, now, then]) => `${r.counts[k]} ${past ? then : now}`);

/** The finding beside the cursor, a count a line, the largest move, and what writes it. */
function tagLines(plan: Plan, format: AppContext['format']): string[] {
  if (!plan.edits.length) return ['Düzeltilecek bir şey yok'];
  return [...counted(plan.result, false), `en büyük kayma ${format.length(plan.result.maxShift)}`, 'Enter: uygula'];
}

/** The counts as the prompt says them: `4 uç birleşir, 1 uç uzar; en büyük kayma 0.030 m`. */
const finding = (r: TopoResult, format: AppContext['format']): string => `${counted(r, false).join(', ')}; en büyük kayma ${format.length(r.maxShift)}`;

/** The counts as the message after writing says them: `4 uç birleşti, 1 uç uzadı`. */
const done = (r: TopoResult): string => counted(r, true).join(', ');

/** A path's bulges as the core takes them: one a vertex, when any bends; none when all are straight. */
const bulgesOf = (pts: readonly Vec2[], bulges: readonly number[] | undefined): { bulges?: number[] } =>
  bulges?.some((b) => b !== 0) ? { bulges: pts.map((_, i) => bulges[i] ?? 0) } : {};

/** A circle as a boundary: two half turns (docs/adr/0148 §2). */
const circleObject = (c: Vec2, r: number): TopoObject => ({
  kind: 'edges',
  fixed: true,
  paths: [{ pts: [{ x: c.x + r, y: c.y }, { x: c.x - r, y: c.y }], bulges: [1, 1], closed: true, zs: [null, null] }],
});

/** An edge of an ellipse's or a curve's outline (its 0.1 mm chords, docs/adr/0149 §5.3) as a path of its own. */
function edgePath(edge: Edge): TopoPath {
  if (edge.kind === 'seg') return { pts: [edge.a, edge.b], closed: false, zs: [null, null] };
  return topologyArcPath({ c: edge.c, r: edge.r, a0: edge.a0, a1: edge.a0 + edge.sweep }) ?? { pts: [], closed: false, zs: [] };
}

/**
 * An object as the cleanup takes it (docs/adr/0148 §2), or null when it takes no part: a line, a polyline and an
 * area by their paths with their elevations (`elevatedPaths`' order), an arc as its two ends and a bulge, a point
 * (always fixed), a circle, an ellipse or a curve as edges (always fixed). The desktop's is
 * `kentos_interaction::topology::object`; the trace checks both give the same.
 */
export function topoObject(e: Entity, fixed: boolean): TopoObject | null {
  const paths = () => elevatedPaths(e).map((p): TopoPath => ({ pts: p.pts, ...bulgesOf(p.pts, p.bulges), closed: p.closed, zs: p.zs }));
  switch (e.kind) {
    case 'line':
      return { kind: 'line', fixed, paths: paths() };
    case 'polyline':
      return { kind: 'polyline', fixed, paths: paths() };
    case 'polygon':
      return { kind: 'area', fixed, paths: paths() };
    case 'arc': {
      const path = topologyArcPath({ c: e.c, r: e.r, a0: e.a0, a1: e.a1 });
      return path ? { kind: 'arc', fixed, paths: [path] } : circleObject(e.c, e.r);
    }
    case 'point':
      return { kind: 'point', fixed: true, paths: [{ pts: [e.p], closed: false, zs: [e.z ?? null] }] };
    case 'circle':
      return circleObject(e.c, e.r);
    case 'ellipse':
    case 'spline':
      return { kind: 'edges', fixed: true, paths: entityEdges(e).map(edgePath).filter((p) => p.pts.length) };
    default:
      return null;
  }
}

/** Whether two lists of paths are the same, bit for bit. */
function samePaths(a: readonly TopoPath[], b: readonly TopoPath[]): boolean {
  const same = <T>(x: readonly T[] | undefined, y: readonly T[] | undefined) => (x ?? []).length === (y ?? []).length && (x ?? []).every((v, i) => v === (y ?? [])[i]);
  return (
    a.length === b.length &&
    a.every((p, k) => {
      const q = b[k];
      return p.closed === q.closed && p.pts.length === q.pts.length && p.pts.every((v, i) => v.x === q.pts[i].x && v.y === q.pts[i].y) && same(p.zs, q.zs) && same(p.bulges, q.bulges);
    })
  );
}

/**
 * The geometry a corrected object takes, as `cad.entities.edit` writes it: its paths back in `elevatedPaths`' order
 * (an area's outer ring, its holes, then each other part's ring and holes), each with its elevations explicit; an
 * arc rebuilt from its ends and bulge. Null for a kind the cleanup does not correct.
 */
export function geometryOf(e: Entity, paths: readonly TopoPath[]): EditGeometry | null {
  const ring = (p: TopoPath): RingGeometry => ({ pts: p.pts, ...bulgesOf(p.pts, p.bulges), zs: p.zs });
  switch (e.kind) {
    case 'line': {
      const [p] = paths;
      return { kind: 'line', a: p.pts[0], b: p.pts[1], zs: p.zs };
    }
    case 'polyline': {
      const [p, ...rest] = paths;
      // Every other part, in `elevatedPaths`' order (docs/adr/0174).
      const parts = rest.map(ring);
      return { kind: 'polyline', pts: p.pts, ...bulgesOf(p.pts, p.bulges), zs: p.zs, ...(parts.length ? { parts } : {}) };
    }
    case 'polygon': {
      let k = 0;
      const next = () => paths[k++];
      const outer = next();
      const holes = (e.holes ?? []).map(() => ring(next()));
      const parts = (e.parts ?? []).map((part) => {
        const own = ring(next());
        const partHoles = (part.holes ?? []).map(() => ring(next()));
        return { ...own, ...(partHoles.length ? { holes: partHoles } : {}) };
      });
      return { kind: 'polygon', pts: outer.pts, ...bulgesOf(outer.pts, outer.bulges), zs: outer.zs, ...(holes.length ? { holes } : {}), ...(parts.length ? { parts } : {}) };
    }
    case 'arc': {
      const arc = topologyPathArc(paths[0]);
      return arc && { kind: 'arc', c: arc.c, r: arc.r, a0: arc.a0, a1: arc.a1 };
    }
    default:
      return null;
  }
}
