import type { AppContext } from '../app/context';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import { Signal } from '../core/signal';
import { drawsLines, textBox, type Entity, type RingGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { polygonOfArea } from '../model/ops/areas';
import { carryElevations, type Elevated } from '../model/ops/elevation';
import { polygonize, type PolyLabel, type PolyResult } from '../model/ops/polygonize';
import { elevatedPaths, hasElevation } from '../product/elevation';
import { entitiesCreate } from '../product/entitiesCreate';
import type { ViewTransform } from '../viewport/Camera';
import { ringMark } from './constructPreview';
import { MAX_GHOSTS } from './modifyTools';
import { drawArea, drawTag, tint } from './preview';
import type { Tool, ToolPointer } from './Tool';

/**
 * Toplu alan (docs/adr/0151): every region the line work closes becomes an area in one step, the text or the named
 * point inside each its attribute. The finding is the shared core's (`polygonize`); the desktop's tool is
 * `kentos_interaction::polygonize`, and both play `fixtures/interaction/v1/polygonize.json`.
 *
 * - Input, taken when it starts (§2): the selection's line work and labels, else every visible layer's. Line work is
 *   lines, polylines, arcs, circles, ellipses, curves and areas; a label is a text (its value its text, its place
 *   its box's middle) or a point with a label.
 * - Adalar (A) and the attribute's name (Ö) are kept for the session (§3, §4).
 * - Shown first (§8): the areas to be written (one label: the accent, filled; none: the accent, dashed; two or more:
 *   the danger colour, filled), a cross at every free end, a ring at every label on a boundary, the counts beside
 *   the cursor. Enter, Uygula or a quick right click writes them through `cad.entities.create` (operation
 *   `polygonize`) on the active layer, each with its elevations carried from the line work (ADR 0142) and its
 *   label as the attribute; Esc leaves.
 */

const LABEL = 'Toplu alan';
/** The attribute's name before any is typed (docs/adr/0151 §4). */
export const FIRST_ATTRIBUTE = 'Ad';
/** The details said at most (regions with many labels, labels on a boundary). */
const MAX_DETAILS = 20;

type Tone = 'one' | 'none' | 'many';

/** What the tool would write for the drawing as it is, and what it was worked out from. */
interface Plan {
  key: string;
  result: PolyResult;
  labels: PolyLabel[];
  /** The areas to write, in the core's order: their geometry, the label's value when there is one, how they look. */
  areas: { geometry: NewGeometry; value: string | null; tone: Tone }[];
  counts: { unlabelled: number; many: number; existing: number };
}

export class PolygonizeTool implements Tool {
  readonly id = 'polygonize';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  /** Adalar and the attribute's name, kept for the session (docs/adr/0151 §3, §4). */
  static islands = true;
  static attribute = FIRST_ATTRIBUTE;
  private readonly ctx: AppContext;
  /** The objects it reads, by id, taken when it starts. */
  private lineIds = new Set<number>();
  private labelIds = new Set<number>();
  private whole = true;
  /** Ö: the attribute's name is being typed in its field. */
  private typing = false;
  private plan: Plan | null = null;
  /** The finding last said, so a change that finds the same says nothing again. */
  private said = '';
  private hover: Vec2 | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    this.typing = false;
    this.plan = null;
    this.said = '';
    if (!this.takeInput()) return void queueMicrotask(() => this.ctx.tools.exit());
    const plan = this.current();
    this.tell(`${this.whole ? 'bütün çizim: ' : 'seçili '}${this.lineIds.size} çizgi, ${this.labelIds.size} etiket`);
    this.details(plan);
    this.refresh();
  }

  /** The line work and the labels (docs/adr/0151 §2); false when there is no line work. */
  private takeInput(): boolean {
    const { doc, log, selection } = this.ctx;
    this.whole = selection.size === 0;
    const chosen = this.whole ? [...doc.all()] : [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
    const seen = chosen.filter((e) => doc.layers.isVisible(e.layerId));
    this.lineIds = new Set(seen.filter(isLineWork).map((e) => e.id));
    this.labelIds = new Set(seen.filter((e) => labelValue(e) !== null).map((e) => e.id));
    if (!this.lineIds.size) log.warn(`${LABEL}: bölge kapatacak çizgi yok; çizgi, çoklu çizgi, yay ya da alan seçin.`);
    return this.lineIds.size > 0;
  }

  private key(): string {
    return `${this.ctx.doc.revision}|${PolygonizeTool.islands}|${PolygonizeTool.attribute}`;
  }

  /** The plan for the drawing as it is, kept until the drawing, Adalar or the attribute's name change. */
  private current(): Plan {
    const key = this.key();
    if (this.plan?.key === key) return this.plan;
    const { doc } = this.ctx;
    const font = doc.settings.drawingFont.value;
    // In the drawing's order: an input area is named by its place there.
    const lines: Entity[] = [];
    const labels: PolyLabel[] = [];
    for (const e of doc.all()) {
      if (this.lineIds.has(e.id)) lines.push(e);
      if (this.labelIds.has(e.id)) {
        const value = labelValue(e);
        if (value !== null) labels.push({ at: labelAt(e, font), value });
      }
    }
    const result = polygonize(lines, labels, PolygonizeTool.islands);
    const sources = lines.flatMap((e) => elevatedPaths(e));
    const carry = hasElevation(sources);
    const areas: Plan['areas'] = [];
    const counts = { unlabelled: 0, many: 0, existing: 0 };
    for (const r of result.regions) {
      if (r.existing !== undefined) {
        counts.existing++;
        continue;
      }
      const tone: Tone = r.labels.length === 1 ? 'one' : r.labels.length ? 'many' : 'none';
      if (tone === 'none') counts.unlabelled++;
      if (tone === 'many') counts.many++;
      const geometry = polygonOfArea(r.area) as Extract<NewGeometry, { kind: 'polygon' }>;
      areas.push({ geometry: carry ? withElevations(geometry, sources) : geometry, value: tone === 'one' ? labels[r.labels[0]].value : null, tone });
    }
    this.plan = { key, result, labels, areas, counts };
    return this.plan;
  }

  private refresh(): void {
    if (this.typing) this.prompt.set(`${LABEL}: öznitelik adını yazın (şimdi: ${PolygonizeTool.attribute})`);
    else {
      const plan = this.current();
      this.prompt.set(`${LABEL}: ${finding(plan)} [Adalar (A): ${PolygonizeTool.islands ? 'açık' : 'kapalı'} / Öznitelik (Ö): ${PolygonizeTool.attribute} / Uygula (Enter)]`);
    }
    this.ctx.view.requestOverlay();
  }

  /** Says the finding when it changed; when the tool starts, with what it read and what to do. */
  private tell(scope?: string): void {
    const plan = this.current();
    const text = finding(plan);
    if (!scope && text === this.said) return;
    this.said = text;
    const { log } = this.ctx;
    if (plan.areas.length) log.info(`${LABEL}: ${text}.${scope ? ` Enter ile uygulayın (${scope}).` : ''}`);
    else log.warn(`${LABEL}: ${text}${scope ? ` (${scope})` : ''}.`);
  }

  /** What to look at: free ends (with Topolojik temizlik's offer), regions with many labels, labels on a boundary. */
  private details(plan: Plan): void {
    const { log } = this.ctx;
    const free = plan.result.freeEnds.length;
    if (free) log.warn(`${LABEL}: ${free} çizgi ucu boşta; kapanmayan bölge alan olmaz. Önce Topolojik temizlik'i deneyin.`);
    const lines = [
      ...plan.result.regions.filter((r) => r.existing === undefined && r.labels.length > 1).map((r) => `${LABEL}: bir bölgede ${r.labels.length} etiket: ${r.labels.map((i) => `“${plan.labels[i].value}”`).join(', ')}.`),
      ...plan.result.onBoundary.map((i) => `${LABEL}: “${plan.labels[i].value}” etiketi bir sınırın üstünde; hiçbir bölgeye verilmedi.`),
    ];
    for (const line of lines.slice(0, MAX_DETAILS)) log.warn(line);
    if (lines.length > MAX_DETAILS) log.warn(`${LABEL}: ${lines.length - MAX_DETAILS} ayrıntı daha.`);
  }

  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    if (this.plan && this.plan.key !== this.key()) {
      this.refresh();
      this.tell();
    }
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    if (this.typing) return false;
    const key = text.trim().toLocaleUpperCase('tr-TR');
    if (key === 'A') {
      PolygonizeTool.islands = !PolygonizeTool.islands;
      this.refresh();
      this.tell();
      return true;
    }
    if (key === 'Ö' || key === 'O') {
      this.askName();
      return true;
    }
    return false;
  }

  /**
   * Ö: a text field by the cursor, as Yazı's and Kılavuz's (a name is no command: letters typed over the drawing
   * would start one), the name in it selected. Enter keeps what is typed, Esc (or an empty Enter) the old name.
   */
  private askName(): void {
    const { view } = this.ctx;
    const b = view.camera.visibleBounds();
    const at = this.hover ?? { x: (b.minX + b.maxX) / 2, y: (b.minY + b.maxY) / 2 };
    this.typing = true;
    this.refresh();
    // After the key or the chip that asked has given the drawing its focus back: the field takes it then.
    queueMicrotask(() => this.typing && view.requestTextInput({
      at,
      // Readable at any zoom: 14 px on the screen.
      height: view.worldTolerance(14),
      rotation: 0,
      initial: PolygonizeTool.attribute,
      placeholder: 'Öznitelik adı',
      hint: 'Enter: kaydet · Esc: vazgeç',
      commit: (text) => this.takeName(text),
      cancel: () => this.takeName(null),
    }));
  }

  /** The field's answer: the name typed, or none (Esc). An empty or too long name is said and the old one kept. */
  private takeName(text: string | null): void {
    this.typing = false;
    const name = text?.trim() ?? '';
    if (text !== null && !name) this.ctx.log.warn('Öznitelik adı boş olamaz.');
    else if ([...name].length > 60) this.ctx.log.warn('Öznitelik adı en çok 60 harf olabilir.');
    else if (name) PolygonizeTool.attribute = name;
    this.refresh();
    this.tell();
    this.ctx.view.focus();
  }

  /** Enter: the areas written in one step, and the tool leaves (the name's field answers its own Enter). */
  confirm(): void {
    if (this.typing) return;
    const { log, doc, settings } = this.ctx;
    const plan = this.current();
    if (!plan.areas.length) {
      log.warn(`${LABEL}: yazılacak alan yok; hiçbir şey değişmedi.`);
      return this.ctx.tools.exit();
    }
    const attribute = PolygonizeTool.attribute;
    const color = settings.color.value;
    const lineWeight = settings.lineWeight.value;
    const objects = plan.areas.map((a) => ({
      geometry: a.geometry,
      ...(color !== null && { color }),
      ...(lineWeight !== null && drawsLines(a.geometry) && { lineWeight }),
      ...(a.value !== null && { attrs: { [attribute]: a.value } }),
    }));
    const result = entitiesCreate.execute({ doc }, { layerId: doc.layers.active.value, objects, operation: 'polygonize' });
    if (result.status !== 'completed') {
      if ('error' in result) log.warn(result.error.message);
      return;
    }
    for (const w of result.warnings) log.warn(w.message);
    const named = plan.areas.filter((a) => a.value !== null).length;
    const empty = [...(plan.counts.unlabelled ? [`${plan.counts.unlabelled} etiketsiz`] : []), ...(plan.counts.many ? [`${plan.counts.many} çok etiketli`] : [])];
    log.success(`${LABEL}: ${plan.areas.length} alan oluşturuldu; “${attribute}” ${named} alana yazıldı.${empty.length ? ` Özniteliği boş kalan: ${empty.join(', ')}.` : ''}`);
    this.ctx.tools.exit();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const plan = this.plan;
    if (!plan) return;
    const pal = this.ctx.view.palette;
    for (const a of plan.areas.slice(0, MAX_GHOSTS)) {
      const g2 = a.geometry as Extract<NewGeometry, { kind: 'polygon' }>;
      const area = { outer: { pts: g2.pts, ...(g2.bulges && { bulges: g2.bulges }) } as RingGeometry, holes: (g2.holes ?? []) as RingGeometry[] };
      if (a.tone === 'one') drawArea(g, view, area, { color: pal.accent, fill: tint(pal.accent, 0.16), width: 1.5 });
      else if (a.tone === 'none') drawArea(g, view, area, { color: pal.accent, dash: [6, 4], width: 1.5 });
      else drawArea(g, view, area, { color: pal.danger, fill: tint(pal.danger, 0.16), width: 1.5 });
    }
    for (const p of plan.result.freeEnds.slice(0, 2000)) crossMark(g, view, p, pal.danger);
    for (const i of plan.result.onBoundary) ringMark(g, view, plan.labels[i].at, pal.danger, 5);
    if (this.hover) drawTag(g, view.worldToScreen(this.hover), tagLines(plan), pal.accent, pal.labelHalo);
  }
}

