import { lookOfDimension } from '../model/annotationStyles';
import type { EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { layoutDimension, quickDimensions, type QuickDimensions } from '../model/geom/dimension';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { writeObjects } from './createCommand';
import { dimensionZemin } from './dimensionTool';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { drawTag, strokeLayout } from './preview';
import { dimensionLookNow, dimensionStyleChoices, dimensionStyleName, standardDimensionHeight, stylesShown, takeDimensionStyle } from './styleOption';
import type { OptionChoice } from './Tool';

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
  /** Stil (S): a dimension style's name asked for (docs/adr/0183 §4). */
  private askingStyle = false;

  protected begin(): void {
    if (this.targets().some((e) => e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon')) return;
    this.ctx.log.warn(NOTHING);
    // Leaving from inside activate() would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }

  protected stagePrompt(): string {
    if (this.askingStyle) return `ölçü stilini menüden seçin ya da adını yazın [Stil (S): ${dimensionStyleName(this.ctx)}]`;
    const stil = stylesShown(this.ctx) ? `Stil (S): ${dimensionStyleName(this.ctx)} / ` : '';
    return `ölçülerin yerini gösterin ya da uzaklık yazın [${stil}Zemin (Z): ${dimensionZemin.on ? 'açık' : 'kapalı'}]`;
  }

  /** Stil's menu: Standart, the project's dimension styles and their window (docs/adr/0183 §4). */
  optionChoices(key: string): readonly OptionChoice[] | null {
    return key === 'S' && !this.picking && stylesShown(this.ctx) ? dimensionStyleChoices(this.ctx) : null;
  }

  /** A style's name is words: Space types a space (docs/adr/0183 §4). */
  takesWords(): boolean {
    return this.askingStyle;
  }

  chooseOption(key: string, typed: string): boolean {
    if (key !== 'S' || this.picking || !stylesShown(this.ctx)) return false;
    if (takeDimensionStyle(this.ctx, typed)) this.askingStyle = false;
    this.refresh();
    return true;
  }

  /** The dimensions with the cursor at `at` and `typed` the distance typed, in the style's look: the selection in the drawing's order. */
  private quick(at: Vec2, typed: number | null): QuickDimensions {
    const objects = this.targets().sort((a, b) => a.id - b.id);
    const { look, height } = dimensionLookNow(this.ctx, standardDimensionHeight(this.ctx));
    const q = quickDimensions(objects, at, typed, height);
    return { ...q, dimensions: q.dimensions.map((d) => ({ ...d, ...look })) };
  }

  protected point(p: Vec2): void {
    this.write(p, null);
  }

  override input(text: string): boolean {
    if (this.picking) return false;
    // A dimension style's name: one the project has none of is said, and the tool waits for another.
    if (this.askingStyle) {
      if (takeDimensionStyle(this.ctx, text)) this.askingStyle = false;
      this.refresh();
      return true;
    }
    if (text.trim().toLocaleUpperCase('tr-TR') === 'Z') {
      dimensionZemin.on = !dimensionZemin.on;
      this.refresh();
      return true;
    }
    if (text.trim().toLocaleUpperCase('tr-TR') === 'S' && stylesShown(this.ctx)) {
      this.askingStyle = true;
      this.refresh();
      return true;
    }
    const n = parseNumber(text);
    if (n === null || /[,;@<]/.test(text)) return false;
    // The cursor still gives an open path's side; without one, the origin does.
    this.write(this.hover ?? { x: 0, y: 0 }, this.ctx.format.toMetres(n));
    return true;
  }

  /** Enter while a style's name is asked for: back to placing them. */
  override confirm(): void {
    if (this.askingStyle) {
      this.askingStyle = false;
      return this.refresh();
    }
    super.confirm();
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
        ...lookOfDimension(d),
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
      if (l) strokeLayout(g, view, l, pal.accent);
    }
    if (dimensions.length) {
      const tag = [`${dimensions.length} ölçü`, this.ctx.format.length(Math.abs(dimensions[0].offset))];
      drawTag(g, view.worldToScreen(this.hover), tag, pal.accent, pal.labelHalo);
    }
  }
}
