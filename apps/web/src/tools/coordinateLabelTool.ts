import type { AppContext } from '../app/context';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { coordinateLabels, coordinatePlaces, type CoordinateLabels, type LabelDirection, type LabelOptions, type LabelPlace, type LabelUnits } from '../model/ops/coordinateLabels';
import type { TextFace } from '../model/annotationStyles';
import { elevationAt } from '../product/elevationValues';
import type { ViewTransform } from '../viewport/Camera';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { writeObjects } from './createCommand';
import { PointInputTool } from './drawTools';
import { listedOf, newTable, scheduleOf, sourceOf } from './newTable';
import { drawTag, drawTextGhost, strokePath } from './preview';
import { annotationHeightMm, setAnnotationHeightMm } from './annotationHeights';
import { stylesShown, takeTextStyle, textFaceNow, textStyleChoices, textStyleName, textStyleNow } from './styleOption';
import { TablePlaceTool } from './tablePlaceTool';
import type { OptionChoice, ToolPointer } from './Tool';

/**
 * Koordinat yaz and Köşelere koordinat yaz (docs/adr/0185; Netcad's Koordinat Yaz; the desktop's
 * `kentos_interaction::coordinate_labels`). A label is a leader from its place to an elbow and a bar, the template's
 * first line over the bar and the others under it; without the leader its lines stand beside the place. Where and what
 * is the core's (`ops::coordinate_labels`); the tools give it the places and the options and write what it gives back
 * through `cad.entities.create` (step “Koordinat yaz”) on the active layer, in the current colour.
 *
 * - Koordinat yaz: each click, typed point or `#ad` gets a label, shown at the cursor before; the elevation and the
 *   point's name of the visible objects at the place are its. Enter or Esc ends.
 * - Köşelere koordinat yaz: selection first (points, lines, polylines and areas), as the modify tools; every place of
 *   their vertices is shown with its label (the coordinate schedule's places and names); Enter, Uygula or a quick right
 *   click writes them in one step and the tool leaves. With Çizelge (Ç) the coordinate schedule of the same objects
 *   then hangs from the cursor (Tablo ekle's placement).
 *
 * The options are kept for the app's life (`coordinateOptions`; the desktop's `Memory::coordinate_*`): Stil (S, a CAD
 * project's), Kollu (K), Yön (O: the chip's menu; the key turns to the next one), Şablon (Ş, in the text field; an
 * empty Enter gives the type's back), Basamak (B; an empty Enter gives the project's back), Yükseklik (Y, paper mm).
 * Both play `fixtures/interaction/v1/coordinate-labels.json`.
 */

const LABEL = 'Koordinat yaz';
const VERTICES_LABEL = 'Köşelere koordinat yaz';
/** The coordinate schedule's placement (Çizelge). */
export const SCHEDULE_LABEL = 'Koordinat çizelgesi';
/** The most decimals Basamak takes, and the longest template (the desktop's `Name::MAX_CHARS`). */
const MOST_DECIMALS = 8;
const MAX_TEMPLATE = 60;
const NOTHING = `${VERTICES_LABEL}: seçimde nokta, çizgi, çoklu çizgi ya da alan yok.`;

/** The template a project's type starts with: east over north, as Netcad writes them. */
export const firstTemplate = (cad: boolean): string => (cad ? '{X}|{Y}' : '{Y}|{X}');

/** What the tools remember for as long as the app lives (docs/adr/0185 §5). */
export const coordinateOptions = {
  leader: true,
  direction: 'auto' as LabelDirection,
  /** Empty: the project type's. */
  template: '',
  /** Null: the project's length decimals. */
  decimals: null as number | null,
  schedule: false,
};

/** Yön's ways in the order its key turns them: each its name and icon. */
const DIRECTIONS: readonly { value: LabelDirection; name: string; icon: string }[] = [
  { value: 'auto', name: 'Otomatik', icon: 'labelAuto' },
  { value: 'ne', name: 'Sağ üst', icon: 'labelNorthEast' },
  { value: 'nw', name: 'Sol üst', icon: 'labelNorthWest' },
  { value: 'sw', name: 'Sol alt', icon: 'labelSouthWest' },
  { value: 'se', name: 'Sağ alt', icon: 'labelSouthEast' },
];