/** The kinds that close regions (docs/adr/0151 §2): construction lines, leaders and blocks do not. */
const LINE_WORK = new Set<Entity['kind']>(['line', 'polyline', 'arc', 'circle', 'ellipse', 'spline', 'polygon']);
export const isLineWork = (e: Entity): boolean => LINE_WORK.has(e.kind);

/** A label's value: a text's (trimmed), a point's label; null for anything else or an empty one. */
export function labelValue(e: Entity): string | null {
  const v = e.kind === 'text' ? e.text.trim() : e.kind === 'point' ? (e.label?.trim() ?? '') : '';
  return v ? v : null;
}

/** Where a label is: a text's box middle (its alignment and the drawing's typeface counted), a point itself. */
export function labelAt(e: Entity, font: string): Vec2 {
  if (e.kind !== 'text') return 'p' in e ? (e.p as Vec2) : { x: 0, y: 0 };
  const b = textBox({ p: e.p, text: e.text, height: e.height, rotation: e.rotation, ...(e.align && { align: e.align }), ...(e.widthFactor !== undefined && { widthFactor: e.widthFactor }), font });
  return { x: (b[0].x + b[1].x + b[2].x + b[3].x) / 4, y: (b[0].y + b[1].y + b[2].y + b[3].y) / 4 };
}

