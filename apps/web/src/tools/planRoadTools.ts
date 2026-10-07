import type { AppContext } from '../app/context';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import { Signal } from '../core/signal';
import type { Entity, EntityGeometry } from '../model/entities';
import type { Area, Ring } from '../model/geom/overlay';
import { cleanAxis } from '../model/geom/parallel';
import { dist, type Vec2 } from '../model/geometry';
import { areasOfEntity, oneArea, polygonOfArea } from '../model/ops/areas';
import { CARRIAGEWAY, KIND, medianRing, NOT_JOINED, roadJunctions, roadParts, ROAD_KINDS, WIDTH } from '../model/planRoad';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import * as createCommand from './createCommand';
import { PointInputTool } from './drawTools';
import { uidOf, writeEdit } from './editCommand';
import { SelectionFirstTool } from './modifyTools';
import { drawTag, strokeGeometry, strokePath } from './preview';
import type { OptionChoice, Tool, ToolPointer } from './Tool';

/**
 * Plan yolu çizimi (docs/adr/0198; the desktop's `kentos_interaction::plan_road`).
 *
 * - **Plan yolu** (`planRoad`): the axis clicked (snapping) or typed; Enter or a right click ends it. The road is an
 *   area `Genişlik` wide around it; a Yol's kerbs (Kaldırım) leave a carriageway area inside, its median (Refüj) an
 *   area closed with half circles; Eksen writes the axis too. All on the active layer, their `Tür` and `Genişlik`
 *   attributes, one step “Plan yolu” through `cad.entities.create`.
 * - **Kavşak temizle** (`roadJunctions`): selection first; the selected areas joined kind by kind (`Tür`; Refüj and Yol
 *   ekseni left out) and their inner corners rounded, Ada köşesi (A) for roads, Kaldırım köşesi (K) for carriageways;
 *   the result previewed dashed, Enter writes it in one step “Kavşak temizle” through `cad.entities.edit`.
 * - **Refüj kapat** (`medianClose`): two lines picked, closed into a median with half circles (Uç (U): Düz straight);
 *   the first becomes the area, the second goes, one step “Refüj kapat”.
 *
 * The kinds' widths, Kaldırım, Refüj, Eksen, the radii and Uç stay for the session (the desktop's `Memory`).
 */

/** A number and nothing else (the desktop's `plain_number`). */
const plainNumber = (text: string): number | null => (/[,;@<]/.test(text) ? null : parseNumber(text));

/** A metre value as an attribute's text: the number as JavaScript writes it (the desktop's `js_number`). */
const metres = (m: number): string => String(m);

/** A geometry the command takes, from an area. */
const areaGeometry = (a: Area): NewGeometry => polygonOfArea(a) as unknown as NewGeometry;

/** Plan yolu. */
export class PlanRoadTool extends PointInputTool {
  readonly id = 'planRoad';
  protected readonly label = 'Plan yolu';
  protected override readonly stepsFromPoints = true;
  /** The kind (an index of ROAD_KINDS), each kind's width, the kerbs and median (metres, 0 none), the axis written. */
  static kind = 0;
  static widths: number[] = ROAD_KINDS.map((k) => k.width);
  static kerb = 0;
  static median = 0;
  static axis = false;
  /** A value being typed. */
  private asking: 'width' | 'kerb' | 'median' | null = null;

  private get isRoad(): boolean {
    return PlanRoadTool.kind === 0;
  }

  private options(): string {
    const f = this.ctx.format;
    const S = PlanRoadTool;
    const parts = [`Tür (T): ${ROAD_KINDS[S.kind].kind}`, `Genişlik (G): ${f.length(S.widths[S.kind])}`];
    if (this.isRoad) parts.push(`Kaldırım (K): ${S.kerb > 0 ? f.length(S.kerb) : 'yok'}`, `Refüj (R): ${S.median > 0 ? f.length(S.median) : 'yok'}`);
    parts.push(S.axis ? 'Eksen (E): açık' : 'Eksen (E)');
    return parts.join(' / ');
  }

