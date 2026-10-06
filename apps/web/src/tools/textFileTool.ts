import type { FileKind } from '../app/fileIO';
import type { EntityGeometry } from '../contracts/generated/EntityGeometry';
import { textAlignName, textAlignShares, textBox } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { textFileLines } from '../model/textFile';
import type { ViewTransform } from '../viewport/Camera';
import { TextTool } from './annotateTools';
import { PointInputTool } from './drawTools';
import { fixed } from '../core/displayNumber';
import { stylesShown, textFaceNow, textStyleName } from './styleOption';

/** The text files Metin dosyası yerleştir offers to open. */
export const TEXT_FILES: FileKind = { description: 'Metin dosyası (UTF-8)', accept: { 'text/plain': ['.txt', '.csv', '.lst'] } };

/** The line spacing, in text heights (docs/adr/0145 §6). */
const SPACING = 1.5;

/**
 * Metin dosyası yerleştir (docs/adr/0145 §6): a UTF-8 text file is chosen as the tool starts (the click that started
 * it lets the browser open the picker), then a point: each line becomes a text with Yazı's options (height, angle,
 * alignment, width factor, mask), the lines one under the other 1.5 heights apart, an empty line keeping its place.
 * One step “Metin dosyası yerleştir” (`cad.entities.create`'s `textFile`). A file refused (more than 1 MB, not UTF-8,
 * more than 10 000 lines, nothing but empty lines) is said and the tool leaves; so does a cancelled picker. The
 * desktop's is `kentos_interaction::text_file`.
 */
export class PlaceTextFileTool extends PointInputTool {
  readonly id = 'placeTextFile';
  protected readonly label = 'Metin dosyası yerleştir';
  private file: { name: string; lines: string[]; widths: number[] } | null = null;

  override activate(): void {
    super.activate();
    void this.pick();
  }

  private async pick(): Promise<void> {
    const picked = await this.ctx.files.pickForImport(TEXT_FILES);
    if (!picked) return this.ctx.tools.exit();
    const read = textFileLines(picked.name, picked.bytes);
    if (!read.ok) {
      this.ctx.log.warn(read.error);
      return this.ctx.tools.exit();
    }
    // Each line's width at a height of 1, for the preview: in Yazı's style's typeface and bold, else the drawing's
    // (docs/adr/0183 §2).
    const face = textFaceNow(this.ctx);
    const font = face.font ?? this.ctx.doc.settings.drawingFont.value;
    const widths = read.lines.map((text) => {
      if (!text) return 0;
      const b = textBox({ p: { x: 0, y: 0 }, text, height: 1, rotation: 0, font, ...(face.bold && { bold: true }) });
      return Math.hypot(b[1].x - b[0].x, b[1].y - b[0].y);
    });
    this.file = { name: picked.name, lines: read.lines, widths };
    this.refreshPrompt();
  }

  protected promptFor(): string {
    if (!this.file) return 'metin dosyasını seçin';
    const o = TextTool.options();
    const texts = this.file.lines.filter(Boolean).length;
    // A CAD project's style first (docs/adr/0183 §4).
    const style = stylesShown(this.ctx) ? `${textStyleName(this.ctx)}, ` : '';
    return `ilk satırın başlangıcına tıklayın [“${this.file.name}”: ${texts} yazı; Yazı'nın seçenekleriyle: ${style}${o.heightMm} mm, ${+fixed(o.angle, 4)}°, ${textAlignName(o.align)}]`;
  }

  protected onPoint(p: Vec2): void {
    if (!this.file) return;
    const layers = this.ctx.doc.layers;
    if (layers.isLocked(layers.active.value)) return void this.targetLayer();
    const o = TextTool.options();
    const height = (o.heightMm / 1000) * this.ctx.doc.settings.plotScale.value;
    const r = (o.angle * Math.PI) / 180;
    // Down the text's own up: each line 1.5 heights under the one before.
    const down = { x: Math.sin(r), y: -Math.cos(r) };
    const geometries: EntityGeometry[] = [];
    // Yazı's style (docs/adr/0183 §4).
    const face = textFaceNow(this.ctx);
    this.file.lines.forEach((text, i) => {
      if (!text) return;
      const at = { x: p.x + down.x * SPACING * height * i, y: p.y + down.y * SPACING * height * i };
      geometries.push({ kind: 'text', p: at, text, height, rotation: o.angle, ...(o.align && { align: o.align }), ...(o.widthFactor !== 1 && { widthFactor: o.widthFactor }), ...(o.mask && { mask: true }), ...face } as EntityGeometry);
    });
    if (this.writeObjects(geometries, 'textFile')) this.ctx.log.success(`“${this.file.name}”: ${geometries.length} satır yazı olarak yerleştirildi.`);
    this.file = null;
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const at = this.hover;
    if (!this.file || !at) return;
    const o = TextTool.options();
    const px = Math.max(8, (o.heightMm / 1000) * this.ctx.doc.settings.plotScale.value * view.scale);
    const [along, up] = textAlignShares(o.align);
    const s = view.worldToScreen(at);
    const pal = this.ctx.view.palette;
    g.save();
    g.translate(s.x, s.y);
    g.rotate((-o.angle * Math.PI) / 180);
    g.strokeStyle = pal.accent;
    g.setLineDash([3, 3]);
    // Each line's box, the first 200 of them.
    this.file.widths.slice(0, 200).forEach((w, i) => {
      if (!w) return;
      const width = w * px * o.widthFactor;
      g.strokeRect(-along * width, -(1 - up) * px + i * SPACING * px, width, px);
    });
    g.restore();
  }
}