/** The area's rings with the elevations their corners carry from the line work (docs/adr/0142); none where none does. */
function withElevations(g: Extract<NewGeometry, { kind: 'polygon' }>, sources: Elevated[]): NewGeometry {
  const zs = (r: { pts: Vec2[] }) => {
    const out = carryElevations(r.pts, true, null, sources, false);
    return out.some((z) => z !== null) ? { zs: out } : {};
  };
  return { ...g, ...zs(g), ...(g.holes && { holes: g.holes.map((h) => ({ ...h, ...zs(h) })) }) };
}

/** `24 alan; 2 etiketsiz, 3 uç boşta`: the areas to write, then what to look at. */
function finding(plan: Plan): string {
  const r = plan.result;
  const extras = [
    ...(plan.counts.unlabelled ? [`${plan.counts.unlabelled} etiketsiz`] : []),
    ...(plan.counts.many ? [`${plan.counts.many} çok etiketli`] : []),
    ...(r.onBoundary.length ? [`${r.onBoundary.length} etiket sınırda`] : []),
    ...(plan.counts.existing ? [`${plan.counts.existing} zaten alan`] : []),
    ...(r.freeEnds.length ? [`${r.freeEnds.length} uç boşta`] : []),
  ];
  const head = plan.areas.length ? `${plan.areas.length} alan` : 'yazılacak alan yok';
  return extras.length ? `${head}; ${extras.join(', ')}` : head;
}

/** The finding beside the cursor, a count a line, and what writes it. */
function tagLines(plan: Plan): string[] {
  if (!plan.areas.length) return ['Yazılacak alan yok'];
  return [...finding(plan).split('; ').flatMap((part) => part.split(', ')), 'Enter: uygula'];
}

/** An × of fixed size: a free end. */
function crossMark(g: CanvasRenderingContext2D, view: ViewTransform, p: Vec2, color: string): void {
  const s = view.worldToScreen(p);
  g.save();
  g.strokeStyle = color;
  g.lineWidth = 2;
  g.setLineDash([]);
  g.beginPath();
  g.moveTo(s.x - 4, s.y - 4);
  g.lineTo(s.x + 4, s.y + 4);
  g.moveTo(s.x + 4, s.y - 4);
  g.lineTo(s.x - 4, s.y + 4);
  g.stroke();
  g.restore();
}