const directionName = (d: LabelDirection): string => DIRECTIONS.find((w) => w.value === d)?.name ?? 'Otomatik';

/** The kinds whose places Köşelere koordinat yaz takes. */
const isSource = (e: Entity): boolean => e.kind === 'point' || e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon';

/** What an option asks for while it is asked. */
type Asking = 'decimals' | 'height' | 'style' | 'template';

/** What the tools write with: the core's options and units, the texts' face and width factor. */
interface Look {
  readonly options: LabelOptions;
  readonly units: LabelUnits;
  readonly face: TextFace;
  readonly widthFactor: number | undefined;
}

const template = (ctx: AppContext): string => coordinateOptions.template || firstTemplate(ctx.format.axes === 'cad');

const decimals = (ctx: AppContext): number => coordinateOptions.decimals ?? ctx.doc.settings.lengthDecimals.value;

/**
 * The paper height, mm: a CAD project's chosen style's when it fixes one, else Yükseklik's (typed in this drawing, else
 * the project's Koordinat yazısı height, docs/adr/0205 §2).
 */
const heightMm = (ctx: AppContext): number => (stylesShown(ctx) ? textStyleNow(ctx)?.height : undefined) ?? annotationHeightMm(ctx, 'coordinate');

function look(ctx: AppContext): Look {
  const O = coordinateOptions;
  const face = textFaceNow(ctx);
  const factor = stylesShown(ctx) ? textStyleNow(ctx)?.widthFactor : undefined;
  const widthFactor = factor !== undefined && factor !== 1 ? factor : undefined;
  return {
    options: {
      template: template(ctx),
      decimals: decimals(ctx),
      height: (heightMm(ctx) / 1000) * ctx.doc.settings.plotScale.value,
      leader: O.leader,
      direction: O.direction,
      font: face.font ?? ctx.doc.settings.drawingFont.value,
      bold: face.bold === true,
      widthFactor: widthFactor ?? 1,
    },
    units: { axes: ctx.format.axes, unit: ctx.format.unit },
    face,
    widthFactor,
  };
}

/** The objects a label is written as: its leader (an open polyline of three vertices) when it has one, then its lines. */
function geometries(r: CoordinateLabels, l: Look): NewGeometry[] {
  const out: NewGeometry[] = [];
  for (const label of r.labels) {
    if (label.leader) out.push({ kind: 'polyline', pts: label.leader.map((p) => ({ x: p.x, y: p.y })) } as NewGeometry);
    for (const t of label.texts)
      out.push({
        kind: 'text',
        p: { x: t.p.x, y: t.p.y },
        text: t.text,
        height: l.options.height,
        rotation: 0,
        ...(t.align && { align: t.align }),
        ...(l.widthFactor !== undefined && { widthFactor: l.widthFactor }),
        ...l.face,
      } as NewGeometry);
  }
  return out;
}

/** Labels drawn as they will be: their leaders dashed, their lines faint in their face; the first `most` of them. */
function drawLabels(ctx: AppContext, g: CanvasRenderingContext2D, view: ViewTransform, r: CoordinateLabels, l: Look, most: number): void {
  const pal = ctx.view.palette;
  for (const label of r.labels.slice(0, most)) {
    if (label.leader) strokePath(g, view, label.leader, { color: pal.accent, dash: [4, 3] });
    for (const t of label.texts)
      drawTextGhost(g, view, { p: t.p, text: t.text, height: l.options.height, rotation: 0, align: t.align, face: l.face, widthFactor: l.options.widthFactor }, { color: pal.accent, font: pal.drawingFont, mask: null });
  }
}

