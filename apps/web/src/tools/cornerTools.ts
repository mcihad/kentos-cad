import { entityGeometry, type Entity, type EntityGeometry, type LineEntity, type NewEntity, type PolylineEntity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { bulgeAt } from '../model/geom/bulge';
import { chamferLines, cornerOfPath, filletLines } from '../model/ops/fillet';
import { nearestSegment } from '../model/ops/vertex';
import type { ViewTransform } from '../viewport/Camera';
import { chamferLine, filletArc, filletRadiusFor, linesCornerAt, offsetAlong, pulledDistance, vertexCorner, type CornerGeom } from './constructions';
import { parseNumber } from './coordinateInput';
import { EdgePickTool } from './edgeTools';
import { drawTag, strokeGeometry, strokePath } from './preview';
import type { ToolPointer } from './Tool';

type CornerOp = { radius: number } | { d1: number; d2: number };

interface CornerPlan {
  updates: { entity: Entity; geometry: EntityGeometry }[];
  /** New piece (fillet arc or chamfer cut) carrying the look of `like`. */
  add: { like: Entity; geometry: EntityGeometry } | null;
}

/**
 * A corner that can be rounded or cut: a polyline/polygon vertex between
 * two straight segments, or the meeting point of two lines (their
 * intersection when they do not touch). Its geometry (`u1`/`u2` along the
 * kept sides, `reach` of the shorter one, the angle `phi`) comes from the
 * core; the tool adds the entities and how to apply an operation.
 */
interface Corner extends CornerGeom {
  entities: Entity[];
  plan(op: CornerOp): CornerPlan | { error: string };
}

const HOVER_PX = 12;

/** The corner's geometry alone, for the core (its entities stay out of the call). */
const geomOf = (c: Corner): CornerGeom => ({ at: c.at, u1: c.u1, u2: c.u2, reach: c.reach, phi: c.phi });

function pathCorner(e: PolylineEntity, i: number): Corner | null {
  const n = e.pts.length;
  const closed = e.kind === 'polygon';
  if (!closed && (i <= 0 || i >= n - 1)) return null;
  const iPrev = (i - 1 + n) % n;
  const g = vertexCorner(e.pts[iPrev], e.pts[i], e.pts[(i + 1) % n], bulgeAt(e.bulges, iPrev), bulgeAt(e.bulges, i));
  if (!g) return null;
  return {
    ...g,
    entities: [e],
    plan: (op) => {
      const r = cornerOfPath(e.pts, e.bulges, closed, i, op);
      if ('error' in r) return r;
      return { updates: [{ entity: e, geometry: { kind: e.kind, pts: r.pts, ...(r.bulges && { bulges: r.bulges }) } }], add: null };
    },
  };
}

/** Corner of two lines; each keeps the side its pick point is on. */
function linesCorner(l1: LineEntity, p1: Vec2, l2: LineEntity, p2: Vec2): Corner | null {
  const g = linesCornerAt(l1.a, l1.b, p1, l2.a, l2.b, p2);
  if (!g) return null;
  return {
    ...g,
    entities: [l1, l2],
    plan: (op) => {
      const r = 'radius' in op ? filletLines(l1, p1, l2, p2, op.radius) : chamferLines(l1, p1, l2, p2, op.d1, op.d2);
      if ('error' in r) return r;
      const extra: EntityGeometry | null = 'arc' in r ? r.arc && { kind: 'arc', ...r.arc } : r.cut && { kind: 'line', ...r.cut };
      return {
        updates: [
          { entity: l1, geometry: { kind: 'line', a: r.line1.a, b: r.line1.b } },
          { entity: l2, geometry: { kind: 'line', a: r.line2.a, b: r.line2.b } },
        ],
        add: extra && { like: l1, geometry: extra },
      };
    },
  };
}

const farEnd = (l: LineEntity, near: Vec2) => (dist(l.a, near) <= dist(l.b, near) ? l.b : l.a);

/**
 * Fillet and chamfer. Point at a corner (a polyline vertex, or where two
 * lines meet), click, then pull the mouse along a side: the rounding or
 * cut grows live and a second click applies it. A typed value applies
 * exactly; Enter or right click reuses the last one. Two lines that do
 * not touch are picked one after the other.
 */
abstract class CornerTool extends EdgePickTool {
  /** AutoCAD TRIMMODE: off leaves the sides alone and only adds the arc or cut. Shared by fillet and chamfer. */
  private static trimSides = true;
  private stage: 'find' | 'second' | 'size' = 'find';
  private corner: Corner | null = null;
  private first: { entity: LineEntity | PolylineEntity; pick: Vec2 } | null = null;
  private mouse: Vec2 | null = null;
  protected override editable = (e: Entity) => (e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon') && !this.ctx.doc.layers.isLocked(e.layerId);

  protected abstract readonly title: string;
  /** Operation for a size pulled out with the mouse (distance t from the corner along a side). */
  protected abstract opForPull(t: number, c: Corner): CornerOp;
  /** Parses a typed value; null when not understood. */
  protected abstract opForText(text: string): CornerOp | null;
  protected abstract lastOp(): CornerOp | null;
  protected abstract remember(op: CornerOp): void;
  protected abstract describe(op: CornerOp): string;
  protected abstract done(op: CornerOp): string;
  protected abstract prompts(): { find: string; second: string; size: string };
  /** The arc or cut alone, for "Kırp: hayır": sides of the corner stay untouched. */
  protected abstract piece(op: CornerOp, c: Corner): EntityGeometry | null;

  protected refresh(): void {
    const p = this.prompts();
    const step = this.stage === 'find' ? p.find : this.stage === 'second' ? p.second : p.size;
    const trim = `Kırp (K): ${CornerTool.trimSides ? 'evet' : 'hayır'}`;
    // Options live in one bracket: append to the step's own, or open one.
    this.prompt.set(`${this.title}: ${this.stage === 'second' ? step : step.endsWith(']') ? `${step.slice(0, -1)} / ${trim}]` : `${step} [${trim}]`}`);
    this.ctx.view.requestOverlay();
  }

  /** Nearest corner within reach of the cursor: polyline vertices and shared line ends. */
  private cornerAt(p: ToolPointer): Corner | null {
    const { view, doc } = this.ctx;
    const tol = view.worldTolerance(HOVER_PX);
    const box = { minX: p.raw.x - tol, minY: p.raw.y - tol, maxX: p.raw.x + tol, maxY: p.raw.y + tol };
    const near = view
      .pickRect(box, true)
      .map((id) => doc.get(id))
      .filter((e): e is LineEntity | PolylineEntity => !!e && this.editable(e));
    let best: { c: Corner; d: number } | null = null;
    const consider = (c: Corner | null) => {
      if (!c) return;
      const d = dist(c.at, p.raw);
      if (d <= tol && (!best || d < best.d)) best = { c, d };
    };
    const same = view.worldTolerance(2);
    for (const e of near) {
      if (e.kind === 'line') {
        for (const end of [e.a, e.b]) {
          if (dist(end, p.raw) > tol) continue;
          for (const o of near) {
            if (o === e || o.kind !== 'line') continue;
            const oEnd = dist(o.a, end) <= same ? o.a : dist(o.b, end) <= same ? o.b : null;
            if (oEnd) consider(linesCorner(e, farEnd(e, end), o, farEnd(o, oEnd)));
          }
        }
      } else e.pts.forEach((_, i) => dist(e.pts[i], p.raw) <= tol && consider(pathCorner(e, i)));
    }
    return (best as { c: Corner } | null)?.c ?? null;
  }

  override pointerMove(p: ToolPointer): void {
    this.mouse = p.raw;
    if (this.stage === 'size') return this.ctx.view.requestOverlay();
    if (this.stage === 'find') {
      this.corner = this.cornerAt(p);
      if (this.corner) {
        this.hover = null;
        this.ctx.selection.hover.set(null);
        return this.ctx.view.requestOverlay();
      }
    }
    super.pointerMove(p);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.stage === 'size') return this.commit(this.pulled());
    if (this.stage === 'find') {
      const c = this.cornerAt(p);
      if (c) return this.lock(c);
    }
    const e = this.ctx.view.pickEdge(p.screen, this.editable) as LineEntity | PolylineEntity | null;
    if (!e) return this.ctx.log.warn('Bir köşeye ya da düzenlenebilir bir çizgi veya çoklu çizgi kenarına tıklayın.');
    if (this.stage === 'find') {
      this.first = { entity: e, pick: p.raw };
      this.stage = 'second';
      this.ctx.selection.hover.set(null);
      return this.refresh();
    }
    const c = this.cornerOfPicks(this.first!, { entity: e, pick: p.raw });
    if ('error' in c) return this.ctx.log.warn(c.error);
    this.lock(c);
  }

  private cornerOfPicks(a: { entity: LineEntity | PolylineEntity; pick: Vec2 }, b: { entity: LineEntity | PolylineEntity; pick: Vec2 }): Corner | { error: string } {
    if (a.entity.kind === 'line' && b.entity.kind === 'line') {
      if (a.entity.id === b.entity.id) return { error: 'İkinci çizgi ilkinden farklı olmalı.' };
      return linesCorner(a.entity, a.pick, b.entity, b.pick) ?? { error: 'Çizgiler paralel ya da seçilen tarafta çizgi yok.' };
    }
    if (a.entity.id !== b.entity.id || a.entity.kind === 'line') return { error: 'Çoklu çizgide köşe için köşenin kendisine ya da aynı nesnenin iki komşu kenarına tıklayın.' };
    const e = a.entity as PolylineEntity;
    const n = e.pts.length;
    const i = nearestSegment(e, a.pick);
    const j = nearestSegment(e, b.pick);
    let v = -1;
    if (j === i + 1) v = j;
    else if (i === j + 1) v = i;
    else if (e.kind === 'polygon' && ((i === n - 1 && j === 0) || (j === n - 1 && i === 0))) v = 0;
    if (v < 0) return { error: 'Seçilen kenarlar komşu değil; ortak köşesi olan iki kenar seçin.' };
    return pathCorner(e, v) ?? { error: 'Bu köşenin kenarlarından biri yay; yalnızca düz kenarlar arasındaki köşe işlenebilir.' };
  }

  private lock(c: Corner): void {
    this.corner = c;
    this.stage = 'size';
    this.hover = null;
    this.ctx.selection.hover.set(null);
    this.refresh();
  }

  /** How far the cursor has been pulled along the nearer side, rounded to a step that suits the zoom (four pixels). */
  private pulledDistance(): number {
    return pulledDistance(geomOf(this.corner!), this.mouse, this.ctx.view.worldTolerance(4));
  }

  private pulled(): CornerOp {
    return this.opForPull(this.pulledDistance(), this.corner!);
  }

  input(text: string): boolean {
    if (text.trim().toLocaleUpperCase('tr-TR') === 'K' && this.stage !== 'second') {
      CornerTool.trimSides = !CornerTool.trimSides;
      this.refresh();
      return true;
    }
    if (this.stage !== 'size') return false;
    const op = this.opForText(text);
    if (!op) return false;
    this.commit(op);
    return true;
  }

  confirm(): void {
    if (this.stage !== 'size') return this.ctx.tools.exit();
    const last = this.lastOp();
    // Two separate lines with no size yet: join them at their intersection.
    const fallback: CornerOp | null = this.corner!.entities.length === 2 ? this.opForPull(0, this.corner!) : null;
    const op = last ?? fallback;
    if (!op) return this.ctx.log.warn('Önce fareyle boyutu gösterip tıklayın ya da bir değer yazın.');
    this.commit(op);
  }

  cancel(): boolean {
    if (this.stage === 'find') return false;
    this.reset();
    return true;
  }

  private reset(): void {
    this.stage = 'find';
    this.corner = null;
    this.first = null;
    this.refresh();
  }

  private commit(op: CornerOp): void {
    const plan = this.corner!.plan(op);
    if ('error' in plan) return this.ctx.log.warn(plan.error);
    const { doc } = this.ctx;
    if (!CornerTool.trimSides) {
      const extra = this.piece(op, this.corner!);
      if (!extra) return this.ctx.log.warn('Kırpmadan çalışırken sıfırdan büyük bir boyut verin; yoksa eklenecek bir şey yok.');
      const like = this.corner!.entities[0];
      doc.transact(this.title, () => doc.add({ ...extra, layerId: like.layerId, color: like.color, attrs: {} } as NewEntity));
      this.remember(op);
      this.ctx.log.success(`${this.done(op)} Kenarlar kırpılmadı.`);
      return this.reset();
    }
    doc.transact(this.title, () => {
      // Whole geometry replaced: a shape without arcs must not keep old bulges.
      for (const u of plan.updates) doc.update(u.entity.id, { bulges: undefined, ...u.geometry } as Partial<Entity>);
      if (plan.add) doc.add({ ...plan.add.geometry, layerId: plan.add.like.layerId, color: plan.add.like.color, attrs: {} } as NewEntity);
    });
    this.remember(op);
    this.ctx.log.success(this.done(op));
    this.reset();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.stage === 'second' && this.first) {
      strokeGeometry(g, view, entityGeometry(this.first.entity), { color: pal.accent, width: 2 });
      return;
    }
    const c = this.corner;
    if (!c) return;
    for (const e of c.entities) strokeGeometry(g, view, entityGeometry(e), { color: pal.accent, width: 1.5 });
    const s = view.worldToScreen(c.at);
    g.save();
    g.strokeStyle = pal.accent;
    g.lineWidth = 2;
    g.beginPath();
    g.arc(s.x, s.y, 7, 0, Math.PI * 2);
    g.stroke();
    g.restore();
    if (this.stage === 'find') return drawTag(g, s, ['Köşeyi seçmek için tıklayın'], pal.accent, pal.labelHalo);
    const t = this.pulledDistance();
    const op = this.pulled();
    const plan = c.plan(op);
    // Where the rounding/cut starts on each side.
    for (const u of [c.u1, c.u2]) strokePath(g, view, [c.at, offsetAlong(c.at, u, t)], { color: pal.snap, width: 2 });
    if ('error' in plan) return;
    if (!CornerTool.trimSides) {
      const extra = this.piece(op, c);
      if (extra) strokeGeometry(g, view, extra, { color: pal.accent, width: 2.5 });
    } else {
      for (const u of plan.updates) strokeGeometry(g, view, u.geometry, { color: pal.accent, dash: [5, 3], width: 2 });
      if (plan.add) strokeGeometry(g, view, plan.add.geometry, { color: pal.accent, width: 2.5 });
    }
    if (this.mouse) drawTag(g, view.worldToScreen(this.mouse), [this.describe(op), 'Tıklayın: uygula'], pal.accent, pal.labelHalo);
  }
}

