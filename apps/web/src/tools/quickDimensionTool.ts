import type { EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { layoutDimension, quickDimensions, type QuickDimensions } from '../model/geom/dimension';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { writeObjects } from './createCommand';
import { dimensionZemin, paper } from './dimensionTool';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { drawTag, strokePath } from './preview';

/** What is said when the selection has nothing to measure. */
const NOTHING = 'Seçimde ölçülecek çizgi, çoklu çizgi ya da alan yok.';

/**
 * Hızlı ölçü (docs/adr/0147 §7, AutoCAD's QDIM): the selected lines, polylines and areas measured at once, an aligned
 * dimension along each straight edge and an arc length along each arc; out of an area, into a hole, on the cursor's
 * side of an open path; an edge two objects share measured once. The geometry core gives them (`quickDimensions`), in
 * the drawing's order. Selection first, as the modify tools; then the cursor shows where they go (its distance to the
 * nearest edge, no snapping) or a distance is typed, and all of them are written in one step “Ekle” through
 * `cad.entities.create`. Zemin (Z) is Ölçülendirme's. The desktop's is `kentos_interaction::quick_dimension`.
 */
export class QuickDimensionTool extends SelectionFirstTool {
  readonly id = 'quickDimension';
  protected readonly label = 'Hızlı ölçü';
  // The cursor's distance to the nearest edge places them: a snap would put it on an edge.
  override readonly snaps = false;

  protected begin(): void {
    if (this.targets().some((e) => e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon')) return;
    this.ctx.log.warn(NOTHING);
    // Leaving from inside activate() would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }

  protected stagePrompt(): string {
    return `ölçülerin yerini gösterin ya da uzaklık yazın [Zemin (Z): ${dimensionZemin.on ? 'açık' : 'kapalı'}]`;
  }

  /** The dimensions with the cursor at `at` and `typed` the distance typed: the selection in the drawing's order. */
  private quick(at: Vec2, typed: number | null): QuickDimensions {
    const objects = this.targets().sort((a, b) => a.id - b.id);
    return quickDimensions(objects, at, typed, paper(this.ctx, 2.5));
  }

  protected point(p: Vec2): void {
    this.write(p, null);
  }

  override input(text: string): boolean {
    if (this.picking) return false;
    if (text.trim().toLocaleUpperCase('tr-TR') === 'Z') {
      dimensionZemin.on = !dimensionZemin.on;
      this.refresh();
      return true;
    }
    const n = parseNumber(text);
    if (n === null || /[,;@<]/.test(text)) return false;
    // The cursor still gives an open path's side; without one, the origin does.
    this.write(this.hover ?? { x: 0, y: 0 }, this.ctx.format.toMetres(n));
    return true;
  }

  private write(at: Vec2, typed: number | null): void {
    const { dimensions, skipped } = this.quick(at, typed);
    if (!dimensions.length) {
      this.ctx.log.warn(NOTHING);
      return this.ctx.tools.exit();
    }
    const mask = dimensionZemin.on;
    const geometries = dimensions.map(
      (d): EntityGeometry => ({
        kind: 'dimension',
        a: d.a,
        b: d.b,
        offset: d.offset,
        height: d.height,
        ...(d.style && { style: d.style }),
        ...(d.c && { c: d.c }),
        ...(mask && { mask: true }),
      }),
    );
    const out = writeObjects(this.ctx, geometries);
    if (out) {
      const left = skipped ? `; ${skipped} nesne atlandı (yalnız çizgi, çoklu çizgi ve alan ölçülür)` : '';
      this.ctx.log.success(`Hızlı ölçü: ${out.ids.length} ölçü eklendi${left}.`);
    }
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking || !this.hover) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    const { dimensions } = this.quick(this.hover, null);
    // As the modify tools' ghosts: a very large selection previews its first ones.
    for (const d of dimensions.slice(0, MAX_GHOSTS)) {
      const l = layoutDimension(d);
      if (l) for (const [p, q] of l.lines) strokePath(g, view, [p, q], { color: pal.accent });
    }
    if (dimensions.length) {
      const tag = [`${dimensions.length} ölçü`, this.ctx.format.length(Math.abs(dimensions[0].offset))];
      drawTag(g, view.worldToScreen(this.hover), tag, pal.accent, pal.labelHalo);
    }
  }
}
