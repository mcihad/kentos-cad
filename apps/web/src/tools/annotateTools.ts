import type { AppContext } from '../app/context';
import { angleDeg, dist, type Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { textAngle } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

// ── Yazı ────────────────────────────────────────────────────────────────

/**
 * Single-line text: click where it starts and type right there — a field
 * opens in place (the drawing never sees the keystrokes). Height (Y, paper
 * mm) and angle (A: typed degrees, or two clicks along an edge) are set
 * before the click and kept for the next texts. Enter adds the text and
 * waits for the next one; Esc drops the field.
 */
export class TextTool extends PointInputTool {
  readonly id = 'text';
  protected readonly label = 'Yazı';
  private static heightMm = 2.5;
  private static angle = 0;
  private stage: 'pos' | 'height' | 'angle' | 'typing' = 'pos';
  private at: Vec2 | null = null;
  /** First of the two clicks that give the angle. */
  private angleFrom: Vec2 | null = null;

  protected promptFor(): string {
    switch (this.stage) {
      case 'height':
        return 'kâğıt üzerindeki yazı yüksekliğini mm olarak yazın';
      case 'angle':
        return this.angleFrom ? 'doğrultunun ikinci noktasına tıklayın' : 'açıyı yazın (derece) ya da doğrultu için iki noktaya tıklayın';
      case 'typing':
        return 'yazıyı tıkladığınız yere yazın; Enter ekler, Esc vazgeçer';
      default:
        return `yazının başlangıcına tıklayın [Yükseklik (Y): ${TextTool.heightMm} mm / Açı (A): ${+TextTool.angle.toFixed(4)}°]`;
    }
  }

  override snapFrom(): Vec2 | null {
    return this.angleFrom ?? this.at;
  }

  protected override get last(): Vec2 | null {
    return this.stage === 'angle' ? this.angleFrom : null;
  }

  protected override option(key: string): boolean {
    if (this.stage !== 'pos' || (key !== 'Y' && key !== 'A')) return false;
    this.stage = key === 'Y' ? 'height' : 'angle';
    this.angleFrom = null;
    this.refreshPrompt();
    return true;
  }

  protected onPoint(p: Vec2): void {
    if (this.stage === 'angle') {
      if (!this.angleFrom) return void (this.angleFrom = p);
      if (dist(this.angleFrom, p) < 1e-9) return;
      // Kept readable: a direction pointing left is turned around.
      TextTool.angle = textAngle(this.angleFrom, p);
      this.angleFrom = null;
      this.stage = 'pos';
      return;
    }
    if (this.stage !== 'pos') return;
    this.at = p;
    this.stage = 'typing';
    const height = paper(this.ctx, TextTool.heightMm);
    this.ctx.view.requestTextInput({
      at: p,
      height,
      rotation: TextTool.angle,
      commit: (text) => {
        this.create({ kind: 'text', p, text, height, rotation: TextTool.angle });
        this.ctx.log.success(`Yazı eklendi: “${text}”`);
        this.afterTyping();
      },
      cancel: () => this.afterTyping(),
    });
  }

  private afterTyping(): void {
    this.at = null;
    this.stage = 'pos';
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
      TextTool.heightMm = n;
      this.stage = 'pos';
      this.refreshPrompt();
      return true;
    }
    if (this.stage === 'angle') {
      if (n === null) return false;
      TextTool.angle = n;
      this.angleFrom = null;
      this.stage = 'pos';
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  override confirm(): void {
    if (this.stage === 'pos') return this.ctx.tools.exit();
    this.afterTyping();
  }

  protected override reset(): void {
    this.stage = 'pos';
    this.at = null;
    this.angleFrom = null;
    super.reset();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.stage === 'angle' && this.angleFrom && this.hover) {
      strokePath(g, view, [this.angleFrom, this.hover], { color: pal.accent, dash: [3, 3] });
      drawTag(g, view.worldToScreen(this.hover), [`Açı ${angleDeg(this.angleFrom, this.hover).toFixed(2)}°`], pal.accent, pal.labelHalo);
      return;
    }
    // Where the text will sit: a box of its height along its angle.
    const at = this.stage === 'pos' ? this.hover : null;
    if (!at) return;
    const s = view.worldToScreen(at);
    const px = Math.max(8, paper(this.ctx, TextTool.heightMm) * view.scale);
    g.save();
    g.translate(s.x, s.y);
    g.rotate((-TextTool.angle * Math.PI) / 180);
    g.strokeStyle = pal.accent;
    g.setLineDash([3, 3]);
    g.strokeRect(0, -px, px * 4, px);
    g.restore();
  }
}