export class FilletTool extends CornerTool {
  readonly id = 'fillet';
  protected readonly title = 'Köşe yuvarla';
  private static last: number | null = null;

  protected prompts() {
    const f = this.ctx.format;
    const last = FilletTool.last;
    return {
      find: `yuvarlanacak köşeye tıklayın ya da sırayla iki çizgi seçin${last !== null ? ` [son yarıçap ${f.length(last)}]` : ''}`,
      second: 'ikinci çizgiyi ya da aynı çoklu çizginin komşu kenarını seçin',
      size: `fareyi kenar boyunca kaydırıp tıklayın ya da yarıçap yazın${last !== null ? ` [Son yarıçap (Enter): ${f.length(last)}]` : ''}`,
    };
  }
  protected opForPull(t: number, c: Corner): CornerOp {
    // Pulled distance is where the arc meets the side (tangent length).
    return { radius: filletRadiusFor(t, c.phi) };
  }
  protected opForText(text: string): CornerOp | null {
    const n = parseNumber(text);
    return n !== null && n >= 0 && !/[,;@<]/.test(text) ? { radius: n } : null;
  }
  protected lastOp(): CornerOp | null {
    return FilletTool.last !== null ? { radius: FilletTool.last } : null;
  }
  protected remember(op: CornerOp): void {
    if ('radius' in op) FilletTool.last = op.radius;
  }
  protected describe(op: CornerOp): string {
    return 'radius' in op ? `Yarıçap ${this.ctx.format.length(op.radius)}` : '';
  }
  protected done(op: CornerOp): string {
    return 'radius' in op && op.radius > 0 ? `Köşe ${this.ctx.format.length(op.radius)} yarıçapla yuvarlandı.` : 'Çizgiler köşede birleştirildi.';
  }
  protected piece(op: CornerOp, c: Corner): EntityGeometry | null {
    // The short arc between the tangent points, counter-clockwise.
    const arc = 'radius' in op ? filletArc(geomOf(c), op.radius) : null;
    return arc && { kind: 'arc', ...arc };
  }
}