/** The options as a prompt shows them: `[Stil (S): Standart / Kollu (K): açık / …]`. */
function optionsText(ctx: AppContext, schedule: boolean): string {
  const O = coordinateOptions;
  const on = (b: boolean) => (b ? 'açık' : 'kapalı');
  const parts = [
    ...(stylesShown(ctx) ? [`Stil (S): ${textStyleName(ctx)}`] : []),
    `Kollu (K): ${on(O.leader)}`,
    `Yön (O): ${directionName(O.direction)}`,
    `Şablon (Ş): ${template(ctx)}`,
    `Basamak (B): ${O.decimals ?? `${ctx.doc.settings.lengthDecimals.value} (proje)`}`,
    `Yükseklik (Y): ${annotationHeightMm(ctx, 'coordinate')} mm`,
    ...(schedule ? [`Çizelge (Ç): ${on(O.schedule)}`] : []),
  ];
  return parts.join(' / ');
}

/** What a stage that asks says. */
function askingText(ctx: AppContext, asking: Asking): string {
  switch (asking) {
    case 'decimals':
      return `ondalık basamak sayısını yazın (0–${MOST_DECIMALS}; boş Enter: projeninki)`;
    case 'height':
      return 'kâğıt üzerindeki yazı yüksekliğini mm olarak yazın';
    case 'style':
      return `yazı stilini menüden seçin ya da adını yazın [Stil (S): ${textStyleName(ctx)}]`;
    case 'template':
      return 'şablonu yazın';
  }
}

/**
 * The options both tools share (the desktop's `option`, `answer`, `choices`, `choose`): their keys turn, cycle or ask,
 * a value typed answers what is asked.
 */
class Options {
  asking: Asking | null = null;
  private readonly ctx: AppContext;
  private readonly schedule: boolean;
  private readonly changed: () => void;

  constructor(ctx: AppContext, schedule: boolean, changed: () => void) {
    this.ctx = ctx;
    this.schedule = schedule;
    this.changed = changed;
  }

  /** A key: whether it is one of the options (one that asks starts asking). */
  option(key: string, near: Vec2 | null): boolean {
    const O = coordinateOptions;
    switch (key) {
      case 'K':
        O.leader = !O.leader;
        break;
      case 'O': {
        const at = DIRECTIONS.findIndex((w) => w.value === O.direction);
        O.direction = DIRECTIONS[(at + 1) % DIRECTIONS.length].value;
        break;
      }
      case 'Ç':
        if (!this.schedule) return false;
        O.schedule = !O.schedule;
        break;
      case 'B':
        this.asking = 'decimals';
        break;
      case 'Y':
        this.asking = 'height';
        break;
      case 'S':
        if (!stylesShown(this.ctx)) return false;
        this.asking = 'style';
        break;
      case 'Ş':
        this.asking = 'template';
        this.askTemplate(near);
        break;
      default:
        return false;
    }
    return true;
  }

  /** A value typed for what is asked: whether it was taken (a wrong one is said and asked again). */
  answer(text: string): boolean {
    const t = text.trim();
    const { log } = this.ctx;
    switch (this.asking) {
      case 'decimals': {
        const d = /^\d+$/.test(t) ? Number(t) : NaN;
        if (Number.isInteger(d) && d <= MOST_DECIMALS) {
          coordinateOptions.decimals = d;
          this.asking = null;
        } else log.warn(`Basamak 0 ile ${MOST_DECIMALS} arasında bir tam sayı olmalı; “${t}” yazıldı.`);
        return true;
      }
      case 'height': {
        const n = parseNumberText(t);
        if (n !== null && n > 0 && Number.isFinite(n)) {
          setAnnotationHeightMm(this.ctx, 'coordinate', n);
          this.asking = null;
        } else log.warn(`Yükseklik sıfırdan büyük bir sayı olmalı (kâğıtta mm); “${t}” yazıldı.`);
        return true;
      }
      case 'style':
        if (takeTextStyle(this.ctx, t) !== undefined) this.asking = null;
        return true;
      default:
        return false;
    }
  }

  /** Enter while a value is asked: Basamak's empty Enter gives the project's back; every other one keeps the value. */
  confirm(): void {
    if (this.asking === 'decimals') coordinateOptions.decimals = null;
    if (this.asking !== 'template') this.asking = null;
  }

  /** Esc while a value is asked keeps the old one: whether there was one. */
  cancel(): boolean {
    if (this.asking === null) return false;
    this.asking = null;
    return true;
  }

