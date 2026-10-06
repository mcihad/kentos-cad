import type { AppContext } from '../app/context';
import type { TableEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { tableLayout, type TableGeometry } from '../model/ops/table';
import { faceFont, leanOf } from '../render/drawingFaces';
import type { ViewTransform } from '../viewport/Camera';
import { PointInputTool } from './drawTools';
import { strokePath, tint } from './preview';
import type { EntityGeometry } from '../model/entities';

/** A new table at a point: Tablo ekle's (app/tables.ts `newTable`). */
export type TableAt = (p: Vec2) => Omit<TableEntity, 'id' | 'uid' | 'layerId' | 'attrs'>;

/**
 * Tablo ekle's placement (docs/adr/0184 §3; the desktop's `kentos_interaction::table_place`): the table Tablo ekle made
 * hangs from the cursor by its top left corner, its lines, frame and words faint; a click or a typed Y,X writes it on
 * the active layer through `cad.entities.create` (step “Tablo”), selected, and the tool ends. Esc leaves it.
 */
export class TablePlaceTool extends PointInputTool {
  readonly id = 'tablePlace';
  /** Tablo ekle, or the command that made the table (Koordinat çizelgesi, docs/adr/0185 §1). */
  protected readonly label: string;
  private readonly tableAt: TableAt;

  constructor(ctx: AppContext, tableAt: TableAt, label = 'Tablo ekle') {
    super(ctx);
    this.tableAt = tableAt;
    this.label = label;
  }

  protected promptFor(): string {
    return 'tablonun sol üst köşesine tıklayın ya da Y,X yazın';
  }

  protected onPoint(p: Vec2): void {
    const out = this.writeObjects([this.tableAt(p) as unknown as EntityGeometry], 'table');
    if (!out) return;
    this.ctx.selection.set(out.ids);
    this.ctx.log.success('Tablo eklendi: düzenlemek için çift tıklayın.');
    this.ctx.tools.exit();
  }

  /** The table as it would be placed at the cursor: lines and frame dashed, words faint. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.hover) return;
    const t = this.tableAt(this.hover);
    const layout = tableLayout(t as unknown as TableGeometry, this.ctx.doc.settings.drawingFont.value);
    if (!layout) return;
    const pal = this.ctx.view.palette;
    for (const strip of layout.frame) strokePath(g, view, strip, { color: pal.accent, closed: true, fill: tint(pal.accent, 0.35) });
    for (const [a, b] of layout.lines) strokePath(g, view, [a, b], { color: pal.accent, dash: [4, 3] });
    const px = t.height * view.scale;
    if (px >= 1) {
      g.save();
      g.globalAlpha = 0.65;
      g.fillStyle = pal.accent;
      g.textAlign = 'left';
      g.textBaseline = 'alphabetic';
      const lean = leanOf(t);
      for (const c of layout.cells) {
        const words = t.cells[c.row]?.[c.col];
        if (!words) continue;
        const s = view.worldToScreen(c.at);
        g.save();
        g.translate(s.x, s.y);
        if (lean) g.transform(1, 0, -lean, 1, 0, 0);
        g.font = faceFont(t, px, pal.drawingFont, c.bold ? '600' : '400', { bold: c.bold });
        g.fillText(words, 0, 0);
        g.restore();
      }
      g.restore();
    }
    this.drawTracking(g, view);
  }
}
