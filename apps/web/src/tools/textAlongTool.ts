import type { AppContext } from '../app/context';
import type { EditOperation } from '../contracts/generated/EditOperation';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry } from '../contracts/generated/EntityGeometry';
import { Signal } from '../core/signal';
import { textBox, type Entity, type TextAlign, type TextEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { textLines } from '../model/paragraph';
import { ALONG_SHARES, ALONG_SIDES, alongAlign, alongSplit, textAlongLength, textAlongPiece, textAlongStraight, textAlongTurn, type AlongSide } from '../model/textAlong';
import type { ViewTransform } from '../viewport/Camera';
import { LABEL, LABEL_STRIDE } from '../viewport/storeRecords';
import { TextTool } from './annotateTools';
import { parseNumber } from './coordinateInput';
import * as createCommand from './createCommand';
import { geometryOf } from '../product/entitiesEdit';
import { uidOf, writeEdit } from './editCommand';
import { pickedEdge } from './locks';
import { SelectionFirstTool } from './modifyTools';
import { drawTextGhost, strokePath } from './preview';
import { stylesShown, takeTextStyle, textFaceNow, textStyleChoices, textStyleName } from './styleOption';
import type { OptionChoice, Tool, ToolPointer } from './Tool';

/**
 * Eğri boyunca yazı (docs/adr/0196 §4; the desktop's `kentos_interaction::text_along`).
 *
 * - **Eğri boyunca yazı** (`textAlong`): a click picks the curve (a line, an arc, a circle, a polyline's part, an area's
 *   ring or hole, an ellipse, a spline); the next places the text on it, its letters following the cursor along the
 *   curve (the last text written, “Yazı” at first); a text field opens there, and Enter writes the text along the
 *   piece of the curve its letters take (the core's `textAlongPiece`), through `cad.entities.create` (step “Eğri
 *   boyunca yazı”). Yükseklik (Y) is Yazı's; Hiza (H: Başı, Ortası, Sonu) and Konum (K: Üstünde, Ortasında, Altında)
 *   stay for the session; Stil (S) in a CAD project.
 * - **Yazıyı eğriye oturt** (`textCurve`), on the modify tools' base, its methods' letters: Eğriye oturt (O), Doğrultuya
 *   döndür (D), Düzleştir (Z); through `cad.entities.edit` (`textPath`, `textTurn`, `textStraighten`), one step each.
 */

const LABEL_ALONG = 'Eğri boyunca yazı';
const LABEL_CURVE = 'Yazıyı eğriye oturt';
/** What the preview writes before anything was typed. */
const SAMPLE = 'Yazı';
/** At most this many texts are previewed at once. */
const MAX_PREVIEWED = 50;
/** The kinds a text may follow (§4). */
const CURVE_KINDS: readonly Entity['kind'][] = ['line', 'arc', 'circle', 'polyline', 'polygon', 'ellipse', 'spline'];

const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

/** A text's face fields (docs/adr/0183 §2), as a text carries them. */
type Face = Pick<TextEntity, 'textStyle' | 'font' | 'bold' | 'italic' | 'oblique'>;

/** The letters' length of a text, metres, in its face or the drawing's typeface. */
function lettersLength(ctx: AppContext, t: Pick<TextEntity, 'text' | 'height' | 'widthFactor' | 'runs'> & Face): number {
  return textAlongLength({ ...t, font: t.font ?? ctx.doc.settings.drawingFont.value });
}

/** A text along its curve, faint, letter by letter (the store's records, §2.6). */
function drawLetters(g: CanvasRenderingContext2D, view: ViewTransform, ctx: AppContext, t: TextEntity): void {
  const pal = ctx.view.palette;
  const letters = [...t.text];
  const records = textLines({ ...t, font: t.font ?? ctx.doc.settings.drawingFont.value, mask: false });
  for (let i = 0; i + LABEL_STRIDE <= records.length; i += LABEL_STRIDE) {
    if (records[i + 1] !== LABEL.line) continue;
    const ch = letters[records[i + 7]];
    if (!ch || !ch.trim()) continue;
    const face: Face = { textStyle: t.textStyle, font: t.font, bold: t.bold, italic: t.italic, oblique: t.oblique };
    drawTextGhost(g, view, { p: { x: records[i + 2], y: records[i + 3] }, text: ch, height: t.height, rotation: records[i + 4], face, widthFactor: t.widthFactor }, { color: pal.accent, font: pal.drawingFont, mask: null });
  }
}

/** A text as the edit command takes it: its own geometry fields. */
const textGeometry = (t: TextEntity): EntityGeometry => geometryOf(t as unknown as EntityGeometry) as unknown as EntityGeometry;

// ── Eğri boyunca yazı ───────────────────────────────────────────────────

export class TextAlongTool implements Tool {
  readonly id = 'textAlong';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  /** Hiza and Konum as one alignment, kept for the session (the desktop's `Memory::along_align`). */
  static align: TextAlign = 'bottomCenter';
  private readonly ctx: AppContext;
  private stage: 'curve' | 'place' | 'height' | 'style' | 'typing' = 'curve';
  private curve: Entity | null = null;
  private shown: TextEntity | null = null;
  private lastText: string | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    this.refresh();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  private refresh(): void {
    const [share, side] = alongSplit(TextAlongTool.align);
    const options = `[${stylesShown(this.ctx) ? `Stil (S): ${textStyleName(this.ctx)} / ` : ''}Yükseklik (Y): ${TextTool.options().heightMm} mm / Hiza (H): ${ALONG_SHARES.find(([s]) => s === share)?.[1] ?? 'Ortası'} / Konum (K): ${ALONG_SIDES.find(([s]) => s === side)?.[1] ?? 'Üstünde'}]`;
    const step = {
      curve: `yazının izleyeceği çizgiye, yaya, daireye, alana ya da eğriye tıklayın ${options}`,
      place: `yazının yerine tıklayın; Esc başka eğri seçtirir ${options}`,
      height: 'kâğıt üzerindeki yazı yüksekliğini mm olarak yazın',
      style: 'yazı stilini menüden seçin ya da adını yazın',
      typing: 'yazıyı yazın; Enter eğri boyunca ekler, Esc vazgeçer',
    }[this.stage];
    this.prompt.set(`${LABEL_ALONG}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  /** The text `words` where the click puts it on the curve; null off any piece. */
  private placed(words: string, at: Vec2): TextEntity | null {
    if (!this.curve) return null;
    const height = paper(this.ctx, TextTool.options().heightMm);
    const face = textFaceNow(this.ctx);
    const length = lettersLength(this.ctx, { text: words, height, ...face });
    const [share] = alongSplit(TextAlongTool.align);
    const piece = textAlongPiece(this.curve, at, length, share);
    if (!piece) return null;
    return { kind: 'text', id: 0, layerId: '', attrs: {}, p: piece.p, text: words, height, rotation: piece.rotation, align: TextAlongTool.align, ...face, path: piece.path };
  }

  private curveUnder(p: ToolPointer): Entity | null {
    return this.ctx.view.pickEdge(p.screen, (e) => CURVE_KINDS.includes(e.kind));
  }

  pointerMove(p: ToolPointer): void {
    if (this.stage === 'curve') this.ctx.selection.hover.set(this.curveUnder(p)?.id ?? null);
    else if (this.stage === 'place') {
      // The curve picked stays lit while the text is placed.
      this.ctx.selection.hover.set(this.curve?.id ?? null);
      this.shown = this.placed(this.lastText ?? SAMPLE, p.raw);
    }
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.stage === 'curve') {
      const e = this.curveUnder(p);
      if (!e) return void this.ctx.log.warn('Tıklanan yerde çizgi, yay, daire, alan ya da eğri yok. Yazının izleyeceği nesneye tıklayın.');
      this.curve = e;
      this.stage = 'place';
      return this.refresh();
    }
    if (this.stage !== 'place') return;
    const layers = this.ctx.doc.layers;
    if (layers.isLocked(layers.active.value)) {
      const name = layers.get(layers.active.value)?.name ?? layers.active.value;
      return void this.ctx.log.warn(`“${name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.`);
    }
    const t = this.placed(this.lastText ?? SAMPLE, p.raw);
    if (!t) return;
    const at = p.raw;
    this.stage = 'typing';
    this.refresh();
    this.ctx.view.requestTextInput({
      at,
      height: t.height,
      rotation: t.rotation,
      align: 'baselineCenter',
      initial: this.lastText ?? undefined,
      hint: 'Eğri boyunca yazılacak yazı',
      face: textFaceNow(this.ctx),
      commit: (text) => {
        const words = text.trim();
        const made = words ? this.placed(words, at) : null;
        if (made) {
          const { kind, p: q, text: w, height, rotation, align, path, textStyle, font, bold, italic, oblique } = made;
          const geometry = { kind, p: q, text: w, height, rotation, align, path, ...(textStyle !== undefined && { textStyle }), ...(font !== undefined && { font }), ...(bold && { bold }), ...(italic && { italic }), ...(oblique !== undefined && { oblique }) } as EntityGeometry;
          if (createCommand.writeObjects(this.ctx, [geometry], 'textAlong')) {
            this.lastText = words;
            this.ctx.log.success(`Eğri boyunca yazı eklendi: “${words}”`);
          }
        }
        this.afterTyping();
      },
      cancel: () => this.afterTyping(),
    });
  }

  private afterTyping(): void {
    this.curve = null;
    this.shown = null;
    this.ctx.selection.hover.set(null);
    this.stage = 'curve';
    this.refresh();
    this.ctx.view.focus();
  }

  input(text: string): boolean {
    const t = text.trim();
    const key = t.toLocaleUpperCase('tr-TR');
    if (this.stage === 'curve' || this.stage === 'place') {
      const [share, side] = alongSplit(TextAlongTool.align);
      if (key === 'Y') return (this.stage = 'height'), this.refresh(), true;
      if (key === 'H') {
        TextAlongTool.align = alongAlign(share < 0.25 ? 0.5 : share < 0.75 ? 1 : 0, side);
        return this.refresh(), true;
      }
      if (key === 'K') {
        TextAlongTool.align = alongAlign(share, side === 'over' ? 'on' : side === 'on' ? 'under' : 'over');
        return this.refresh(), true;
      }
      if (key === 'S' && stylesShown(this.ctx)) return (this.stage = 'style'), this.refresh(), true;
    }
    if (this.stage === 'style') return this.takeStyle(t);
    if (this.stage === 'height') {
      const n = parseNumber(t);
      if (n === null || n <= 0) return false;
      TextTool.setHeight(n);
      this.stage = this.curve ? 'place' : 'curve';
      return this.refresh(), true;
    }
    return false;
  }

  private takeStyle(typed: string): boolean {
    const s = takeTextStyle(this.ctx, typed);
    if (s === undefined) return true;
    TextTool.useStyle(s);
    if (this.stage === 'style') this.stage = this.curve ? 'place' : 'curve';
    this.refresh();
    return true;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    const [share, side] = alongSplit(TextAlongTool.align);
    if (key === 'S' && stylesShown(this.ctx)) return textStyleChoices(this.ctx);
    if (key === 'H')
      return ALONG_SHARES.map(([s, label, typed]) => ({ label, typed, icon: s === 0 ? 'textAlignBaselineLeft' : s === 1 ? 'textAlignBaselineRight' : 'textAlignBaselineCenter', checked: s === share }));
    if (key === 'K')
      return ALONG_SIDES.map(([s, label, typed]) => ({ label, typed, icon: s === 'over' ? 'textAlignBottomCenter' : s === 'on' ? 'textAlignMiddleCenter' : 'textAlignTopCenter', checked: s === side }));
    return null;
  }

  chooseOption(key: string, typed: string): boolean {
    const [share, side] = alongSplit(TextAlongTool.align);
    if (key === 'S' && stylesShown(this.ctx)) return this.takeStyle(typed);
    if (key === 'H') {
      const found = ALONG_SHARES.find(([, , w]) => w === typed);
      if (!found) return false;
      TextAlongTool.align = alongAlign(found[0], side);
      return this.refresh(), true;
    }
    if (key === 'K') {
      const found = ALONG_SIDES.find(([, , w]) => w === typed);
      if (!found) return false;
      TextAlongTool.align = alongAlign(share, found[0] as AlongSide);
      return this.refresh(), true;
    }
    return false;
  }

  takesWords(): boolean {
    return this.stage === 'style';
  }

  /** Esc steps back: from the place to the curve, then out. */
  cancel(): boolean {
    if (this.stage === 'curve') return false;
    if (this.stage === 'place') {
      this.curve = null;
      this.shown = null;
    }
    this.stage = this.curve ? 'place' : 'curve';
    this.refresh();
    return true;
  }

  confirm(): void {
    if (!this.cancel()) this.ctx.tools.exit();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.stage === 'place' && this.shown) {
      drawLetters(g, view, this.ctx, this.shown);
      const s = view.worldToScreen(this.shown.p);
      g.save();
      g.strokeStyle = pal.accent;
      g.lineWidth = 2;
      g.beginPath();
      g.arc(s.x, s.y, 2.5, 0, Math.PI * 2);
      g.stroke();
      g.restore();
    }
  }
}

// ── Yazıyı eğriye oturt ─────────────────────────────────────────────────

type Method = 'fit' | 'turn' | 'straighten';

/** The methods' chips: their words and letters, in the tool's order. */
const METHODS: readonly [Method, string, string][] = [
  ['fit', 'Eğriye oturt', 'O'],
  ['turn', 'Doğrultuya döndür', 'D'],
  ['straighten', 'Düzleştir', 'Z'],
];

const OPERATION: Record<Method, EditOperation> = { fit: 'textPath', turn: 'textTurn', straighten: 'textStraighten' };

export class TextCurveTool extends SelectionFirstTool {
  readonly id = 'textCurve';
  protected readonly label = LABEL_CURVE;
  private method: Method = 'fit';
  private texts: TextEntity[] = [];
  /** The selection's texts the method leaves (other kinds of text, on locked layers) and whether it has any text. */
  private left: [number, number] = [0, 0];
  private any = false;
  private shown: TextEntity[] = [];

  private chips(): string {
    return `[${METHODS.map(([m, word, key]) => `${word} (${key})${m === this.method ? ': açık' : ''}`).join(' / ')}]`;
  }

  protected override pickHint(): string {
    return this.chips();
  }

  protected stagePrompt(): string {
    const step = { fit: 'yazıların oturacağı eğriye tıklayın', turn: 'doğrultusu alınacak kenara tıklayın', straighten: 'Enter ile düzleştirin' }[this.method];
    return `${step} ${this.chips()}`;
  }

  /** The selection's texts the method takes, off locked layers. */
  private gather(): void {
    const { doc } = this.ctx;
    this.texts = [];
    this.left = [0, 0];
    this.any = false;
    for (const e of this.targets()) {
      if (e.kind !== 'text') continue;
      this.any = true;
      if (doc.layers.isLocked(e.layerId)) {
        this.left[1]++;
        continue;
      }
      const taken =
        this.method === 'fit'
          ? !e.text.includes('\n') && e.boxWidth === undefined && e.lineSpacing === undefined && e.labelOf === undefined
          : this.method === 'turn'
            ? e.path === undefined
            : e.path !== undefined;
      if (taken) this.texts.push(e);
      else this.left[0]++;
    }
  }

  protected begin(): void {
    this.shown = [];
    this.gather();
    if (!this.any) {
      this.ctx.log.warn('Seçimde yazı yok. Yazıları seçip yeniden deneyin.');
      return void queueMicrotask(() => this.ctx.tools.exit());
    }
    if (this.method === 'straighten') this.straightenNow();
  }

  private straightenNow(): void {
    if (!this.texts.length) return this.noneLeft();
    this.write(this.straightened());
  }

  private noneLeft(): void {
    this.ctx.log.warn(
      {
        fit: 'Seçimde eğriye oturacak yazı yok. Tek satırlık yazıları seçip yeniden deneyin.',
        turn: 'Seçimde döndürülecek düz yazı yok. Yazıları seçip yeniden deneyin.',
        straighten: 'Seçimde eğri boyunca yazı yok. Eğri boyunca yazıları seçip yeniden deneyin.',
      }[this.method],
    );
    queueMicrotask(() => this.ctx.tools.exit());
  }

  override input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    const chip = METHODS.find(([, , k]) => k === key);
    if (chip) {
      this.method = chip[0];
      this.shown = [];
      if (!this.picking) this.begin();
      this.refresh();
      return true;
    }
    return super.input(text);
  }

  /** Each text put on the curve `e` where its box's middle falls. */
  private fitted(e: Entity): TextEntity[] {
    const font = this.ctx.doc.settings.drawingFont.value;
    const out: TextEntity[] = [];
    for (const t of this.texts) {
      const { path: _, ...straight } = t;
      const ring = textBox({ ...straight, font: t.font ?? font });
      const xs = ring.map((q) => q.x);
      const ys = ring.map((q) => q.y);
      const middle = { x: (Math.min(...xs) + Math.max(...xs)) / 2, y: (Math.min(...ys) + Math.max(...ys)) / 2 };
      const piece = textAlongPiece(e, middle, lettersLength(this.ctx, t), 0.5);
      if (piece) out.push({ ...t, p: piece.p, rotation: piece.rotation, path: piece.path });
    }
    return out;
  }

  private turned(degrees: number): TextEntity[] {
    return this.texts.map((t) => ({ ...t, rotation: degrees }));
  }

  private straightened(): TextEntity[] {
    const font = this.ctx.doc.settings.drawingFont.value;
    const out: TextEntity[] = [];
    for (const t of this.texts) {
      const s = textAlongStraight({ ...t, font: t.font ?? font });
      if (!s) continue;
      const { path: _, align: __, ...rest } = t;
      out.push({ ...rest, p: s.p, rotation: s.rotation });
    }
    return out;
  }

  private write(made: TextEntity[]): void {
    const { log } = this.ctx;
    const [other, locked] = this.left;
    if (other)
      log.info(
        { fit: `${other} çok satırlı ya da nesneye bağlı yazı eğriye oturmaz; atlandı.`, turn: `${other} eğri boyunca yazı döndürülmez; önce Düzleştir. Atlandı.`, straighten: `${other} yazı zaten düz; atlandı.` }[this.method],
      );
    if (locked) log.warn(`${locked} yazı kilitli katmanda; atlandı.`);
    if (!made.length) {
      log.info('Değişecek yazı yok.');
      return void queueMicrotask(() => this.ctx.tools.exit());
    }
    const changes: EntityEdit[] = made.map((t) => ({ kind: 'update', uid: uidOf(this.ctx, t.id), geometry: textGeometry(t) }));
    if (!writeEdit(this.ctx, OPERATION[this.method], changes)) return this.refresh();
    const n = made.length;
    log.success({ fit: `${n} yazı eğriye oturtuldu.`, turn: `${n} yazı doğrultuya döndürüldü.`, straighten: `${n} yazı düzleştirildi.` }[this.method]);
    queueMicrotask(() => this.ctx.tools.exit());
  }

  override pointerMove(p: ToolPointer): void {
    super.pointerMove(p);
    if (this.picking) return;
    this.shown = this.made(p);
    this.ctx.view.requestOverlay();
  }

  /** What each text would become with the cursor at `p`. */
  private made(p: ToolPointer): TextEntity[] {
    if (this.method === 'fit') {
      const e = this.ctx.view.pickEdge(p.screen, (x) => CURVE_KINDS.includes(x.kind));
      this.ctx.selection.hover.set(e?.id ?? null);
      return e ? this.fitted(e) : [];
    }
    if (this.method === 'turn') {
      const edge = pickedEdge(this.ctx, p);
      return edge ? this.turned(textAlongTurn(edge.u)) : [];
    }
    return [];
  }

  override pointerDown(p: ToolPointer): void {
    if (this.picking || p.button !== 0) return super.pointerDown(p);
    if (!this.texts.length) return this.noneLeft();
    if (this.method === 'straighten') return this.write(this.straightened());
    const made = this.made(p);
    if (!made.length) {
      this.ctx.log.warn(
        this.method === 'fit'
          ? 'Tıklanan yerde çizgi, yay, daire, alan ya da eğri yok. Yazıların oturacağı nesneye tıklayın.'
          : 'Tıklanan yerde kenar ya da yay yok. Doğrultusu alınacak kenara tıklayın.',
      );
      return;
    }
    this.write(made);
  }

  protected point(_p: Vec2): void {}

  override confirm(): void {
    if (this.picking) return super.confirm();
    if (this.method === 'straighten') return this.write(this.straightened());
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    super.draw(g, view);
    if (this.picking) return;
    const pal = this.ctx.view.palette;
    const font = this.ctx.doc.settings.drawingFont.value;
    for (const t of this.shown.slice(0, MAX_PREVIEWED)) {
      if (t.path) {
        drawLetters(g, view, this.ctx, t);
        continue;
      }
      strokePath(g, view, textBox({ ...t, font: t.font ?? font }), { color: pal.accent, closed: true, dash: [3, 3] });
      const face: Face = { textStyle: t.textStyle, font: t.font, bold: t.bold, italic: t.italic, oblique: t.oblique };
      drawTextGhost(g, view, { p: t.p, text: t.text, height: t.height, rotation: t.rotation, align: t.align, face, widthFactor: t.widthFactor }, { color: pal.accent, font: pal.drawingFont, mask: null });
    }
  }
}