  /** Yön's and Stil's menus. */
  choices(key: string): readonly OptionChoice[] | null {
    if (key === 'O') return DIRECTIONS.map((w) => ({ label: w.name, typed: w.name.toLocaleLowerCase('tr-TR'), icon: w.icon, checked: w.value === coordinateOptions.direction }));
    if (key === 'S' && stylesShown(this.ctx)) return textStyleChoices(this.ctx);
    return null;
  }

  /** One of Yön's or Stil's values chosen from its menu. */
  choose(key: string, typed: string): boolean {
    if (key === 'O') {
      const w = DIRECTIONS.find((d) => d.name.toLocaleLowerCase('tr-TR') === typed.trim().toLocaleLowerCase('tr-TR'));
      if (!w) return false;
      coordinateOptions.direction = w.value;
      return true;
    }
    if (key === 'S' && stylesShown(this.ctx)) {
      if (takeTextStyle(this.ctx, typed) !== undefined) this.asking = null;
      return true;
    }
    return false;
  }

  /** Şablon's field by the cursor (or the view's middle), the template in it: an empty Enter gives the type's back. */
  private askTemplate(near: Vec2 | null): void {
    const { view, log } = this.ctx;
    const b = view.camera.visibleBounds();
    const at = near ?? { x: (b.minX + b.maxX) / 2, y: (b.minY + b.maxY) / 2 };
    const end = (typed: string | null) => {
      if (this.asking === 'template') this.asking = null;
      const v = typed?.trim() ?? null;
      if (v !== null && [...v].length > MAX_TEMPLATE) log.warn(`Şablon en çok ${MAX_TEMPLATE} harf olabilir.`);
      else if (v !== null) coordinateOptions.template = v;
      this.changed();
      view.focus();
    };
    queueMicrotask(() =>
      view.requestTextInput({
        at,
        // Readable at any zoom: 14 px on the screen.
        height: view.worldTolerance(14),
        rotation: 0,
        initial: template(this.ctx),
        placeholder: 'Koordinat yazısının şablonu',
        hint: 'Satırlar | ile ayrılır; {Y}, {X}, {Z} ve {ad} yerine değerler · boş Enter: türün şablonu · Esc: vazgeç',
        commit: (v) => end(v),
        empty: () => end(''),
        cancel: () => end(null),
      }),
    );
  }
}

/** A plain number as typed (a dot or a comma for the decimals). */
function parseNumberText(t: string): number | null {
  if (!/^[+-]?(\d+([.,]\d*)?|[.,]\d+)$/.test(t)) return null;
  return Number(t.replace(',', '.'));
}

/** The name a point object gives the place `at`: its label, the k-th of its other points “label (k+1)”; none elsewhere. */
function nameAt(e: Entity, at: Vec2): string | undefined {
  if (e.kind !== 'point') return undefined;
  const label = e.label?.replace(/^ +| +$/g, '');
  if (!label) return undefined;
  const near = (q: Vec2) => Math.abs(q.x - at.x) <= 1e-6 && Math.abs(q.y - at.y) <= 1e-6;
  if (near(e.p)) return label;
  const k = e.parts?.findIndex((q) => near(q.p)) ?? -1;
  return k < 0 ? undefined : `${label} (${k + 2})`;
}

/**
 * The place `p` with the elevation and the name the visible objects there have (1 µm, in the document's order): the
 * first elevation, the first point's name; a click, a typed point and `#ad` alike, whatever the snap stood on.
 */
function placeAt(ctx: AppContext, p: Vec2): LabelPlace {
  const d = 1e-6;
  let z: number | undefined;
  let name: string | undefined;
  for (const id of ctx.view.pickRect({ minX: p.x - d, minY: p.y - d, maxX: p.x + d, maxY: p.y + d }, true)) {
    const e = ctx.doc.get(id);
    if (!e) continue;
    if (z === undefined) z = elevationAt(e, p) ?? undefined;
    if (name === undefined) name = nameAt(e, p);
    if (z !== undefined && name !== undefined) break;
  }
  return { p: { x: p.x, y: p.y }, ...(z !== undefined && { z }), ...(name !== undefined && { name }) };
}

