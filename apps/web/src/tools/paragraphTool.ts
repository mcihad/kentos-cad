import { MAX_LINE_SPACING, MIN_LINE_SPACING, type TextRun } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { textCornerBox, trimmedParagraph } from '../model/paragraph';
import type { ViewTransform } from '../viewport/Camera';
import { TextTool } from './annotateTools';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';
import { fixed } from '../core/displayNumber';

/** Paper sizes (mm) as world metres at the project's plot scale. */
const paper = (plotScale: number, mm: number) => (mm / 1000) * plotScale;

/**
 * Çok satırlı yazı (docs/adr/0182 §4; the desktop's `kentos_interaction::paragraph`), step for step:
 *
 * - two corners give the box (the core's `textCornerBox`): its width along the text's angle (Yazı's Açı) and its top
 *   left corner, which the text hangs from, its alignment the top's left; two clicks on one point give no box (the
 *   lines end only at their breaks);
 * - the paragraph editor opens at the box (`view.requestParagraphInput`, ui/shell/ParagraphEditor.ts): Tamam writes the
 *   text and its letter formats through `cad.entities.create` (one step, “Ekle”), its ends' white space left out;
 *   Vazgeç drops it; the tool waits for the next box;
 * - Yükseklik (Y) and Açı (A) are Yazı's, Zemin (Z) Yazı's mask; Satır aralığı (S) the lines' spacing, from 0.25 to
 *   4, kept as long as the page lives;
 * - a locked active layer is said at the first corner and nothing opens.
 */
export class ParagraphTextTool extends PointInputTool {
  readonly id = 'mtext';
  protected readonly label = 'Çok satırlı yazı';
  private static spacing = 1;
  private stage: 'first' | 'second' | 'height' | 'angle' | 'spacing' | 'typing' = 'first';
  private first: Vec2 | null = null;

  protected promptFor(): string {
    const o = TextTool.options();
    switch (this.stage) {
      case 'height':
        return 'kâğıt üzerindeki yazı yüksekliğini mm olarak yazın';
      case 'angle':
        return 'yazının açısını derece olarak yazın';
      case 'spacing':
        return `satır aralığını yazın (1: yüksekliğin 5/3'ü; ${MIN_LINE_SPACING} ile ${MAX_LINE_SPACING} arası)`;
      case 'second':
        return 'kutunun karşı köşesine tıklayın';
      case 'typing':
        return 'yazıyı kutuya yazın; Enter yeni satır, Ctrl+Enter ya da Tamam ekler, Esc vazgeçer';
      default:
        return (
          `yazı kutusunun ilk köşesine tıklayın [Yükseklik (Y): ${o.heightMm} mm / Açı (A): ${+fixed(o.angle, 4)}° / ` +
          `Satır aralığı (S): ${+fixed(ParagraphTextTool.spacing, 4)} / Zemin (Z): ${o.mask ? 'açık' : 'kapalı'}]`
        );
    }
  }

  /** The box's first corner while the opposite one is awaited, as the desktop counts it. */
  override get pointCount(): number {
    return this.stage === 'second' ? 1 : 0;
  }

  override snapFrom(): Vec2 | null {
    return this.first;
  }

  protected override get last(): Vec2 | null {
    return this.first;
  }

  protected override option(key: string): boolean {
    if (this.stage !== 'first') return false;
    switch (key) {
      case 'Y':
        this.stage = 'height';
        break;
      case 'A':
        this.stage = 'angle';
        break;
      case 'S':
        this.stage = 'spacing';
        break;
      case 'Z':
        TextTool.setMask(!TextTool.options().mask);
        break;
      default:
        return false;
    }
    this.refreshPrompt();
    return true;
  }

