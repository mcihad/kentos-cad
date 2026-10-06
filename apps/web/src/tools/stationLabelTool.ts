import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { Entity, EntityGeometry } from '../model/entities';
import type { ViewTransform } from '../viewport/Camera';
import { kmText, kmValue, stationing, type Stationing, type StationLook, type StationRules } from './constructions';
import { writeObjectsEach } from './createCommand';
import { MAX_GHOSTS } from './modifyTools';
import { NO_ROUTE, pickRoute } from './pointCalcRoute';
import { drawTextGhost, strokePath } from './preview';
import { stylesShown, takeTextStyle, textFaceNow, textStyleChoices, textStyleName, textStyleNow, textWidthFactorNow } from './styleOption';
import type { OptionChoice, Tool, ToolPointer } from './Tool';

/**
 * Km yaz (docs/adr/0189; Netcad's Obje Üzerinde Dizi with ??KM labels; the desktop's `kentos_interaction::station_labels`):
 * a route's stations at the multiples of an interval of its km and its ends, and at each a tick, the km as a text
 * square to the route, a cross-section and a point. Where and what is the core's (`ops::stationing`); the tool picks
 * the route, shows everything faint and writes it through `cad.entities.create` (step “Km yaz”) on the active layer.
 * Enter, Uygula or a quick right click writes and the tool waits for the next route; Esc and Ctrl+Z let the route go,
 * Esc with none leaves. The options are the session's (`stationOptions`).
 */

export const LABEL = 'Km yaz';
/** The tick's half length, paper mm. */
const TICK_MM = 2;

/** The session's options (the desktop's `Memory::station_*`): metres of km, paper mm, metres. */
export const stationOptions = {
  interval: 20,
  start: 0,
  text: 'left' as 'left' | 'right' | null,
  heightMm: 2,
  tick: true,
  section: 0,
  point: null as number | null,
  ends: true,
};

/** Yazı's values in its key's order. */
const SIDES: readonly { value: 'left' | 'right' | null; name: string }[] = [
  { value: 'left', name: 'sol' },
  { value: 'right', name: 'sağ' },
  { value: null, name: 'yok' },
];
const sideName = (v: 'left' | 'right' | null): string => SIDES.find((s) => s.value === v)?.name ?? 'sol';

type Asking = 'interval' | 'start' | 'height' | 'section' | 'point' | 'style';

const ASKING: Record<Asking, string> = {
  interval: 'km aralığını metre olarak yazın (ör. 20)',
  start: 'güzergâhın başındaki km’yi yazın (ör. 0+000)',
  height: 'kâğıt üzerindeki yazı yüksekliğini mm olarak yazın',
  section: 'enkesitin yarı genişliğini yazın (0: yok)',
  point: 'noktaların sapmasını yazın (sağa artı; boş Enter: nokta yok)',
  style: 'yazı stilini menüden seçin ya da adını yazın',
};

/** A plain number as typed (a dot or a comma for the decimals). */
function parseNumberText(t: string): number | null {
  if (!/^[+-]?(\d+([.,]\d*)?|[.,]\d+)$/.test(t)) return null;
  return Number(t.replace(',', '.'));
}

/** The paper height, mm: a CAD project's chosen style's when it fixes one, else Yükseklik's. */
const heightMm = (ctx: AppContext): number => (stylesShown(ctx) ? textStyleNow(ctx)?.height : undefined) ?? stationOptions.heightMm;

