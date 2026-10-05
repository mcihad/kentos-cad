import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import { TEXT_ALIGN_ROWS, textAlignFromName, textAlignName, textAlignShares, widthFactorOk, MAX_WIDTH_FACTOR, type Entity, type TextAlign, type TextEntity } from '../model/entities';
import { angleDeg, dist, type Vec2 } from '../model/geometry';
import { textIncrement, textReadable } from '../model/textEdit';
import { geometryOf } from '../product/entitiesEdit';
import type { ViewTransform } from '../viewport/Camera';
import { textAngle } from './constructions';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { uidOf, writeEdit } from './editCommand';
import { SelectionActionTool } from './editTools';
import { drawTag, strokePath } from './preview';
import type { OptionChoice } from './Tool';
import { fixed } from '../core/displayNumber';

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

/** “sol üst” → “Sol üst”: a name at the head of a menu row. */
const capital = (s: string) => s.charAt(0).toLocaleUpperCase('tr-TR') + s.slice(1);

/** An alignment's icon in the web's set (`ui/icons.ts`): `textAlign` and its name, the left of the baseline too. */
const alignIcon = (a: TextAlign | null) => {
  const name = a ?? 'baselineLeft';
  return `textAlign${name.charAt(0).toUpperCase()}${name.slice(1)}`;
};

// ── Yazı ────────────────────────────────────────────────────────────────

/**
 * Single-line text: click where it starts and type right there — a field
 * opens in place (the drawing never sees the keystrokes). Height (Y, paper
 * mm) and angle (A: typed degrees, or two clicks along an edge) are set
 * before the click and kept for the next texts, and so are (docs/adr/0145
 * §6) the point of the text the click is (Hiza, H: its menu or its name
 * typed together, “sağüst”), the letters' width factor (Genişlik, G), the
 * box filled with the drawing's colour (Zemin, Z) and Artır (R): the next
 * field opens with the last text's number one more. Enter adds the text
 * and waits for the next one; Esc drops the field.
 */
export class TextTool extends PointInputTool {
  readonly id = 'text';
  protected readonly label = 'Yazı';
  private static heightMm = 2.5;
  private static angle = 0;
  private static align: TextAlign | null = null;
  private static widthFactor = 1;
  private static mask = false;
  private static increment = false;
  private stage: 'pos' | 'height' | 'angle' | 'align' | 'width' | 'typing' = 'pos';
  private at: Vec2 | null = null;
  /** First of the two clicks that give the angle. */
  private angleFrom: Vec2 | null = null;
  /** The last text this run wrote: Artır's next field starts from it. */
  private lastText: string | null = null;

  /** Yükseklik, shared with Kılavuz (docs/adr/0146 §7): the paper height of both tools' texts. */
  static setHeight(mm: number): void {
    TextTool.heightMm = mm;
  }

  /** Yazı's options as they are now: Metin dosyası yerleştir writes its lines with them (docs/adr/0145 §6). */
  static options(): { heightMm: number; angle: number; align: TextAlign | null; widthFactor: number; mask: boolean } {
    const S = TextTool;
    return { heightMm: S.heightMm, angle: S.angle, align: S.align, widthFactor: S.widthFactor, mask: S.mask };
  }

  /**
   * A text template's height on paper, alignment and mask for its run (docs/adr/0176 §3b, tools/templateSeeds.ts);
   * gives back the ones it replaced, which the run's end puts back.
   */
  static useOptions(o: { readonly heightMm: number; readonly align: TextAlign | null; readonly mask: boolean }): { heightMm: number; align: TextAlign | null; mask: boolean } {
    const S = TextTool;
    const before = { heightMm: S.heightMm, align: S.align, mask: S.mask };
    S.heightMm = o.heightMm;
    S.align = o.align;
    S.mask = o.mask;
    return before;
  }