  protected onPoint(p: Vec2): void {
    if (this.stage === 'first') {
      const layers = this.ctx.doc.layers;
      if (layers.isLocked(layers.active.value)) return void this.targetLayer();
      this.first = p;
      this.stage = 'second';
      return;
    }
    if (this.stage !== 'second' || !this.first) return;
    const o = TextTool.options();
    const { corner, width } = textCornerBox(this.first, p, o.angle);
    const height = paper(this.ctx.doc.settings.plotScale.value, o.heightMm);
    const spacing = ParagraphTextTool.spacing;
    this.stage = 'typing';
    this.ctx.view.requestParagraphInput({
      at: corner,
      height,
      rotation: o.angle,
      ...(width !== undefined && { boxWidth: width }),
      ...(spacing !== 1 && { lineSpacing: spacing }),
      ...(o.mask && { mask: true }),
      commit: (typed, typedRuns) => {
        const { text, runs } = trimmedParagraph(typed, typedRuns);
        if (text) {
          const written = this.writeObjects([
            {
              kind: 'text',
              p: corner,
              text,
              height,
              rotation: o.angle,
              align: 'topLeft',
              ...(o.mask && { mask: true }),
              ...(width !== undefined && { boxWidth: width }),
              ...(spacing !== 1 && { lineSpacing: spacing }),
              ...(runs.length && { runs: runs as TextRun[] }),
            },
          ]);
          if (written) this.ctx.log.success(`Çok satırlı yazı eklendi: “${text.split('\n')[0]}${text.includes('\n') ? ' …' : ''}”`);
        }
        this.restart();
      },
      cancel: () => this.restart(),
    });
  }

  /** The editor closed, or the box was dropped: back to the first corner. */
  private restart(): void {
    this.first = null;
    this.stage = 'first';
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    this.ctx.view.focus();
  }

  override input(text: string): boolean {
    const t = text.trim();
    if (this.option(t.toLocaleUpperCase('tr-TR'))) return true;
    const n = parseNumber(t);
    if (this.stage === 'height') {
      if (n === null || n <= 0) return false;
      TextTool.setHeight(n);
      this.stage = 'first';
      this.refreshPrompt();
      return true;
    }
    if (this.stage === 'angle') {
      if (n === null) return false;
      TextTool.setAngle(n);
      this.stage = 'first';
      this.refreshPrompt();
      return true;
    }
    if (this.stage === 'spacing') {
      if (n === null) return false;
      if (n < MIN_LINE_SPACING || n > MAX_LINE_SPACING) {
        this.ctx.log.warn(`Satır aralığı ${MIN_LINE_SPACING} ile ${MAX_LINE_SPACING} arasında olmalı; ${t} verildi. Yüksekliğin 5/3'ü için 1 yazın.`);
        return true;
      }
      ParagraphTextTool.spacing = n;
      this.stage = 'first';
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  override confirm(): void {
    if (this.stage === 'first') return this.ctx.tools.exit();
    this.restart();
  }

  /** Esc past the first corner drops the box; at it the tool leaves. */
  cancel(): boolean {
    if (this.stage === 'first') return false;
    this.restart();
    return true;
  }

  protected override reset(): void {
    this.stage = 'first';
    this.first = null;
    super.reset();
  }

  /** The box from its first corner to the pointer, dashed, its width beside it. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.stage !== 'second' || !this.first || !this.hover) return;
    const pal = this.ctx.view.palette;
    const o = TextTool.options();
    const r = (o.angle * Math.PI) / 180;
    const [c, s] = [Math.cos(r), Math.sin(r)];
    const { corner, width } = textCornerBox(this.first, this.hover, o.angle);
    const up = (p: Vec2) => -p.x * s + p.y * c;
    const depth = Math.abs(up(this.first) - up(this.hover));
    const w = width ?? 0;
    const at = (x: number, y: number): Vec2 => ({ x: corner.x + c * x + s * y, y: corner.y + s * x - c * y });
    strokePath(g, view, [at(0, 0), at(w, 0), at(w, depth), at(0, depth), at(0, 0)], { color: pal.accent, dash: [4, 3] });
    drawTag(g, view.worldToScreen(this.hover), [width !== undefined ? `Kutu genişliği ${this.ctx.format.length(width)}` : 'Kutusuz: satırlar yalnız satır sonlarında biter'], pal.accent, pal.labelHalo);
  }
}