export class StationLabelTool implements Tool {
  readonly id = 'stationLabels';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private route: Entity | null = null;
  private reverse = false;
  private asking: Asking | null = null;
  private plan: Stationing | null = null;
  private problem: string | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    // A single selected route is taken at once.
    const ids = [...this.ctx.selection.ids.value];
    const e = ids.length === 1 ? this.ctx.doc.get(ids[0]) : undefined;
    if (e && hasRoute(e)) this.route = e;
    this.refresh();
  }

  /** The texts' height (m), face and width factor as they are written now. */
  private look(): { height: number; face: ReturnType<typeof textFaceNow>; widthFactor: number | undefined } {
    const height = (heightMm(this.ctx) / 1000) * this.ctx.doc.settings.plotScale.value;
    const factor = stylesShown(this.ctx) ? textWidthFactorNow(this.ctx) : undefined;
    return { height, face: textFaceNow(this.ctx), widthFactor: factor !== undefined && factor !== 1 ? factor : undefined };
  }

  /** The plan for the route as the options are now, and the prompt. */
  private refresh(): void {
    const O = stationOptions;
    this.plan = null;
    const problem = this.problem;
    this.problem = null;
    if (this.route) {
      const scale = this.ctx.doc.settings.plotScale.value / 1000;
      const rules: StationRules = { interval: O.interval, start: O.start, reverse: this.reverse, ends: O.ends, decimals: this.ctx.doc.settings.lengthDecimals.value };
      const look: StationLook = { text: O.text, height: this.look().height, tick: O.tick ? TICK_MM * scale : 0, section: O.section, point: O.point };
      const got = stationing(this.route, rules, look);
      this.plan = got.stationing ?? null;
      this.problem = got.problem ?? null;
      if (this.problem && this.problem !== problem) this.ctx.log.warn(this.problem);
    }
    this.prompt.set(`${LABEL}: ${this.step()}`);
    this.ctx.view.requestOverlay();
  }

  private step(): string {
    if (this.asking) {
      if (this.asking === 'style') return `${ASKING.style} [Stil (S): ${textStyleName(this.ctx)}]`;
      return ASKING[this.asking];
    }
    const O = stationOptions;
    const f = this.ctx.format;
    const on = (b: boolean) => (b ? 'açık' : 'kapalı');
    const step = this.plan
      ? `${this.plan.stations.length} istasyon; Enter ile yazın ya da başka bir güzergâha tıklayın`
      : this.route
        ? 'bu seçeneklerle yazılamıyor; seçenekleri değiştirin ya da başka bir güzergâha tıklayın'
        : 'güzergâha tıklayın (km’si ilk köşesinde başlar)';
    const parts = [
      `Aralık (A): ${O.interval} m`,
      `Başlangıç (B): ${kmText(O.start, f.lengthDecimals)}`,
      this.reverse ? 'Ters (T): açık' : 'Ters (T)',
      `Yazı (Y): ${sideName(O.text)}`,
      `Yükseklik (H): ${O.heightMm} mm`,
      ...(stylesShown(this.ctx) ? [`Stil (S): ${textStyleName(this.ctx)}`] : []),
      `İşaret (İ): ${on(O.tick)}`,
      `Enkesit (E): ${O.section > 0 ? f.length(O.section) : 'yok'}`,
      `Nokta (N): ${O.point === null ? 'yok' : f.length(O.point)}`,
      `Uçlar (U): ${on(O.ends)}`,
      ...(this.route ? ['Uygula (Enter)'] : []),
    ];
    return `${step} [${parts.join(' / ')}]`;
  }

  /** References picked so far: the route. */
  get pointCount(): number {
    return this.route ? 1 : 0;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0 || this.asking) return;
    const route = pickRoute(this.ctx, p.screen, p.raw, false);
    if (!route) return;
    this.route = route.entity;
    this.reverse = false;
    this.refresh();
  }

  input(text: string): boolean {
    if (this.asking) this.answer(text);
    else if (!this.option(text.trim().toLocaleUpperCase('tr-TR'))) return false;
    this.refresh();
    return true;
  }

  /** A key's option: whether it is one (one that asks starts asking). */
  private option(key: string): boolean {
    const O = stationOptions;
    switch (key) {
      case 'A':
        this.asking = 'interval';
        break;
      case 'B':
        this.asking = 'start';
        break;
      case 'T':
        this.reverse = !this.reverse;
        break;
      case 'Y': {
        const at = SIDES.findIndex((s) => s.value === O.text);
        O.text = SIDES[(at + 1) % SIDES.length].value;
        break;
      }
      case 'H':
        this.asking = 'height';
        break;
      case 'S':
        if (!stylesShown(this.ctx)) return false;
        this.asking = 'style';
        break;
      case 'İ':
        O.tick = !O.tick;
        break;
      case 'E':
        this.asking = 'section';
        break;
      case 'N':
        this.asking = 'point';
        break;
      case 'U':
        O.ends = !O.ends;
        break;
      default:
        return false;
    }
    return true;
  }

  /** A value typed for what is asked: a wrong one is said and asked again. */
  private answer(text: string): void {
    const O = stationOptions;
    const t = text.trim();
    const f = this.ctx.format;
    const refused = (why: string) => this.ctx.log.warn(`${why}; “${t}” yazıldı.`);
    const n = parseNumberText(t);
    switch (this.asking) {
      case 'interval':
        if (n !== null && n > 0 && Number.isFinite(n)) {
          O.interval = n;
          this.asking = null;
        } else refused('Aralık sıfırdan büyük bir uzunluk olmalı (km’nin metresi)');
        return;
      case 'start': {
        const v = kmValue(t);
        if (v !== null) {
          O.start = v;
          this.asking = null;
        } else refused('Başlangıç bir km olmalı: k+mmm.mmm ya da metre (ör. 0+000)');
        return;
      }
      case 'height':
        if (n !== null && n > 0 && Number.isFinite(n)) {
          O.heightMm = n;
          this.asking = null;
        } else refused('Yükseklik sıfırdan büyük bir sayı olmalı (kâğıtta mm)');
        return;
      case 'section':
        if (n !== null && n >= 0 && Number.isFinite(n)) {
          O.section = f.toMetres(n);
          this.asking = null;
        } else refused('Enkesitin yarı genişliği sıfır ya da daha büyük bir uzunluk olmalı (0: yok)');
        return;
      case 'point':
        if (n !== null && Number.isFinite(n)) {
          O.point = f.toMetres(n);
          this.asking = null;
        } else refused('Noktanın sapması bir uzunluk olmalı (sağa artı; boş Enter: nokta yok)');
        return;
      case 'style':
        if (takeTextStyle(this.ctx, t) !== undefined) this.asking = null;
        return;
    }
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key === 'Y') return SIDES.map((s) => ({ label: s.name, typed: s.name, checked: s.value === stationOptions.text }));
    if (key === 'S' && stylesShown(this.ctx)) return textStyleChoices(this.ctx);
    return null;
  }

  chooseOption(key: string, typed: string): boolean {
    let taken = false;
    if (key === 'Y') {
      const side = SIDES.find((s) => s.name === typed.trim());
      if (side) {
        stationOptions.text = side.value;
        taken = true;
      }
    } else if (key === 'S' && stylesShown(this.ctx)) {
      if (takeTextStyle(this.ctx, typed) !== undefined) this.asking = null;
      taken = true;
    }
    this.refresh();
    return taken;
  }

  /** A style's name is words: Space types a space (docs/adr/0183 §4). */
  takesWords(): boolean {
    return this.asking === 'style';
  }

  /** Enter: a value being asked ends (Nokta's empty one is no point); with a route, the plan is written; with none, the tool leaves. */
  confirm(): void {
    if (this.asking) {
      if (this.asking === 'point') stationOptions.point = null;
      this.asking = null;
      return this.refresh();
    }
    if (!this.route) return this.ctx.tools.exit();
    this.write();
    this.refresh();
  }

  /** Esc: a value being asked, then the route, go first. */
  cancel(): boolean {
    if (this.asking) {
      this.asking = null;
      this.refresh();
      return true;
    }
    if (this.route) {
      this.route = null;
      this.reverse = false;
      this.refresh();
      return true;
    }
    return false;
  }

  /** Ctrl+Z: the route goes; with none the drawing is undone. */
  undoStep(): boolean {
    return this.cancel();
  }

  /** Writes the plan in one step and says what came of it; the tool waits for the next route. */
  private write(): void {
    const plan = this.plan;
    if (!plan) return;
    const l = this.look();
    const km = (text: string) => ({ Km: text });
    const items: { geometry: EntityGeometry; attrs?: Record<string, string> }[] = [
      ...plan.ticks.map((t) => ({ geometry: { kind: 'line', a: t.a, b: t.b } as EntityGeometry })),
      ...plan.texts.map((t) => ({
        geometry: {
          kind: 'text',
          p: t.p,
          text: t.text,
          height: l.height,
          rotation: t.rotation,
          ...(t.align && { align: t.align }),
          ...(l.widthFactor !== undefined && { widthFactor: l.widthFactor }),
          ...l.face,
        } as unknown as EntityGeometry,
      })),
      ...plan.sections.map((s) => ({ geometry: { kind: 'line', a: s.a, b: s.b } as EntityGeometry, attrs: km(s.km) })),
      ...plan.points.map((p) => ({ geometry: { kind: 'point', p: p.p } as EntityGeometry, attrs: km(p.km) })),
    ];
    if (!items.length) return this.ctx.log.warn(`${LABEL}: yazılacak bir şey yok; Yazı, İşaret, Enkesit ya da Nokta’yı açın.`);
    if (!writeObjectsEach(this.ctx, items, 'stations')) return;
    this.ctx.log.success(`${LABEL}: ${plan.stations.length} istasyon yazıldı.`);
    this.route = null;
    this.reverse = false;
  }

  /** Everything to be written, faint: ticks, cross-sections dashed, points as rings, the texts in their face. */
  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const plan = this.plan;
    if (!plan || this.asking) return;
    const pal = this.ctx.view.palette;
    for (const t of plan.ticks.slice(0, MAX_GHOSTS)) strokePath(g, view, [t.a, t.b], { color: pal.snap });
    for (const s of plan.sections.slice(0, MAX_GHOSTS)) strokePath(g, view, [s.a, s.b], { color: pal.snap, dash: [4, 3] });
    g.save();
    g.strokeStyle = pal.snap;
    g.lineWidth = 1.5;
    for (const p of plan.points.slice(0, MAX_GHOSTS)) {
      const s = view.worldToScreen(p.p);
      g.beginPath();
      g.arc(s.x, s.y, 3, 0, Math.PI * 2);
      g.stroke();
    }
    g.restore();
    const l = this.look();
    for (const t of plan.texts.slice(0, MAX_GHOSTS))
      drawTextGhost(g, view, { p: t.p, text: t.text, height: l.height, rotation: t.rotation, align: t.align ?? undefined, face: l.face, widthFactor: l.widthFactor ?? 1 }, { color: pal.snap, font: pal.drawingFont, mask: null });
  }
}

/** Whether Km yaz can walk the object (Obje üzerinde nokta's routes). */
function hasRoute(e: Entity): boolean {
  return stationing(e, { interval: 1e9, start: 0, reverse: false, ends: false, decimals: 0 }, { text: null, height: 1, tick: 0, section: 0, point: null }).problem !== NO_ROUTE;
}