  protected promptFor(n: number): string {
    const f = this.ctx.format;
    const S = PlanRoadTool;
    const unit = f.lengthUnitLabel;
    if (this.asking === 'width') return `yolun genişliğini ${unit} olarak yazın (Enter: ${f.plain(S.widths[S.kind])})`;
    if (this.asking === 'kerb') return `kaldırım genişliğini ${unit} olarak yazın; 0 kaldırımsız (Enter: ${f.plain(S.kerb)})`;
    if (this.asking === 'median') return `refüj genişliğini ${unit} olarak yazın; 0 refüjsüz (Enter: ${f.plain(S.median)})`;
    if (n === 0) return `eksenin ilk noktasını belirtin [${this.options()}]`;
    return `eksenin sonraki noktasını belirtin; Enter ya da sağ tık bitirir [${this.options()}]`;
  }

  private ask(what: 'width' | 'kerb' | 'median'): true {
    this.asking = what;
    this.refreshPrompt();
    return true;
  }

  protected override option(key: string): boolean {
    const S = PlanRoadTool;
    if (key === 'T') S.kind = (S.kind + 1) % ROAD_KINDS.length;
    else if (key === 'G') return this.ask('width');
    else if (key === 'K' && this.isRoad) return this.ask('kerb');
    else if (key === 'R' && this.isRoad) return this.ask('median');
    else if (key === 'E') S.axis = !S.axis;
    else return false;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key !== 'T') return null;
    return ROAD_KINDS.map((k, i) => ({ label: k.kind, typed: k.kind.toLocaleLowerCase('tr-TR'), checked: i === PlanRoadTool.kind }));
  }

  chooseOption(key: string, typed: string): boolean {
    const i = key === 'T' ? ROAD_KINDS.findIndex((k) => k.kind.toLocaleLowerCase('tr-TR') === typed.trim().toLocaleLowerCase('tr-TR')) : -1;
    if (i < 0) return false;
    PlanRoadTool.kind = i;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  /** A value out of range: said, and taken as understood. */
  private refuse(why: string): true {
    this.ctx.log.warn(why);
    return true;
  }

  private value(text: string): boolean {
    const n = plainNumber(text);
    if (n === null) return false;
    const m = this.ctx.format.toMetres(n);
    const S = PlanRoadTool;
    if (this.asking === 'width') {
      if (!(m > 0)) return this.refuse('Yolun genişliği sıfırdan büyük olmalı.');
      S.widths[S.kind] = m;
    } else if (!(m >= 0)) return this.refuse(this.asking === 'kerb' ? 'Kaldırım genişliği sıfır ya da pozitif olmalı.' : 'Refüj genişliği sıfır ya da pozitif olmalı.');
    else if (this.asking === 'kerb') S.kerb = m;
    else S.median = m;
    this.asking = null;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  override input(text: string): boolean {
    if (this.asking) {
      if (this.option(text.trim().toLocaleUpperCase('tr-TR'))) return true;
      return this.value(text);
    }
    return super.input(text);
  }

  protected onPoint(p: Vec2): void {
    this.asking = null;
    const last = this.pts.at(-1);
    if (last && dist(last, p) < 1e-9) return;
    this.pts.push(p);
  }

  /** Why the widths cannot make the road, or null. */
  private misfit(): string | null {
    const f = this.ctx.format;
    const S = PlanRoadTool;
    if (!this.isRoad) return null;
    const w = S.widths[0];
    if (2 * S.kerb >= w) return `Kaldırımlar (2 × ${f.length(S.kerb)}) yolun genişliğine (${f.length(w)}) sığmıyor; Kaldırım'ı ya da Genişlik'i değiştirin.`;
    if (S.median >= w - 2 * S.kerb) return `Refüj (${f.length(S.median)}) taşıt yoluna (${f.length(w - 2 * S.kerb)}) sığmıyor; Refüj'ü ya da Genişlik'i değiştirin.`;
    return null;
  }

  /** The road's parts along `axis` at the kept values, or null. */
  private parts(axis: Vec2[]) {
    const S = PlanRoadTool;
    if (this.misfit()) return null;
    return roadParts(axis, S.widths[S.kind], this.isRoad ? S.kerb : 0, this.isRoad ? S.median : 0);
  }

  protected override finish(): void {
    const S = PlanRoadTool;
    const axis = cleanAxis(this.pts, false);
    if (axis.length < 2) return this.reset();
    const why = this.misfit();
    if (why) return void this.ctx.log.warn(why);
    const parts = this.parts(axis);
    if (!parts) return this.reset();
    const kind = ROAD_KINDS[S.kind].kind;
    const width = S.widths[S.kind];
    const items: { geometry: EntityGeometry; attrs?: Record<string, string> }[] = [{ geometry: areaGeometry(parts.road) as unknown as EntityGeometry, attrs: { [KIND]: kind, [WIDTH]: metres(width) } }];
    if (parts.carriageway) items.push({ geometry: areaGeometry(parts.carriageway) as unknown as EntityGeometry, attrs: { [KIND]: CARRIAGEWAY, [WIDTH]: metres(width - 2 * S.kerb) } });
    if (parts.median) items.push({ geometry: areaGeometry(parts.median) as unknown as EntityGeometry, attrs: { [KIND]: 'Refüj', [WIDTH]: metres(S.median) } });
    if (S.axis) items.push({ geometry: { kind: 'polyline', pts: axis } as EntityGeometry, attrs: { [KIND]: 'Yol ekseni' } });
    const length = axis.slice(1).reduce((s, p, i) => s + dist(axis[i], p), 0);
    const f = this.ctx.format;
    if (createCommand.writeObjectsEach(this.ctx, items, 'planRoad')) this.ctx.log.success(`${kind} eklendi: genişlik ${f.length(width)}, eksen ${f.length(length)}.`);
    this.reset();
  }

  override confirm(): void {
    if (this.asking) {
      this.asking = null;
      return this.refreshPrompt();
    }
    super.confirm();
  }

  override undoStep(): boolean {
    if (this.asking) {
      this.asking = null;
      this.refreshPrompt();
      return true;
    }
    if (!this.pts.length) return false;
    this.pts.pop();
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
    return true;
  }

  cancel(): boolean {
    return this.undoStep();
  }

  /** The axis so far to the cursor, dashed; the road's areas along it. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const h = this.hover;
    if (!h || !this.pts.length) return;
    const pal = this.ctx.view.palette;
    const axis = cleanAxis([...this.pts, h], false);
    strokePath(g, view, axis, { color: pal.accent, dash: [5, 3] });
    const parts = axis.length >= 2 ? this.parts(axis) : null;
    if (!parts) return;
    strokeGeometry(g, view, polygonOfArea(parts.road), { color: pal.accent, width: 1.5 });
    if (parts.carriageway) strokeGeometry(g, view, polygonOfArea(parts.carriageway), { color: pal.accent, dash: [4, 3] });
    if (parts.median) strokeGeometry(g, view, polygonOfArea(parts.median), { color: pal.accent });
    const f = this.ctx.format;
    const length = axis.slice(1).reduce((s, p, i) => s + dist(axis[i], p), 0);
    drawTag(g, view.worldToScreen(h), [ROAD_KINDS[PlanRoadTool.kind].kind, `${f.length(PlanRoadTool.widths[PlanRoadTool.kind])} × ${f.length(length)}`], pal.accent, pal.labelHalo);
  }
}

/** One kind's work in Kavşak temizle: its areas, the first one that keeps the result, the result and its counts. */
interface Group {
  kind: string;
  objects: Entity[];
  geometry: EntityGeometry | null;
  areas: Area[];
  done: number;
  skipped: number;
}

/** Kavşak temizle. */
export class RoadJunctionsTool extends SelectionFirstTool {
  readonly id = 'roadJunctions';
  protected readonly label = 'Kavşak temizle';
  override readonly snaps = false;
  /** Ada köşesi and Kaldırım köşesi, metres (0: not rounded); the tool's first values (docs/adr/0198 §5). */
  static ada = 5;
  static kerb = 8;
  private ids: number[] = [];
  private asking: 'ada' | 'kerb' | null = null;
  private plan: { key: string; groups: Group[] } | null = null;

  protected begin(): void {
    this.asking = null;
    this.plan = null;
    this.ids = this.targets()
      .filter((e) => e.kind === 'polygon' && !NOT_JOINED.includes(e.attrs[KIND] ?? ''))
      .map((e) => e.id);
    if (this.ids.length) return;
    this.ctx.log.warn('Kavşak temizle: seçimde yol alanı yok.');
    queueMicrotask(() => this.ctx.tools.exit());
  }

  /** The groups for the drawing as it is, kept until the drawing or a radius changes. */
  private current(): Group[] {
    const { doc } = this.ctx;
    const S = RoadJunctionsTool;
    const key = `${doc.revision}|${S.ada}|${S.kerb}`;
    if (this.plan?.key === key) return this.plan.groups;
    const chosen = new Set(this.ids);
    const byKind = new Map<string, Entity[]>();
    // In the drawing's order: a kind's first area keeps the result.
    for (const e of doc.all()) {
      if (!chosen.has(e.id)) continue;
      const kind = e.attrs[KIND] ?? '';
      byKind.set(kind, [...(byKind.get(kind) ?? []), e]);
    }
    const groups = [...byKind].map(([kind, objects]) => {
      const radius = kind === CARRIAGEWAY ? S.kerb : S.ada;
      const result = roadJunctions(objects.flatMap((e) => areasOfEntity(e)), radius);
      return { kind, objects, geometry: oneArea(result.areas), areas: result.areas, done: result.done, skipped: result.skipped };
    });
    this.plan = { key, groups };
    return groups;
  }

  /** `3 alan → 1 alan, 4 köşe yuvarlanacak; 1 köşe sığmıyor`. */
  private finding(groups: Group[]): string[] {
    const before = groups.reduce((s, g) => s + g.objects.length, 0);
    const after = groups.reduce((s, g) => s + (g.geometry ? 1 : 0), 0);
    const done = groups.reduce((s, g) => s + g.done, 0);
    const skipped = groups.reduce((s, g) => s + g.skipped, 0);
    const lines = [`${before} alan → ${after} alan, ${done} köşe yuvarlanacak`];
    if (skipped) lines.push(`${skipped} köşe sığmıyor`);
    return lines;
  }

  protected stagePrompt(): string {
    const f = this.ctx.format;
    const S = RoadJunctionsTool;
    if (this.asking === 'ada') return `ada köşesi yarıçapını ${f.lengthUnitLabel} olarak yazın; 0 yuvarlamaz (Enter: ${f.plain(S.ada)})`;
    if (this.asking === 'kerb') return `kaldırım köşesi yarıçapını ${f.lengthUnitLabel} olarak yazın; 0 yuvarlamaz (Enter: ${f.plain(S.kerb)})`;
    return `${this.finding(this.current()).join('; ')} [Ada köşesi (A): ${f.length(S.ada)} / Kaldırım köşesi (K): ${f.length(S.kerb)} / Uygula (Enter)]`;
  }

  protected point(): void {}

  override acceptPoint(): boolean {
    return false;
  }

  override input(text: string): boolean {
    if (this.picking) return false;
    const key = text.trim().toLocaleUpperCase('tr-TR');
    if (key === 'A' || key === 'K') {
      this.asking = key === 'A' ? 'ada' : 'kerb';
      this.refresh();
      return true;
    }
    if (!this.asking) return false;
    const n = plainNumber(text);
    if (n === null) return false;
    const m = this.ctx.format.toMetres(n);
    if (!(m >= 0)) {
      this.ctx.log.warn('Yarıçap sıfır ya da pozitif olmalı.');
      return true;
    }
    if (this.asking === 'ada') RoadJunctionsTool.ada = m;
    else RoadJunctionsTool.kerb = m;
    this.asking = null;
    this.refresh();
    return true;
  }

  /** Enter: while picking, the selection is taken; a value asked keeps its value; then the result is written. */
  override confirm(): void {
    if (this.picking) return super.confirm();
    if (this.asking) {
      this.asking = null;
      return this.refresh();
    }
    const groups = this.current();
    const changes: EntityEdit[] = [];
    for (const g of groups) {
      const [first, ...others] = g.objects;
      if (!g.geometry || (!others.length && !g.done)) continue;
      changes.push({ kind: 'update', uid: uidOf(this.ctx, first), geometry: g.geometry as unknown as NewGeometry });
      for (const e of others) changes.push({ kind: 'remove', uid: uidOf(this.ctx, e) });
    }
    if (!changes.length) {
      this.ctx.log.info('Kavşak temizle: değişecek bir şey yok.');
      return this.ctx.tools.exit();
    }
    if (!writeEdit(this.ctx, 'roadJunctions', changes)) return this.refresh();
    const [head, ...rest] = this.finding(groups);
    this.ctx.log.success(`Kavşak temizlendi: ${head.replace(' yuvarlanacak', ' yuvarlandı')}${rest.length ? `; ${rest.join('; ')}` : ''}.`);
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    const groups = this.current();
    for (const gr of groups) if (gr.geometry) strokeGeometry(g, view, gr.geometry, { color: pal.accent, dash: [5, 3], width: 1.5 });
    if (this.hover) drawTag(g, view.worldToScreen(this.hover), [...this.finding(groups), 'Enter: uygula'], pal.accent, pal.labelHalo);
  }
}

/** A line's or an open polyline's path, as the core takes a ring's. */
function pathOf(e: Entity): Ring | null {
  if (e.kind === 'line') return { pts: [e.a, e.b] };
  if (e.kind === 'polyline' && !e.parts?.length) return { pts: e.pts, ...(e.bulges && { bulges: e.bulges }) };
  return null;
}

/** Refüj kapat. */
export class MedianCloseTool implements Tool {
  readonly id = 'medianClose';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  /** Uç: half circles, else straight; kept for the session (the desktop's `Memory::median_round`). */
  static round = true;
  private readonly ctx: AppContext;
  private first: { e: Entity; at: Vec2 } | null = null;
  private under: Entity | null = null;

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
    const step = this.first ? 'refüjün ikinci kenarına tıklayın; Esc birinciyi bıraktırır' : 'refüjün birinci kenarına tıklayın';
    this.prompt.set(`Refüj kapat: ${step} [Uç (U): ${MedianCloseTool.round ? 'Yarım daire' : 'Düz'}]`);
    this.ctx.view.requestOverlay();
  }

  private lineUnder(p: ToolPointer): Entity | null {
    const besides = this.first?.e.id;
    return this.ctx.view.pickEdge(p.screen, (e) => (e.kind === 'line' || e.kind === 'polyline') && e.id !== besides);
  }

  pointerMove(p: ToolPointer): void {
    this.under = this.lineUnder(p);
    this.ctx.selection.hover.set(this.under?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const { log } = this.ctx;
    const e = this.lineUnder(p);
    if (!e) return void log.warn('Tıklanan yerde çizgi ya da çoklu çizgi yok. Refüjün kenarına tıklayın.');
    if (!pathOf(e)) return void log.warn('Çok parçalı çizgi refüjün kenarı olamaz; parçalarına ayırın.');
    if (!this.first) {
      this.first = { e, at: p.raw };
      this.under = null;
      this.ctx.selection.hover.set(null);
      return this.refresh();
    }
    const ring = medianRing(pathOf(this.first.e)!, pathOf(e)!, MedianCloseTool.round);
    if (!ring) return void log.warn('Bu iki çizgi refüj olarak kapatılamaz.');
    const changes: EntityEdit[] = [
      { kind: 'replace', uid: uidOf(this.ctx, this.first.e), geometry: { kind: 'polygon', pts: ring.pts, ...(ring.bulges && { bulges: ring.bulges }) } as NewGeometry, keepData: true },
      { kind: 'remove', uid: uidOf(this.ctx, e) },
    ];
    if (writeEdit(this.ctx, 'medianClose', changes)) log.success('Refüj kapatıldı.');
    this.first = null;
    this.under = null;
    this.ctx.selection.hover.set(null);
    this.refresh();
  }

  input(text: string): boolean {
    if (text.trim().toLocaleUpperCase('tr-TR') !== 'U') return false;
    MedianCloseTool.round = !MedianCloseTool.round;
    this.refresh();
    return true;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    if (key !== 'U') return null;
    return [
      { label: 'Yarım daire', typed: 'yarım daire', checked: MedianCloseTool.round },
      { label: 'Düz', typed: 'düz', checked: !MedianCloseTool.round },
    ];
  }

  chooseOption(key: string, typed: string): boolean {
    const word = typed.trim().toLocaleLowerCase('tr-TR');
    if (key !== 'U' || (word !== 'yarım daire' && word !== 'düz')) return false;
    MedianCloseTool.round = word === 'yarım daire';
    this.refresh();
    return true;
  }

  /** The first let go; without one the tool leaves. */
  confirm(): void {
    if (!this.first) return this.ctx.tools.exit();
    this.first = null;
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

  /** Where the first was picked; over a second, the median it closes, dashed. */
  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.first) return;
    const pal = this.ctx.view.palette;
    const s = view.worldToScreen(this.first.at);
    g.save();
    g.strokeStyle = pal.accent;
    g.strokeRect(Math.round(s.x) - 4.5, Math.round(s.y) - 4.5, 9, 9);
    g.restore();
    const second = this.under && pathOf(this.under);
    const ring = second ? medianRing(pathOf(this.first.e)!, second, MedianCloseTool.round) : null;
    if (ring) strokeGeometry(g, view, { kind: 'polygon', pts: ring.pts, ...(ring.bulges && { bulges: ring.bulges }) } as EntityGeometry, { color: pal.accent, dash: [5, 3], width: 1.5 });
  }
}
