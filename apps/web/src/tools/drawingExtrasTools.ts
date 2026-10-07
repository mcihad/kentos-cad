import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { chosenTangent, commonTangents, fourthCorner, rangeRings, type Tangent } from '../model/drawingExtras';
import { tessellateCircle, type Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { ringMark } from './constructPreview';
import * as createCommand from './createCommand';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';
import type { OptionChoice, Tool, ToolPointer } from './Tool';

/**
 * Çizim ekleri (docs/adr/0197 §4; the desktop's `kentos_interaction::drawing_extras`).
 *
 * - **İki daireye teğet** (`tangentLine`): a click picks the first circle or arc near where the tangent is to touch it;
 *   over the second one the preview shows the tangent the clicks choose bright and the others dashed (the core's
 *   `commonTangents`, `chosenTangent`); the click writes it as a line through `cad.entities.create` (step “İki daireye
 *   teğet”), and the tool waits for the next first circle.
 * - **Dördüncü köşe** (`fourthCorner`): three corners clicked (snapping) or typed, the middle one second; the preview
 *   draws the parallelogram dashed; the third writes the fourth corner (`fourthCorner`) as a point, or with Çıktı (Ç)
 *   Alan the four corners as an area (step “Dördüncü köşe”).
 * - **Menzil halkaları** (`rangeRings`): the centre clicked (snapping) or typed, the rings following the cursor; Aralık
 *   (A, in the project's unit), Sayı (S, 1 to 100) and Işın (I, 0 to 360) typed; the circles and rays in one step
 *   (“Menzil halkaları”).
 *
 * Çıktı, Aralık, Sayı and Işın stay for the session (the desktop's `Memory`).
 */

const TANGENT_LABEL = 'İki daireye teğet';
const SAME = 1e-9;
const MAX_RINGS = 100;
const MAX_RAYS = 360;

/** A number and nothing else: no separator, relative or polar mark (the desktop's `plain_number`). */
function plainNumber(text: string): number | null {
  return /[,;@<]/.test(text) ? null : parseNumber(text);
}

/** What a tangent is called beside the cursor. */
const tangentName = (t: Tangent) => (t.kind.startsWith('outer') ? 'Dış teğet' : 'İç teğet');

/** Why two circles or arcs have no tangent to write. */
function noTangent(first: Entity, second: Entity): string {
  return first.kind === 'circle' && second.kind === 'circle'
    ? 'Bu iki dairenin ortak teğeti yok: biri ötekinin içinde. Birbirinin dışındaki ya da kesişen iki daire seçin.'
    : 'Bu iki nesnenin ortak teğeti yok: teğetlerin değdiği yerler yayın dışında kalıyor. Başka bir yay ya da daire seçin.';
}

/** İki daireye teğet. */
export class TangentLineTool implements Tool {
  readonly id = 'tangentLine';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  /** The first circle or arc and where it was clicked. */
  private first: { e: Entity; at: Vec2 } | null = null;
  /** The circle or arc under the cursor, the first's partner to be. */
  private under: Entity | null = null;
  private hover: Vec2 | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  get pointCount(): number {
    return this.first ? 1 : 0;
  }

  activate(): void {
    this.refresh();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  private refresh(): void {
    const step = this.first
      ? 'ikinci daireye ya da yaya, teğetin değeceği yerin yakınından tıklayın; Esc ilkini bıraktırır'
      : 'ilk daireye ya da yaya, teğetin değeceği yerin yakınından tıklayın';
    this.prompt.set(`${TANGENT_LABEL}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  /** A circle or an arc under the cursor, the first one aside. */
  private roundUnder(p: ToolPointer): Entity | null {
    const besides = this.first?.e.id;
    return this.ctx.view.pickEdge(p.screen, (e) => (e.kind === 'circle' || e.kind === 'arc') && e.id !== besides);
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.raw;
    this.under = this.roundUnder(p);
    this.ctx.selection.hover.set(this.under?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    this.hover = p.raw;
    const { log } = this.ctx;
    if (!this.first) {
      const e = this.roundUnder(p);
      if (!e) return void log.warn('Tıklanan yerde daire ya da yay yok. Teğetin değeceği daireye ya da yaya tıklayın.');
      this.first = { e, at: p.raw };
      this.under = null;
      this.ctx.selection.hover.set(null);
      return this.refresh();
    }
    const second = this.roundUnder(p);
    if (!second) return void log.warn('Tıklanan yerde ikinci bir daire ya da yay yok. Teğetin değeceği ikinci daireye ya da yaya tıklayın.');
    const t = chosenTangent(this.first.e, second, this.first.at, p.raw);
    if (!t) return void log.warn(noTangent(this.first.e, second));
    if (createCommand.writeObjects(this.ctx, [{ kind: 'line', a: t.a, b: t.b }], 'tangentLine')) log.success(`${tangentName(t)} eklendi: ${this.ctx.format.length(dist(t.a, t.b))}.`);
    this.first = null;
    this.under = null;
    this.ctx.selection.hover.set(null);
    this.refresh();
  }

  input(): boolean {
    return false;
  }

  /** The first let go; without one the tool leaves. */
  confirm(): void {
    if (!this.first) return this.ctx.tools.exit();
    this.first = null;
    this.ctx.selection.hover.set(null);
    this.refresh();
  }

  cancel(): boolean {
    this.ctx.selection.hover.set(null);
    this.under = null;
    if (!this.first) return false;
    this.first = null;
    this.refresh();
    return true;
  }

  undoStep(): boolean {
    return this.cancel();
  }

  /** Where the first was clicked; over a second, the tangent the clicks choose bright, the others dashed, its kind and length beside the cursor. */
  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.first) return;
    const pal = this.ctx.view.palette;
    const s = view.worldToScreen(this.first.at);
    g.save();
    g.strokeStyle = pal.accent;
    g.strokeRect(Math.round(s.x) - 4.5, Math.round(s.y) - 4.5, 9, 9);
    g.restore();
    if (!this.under || !this.hover) return;
    const all = commonTangents(this.first.e, this.under);
    const chosen = chosenTangent(this.first.e, this.under, this.first.at, this.hover);
    const same = (t: Tangent) => chosen !== null && t.kind === chosen.kind;
    for (const t of all) if (!same(t)) strokePath(g, view, [t.a, t.b], { color: pal.accent, dash: [4, 3] });
    const at = view.worldToScreen(this.hover);
    if (!chosen) return drawTag(g, at, ['Ortak teğet yok'], pal.danger, pal.labelHalo);
    strokePath(g, view, [chosen.a, chosen.b], { color: pal.accent, width: 1.5 });
    ringMark(g, view, chosen.a, pal.snap, 4.5);
    ringMark(g, view, chosen.b, pal.snap, 4.5);
    drawTag(g, at, [tangentName(chosen), this.ctx.format.length(dist(chosen.a, chosen.b))], pal.accent, pal.labelHalo);
  }
}

/** Whether three corners stand on one line: no parallelogram. */
function inLine(a: Vec2, b: Vec2, c: Vec2): boolean {
  const cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
  return Math.abs(cross) <= 1e-9 * dist(a, b) * dist(b, c);
}

/** Çıktı's choices: area or not, its word typed, its label, its icon. */
const OUTPUTS: readonly [boolean, string, string, string][] = [
  [false, 'nokta', 'Nokta', 'point'],
  [true, 'alan', 'Alan', 'polygon'],
];

/** Dördüncü köşe. */
export class FourthCornerTool extends PointInputTool {
  readonly id = 'fourthCorner';
  protected readonly label = 'Dördüncü köşe';
  protected override readonly stepsFromPoints = true;
  /** Çıktı: the fourth corner as a point, or the four as an area; kept for the session (the desktop's `Memory::fourth_area`). */
  static area = false;

  protected promptFor(n: number): string {
    const step = ['birinci köşeyi belirtin', 'ortadaki köşeyi belirtin (dördüncünün karşısı)', 'üçüncü köşeyi belirtin'][n] ?? '';
    return `${step} [Çıktı (Ç): ${FourthCornerTool.area ? 'Alan' : 'Nokta'}]`;
  }

  protected onPoint(p: Vec2): void {
    const last = this.pts.at(-1);
    if (last && dist(last, p) < SAME) return void this.ctx.log.warn('Köşe bir öncekiyle çakışıyor; başka bir yer gösterin.');
    this.pts.push(p);
    if (this.pts.length === 3) this.write();
  }

  /** The fourth corner of the three in, as a point or the four as an area; the corners start over. */
  private write(): void {
    const [a, b, c] = this.pts;
    this.reset();
    if (inLine(a, b, c)) return void this.ctx.log.warn('Üç köşe bir doğru üzerinde: paralelkenar olmaz. Köşeleri yeniden gösterin.');
    const d = fourthCorner(a, b, c);
    const area = FourthCornerTool.area;
    const geometry = area ? { kind: 'polygon' as const, pts: [a, b, c, d] } : { kind: 'point' as const, p: d };
    if (this.writeObjects([geometry], 'fourthCorner')) this.ctx.log.success(`${area ? 'Paralelkenar alanı eklendi' : 'Dördüncü köşe kondu'}: ${this.ctx.format.point(d)}.`);
  }

  protected override option(key: string): boolean {
    if (key !== 'Ç') return false;
    FourthCornerTool.area = !FourthCornerTool.area;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key !== 'Ç') return null;
    return OUTPUTS.map(([area, typed, label, icon]) => ({ label, typed, icon, checked: area === FourthCornerTool.area }));
  }

  chooseOption(key: string, typed: string): boolean {
    const found = key === 'Ç' ? OUTPUTS.find(([, w]) => w === typed.trim().toLocaleLowerCase('tr-TR')) : undefined;
    if (!found) return false;
    FourthCornerTool.area = found[0];
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** Corners in: they are let go; none: the tool leaves. */
  override confirm(): void {
    if (!this.pts.length) return this.ctx.tools.exit();
    this.reset();
  }

  cancel(): boolean {
    if (!this.pts.length) return false;
    this.pts.pop();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** The corners so far to the cursor; with two in, the parallelogram the cursor makes, dashed, its fourth corner marked. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const h = this.hover;
    if (!h) return;
    const pal = this.ctx.view.palette;
    const chain = [...this.pts, h];
    for (const p of this.pts) ringMark(g, view, p, pal.snap, 4.5);
    if (this.pts.length === 2 && !inLine(this.pts[0], this.pts[1], h)) {
      const [a, b] = this.pts;
      const d = fourthCorner(a, b, h);
      strokePath(g, view, [a, b, h, d], { color: pal.accent, closed: true, dash: [5, 3] });
      ringMark(g, view, d, pal.accent, 7.5);
      drawTag(g, view.worldToScreen(d), ['Dördüncü köşe', this.ctx.format.point(d)], pal.accent, pal.labelHalo);
    } else if (chain.length > 1) strokePath(g, view, chain, { color: pal.accent, dash: [5, 3] });
  }
}

/** Menzil halkaları. */
export class RangeRingsTool extends PointInputTool {
  readonly id = 'rangeRings';
  protected readonly label = 'Menzil halkaları';
  /** Aralık (metres), Sayı and Işın, kept for the session (the desktop's `Memory::ring_spacing`, `ring_count`, `ring_rays`). */
  static spacing = 10;
  static count = 5;
  static rays = 0;
  private asking: 'centre' | 'spacing' | 'count' | 'rays' = 'centre';

  protected promptFor(): string {
    const f = this.ctx.format;
    const S = RangeRingsTool;
    switch (this.asking) {
      case 'spacing':
        return `halkaların aralığını ${f.lengthUnitLabel} olarak yazın (Enter: ${f.plain(S.spacing)})`;
      case 'count':
        return `halka sayısını yazın, 1 ile 100 arası (Enter: ${S.count})`;
      case 'rays':
        return `ışın sayısını yazın, 0 ile 360 arası; 0 ışınsız (Enter: ${S.rays})`;
      default:
        return `merkezi belirtin [Aralık (A): ${f.length(S.spacing)} / Sayı (S): ${S.count} / Işın (I): ${S.rays}]`;
    }
  }

  protected onPoint(p: Vec2): void {
    const S = RangeRingsTool;
    const rings = rangeRings(p, S.spacing, S.count, S.rays);
    if (!rings) return;
    const objects = [...rings.radii.map((r) => ({ kind: 'circle' as const, c: p, r })), ...rings.rays.map((end) => ({ kind: 'line' as const, a: p, b: end }))];
    const [n, k] = [rings.radii.length, rings.rays.length];
    if (this.writeObjects(objects, 'rangeRings')) this.ctx.log.success(k ? `${n} halka ve ${k} ışın eklendi.` : `${n} halka eklendi.`);
  }

  protected override option(key: string): boolean {
    const asked = ({ A: 'spacing', S: 'count', I: 'rays' } as const)[key as 'A' | 'S' | 'I'];
    if (!asked) return false;
    this.asking = asked;
    this.refreshPrompt();
    return true;
  }

  /** A value out of range: said, and taken as understood. */
  private refuse(why: string): true {
    this.ctx.log.warn(why);
    return true;
  }

  /** A value typed for the value asked; false when it is not a number. */
  private value(text: string): boolean {
    const n = plainNumber(text);
    if (n === null) return false;
    if (this.asking === 'spacing') {
      const metres = this.ctx.format.toMetres(n);
      if (!(metres > 0 && Number.isFinite(metres))) return this.refuse('Aralık sıfırdan büyük olmalı.');
      RangeRingsTool.spacing = metres;
    } else if (this.asking === 'count') {
      if (!Number.isInteger(n) || n < 1 || n > MAX_RINGS) return this.refuse('Halka sayısı 1 ile 100 arasında bir tam sayı olmalı.');
      RangeRingsTool.count = n;
    } else {
      if (!Number.isInteger(n) || n < 0 || n > MAX_RAYS) return this.refuse('Işın sayısı 0 ile 360 arasında bir tam sayı olmalı (0: ışın yok).');
      RangeRingsTool.rays = n;
    }
    this.asking = 'centre';
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override input(text: string): boolean {
    if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
    if (this.asking !== 'centre') return this.value(text);
    return super.input(text);
  }

  override pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    this.asking = 'centre';
    super.pointerDown(p);
  }

  /** A computed point is a centre; a value asked takes none. */
  override acceptPoint(p: Vec2): boolean {
    return this.asking === 'centre' && super.acceptPoint(p);
  }

  /** A value asked: it stays as it was; else the tool leaves. */
  override confirm(): void {
    if (this.asking === 'centre') return this.ctx.tools.exit();
    this.asking = 'centre';
    this.refreshPrompt();
  }

  cancel(): boolean {
    if (this.asking === 'centre') return false;
    this.asking = 'centre';
    this.refreshPrompt();
    return true;
  }

  /** The rings and rays round the cursor, the outer radius beside it. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const c = this.hover;
    if (!c) return;
    const S = RangeRingsTool;
    const rings = rangeRings(c, S.spacing, S.count, S.rays);
    if (!rings) return;
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    for (const r of rings.radii) strokePath(g, view, tessellateCircle(c, r, 96), { color: pal.accent, closed: true });
    for (const end of rings.rays) strokePath(g, view, [c, end], { color: pal.accent, dash: [5, 3] });
    ringMark(g, view, c, pal.snap, 4.5);
    drawTag(g, view.worldToScreen(c), [`${rings.radii.length} × ${f.length(S.spacing)}`, `dış yarıçap ${f.length(rings.radii.at(-1) ?? 0)}`], pal.accent, pal.labelHalo);
  }
}