  protected promptFor(): string {
    const S = TextTool;
    switch (this.stage) {
      case 'height':
        return 'kâğıt üzerindeki yazı yüksekliğini mm olarak yazın';
      case 'angle':
        return this.angleFrom ? 'doğrultunun ikinci noktasına tıklayın' : 'açıyı yazın (derece) ya da doğrultu için iki noktaya tıklayın';
      case 'align':
        return `hizayı seçin ya da adını bitişik yazın: sağüst, orta, soltaban … [Hiza (H): ${textAlignName(S.align)}]`;
      case 'width':
        return `genişlik çarpanını yazın (1: harflerin kendi eni; 0'dan büyük, en çok ${MAX_WIDTH_FACTOR})`;
      case 'typing':
        return 'yazıyı tıkladığınız yere yazın; Enter ekler, Esc vazgeçer';
      default:
        return (
          `yazının başlangıcına tıklayın [Yükseklik (Y): ${S.heightMm} mm / Açı (A): ${+fixed(S.angle, 4)}° / ` +
          `Hiza (H): ${textAlignName(S.align)} / Genişlik (G): ${+fixed(S.widthFactor, 4)} / ` +
          `Zemin (Z): ${S.mask ? 'açık' : 'kapalı'} / Artır (R): ${S.increment ? 'açık' : 'kapalı'}]`
        );
    }
  }

  override snapFrom(): Vec2 | null {
    return this.angleFrom ?? this.at;
  }

  protected override get last(): Vec2 | null {
    return this.stage === 'angle' ? this.angleFrom : null;
  }

  protected override option(key: string): boolean {
    const S = TextTool;
    if (this.stage !== 'pos') return false;
    switch (key) {
      case 'Y':
        this.stage = 'height';
        break;
      case 'A':
        this.stage = 'angle';
        this.angleFrom = null;
        break;
      case 'H':
        this.stage = 'align';
        break;
      case 'G':
        this.stage = 'width';
        break;
      case 'Z':
        S.mask = !S.mask;
        break;
      case 'R':
        S.increment = !S.increment;
        break;
      default:
        return false;
    }
    this.refreshPrompt();
    return true;
  }