// ── Koordinat yaz ──────────────────────────────────────────────────────────────────────────────────────────────────

export class CoordinateLabelTool extends PointInputTool {
  readonly id = 'coordinateLabel';
  protected readonly label = LABEL;
  private readonly options: Options;

  constructor(ctx: AppContext) {
    super(ctx);
    this.options = new Options(ctx, false, () => this.refreshPrompt());
  }

  protected promptFor(): string {
    if (this.options.asking) return askingText(this.ctx, this.options.asking);
    return `yazılacak noktaya tıklayın ya da Y,X yazın [${optionsText(this.ctx, false)}]`;
  }

  override pointerDown(p: ToolPointer): void {
    if (this.options.asking) return;
    super.pointerDown(p);
  }

  protected onPoint(p: Vec2): void {
    const place = placeAt(this.ctx, p);
    const l = look(this.ctx);
    const r = coordinateLabels([place], l.options, l.units);
    if (!r.labels.length) return void this.ctx.log.warn(`${LABEL}: şablon bu noktaya satır bırakmadı (adı ya da kotu yok).`);
    this.writeObjects(geometries(r, l), 'coordinates');
  }

  override input(text: string): boolean {
    if (this.options.asking === 'template') return false;
    const key = text.trim().toLocaleUpperCase('tr-TR');
    let done: boolean;
    if (this.options.option(key, this.hover)) done = true;
    else if (this.options.asking) done = this.options.answer(text);
    else return super.input(text);
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return done;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    return this.options.choices(key);
  }

  chooseOption(key: string, typed: string): boolean {
    const taken = this.options.choose(key, typed);
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return taken;
  }

  /** A style's name is words: Space types a space (docs/adr/0183 §4). */
  takesWords(): boolean {
    return this.options.asking === 'style';
  }

  /** Enter ends, or takes back what is asked. */
  override confirm(): void {
    if (this.options.asking) {
      this.options.confirm();
      this.refreshPrompt();
      return;
    }
    this.ctx.tools.exit();
  }

  /** Esc while a value is asked keeps the old one. */
  cancel(): boolean {
    if (!this.options.cancel()) return false;
    this.refreshPrompt();
    return true;
  }

  override snapFrom(): Vec2 | null {
    return null;
  }

  /** The label at the cursor, as it will be written. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.hover && !this.options.asking) {
      const l = look(this.ctx);
      drawLabels(this.ctx, g, view, coordinateLabels([placeAt(this.ctx, this.hover)], l.options, l.units), l, 1);
    }
    this.drawTracking(g, view);
  }
}

// ── Köşelere koordinat yaz ─────────────────────────────────────────────────────────────────────────────────────────

/** `12 yere yazılacak; şablon 2 yere satır bırakmadı`: what would be written, then what is passed over. */
function finding(r: CoordinateLabels): string {
  const n = r.labels.length;
  const s = r.skipped;
  if (!n) return s ? `yazılacak yazı yok; şablon ${s} yere satır bırakmadı` : 'yazılacak yer yok';
  return s ? `${n} yere yazılacak; şablon ${s} yere satır bırakmadı` : `${n} yere yazılacak`;
}

export class CoordinateVerticesTool extends SelectionFirstTool {
  readonly id = 'coordinateVertices';
  protected readonly label = VERTICES_LABEL;
  // Nothing is placed by the cursor.
  override readonly snaps = false;
  /** The objects it reads, taken when the selection is confirmed, in the drawing's order. */
  private objects: Entity[] = [];
  private readonly options: Options;
  private plan: { key: string; look: Look; labels: CoordinateLabels } | null = null;

  constructor(ctx: AppContext) {
    super(ctx);
    this.options = new Options(ctx, true, () => this.refresh());
  }

