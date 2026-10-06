import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { entityOutline, type Entity, type EntityGeometry } from '../model/entities';
import { bulgePathOutline } from '../model/geom/bulge';
import { joinEntities } from '../model/ops/join';
import { joinChain } from '../model/ops/trace';
import type { ViewTransform } from '../viewport/Camera';
import { centerline, pathStation, type Centerline } from './constructions';
import { parseLength } from './coordinateInput';
import { writeObjects } from './createCommand';
import { JoinTool } from './editTools';
import { NO_OBJECT_HERE, NO_ROUTE } from './pointCalcRoute';
import { strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';

/**
 * Orta hat (docs/adr/0190; Netcad's Orta Hat Çiz; the desktop's `kentos_interaction::centerline`): the axis between two
 * sides, a road's edges or a stream's banks. With Zincir a side is the clicked line's chain joined into one path. The
 * axis is the core's (`ops::centerline`); Enter, Uygula or a quick right click writes it as a polyline through
 * `cad.entities.create` (step “Orta hat”) and the tool waits for the next pair. Esc and Ctrl+Z let the last side go.
 */

export const LABEL = 'Orta hat';
const SAME_SIDE = 'İkinci kenar birincisiyle aynı olamaz; öbür kenara tıklayın.';
const CHAINED = new Set(['line', 'arc', 'polyline']);

/** The session's options (the desktop's `Memory::centerline_*`). */
export const centerlineOptions = { step: 1, chain: true };

interface Side {
  readonly ids: readonly number[];
  readonly shape: Entity;
}

/** Whether the object has a route (Obje üzerinde nokta's). */
const hasRoute = (e: Entity): boolean => pathStation(e, false, 0, 0).point != null;

export class CenterlineTool implements Tool {
  readonly id = 'centerline';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private first: Side | null = null;
  private second: Side | null = null;
  private asking = false;
  private plan: Centerline | null = null;
  private problem: string | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    this.refresh();
  }

  get pointCount(): number {
    return (this.first ? 1 : 0) + (this.second ? 1 : 0);
  }

  private refresh(): void {
    this.plan = null;
    const was = this.problem;
    this.problem = null;
    if (this.first && this.second) {
      const got = centerline(this.first.shape, this.second.shape, centerlineOptions.step);
      this.plan = got.centerline ?? null;
      this.problem = got.problem ?? null;
      if (this.problem && this.problem !== was) this.ctx.log.warn(this.problem);
    }
    this.prompt.set(`${LABEL}: ${this.step()}`);
    this.ctx.view.requestOverlay();
  }

  private step(): string {
    if (this.asking) return 'örnekleme adımını yazın';
    const step = !this.first
      ? 'birinci kenara tıklayın'
      : !this.second
        ? 'ikinci kenara tıklayın'
        : this.plan
          ? `${this.plan.pts.length} köşe; Enter ile yazın ya da yeni bir kenar çiftine tıklayın`
          : 'bu kenarlarla orta hat çizilemiyor; başka kenarlara tıklayın';
    const O = centerlineOptions;
    const parts = [`Adım (B): ${this.ctx.format.length(O.step)}`, O.chain ? 'Zincir (Z): açık' : 'Zincir (Z)', ...(this.plan ? ['Uygula (Enter)'] : [])];
    return `${step} [${parts.join(' / ')}]`;
  }

  /** The line, arc or polyline's chain among the visible ones, joined into one path; the object alone when none goes on from it. */
  private chained(seed: Entity): Side {
    const alone: Side = { ids: [seed.id], shape: seed };
    if (!CHAINED.has(seed.kind)) return alone;
    const { view } = this.ctx;
    const objects = view.entitiesIn(view.camera.visibleBounds()).filter((e) => CHAINED.has(e.kind));
    const at = objects.findIndex((e) => e.id === seed.id);
    if (at < 0) return alone;
    const tolerance = Math.max(JoinTool.tolerance, 1e-9);
    // Read, not written: a locked layer's line is a side too.
    const found = joinChain(
      objects.map((e) => ({ shape: e, locked: false })),
      at,
      tolerance,
    );
    if (found.members.length < 2) return alone;
    const members = found.members.map((i) => objects[i]);
    const { groups } = joinEntities(members, tolerance);
    if (groups.length !== 1) return alone;
    return { ids: members.map((e) => e.id), shape: { ...seed, ...groups[0].geometry } as Entity };
  }

  private pick(p: ToolPointer): Side | null {
    const hits = this.ctx.view.pickAll(p.screen);
    if (!hits.length) {
      this.ctx.log.warn(NO_OBJECT_HERE);
      return null;
    }
    const e = hits.find(hasRoute);
    if (!e) {
      this.ctx.log.warn(NO_ROUTE);
      return null;
    }
    return centerlineOptions.chain ? this.chained(e) : { ids: [e.id], shape: e };
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0 || this.asking) return;
    const side = this.pick(p);
    if (!side) return;
    if (this.first && !this.second) {
      if (side.ids.some((id) => this.first!.ids.includes(id))) return void this.ctx.log.warn(SAME_SIDE);
      this.second = side;
    } else {
      // A new pair starts from any click once the axis is shown.
      this.first = side;
      this.second = null;
    }
    this.refresh();
  }

  input(text: string): boolean {
    const t = text.trim();
    if (this.asking) {
      const n = parseLength(this.ctx.format, t);
      if (n !== null && n > 0 && Number.isFinite(n)) {
        centerlineOptions.step = n;
        this.asking = false;
      } else this.ctx.log.warn(`Adım sıfırdan büyük bir uzunluk olmalı; “${t}” yazıldı.`);
    } else {
      const key = t.toLocaleUpperCase('tr-TR');
      if (key === 'B') this.asking = true;
      else if (key === 'Z') centerlineOptions.chain = !centerlineOptions.chain;
      else return false;
    }
    this.refresh();
    return true;
  }

  /** Enter: Adım's question ends; with an axis it is written; else the tool leaves. */
  confirm(): void {
    if (this.asking) {
      this.asking = false;
      return this.refresh();
    }
    if (!this.plan) return this.ctx.tools.exit();
    const axis = this.plan;
    const bulges = axis.bulges && axis.bulges.some((b) => b !== 0) ? { bulges: axis.bulges } : {};
    const geometry = { kind: 'polyline', pts: axis.pts, ...bulges } as unknown as EntityGeometry;
    if (!writeObjects(this.ctx, [geometry], 'centerline')) return;
    const how = axis.method === 'matched' ? 'kenar kenar' : 'örneklenerek';
    this.ctx.log.success(`${LABEL}: ${axis.pts.length} köşeyle ${how} yazıldı.`);
    this.first = null;
    this.second = null;
    this.refresh();
  }

  /** Esc: Adım's question, then the second side, then the first go first. */
  cancel(): boolean {
    if (this.asking) {
      this.asking = false;
      this.refresh();
      return true;
    }
    return this.undoStep();
  }

  /** Ctrl+Z: the last side goes; with none the drawing is undone. */
  undoStep(): boolean {
    if (this.second) this.second = null;
    else if (this.first) this.first = null;
    else return false;
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    for (const side of [this.first, this.second]) if (side) strokePath(g, view, entityOutline(side.shape, 72), { color: pal.snap });
    if (this.plan) strokePath(g, view, bulgePathOutline(this.plan.pts, this.plan.bulges ?? undefined, false, 0.05), { color: pal.snap, dash: [6, 4] });
  }
}
