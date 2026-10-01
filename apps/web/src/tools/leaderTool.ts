import type { AppContext } from '../app/context';
import type { LeaderArrow } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { leaderLayout, type LeaderLayout } from '../model/geom/leader';
import type { ViewTransform } from '../viewport/Camera';
import { TextTool } from './annotateTools';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { strokePath } from './preview';
import type { OptionChoice } from './Tool';

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

/**
 * The arrowheads in Ok's menu (docs/adr/0146 §7): the value (null: the filled arrow, the field's absence), its name
 * as the prompt writes and the user types it, its menu label and its icon. The desktop's are
 * `kentos_interaction::leader::ARROWS`.
 */
export const LEADER_ARROW_ROWS: readonly (readonly [LeaderArrow | null, string, string, string])[] = [
  [null, 'dolu', 'Dolu', 'leaderArrowFilled'],
  ['open', 'açık', 'Açık', 'leaderArrowOpen'],
  ['dot', 'nokta', 'Nokta', 'leaderArrowDot'],
  ['none', 'yok', 'Yok', 'leaderArrowNone'],
];

/** A name as typed, folded: lower case, the Turkish letters without their marks, letters alone (“Açık” → “acik”). */
const fold = (s: string) =>
  s
    .toLocaleLowerCase('tr-TR')
    .replace(/ç/g, 'c')
    .replace(/ğ/g, 'g')
    .replace(/ı/g, 'i')
    .replace(/ö/g, 'o')
    .replace(/ş/g, 's')
    .replace(/ü/g, 'u')
    .replace(/[^a-z]/g, '');

/** An arrowhead's name: “dolu”, “açık”, “nokta”, “yok”. */
export const leaderArrowName = (a: LeaderArrow | null): string => LEADER_ARROW_ROWS.find((r) => r[0] === a)?.[1] ?? 'dolu';

/** The arrowhead a typed name is, with or without the Turkish marks; undefined when it names none. */
export function leaderArrowFromName(typed: string): LeaderArrow | null | undefined {
  const f = fold(typed);
  return LEADER_ARROW_ROWS.find((r) => fold(r[1]) === f)?.[0];
}

/**
 * Kılavuz (docs/adr/0146 §7): the first click is the arrow's tip, the next ones its vertices. Enter or a right click
 * ends them: a text field opens past the landing's end, on the side the last segment goes, and Enter writes the
 * leader with its note; Enter in the empty field writes it without one (the arrow alone); Esc there goes back to the
 * vertices. Ok (O) chooses the arrowhead from its menu or by its name, Yükseklik (Y) is Yazı's (paper mm, shared),
 * Zemin (Z) fills the note's box, Geri (G) takes the last vertex back. The preview is the core's layout. Written by
 * `cad.entities.create` (step “Kılavuz”), in the current colour and line weight. The desktop's is
 * `kentos_interaction::leader`.
 */
export class LeaderTool extends PointInputTool {
  readonly id = 'leader';
  protected readonly label = 'Kılavuz';
  protected override readonly stepsFromPoints = true;
  private static arrow: LeaderArrow | null = null;
  private static mask = false;
  private stage: 'pts' | 'height' | 'arrow' | 'typing' = 'pts';

  /** Kılavuz's options as they are now: they stay for as long as the app lives, as Yazı's do. */
  static options(): { arrow: LeaderArrow | null; mask: boolean } {
    return { arrow: LeaderTool.arrow, mask: LeaderTool.mask };
  }

  protected promptFor(count: number): string {
    const S = LeaderTool;
    switch (this.stage) {
      case 'height':
        return 'kâğıt üzerindeki not yüksekliğini mm olarak yazın (Yazı ile ortak)';
      case 'arrow':
        return `ok başını seçin ya da adını yazın: dolu, açık, nokta, yok [Ok (O): ${leaderArrowName(S.arrow)}]`;
      case 'typing':
        return 'notu kolun ucuna yazın; Enter ekler, boş Enter notsuz ekler, Esc köşelere döner';
    }
    const options = `[Ok (O): ${leaderArrowName(S.arrow)} / Yükseklik (Y): ${TextTool.options().heightMm} mm / Zemin (Z): ${S.mask ? 'açık' : 'kapalı'}${count ? ' / Geri (G)' : ''}]`;
    if (count === 0) return `okun ucuna tıklayın ${options}`;
    if (count === 1) return `sonraki köşeye tıklayın ${options}`;
    return `sonraki köşeye tıklayın; Enter ya da sağ tık notu yazdırır ${options}`;
  }