  /** Hiza's menu: the twelve points, row by row (docs/adr/0145 §6), while the tool waits for a click or for one. */
  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key !== 'H' || (this.stage !== 'pos' && this.stage !== 'align')) return null;
    return TEXT_ALIGN_ROWS.flat().map((a) => ({ label: capital(textAlignName(a)), typed: textAlignName(a), icon: alignIcon(a), checked: a === TextTool.align }));
  }

  chooseOption(key: string, typed: string): boolean {
    if (key !== 'H' || (this.stage !== 'pos' && this.stage !== 'align')) return false;
    return this.takeAlign(typed);
  }

  /** A typed or chosen alignment: kept, and the tool waits for the click again; a word that names none is said. */
  private takeAlign(typed: string): boolean {
    const align = textAlignFromName(typed);
    if (align === undefined) {
      this.ctx.log.warn(`“${typed}” bir hiza adı değil. Hizayı menüden seçin ya da adını bitişik yazın: solüst, ortaüst, sağüst, solorta, orta, sağorta, solalt, ortaalt, sağalt, soltaban, ortataban, sağtaban.`);
      return true;
    }
    TextTool.align = align;
    this.stage = 'pos';
    this.refreshPrompt();
    return true;
  }

  protected onPoint(p: Vec2): void {
    const S = TextTool;
    if (this.stage === 'angle') {
      if (!this.angleFrom) return void (this.angleFrom = p);
      if (dist(this.angleFrom, p) < 1e-9) return;
      // Kept readable: a direction pointing left is turned around.
      S.angle = textAngle(this.angleFrom, p);
      this.angleFrom = null;
      this.stage = 'pos';
      return;
    }
    if (this.stage !== 'pos') return;
    // A locked layer is said at the click, before the field opens, not after the text is typed.
    const layers = this.ctx.doc.layers;
    if (layers.isLocked(layers.active.value)) return void this.targetLayer();
    this.at = p;
    this.stage = 'typing';
    const height = paper(this.ctx, S.heightMm);
    const { align, widthFactor, mask } = S;
    // Artır: the last text's number one more; a text that ends with no number comes back as it is (docs/adr/0145 §3).
    const initial = S.increment && this.lastText !== null ? (textIncrement(this.lastText) ?? this.lastText) : undefined;
    this.ctx.view.requestTextInput({
      at: p,
      height,
      rotation: S.angle,
      align,
      widthFactor,
      initial,
      commit: (text) => {
        // Written by `cad.entities.create` (step “Ekle”). Refused (the layer was locked meanwhile): the refusal
        // was said, nothing was added. The defaults are no fields: the left of the baseline, a factor of 1, no mask.
        const written = this.writeObjects([
          { kind: 'text', p, text, height, rotation: S.angle, ...(align && { align }), ...(widthFactor !== 1 && { widthFactor }), ...(mask && { mask: true }) },
        ]);
        if (written) {
          this.lastText = text;
          this.ctx.log.success(`Yazı eklendi: “${text}”`);
        }
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
    const S = TextTool;
    const t = text.trim();
    if (this.option(t.toLocaleUpperCase('tr-TR'))) return true;
    if (this.stage === 'align') return this.takeAlign(t);
    const n = parseNumber(t);
    if (this.stage === 'height') {
      if (n === null || n <= 0) return false;
      S.heightMm = n;
      this.stage = 'pos';
      this.refreshPrompt();
      return true;
    }
    if (this.stage === 'angle') {
      if (n === null) return false;
      S.angle = n;
      this.angleFrom = null;
      this.stage = 'pos';
      this.refreshPrompt();
      return true;
    }
    if (this.stage === 'width') {
      if (n === null) return false;
      if (!widthFactorOk(n)) {
        this.ctx.log.warn(`Genişlik çarpanı 0'dan büyük, en çok ${MAX_WIDTH_FACTOR} olmalı; ${t} verildi. Harflerin kendi eni için 1 yazın.`);
        return true;
      }
      S.widthFactor = n;
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
    const S = TextTool;
    const pal = this.ctx.view.palette;
    if (this.stage === 'angle' && this.angleFrom && this.hover) {
      strokePath(g, view, [this.angleFrom, this.hover], { color: pal.accent, dash: [3, 3] });
      drawTag(g, view.worldToScreen(this.hover), [`Açı ${fixed(angleDeg(this.angleFrom, this.hover), 2)}°`], pal.accent, pal.labelHalo);
      return;
    }
    // Where the text will sit: a box of its height along its angle, four heights wide times its width factor,
    // placed about the pointer as its alignment says (docs/adr/0145), the pointer's point marked.
    const at = this.stage === 'pos' ? this.hover : null;
    if (!at) return;
    const s = view.worldToScreen(at);
    const px = Math.max(8, paper(this.ctx, S.heightMm) * view.scale);
    const w = px * 4 * S.widthFactor;
    const [along, up] = textAlignShares(S.align);
    g.save();
    g.translate(s.x, s.y);
    g.rotate((-S.angle * Math.PI) / 180);
    g.strokeStyle = pal.accent;
    g.setLineDash([3, 3]);
    g.strokeRect(-along * w, -(1 - up) * px, w, px);
    g.setLineDash([]);
    g.fillStyle = pal.accent;
    g.beginPath();
    g.arc(0, 0, 2.5, 0, Math.PI * 2);
    g.fill();
    g.restore();
  }
}

// ── Okunur yap ─────────────────────────────────────────────────────────

/**
 * Okunur yap (docs/adr/0145 §6): the selected texts that read upside down (turned more than 90° and at most 270°)
 * turned half round about their box's middle, the box where it was and the alignment kept (the core's
 * `textReadable`, measured in the drawing's typeface). With texts selected it acts at once; otherwise the user picks
 * and presses Enter. One step, “Okunur yap” (`cad.entities.edit`'s `readable`). The desktop's is
 * `kentos_interaction::object` (`ObjectAction::readable`).
 */
export class ReadableTool extends SelectionActionTool {
  readonly id = 'readable';
  protected readonly label = 'Okunur yap';

  protected run(targets: Entity[]): void {
    const { log } = this.ctx;
    const texts = targets.filter((e): e is TextEntity => e.kind === 'text');
    if (!texts.length) return log.warn('Seçimde yazı yok. Okunur yap yazı nesnelerini çevirir; yazıları seçip yeniden deneyin.');
    const font = this.ctx.doc.settings.drawingFont.value;
    const changes: EntityEdit[] = [];
    for (const t of texts) {
      const turned = textReadable({ ...t, font });
      if (!turned) continue;
      const geometry = { ...geometryOf(t as unknown as EditGeometry), p: turned.p, rotation: turned.rotation } as unknown as EditGeometry;
      changes.push({ kind: 'update', uid: uidOf(this.ctx, t), geometry });
    }
    if (!changes.length) return log.info(`Ters okunan yazı yok: ${texts.length} yazının hepsi okunuyor.`);
    if (!writeEdit(this.ctx, 'readable', changes)) return;
    log.success(`${changes.length} yazı okunur yapıldı.`);
  }
}
