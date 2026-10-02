import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import { bulgePathLength, hasBulges } from '../model/geom/bulge';
import type { Area } from '../model/geom/overlay';
import { netArea } from '../model/geom/region';
import type { Vec2 } from '../model/geometry';
import { adjoinWork, type AdjoinWork } from '../model/ops/adjoin';
import type { ViewTransform } from '../viewport/Camera';
import { joinCorners, sayJoined } from './junctions';
import { clippedGeometry, clipNewAreas, overlapLayers, sayClipped, writtenArea } from './overlap';
import { PathTool } from './pathTool';
import { drawArea, tint } from './preview';

/**
 * Bitişik alan (docs/adr/0162 §3): the path tool's shape that draws only the new boundary; the region it closes with
 * the neighbouring areas is the new area. The neighbours are the visible areas in view on the overlap layers (§1; the
 * active layer while the mode is Serbest or no layer is chosen); the region is the shared core's (`adjoinWork`). The
 * desktop's is `crates/native/interaction/src/adjoin.rs` with its path tool; both play
 * fixtures/interaction/v1/adjoin.json.
 */

/** Neighbours with more edges than this are cut through at clicks and Enter only, not at every pointer move (§5). */
export const PREVIEW_EDGES = 2000;

/** Said when the path closes no region with the neighbours (§3). */
export const NO_REGION = 'Yol komşu alanlarla kapalı bir bölge oluşturmuyor: ilk ve son noktayı komşu alanların içine ya da sınırına koyun.';

/** The layers whose areas the path closes against: the overlap mode's, the active layer while it is Serbest or no layer is chosen. */
function neighbourLayers(ctx: AppContext): string[] {
  const active = ctx.doc.layers.active.value;
  const layers = overlapLayers(ctx, active);
  return layers.length ? layers : [active];
}

/**
 * The neighbours of what the view shows, kept in the core and taken again only when the view, the drawing (its layers
 * too) or the overlap layers change, as VisibleTrace keeps İzle's graph; released as soon as they are stale.
 */
class VisibleNeighbours {
  private readonly ctx: AppContext;
  private work: AdjoinWork | null = null;
  private key = '';
  private subs: Disposable[] = [];
  /** Counts the neighbours taken, so a region worked out with older ones is not shown again. */
  generation = 0;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  attach(): void {
    const drop = () => this.drop();
    this.subs = [this.ctx.doc.events.on('changed', drop), this.ctx.doc.layers.events.on('state', drop), this.ctx.doc.layers.events.on('structure', drop)];
  }

  detach(): void {
    this.subs.forEach((d) => d());
    this.subs = [];
    this.drop();
  }

  get(): AdjoinWork {
    const b = this.ctx.view.camera.visibleBounds();
    const layers = neighbourLayers(this.ctx);
    const key = `${b.minX}|${b.minY}|${b.maxX}|${b.maxY}|${layers.join('\n')}`;
    if (!this.work || key !== this.key) {
      this.drop();
      const on = new Set(layers);
      this.work = adjoinWork(this.ctx.view.entitiesIn(b).filter((e) => on.has(e.layerId)));
      this.key = key;
      this.generation++;
    }
    return this.work;
  }

  private drop(): void {
    this.work?.free();
    this.work = null;
  }
}

/**
 * An open path whose ends lie in or on the neighbouring areas (Yay, Uzunluk, İzle, Akış and Geri as Çoklu çizgi has
 * them); the region it closes with them fills as the cursor goes and is written as one area through
 * `cad.entities.create` (`adjoin`), one undo step “Bitişik alan”. With no region Enter says why and the path stays.
 */
export class AdjoinTool extends PathTool {
  private readonly neighbours: VisibleNeighbours;
  /** The region last worked out, and the path and neighbours it was found for. */
  private region: { key: string; areas: Area[] } | null = null;

  constructor(ctx: AppContext) {
    super(ctx, { id: 'adjoin', label: 'Bitişik alan', closed: false });
    this.neighbours = new VisibleNeighbours(ctx);
  }

  override activate(): void {
    this.neighbours.attach();
    super.activate();
  }

  override deactivate(): void {
    super.deactivate();
    this.neighbours.detach();
    this.region = null;
  }

  protected override promptFor(n: number): string {
    return n === 0 ? 'ilk noktayı komşu alanın içinde ya da sınırında belirtin' : super.promptFor(n);
  }

  /** The region the path (one bulge per segment) closes with the neighbours, worked out once per path and neighbours. */
  private fill(pts: readonly Vec2[], bulges: readonly number[]): Area[] {
    const work = this.neighbours.get();
    const key = `${this.neighbours.generation}|${pts.map((p) => `${p.x},${p.y}`).join(';')}|${bulges.join(',')}`;
    if (this.region?.key !== key) this.region = { key, areas: pts.length < 2 ? [] : work.fill(pts, hasBulges(bulges) ? bulges : null) };
    return this.region.areas;
  }

  /** The region the preview fills: the path to the cursor, or as the last click left it when the neighbours are many. */
  private shown(): Area[] {
    if (this.neighbours.get().edgeCount > PREVIEW_EDGES) return this.fill(this.pts, this.bulges);
    const { pts, bulges } = this.previewPath();
    return this.fill(pts, bulges);
  }

  /** The path's length and the region's area beside the cursor. */
  protected override extraTag(pts: Vec2[], bulges: number[]): string[] {
    const f = this.ctx.format;
    const lines = [`Yol ${f.length(bulgePathLength(pts, bulges, false))}`];
    const region = this.shown();
    if (region.length) lines.push(`Alan ${f.area(region.reduce((sum, a) => sum + netArea(a), 0))}`);
    return lines;
  }

  protected override finish(): void {
    if (this.pts.length < 2) return super.finish();
    // The neighbours the preview had; with no region (or a refusal) the path stays.
    if (this.write(this.fill(this.pts, this.bulges))) this.reset();
  }

  /**
   * Writes the region as one area (its parts and holes as they are) on the active layer, in the current colour and
   * weight; the overlap control (§2) cuts what overlaps areas the view does not show, Topoloji joins it with its
   * neighbours corner by corner (§4), in the same step. Whether it was written: when not, the reason is said.
   */
  private write(region: Area[]): boolean {
    if (!region.length) {
      this.ctx.log.warn(NO_REGION);
      return false;
    }
    let areas = region;
    const clipped = clipNewAreas(this.ctx, areas, this.ctx.doc.layers.active.value);
    if (clipped) {
      sayClipped(this.ctx, clipped);
      if (!clipped.areas.length) return false;
      areas = clipped.areas;
    }
    const [total, parts] = [writtenArea(areas), areas.length];
    // Topoloji (§4): the area joined with its neighbours corner by corner, in its step.
    const joining = joinCorners(this.ctx, areas);
    const joined = joining?.areas ?? areas;
    if (!this.writeJoined(joining, 'Bitişik alan', () => this.writeObjects([clippedGeometry(joined)], 'adjoin'))) return false;
    sayJoined(this.ctx, joining);
    const area = this.ctx.format.area(total);
    this.ctx.log.success(parts > 1 ? `Bitişik alan eklendi: ${area} (${parts} parça)` : `Bitişik alan eklendi: ${area}`);
    return true;
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    for (const a of this.shown()) drawArea(g, view, a, { color: pal.accent, fill: tint(pal.accent, 0.16), width: 2 });
    super.draw(g, view);
  }
}