  protected override option(key: string): boolean {
    if (this.stage !== 'pts') return false;
    switch (key) {
      case 'O':
        this.stage = 'arrow';
        break;
      case 'Y':
        this.stage = 'height';
        break;
      case 'Z':
        LeaderTool.mask = !LeaderTool.mask;
        break;
      case 'G':
        if (!this.pts.length) return false;
        this.pts.pop();
        break;
      default:
        return false;
    }
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** Ok's menu: the four arrowheads, while the tool waits for a vertex or for one. */
  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key !== 'O' || (this.stage !== 'pts' && this.stage !== 'arrow')) return null;
    return LEADER_ARROW_ROWS.map(([a, typed, label, icon]) => ({ label, typed, icon, checked: a === LeaderTool.arrow }));
  }

  chooseOption(key: string, typed: string): boolean {
    if (key !== 'O' || (this.stage !== 'pts' && this.stage !== 'arrow')) return false;
    return this.takeArrow(typed);
  }

  /** A typed or chosen arrowhead: kept, and the tool waits for its vertices again; a word that names none is said. */
  private takeArrow(typed: string): boolean {
    const arrow = leaderArrowFromName(typed);
    if (arrow === undefined) {
      this.ctx.log.warn(`“${typed}” bir ok başı adı değil. Ok başını menüden seçin ya da adını yazın: dolu, açık, nokta, yok.`);
      return true;
    }
    LeaderTool.arrow = arrow;
    this.stage = 'pts';
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  protected onPoint(p: Vec2): void {
    if (this.stage !== 'pts') return;
    if (!this.pts.length) {
      // A locked layer is said at the tip's click, not after the note is typed.
      const layers = this.ctx.doc.layers;
      if (layers.isLocked(layers.active.value)) return void this.targetLayer();
    }
    // A click on the last vertex adds none.
    const last = this.pts.at(-1);
    if (last && last.x === p.x && last.y === p.y) return;
    this.pts.push(p);
  }

  override input(text: string): boolean {
    const t = text.trim();
    if (this.option(t.toLocaleUpperCase('tr-TR'))) return true;
    if (this.stage === 'arrow') return this.takeArrow(t);
    if (this.stage === 'height') {
      const n = parseNumber(t);
      if (n === null || n <= 0) return false;
      TextTool.setHeight(n);
      this.stage = 'pts';
      this.refreshPrompt();
      return true;
    }
    return super.input(text);
  }

  /** Enter or a right click: the vertices end and the note's field opens; with the tip alone it is said; with none the tool leaves. */
  override confirm(): void {
    if (this.stage === 'typing') return;
    if (this.stage !== 'pts') {
      this.stage = 'pts';
      return this.refreshPrompt();
    }
    if (!this.pts.length) return this.ctx.tools.exit();
    if (this.pts.length < 2) return void this.ctx.log.warn('Kılavuzun en az 2 köşesi olur: okun ucundan sonra bir köşeye daha tıklayın.');
    this.openNote();
  }

  /** Esc, one step back: out of a question, then the leader being drawn; with nothing drawn the tool leaves. */
  cancel(): boolean {
    if (this.stage !== 'pts') {
      this.stage = 'pts';
      this.refreshPrompt();
      return true;
    }
    if (!this.pts.length) return false;
    this.reset();
    return true;
  }

  /** The note's field, past the landing's end on the note's side; its answer writes the leader. */
  private openNote(): void {
    const height = paper(this.ctx, TextTool.options().heightMm);
    const pts = [...this.pts];
    const l = this.layout(pts, height, true);
    if (!l?.notePoint) return;
    this.stage = 'typing';
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    this.ctx.view.requestTextInput({
      at: l.notePoint,
      height,
      rotation: 0,
      align: l.noteAlign ?? null,
      placeholder: 'Notu yazın',
      hint: 'Enter: ekle · boş Enter: notsuz · Esc: köşelere dön',
      commit: (text) => this.write(pts, height, text),
      empty: () => this.write(pts, height, null),
      cancel: () => {
        this.stage = 'pts';
        this.refreshPrompt();
        this.ctx.view.requestOverlay();
        this.ctx.view.focus();
      },
    });
  }

  /** The leader through `pts`, with its note when one was typed, in one step (“Kılavuz”); the tool waits for the next one. */
  private write(pts: Vec2[], height: number, text: string | null): void {
    const S = LeaderTool;
    // The defaults are no fields: a filled arrow, no mask (a mask without a note masks nothing).
    const written = this.writeObjects(
      [{ kind: 'leader', pts, ...(text && { text }), height, rotation: 0, ...(S.arrow && { arrow: S.arrow }), ...(S.mask && text && { mask: true }) }],
      'leader',
    );
    if (written) this.ctx.log.success(text ? `Kılavuz eklendi: “${text}”` : 'Kılavuz eklendi (notsuz).');
    this.stage = 'pts';
    this.reset();
    this.ctx.view.focus();
  }

  /** The core's layout of a leader through `pts` with this tool's arrowhead, its note's place when `note`. */
  private layout(pts: Vec2[], height: number, note: boolean): LeaderLayout | null {
    const arrow = LeaderTool.arrow;
    return leaderLayout({ kind: 'leader', id: 0, layerId: '', attrs: {}, pts, height, rotation: 0, ...(arrow && { arrow }), ...(note && { text: 'Not' }) });
  }

  /** The leader as it will be, through the pointer: its arrowhead, line and landing, and its note's place as a dashed box. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const pts = this.stage === 'pts' && this.hover ? [...this.pts, this.hover] : this.pts;
    if (pts.length < 2) return;
    const height = paper(this.ctx, TextTool.options().heightMm);
    const l = this.layout(pts, height, true);
    if (!l) return;
    strokePath(g, view, l.landing ? [...pts, l.landing[1]] : pts, { color: pal.accent, width: 1.5 });
    const head = l.head;
    if (head.kind === 'filled') strokePath(g, view, head.triangle, { color: pal.accent, closed: true, fill: pal.accent });
    else if (head.kind === 'open') strokePath(g, view, head.lines, { color: pal.accent, width: 1.5 });
    else if (head.kind === 'dot') {
      const s = view.worldToScreen(head.center);
      g.save();
      g.fillStyle = pal.accent;
      g.beginPath();
      g.arc(s.x, s.y, Math.max(1.5, head.radius * view.scale), 0, Math.PI * 2);
      g.fill();
      g.restore();
    }
    // The field stands there while the note is typed.
    if (!l.notePoint || this.stage === 'typing') return;
    const s = view.worldToScreen(l.notePoint);
    const px = Math.max(8, height * view.scale);
    const w = px * 4;
    g.save();
    g.strokeStyle = pal.accent;
    g.setLineDash([3, 3]);
    g.strokeRect(s.x - (l.noteAlign === 'middleRight' ? w : 0), s.y - px / 2, w, px);
    g.restore();
  }

  protected override reset(): void {
    this.stage = 'pts';
    super.reset();
  }
}