export class ChamferTool extends CornerTool {
  readonly id = 'chamfer';
  protected readonly title = 'Pah';
  private static last: { d1: number; d2: number } | null = null;

  private text(op: { d1: number; d2: number }): string {
    const f = this.ctx.format;
    return op.d1 === op.d2 ? f.length(op.d1) : `${f.length(op.d1, false)} ile ${f.length(op.d2)}`;
  }
  protected prompts() {
    const last = ChamferTool.last;
    return {
      find: `kesilecek köşeye tıklayın ya da sırayla iki çizgi seçin${last ? ` [son mesafe ${this.text(last)}]` : ''}`,
      second: 'ikinci çizgiyi ya da aynı çoklu çizginin komşu kenarını seçin',
      size: `fareyi kenar boyunca kaydırıp tıklayın ya da mesafe yazın (d ya da d1,d2)${last ? ` [Son mesafe (Enter): ${this.text(last)}]` : ''}`,
    };
  }
  protected opForPull(t: number): CornerOp {
    return { d1: t, d2: t };
  }
  protected opForText(text: string): CornerOp | null {
    const m = text.trim().match(/^(\d+(?:\.\d+)?)(?:\s*[,;]\s*(\d+(?:\.\d+)?))?$/);
    return m ? { d1: +m[1], d2: m[2] !== undefined ? +m[2] : +m[1] } : null;
  }
  protected lastOp(): CornerOp | null {
    return ChamferTool.last;
  }
  protected remember(op: CornerOp): void {
    if ('d1' in op) ChamferTool.last = op;
  }
  protected describe(op: CornerOp): string {
    return 'd1' in op ? `Pah ${this.text(op)}` : '';
  }
  protected done(op: CornerOp): string {
    return 'd1' in op && (op.d1 > 0 || op.d2 > 0) ? `Pah kırıldı: ${this.text(op)}.` : 'Çizgiler köşede birleştirildi.';
  }
  protected piece(op: CornerOp, c: Corner): EntityGeometry | null {
    const cut = 'd1' in op ? chamferLine(geomOf(c), op.d1, op.d2) : null;
    return cut && { kind: 'line', ...cut };
  }
}