  protected begin(): void {
    this.options.asking = null;
    this.plan = null;
    const ids = new Set(this.targets().filter(isSource).map((e) => e.id));
    this.objects = [...this.ctx.doc.all()].filter((e) => ids.has(e.id));
    if (this.objects.length) return;
    this.ctx.log.warn(NOTHING);
    // Leaving from inside activate() would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }

  /** The labels for the drawing and the options as they are, kept until either changes. */
  private current(): { look: Look; labels: CoordinateLabels } {
    const l = look(this.ctx);
    const key = `${this.ctx.doc.revision}|${JSON.stringify(l)}`;
    if (this.plan?.key === key) return this.plan;
    const objects = this.objects.flatMap((e) => this.ctx.doc.get(e.id) ?? []);
    const labels = coordinateLabels(coordinatePlaces(objects.map(listedOf)), l.options, l.units);
    this.plan = { key, look: l, labels };
    return this.plan;
  }

  protected stagePrompt(): string {
    if (this.options.asking) return askingText(this.ctx, this.options.asking);
    return `${finding(this.current().labels)} [${optionsText(this.ctx, true)} / Uygula (Enter)]`;
  }

  /** A click places nothing: Enter, Uygula or a quick right click writes. */
  protected point(): void {}

  override input(text: string): boolean {
    if (this.picking || this.options.asking === 'template') return false;
    const key = text.trim().toLocaleUpperCase('tr-TR');
    const taken = this.options.option(key, this.hover) || this.options.answer(text);
    if (taken) this.refresh();
    return taken;
  }

  override acceptPoint(): boolean {
    return false;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    return this.picking ? null : this.options.choices(key);
  }

  chooseOption(key: string, typed: string): boolean {
    if (this.picking) return false;
    const taken = this.options.choose(key, typed);
    this.refresh();
    return taken;
  }

  takesWords(): boolean {
    return this.options.asking === 'style';
  }

  /** Esc while a value is asked keeps the old one; otherwise the tool leaves. */
  cancel(): boolean {
    if (!this.options.cancel()) return false;
    this.refresh();
    return true;
  }

  /** Enter: the labels written in one step and the tool leaves; while a value is asked, it takes it back. */
  override confirm(): void {
    if (this.picking) return super.confirm();
    if (this.options.asking) {
      this.options.confirm();
      return this.refresh();
    }
    const { log } = this.ctx;
    const { look: l, labels } = this.current();
    const n = labels.labels.length;
    const skipped = labels.skipped;
    if (!n) {
      log.warn(`${VERTICES_LABEL}: yazılacak yazı yok; şablon ${skipped} yere satır bırakmadı.`);
      return this.ctx.tools.exit();
    }
    // The active layer, the current colour and line weight, an object template's stamp: as the drawing tools write.
    if (!writeObjects(this.ctx, geometries(labels, l) as never, 'coordinates')) return;
    log.success(`${VERTICES_LABEL}: ${n} yere yazıldı${skipped ? `; şablon ${skipped} yere satır bırakmadı` : ''}.`);
    if (coordinateOptions.schedule) return this.schedule(l);
    this.ctx.tools.exit();
  }

  /** The coordinate schedule of the same objects, its names the labels', placed by Tablo ekle's tool. */
  private schedule(l: Look): void {
    const objects = this.objects.flatMap((e) => this.ctx.doc.get(e.id) ?? []);
    const cells = scheduleOf(this.ctx, 'coordinates', objects);
    if (cells.problem) {
      this.ctx.log.warn(cells.problem);
      return this.ctx.tools.exit();
    }
    const tableLook = { header: true, height: l.options.height, face: l.face };
    const source = sourceOf('coordinates', objects);
    this.ctx.tools.run(new TablePlaceTool(this.ctx, (p) => newTable(this.ctx, cells, tableLook, p, source), SCHEDULE_LABEL), SCHEDULE_LABEL);
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking) return super.draw(g, view);
    const { look: l, labels } = this.current();
    drawLabels(this.ctx, g, view, labels, l, MAX_GHOSTS);
    if (this.hover) {
      const lines = finding(labels).split('; ');
      drawTag(g, view.worldToScreen(this.hover), labels.labels.length ? [...lines, 'Enter: uygula'] : lines, this.ctx.view.palette.accent, this.ctx.view.palette.labelHalo);
    }
  }
}
