import type { AppContext } from '../app/context';
import type { AreaPart } from '../contracts/generated/AreaPart';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity, type PolylineEntity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { entityFaceIndex, intersectAreaSets, netArea, splitArea, subtractAreas, unionAreas, type Area, type Source } from '../model/geom/region';
import { areaOfEntity, areasOfEntity, lineSource, oneArea, polygonOfArea, polylinesOfPolygon } from '../model/ops/areas';
import type { ViewTransform } from '../viewport/Camera';
import { writeObjects } from './createCommand';
import { createdIds, editGeometry, uidOf, writeEdit } from './editCommand';
import { SelectionActionTool } from './editTools';
import { SelectionFirstTool } from './modifyTools';
import { drawArea, drawTag, strokeGeometry, strokePath, tint } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { joinLines, joinPoints, kindsOf, MIXED, ONE_LINE, ONE_POINT, present, splitOf } from './lineParts';
import { drawTracking } from './tracking';
import { VisibleFaces } from './visibleFaces';

/**
 * Area operations (Netcad "Alan işlemleri"): union, intersection,
 * difference and splitting, closed objects to areas, an area by clicking
 * inside line work, and areas back to lines. The geometry is exact and
 * lives in model/geom/region (arcs stay arcs, holes are kept, input
 * corners keep their coordinates); these tools pick, preview and write it
 * through the product commands, one undo step named after the tool: the
 * changes of `cad.entities.edit`, İçine tıklayarak alan `cad.entities.create`
 * (docs/adr/0065). Objects on locked layers are left out before the command.
 *
 * Birleştir, kesiştir and çıkar take a multi-part area whole, as the union of
 * its parts; with Tek nesne (T) their result is one multi-part area rather
 * than an area a piece. Parçaları birleştir makes the selected areas one
 * multi-part area in the first one's place, Parçalara ayır the reverse
 * (docs/adr/0143). The desktop's are `crates/native/interaction/src/area.rs`.
 */

const AREA_KINDS = 'kapalı alan, daire, elips ya da kapalı eğri';

/**
 * Tek nesne (T) of Alan birleştir, kesiştir and çıkar: their result one
 * multi-part area (the desktop's `Memory.area_one_object`). One flag for the
 * three, off until asked.
 */
const oneObject = { on: false };

/** An option's value as the prompt says it. */
const yesNo = (on: boolean) => (on ? 'evet' : 'hayır');

/** The prompt's Tek nesne option. */
const oneObjectOption = () => `Tek nesne (T): ${yesNo(oneObject.on)}`;

/** Whether a typed answer is the option key `key`. */
const isKey = (text: string, key: string) => text.trim().toLocaleUpperCase('tr-TR') === key;

interface Picked {
  e: Entity;
  a: Area;
}

/** The entities of `list` that enclose an area. */
const areasOf = (list: readonly Entity[]): Picked[] =>
  list.flatMap((e) => {
    const a = areaOfEntity(e);
    return a ? [{ e, a }] : [];
  });

/** An object that encloses areas, and all of them: a multi-part area's parts, the one area of anything else (docs/adr/0143). */
interface Whole {
  e: Entity;
  parts: Area[];
}

/** The entities of `list` that enclose areas, each with all of them. */
const wholesOf = (list: readonly Entity[]): Whole[] =>
  list.flatMap((e) => {
    const parts = areasOfEntity(e);
    return parts.length ? [{ e, parts }] : [];
  });

const totalArea = (list: readonly Area[]) => list.reduce((s, a) => s + netArea(a), 0);

/** Every bounded face of the line work of entities (the core's index, released at once). */
function facesOf(lines: readonly Entity[]): Area[] {
  const index = entityFaceIndex(lines);
  try {
    return index.all();
  } finally {
    index.free();
  }
}

/**
 * New areas made from `from` (`add`): its layer and colour, and with
 * `keepData` its attributes and label; never its symbol.
 */
function addAreas(ctx: AppContext, areas: readonly Area[], from: Entity, keepData = true): EntityEdit[] {
  const uid = uidOf(ctx, from);
  return areas.map((a): EntityEdit => ({ kind: 'add', from: uid, geometry: editGeometry(polygonOfArea(a)), keepData }));
}

/**
 * New areas made from `from` as `addAreas` makes them; with `one` (Tek nesne) and two or more of them, one add: a
 * multi-part area of them all, the largest part first.
 */
function addResult(ctx: AppContext, areas: readonly Area[], from: Entity, keepData: boolean, one: boolean): EntityEdit[] {
  if (!one || areas.length < 2) return addAreas(ctx, areas, from, keepData);
  const geometry = oneArea(areas);
  return geometry ? [{ kind: 'add', from: uidOf(ctx, from), geometry: editGeometry(geometry), keepData }] : [];
}

/** `e` goes (`remove`). */
const removal = (ctx: AppContext, e: Entity): EntityEdit => ({ kind: 'remove', uid: uidOf(ctx, e) });

/** Straight cut line through the clicked points. */
function pathSource(pts: readonly Vec2[]): Source {
  const edges = pts.slice(1).map((b, i) => ({ kind: 'seg' as const, a: pts[i], b }));
  return { edges, points: [...pts], cut: true };
}

// ── Birleştir, kesiştir ─────────────────────────────────────────────────

export class AreaUnionTool extends SelectionActionTool {
  readonly id = 'areaUnion';
  protected readonly label = 'Alan birleştir';

  protected override pickHint(): string {
    return `[${oneObjectOption()}]`;
  }

  override input(text: string): boolean {
    if (this.picking && isKey(text, 'T')) {
      oneObject.on = !oneObject.on;
      this.refresh();
      return true;
    }
    return super.input(text);
  }

  protected run(targets: Entity[]): void {
    const { log, selection, format } = this.ctx;
    const list = wholesOf(targets);
    if (list.length < 2) return log.warn(`Birleştirmek için en az iki alan seçin (${AREA_KINDS}).`);
    const result = unionAreas(list.flatMap((x) => x.parts));
    const one = oneObject.on;
    // The first picked area lends its layer, colour and data (tevhit: the parcel kept).
    const out = writeEdit(this.ctx, 'areaUnion', [...list.map((x) => removal(this.ctx, x.e)), ...addResult(this.ctx, result, list[0].e, true, one)]);
    if (!out) return;
    selection.set(createdIds(this.ctx, out));
    const parts = result.length === 1 ? 'tek alan' : one ? `${result.length} parçalı tek alan` : `${result.length} ayrı alan (birbirine değmeyenler ayrı kalır)`;
    log.success(`${list.length} alan birleştirildi: ${parts}, toplam ${format.area(totalArea(result))}.`);
  }
}

export class AreaIntersectTool extends SelectionActionTool {
  readonly id = 'areaIntersect';
  protected readonly label = 'Alan kesiştir';
  private static erase = false;

  protected override pickHint(): string {
    return `[Kaynakları sil (S): ${yesNo(AreaIntersectTool.erase)} / ${oneObjectOption()}]`;
  }

  override input(text: string): boolean {
    if (this.picking && isKey(text, 'S')) AreaIntersectTool.erase = !AreaIntersectTool.erase;
    else if (this.picking && isKey(text, 'T')) oneObject.on = !oneObject.on;
    else return super.input(text);
    this.refresh();
    return true;
  }

  protected run(targets: Entity[]): void {
    const { log, selection, format } = this.ctx;
    const list = wholesOf(targets);
    if (list.length < 2) return log.warn(`Kesiştirmek için en az iki alan seçin (${AREA_KINDS}).`);
    // Each object is taken whole: a multi-part area meets what any of its parts meets.
    const result = intersectAreaSets(list.map((x) => x.parts));
    if (!result.length) return log.warn('Seçili alanların ortak bir parçası yok.');
    const { on: one } = oneObject;
    const erase = AreaIntersectTool.erase;
    // Kept sources keep their data; the overlap is a new, blank area. Erased, it takes the first one's data.
    const out = writeEdit(this.ctx, 'areaIntersect', [...(erase ? list.map((x) => removal(this.ctx, x.e)) : []), ...addResult(this.ctx, result, list[0].e, erase, one)]);
    if (!out) return;
    selection.set(createdIds(this.ctx, out));
    const pieces = result.length > 1 ? (one ? ` (${result.length} parçalı tek alan)` : ` (${result.length} parça)`) : '';
    log.success(`Ortak alan: ${format.area(totalArea(result))}${pieces}${erase ? '; kaynaklar silindi' : ''}.`);
  }
}

// ── Çıkar ──────────────────────────────────────────────────────────────

/**
 * Two selections: the areas to cut from, then the areas to take away. An object is taken whole, a multi-part
 * area's parts together (docs/adr/0143).
 */
export class AreaSubtractTool extends SelectionFirstTool {
  readonly id = 'areaSubtract';
  protected readonly label = 'Alan çıkar';
  private static eraseCutters = false;
  private from: Whole[] | null = null;

  override activate(): void {
    this.from = null;
    super.activate();
  }

  protected override refresh(): void {
    const n = this.ctx.selection.size;
    const step = this.from
      ? `çıkarılacak alanları seçin, bitince sağ tıklayın (${n} seçili) [Çıkarılanları sil (S): ${yesNo(AreaSubtractTool.eraseCutters)} / ${oneObjectOption()}]`
      : `kesilecek alanları seçin, bitince sağ tıklayın (${n} seçili)`;
    this.prompt.set(`${this.label}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  protected begin(): void {
    const { doc, log, selection } = this.ctx;
    const picked = wholesOf(this.targets());
    this.picking = true;
    selection.clear();
    if (!this.from) {
      const editable = picked.filter((x) => !doc.layers.isLocked(x.e.layerId));
      if (editable.length < picked.length) log.warn(`${picked.length - editable.length} alan kilitli katmanda olduğu için atlandı.`);
      if (!editable.length) return log.warn(`Önce kesilecek alanları seçin (${AREA_KINDS}).`);
      this.from = editable;
      return;
    }
    const cutters = picked.filter((x) => !this.from!.some((f) => f.e.id === x.e.id));
    if (!cutters.length) return log.warn(`Çıkarılacak en az bir alan seçin (${AREA_KINDS}).`);
    this.apply(this.from, cutters);
    queueMicrotask(() => this.ctx.tools.exit());
  }

  private apply(from: Whole[], cutters: Whole[]): void {
    const { doc, log, selection, format } = this.ctx;
    const cut = cutters.flatMap((x) => x.parts);
    const changes: EntityEdit[] = [];
    let changed = 0;
    let gone = 0;
    const erase = AreaSubtractTool.eraseCutters;
    const { on: one } = oneObject;
    for (const t of from) {
      const rest = subtractAreas(t.parts, cut);
      // Untouched areas stay as they are (a circle is not turned into a polygon for nothing).
      const size = totalArea(t.parts);
      if (rest.length === t.parts.length && Math.abs(totalArea(rest) - size) <= 1e-9 * Math.max(1, size)) continue;
      changes.push(removal(this.ctx, t.e), ...addResult(this.ctx, rest, t.e, true, one));
      changed++;
      if (!rest.length) gone++;
    }
    if (!changed) return log.warn('Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.');
    // The cutters go too when asked, but not those on a locked layer: they are left out here, before the command.
    if (erase) changes.push(...cutters.filter((x) => !doc.layers.isLocked(x.e.layerId)).map((x) => removal(this.ctx, x.e)));
    const out = writeEdit(this.ctx, 'areaSubtract', changes);
    if (!out) return;
    const created = createdIds(this.ctx, out);
    selection.set(created);
    const left = format.area(totalArea(created.flatMap((id) => areasOfEntity(doc.get(id)!))));
    log.success(`${changed} alandan çıkarıldı; kalan ${left}${gone ? `, ${gone} alan tamamen silindi` : ''}${erase ? '; çıkarılan alanlar silindi' : ''}.`);
  }

  override input(text: string): boolean {
    if (this.picking && this.from && isKey(text, 'S')) AreaSubtractTool.eraseCutters = !AreaSubtractTool.eraseCutters;
    else if (this.picking && this.from && isKey(text, 'T')) oneObject.on = !oneObject.on;
    else return super.input(text);
    this.refresh();
    return true;
  }

  protected stagePrompt(): string {
    return '';
  }
  protected point(): void {}

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    super.draw(g, view);
    const pal = this.ctx.view.palette;
    // The areas to cut from, every part of each.
    for (const f of this.from ?? []) for (const a of f.parts) drawArea(g, view, a, { color: pal.accent, width: 2 });
  }
}

// ── Parçaları birleştir, parçalara ayır ────────────────────────────────

/** A polygon's own fields or a part's as the contract has a part: each field only when it has it. */
function ownPart(p: { pts: Vec2[]; bulges?: number[]; holes?: AreaPart['holes']; zs?: (number | null)[] }): AreaPart {
  return { pts: p.pts, ...(p.bulges && { bulges: p.bulges }), ...(p.holes && { holes: p.holes }), ...(p.zs && { zs: p.zs }) };
}

/** A polygon geometry of one part, for `cad.entities.edit`: the part's fields as they are, elevations too, and no parts. */
const partGeometry = (p: AreaPart): Extract<EditGeometry, { kind: 'polygon' }> => ({ kind: 'polygon', ...ownPart(p) });

/**
 * An object's parts as the contract has them, each with its size (net area, elevations left out): a polygon's own
 * fields, then its parts; anything else that encloses an area, the area as a polygon (no elevations).
 */
function contractParts(x: Whole): { size: number; part: AreaPart }[] {
  const { e } = x;
  if (e.kind !== 'polygon') return x.parts.map((a) => ({ size: netArea(a), part: ownPart(polygonOfArea(a) as AreaPart) }));
  return [ownPart(e), ...(e.parts ?? [])].map((part) => {
    // A part with no area (too few corners) is kept, sized 0.
    const a = areaOfEntity({ kind: 'polygon', pts: part.pts, ...(part.bulges && { bulges: part.bulges }), ...(part.holes && { holes: part.holes }) });
    return { size: a ? netArea(a) : 0, part };
  });
}

/**
 * Parçaları birleştir: the selected areas become one multi-part area in the first one's place (it keeps its slot,
 * persistent id, layer and data; the others go). When no two overlap, every part is kept as it was, its elevations
 * too, the largest first; else the overlapping ones merge into one part (docs/adr/0143).
 */
export class PartsJoinTool extends SelectionActionTool {
  readonly id = 'partsJoin';
  protected readonly label = 'Parçaları birleştir';

  protected run(targets: Entity[]): void {
    const { log, selection, format } = this.ctx;
    // Lines and points join as their own kind, kinds mixed are refused (docs/adr/0174 §4).
    const kinds = kindsOf(targets);
    if (present(kinds) > 1) return log.warn(MIXED);
    if (kinds.lines.length === 1) return log.warn(ONE_LINE);
    if (kinds.points.length === 1) return log.warn(ONE_POINT);
    if (kinds.lines.length > 1) return joinLines(this.ctx, kinds.lines);
    if (kinds.points.length > 1) return joinPoints(this.ctx, kinds.points);
    const list = wholesOf(targets);
    if (list.length < 2) return log.warn(`Parçaları birleştirmek için en az iki alan seçin (${AREA_KINDS}).`);
    const all = list.flatMap((x) => x.parts);
    const merged = unionAreas(all);
    const size = totalArea(all);
    const apart = merged.length === all.length && Math.abs(totalArea(merged) - size) <= 1e-9 * Math.max(1, size);
    let geometry: EditGeometry | null;
    if (apart) {
      // Each part as the contract has it, sized by its area, from the largest down (the sort is stable).
      const [first, ...rest] = list
        .flatMap(contractParts)
        .sort((a, b) => b.size - a.size)
        .map((x) => x.part);
      geometry = { ...partGeometry(first), parts: rest };
    } else {
      const one = oneArea(merged);
      geometry = one && editGeometry(one);
    }
    if (!geometry) return;
    // The first picked lends its place, id, layer and data; a circle becomes the area.
    const first = list[0].e;
    const uid = uidOf(this.ctx, first);
    const out = writeEdit(this.ctx, 'partsJoin', [
      first.kind === 'polygon' ? { kind: 'update', uid, geometry } : { kind: 'replace', uid, geometry, keepData: true },
      ...list.slice(1).map((x) => removal(this.ctx, x.e)),
    ]);
    if (!out) return;
    selection.set([first.id]);
    const [pieces, total, note] = apart ? [all.length, size, ''] : [merged.length, totalArea(merged), 'örtüşenler birleşti, '];
    log.success(`${list.length} alan tek alanda birleşti: ${note}${pieces} parça, toplam ${format.area(total)}.`);
  }
}

/**
 * Parçalara ayır: each multi-part area becomes an area a part; the first part keeps the area's slot and persistent
 * id, the others are new with its layer and data, each part as it was, elevations too (docs/adr/0143); a multi-part
 * polyline and a multi-point object likewise, after the areas (docs/adr/0174).
 */
export class PartsSplitTool extends SelectionActionTool {
  readonly id = 'partsSplit';
  protected readonly label = 'Parçalara ayır';

  protected run(targets: Entity[]): void {
    const { log, selection } = this.ctx;
    const areas = targets.filter((e): e is PolylineEntity => e.kind === 'polygon' && !!e.parts?.length);
    const others = targets.flatMap((e) => {
      const split = splitOf(this.ctx, e);
      return split ? [{ e, ...split }] : [];
    });
    if (!areas.length && !others.length) return log.warn('Parçalarına ayrılacak çok parçalı bir nesne seçin: alan, çoklu çizgi ya da çok noktalı nesne.');
    const changes: EntityEdit[] = [];
    const each: { id: number; made: number }[] = [];
    for (const e of areas) {
      const uid = uidOf(this.ctx, e);
      changes.push({ kind: 'update', uid, geometry: partGeometry(ownPart(e)) });
      for (const part of e.parts ?? []) changes.push({ kind: 'add', from: uid, geometry: partGeometry(part), keepData: true });
      each.push({ id: e.id, made: e.parts?.length ?? 0 });
    }
    // The lines' and the points' after the areas', in the selection's order.
    for (const o of others) {
      changes.push(...o.changes);
      each.push({ id: o.e.id, made: o.made });
    }
    const out = writeEdit(this.ctx, 'partsSplit', changes);
    if (!out) return;
    // Each object followed by the objects made from it.
    const made = createdIds(this.ctx, out);
    let k = 0;
    const created = each.flatMap((x) => [x.id, ...made.slice(k, (k += x.made))]);
    selection.set(created);
    const noun = others.length ? 'nesne' : 'alan';
    log.success(`${each.length} ${noun} parçalarına ayrıldı (${created.length} ${noun}).`);
  }
}

// ── Böl ────────────────────────────────────────────────────────────────

/** Pick areas, then draw the cut line (or pick an existing line); the pieces replace the area. */
export class AreaSplitTool extends SelectionFirstTool {
  readonly id = 'areaSplit';
  protected readonly label = 'Alan böl';
  private list: Picked[] = [];
  private pts: Vec2[] = [];
  private byObject = false;
  private cutter: Entity | null = null;

  override activate(): void {
    this.list = [];
    this.pts = [];
    this.byObject = false;
    this.cutter = null;
    super.activate();
  }

  protected begin(): void {
    const { doc, log, selection } = this.ctx;
    const picked = areasOf(this.targets());
    const editable = picked.filter((x) => !doc.layers.isLocked(x.e.layerId));
    if (editable.length < picked.length) log.warn(`${picked.length - editable.length} alan kilitli katmanda olduğu için atlandı.`);
    selection.clear();
    if (!editable.length) {
      this.picking = true;
      return log.warn(`Bölünecek bir alan seçin (${AREA_KINDS}).`);
    }
    this.list = editable;
  }

  protected stagePrompt(): string {
    if (this.byObject) return 'kesici çizgiye tıklayın: çizgi, çoklu çizgi, yay ya da daire [Noktalarla kes (N)]';
    const n = this.pts.length;
    const step = n === 0 ? 'kesme çizgisinin ilk noktasını gösterin' : n === 1 ? 'sonraki noktayı gösterin' : 'sonraki noktayı gösterin ya da bitirmek için sağ tıklayın';
    return `${step} [${n ? 'Geri (G) / ' : ''}Çizgiyle kes (N)]`;
  }

  protected override anchor(): Vec2 | null {
    return this.byObject ? null : (this.pts.at(-1) ?? null);
  }

  protected point(p: Vec2): void {
    if (!this.pts.length || dist(this.pts[this.pts.length - 1], p) > 1e-9) this.pts.push(p);
  }

  override pointerDown(p: ToolPointer): void {
    if (!this.picking && this.byObject) {
      if (p.button !== 0) return;
      const e = this.ctx.view.pickEdge(p.screen);
      if (!e) return this.ctx.log.warn('Kesici olarak bir çizgiye, çoklu çizgiye, yaya ya da daireye tıklayın.');
      return this.split(lineSource([e]), e.id);
    }
    super.pointerDown(p);
  }

  override pointerMove(p: ToolPointer): void {
    super.pointerMove(p);
    if (!this.picking && this.byObject) {
      this.cutter = this.ctx.view.pickEdge(p.screen);
      this.ctx.selection.hover.set(this.cutter?.id ?? null);
    }
  }

  override input(text: string): boolean {
    if (!this.picking) {
      const t = text.trim().toLocaleUpperCase('tr-TR');
      if (t === 'N') {
        this.byObject = !this.byObject;
        this.ctx.selection.hover.set(null);
        this.refresh();
        return true;
      }
      if (t === 'G' && this.pts.length) {
        this.pts.pop();
        this.refresh();
        return true;
      }
    }
    return super.input(text);
  }

  override confirm(): void {
    if (this.picking) return super.confirm();
    if (this.pts.length >= 2) return this.split(pathSource(this.pts));
    if (this.pts.length === 1) return this.ctx.log.warn('Kesme çizgisi için en az iki nokta gösterin.');
    this.ctx.tools.exit();
  }

  private split(cut: Source, cutterId?: number): void {
    const { log, selection, format } = this.ctx;
    const changes: EntityEdit[] = [];
    /** Each split area and how many new pieces come from it, in order. */
    const split: { e: Entity; more: number }[] = [];
    const sizes: number[] = [];
    for (const t of this.list) {
      if (t.e.id === cutterId) continue;
      const pieces = splitArea(t.a, cut);
      if (pieces.length < 2) continue;
      // The first piece is the area itself, split: it keeps its slot and persistent id; the rest are new (ADR 0014).
      changes.push({ kind: 'replace', uid: uidOf(this.ctx, t.e), geometry: editGeometry(polygonOfArea(pieces[0])), keepData: true }, ...addAreas(this.ctx, pieces.slice(1), t.e));
      split.push({ e: t.e, more: pieces.length - 1 });
      sizes.push(...pieces.map(netArea));
    }
    const changed = split.length;
    if (!changed) {
      this.pts = [];
      this.refresh();
      return log.warn('Kesme çizgisi alanı baştan başa geçmiyor; çizgi alanın sınırını iki yerden kesmeli.');
    }
    const out = writeEdit(this.ctx, 'areaSplit', changes);
    if (!out) return;
    const made = createdIds(this.ctx, out);
    let k = 0;
    const created = split.flatMap((s) => [s.e.id, ...made.slice(k, (k += s.more))]);
    selection.set(created);
    const list = sizes.length <= 4 ? `: ${sizes.map((s) => format.area(s)).join(', ')}` : '';
    log.success(`${changed} alan ${created.length} parçaya bölündü${list}. Parçalar özgün alanın özniteliklerini taşır; parsel numaralarını güncelleyin.`);
    queueMicrotask(() => this.ctx.tools.exit());
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    for (const t of this.list) drawArea(g, view, t.a, { color: pal.accent, width: 2 });
    if (this.byObject) {
      if (this.cutter) strokeGeometry(g, view, entityGeometry(this.cutter), { color: pal.danger, width: 2 });
      return;
    }
    const line = this.hover ? [...this.pts, this.hover] : this.pts;
    strokePath(g, view, line, { color: pal.danger, width: 1.5 });
    // Live pieces: what the cut would leave, with their areas.
    if (line.length >= 2 && this.hover) {
      const pieces = this.list.flatMap((t) => {
        const r = splitArea(t.a, pathSource(line));
        return r.length > 1 ? r : [];
      });
      pieces.forEach((a, i) => drawArea(g, view, a, { color: pal.accent, fill: tint(i % 2 ? pal.snap : pal.accent, 0.18), dash: [4, 3], width: 1 }));
      if (pieces.length) drawTag(g, view.worldToScreen(this.hover), pieces.slice(0, 4).map((a, i) => `${i + 1}: ${this.ctx.format.area(netArea(a))}`), pal.accent, pal.labelHalo);
    }
    if (this.tracking && this.hover) drawTracking(g, view, this.tracking, this.hover, pal.accent, pal.labelHalo);
  }
}

// ── Dönüştür ───────────────────────────────────────────────────────────

/** Closed objects become areas; line work that closes regions adds an area per region. */
export class ToAreaTool extends SelectionActionTool {
  readonly id = 'toArea';
  protected readonly label = 'Alana çevir';

  protected run(targets: Entity[]): void {
    const { log, selection, format } = this.ctx;
    const closed = areasOf(targets.filter((e) => e.kind !== 'polygon'));
    const lines = targets.filter((e) => !areaOfEntity(e) && (e.kind === 'line' || e.kind === 'arc' || e.kind === 'polyline' || e.kind === 'spline' || e.kind === 'ellipse'));
    const faces = lines.length ? facesOf(lines) : [];
    if (!closed.length && !faces.length) {
      const already = targets.some((e) => e.kind === 'polygon');
      return log.warn(already ? 'Seçili nesneler zaten alan.' : 'Alana çevrilecek kapalı nesne ya da kapalı bölge oluşturan çizgi bulunamadı. Çizgilerin uçları birleşmeli ya da kesişmeli.');
    }
    // The object itself becomes an area: it keeps its slot and persistent id (docs/adr/0014).
    // Line work stays; the regions it closes become new areas on its layer.
    const out = writeEdit(this.ctx, 'toArea', [
      ...closed.map((x): EntityEdit => ({ kind: 'replace', uid: uidOf(this.ctx, x.e), geometry: editGeometry(polygonOfArea(x.a)), keepData: true })),
      ...(faces.length ? addAreas(this.ctx, faces, lines[0], false) : []),
    ]);
    if (!out) return;
    selection.set([...closed.map((x) => x.e.id), ...createdIds(this.ctx, out)]);
    const parts = [closed.length ? `${closed.length} nesne alana çevrildi` : '', faces.length ? `çizgilerden ${faces.length} alan oluştu (${format.area(totalArea(faces))})` : ''].filter(Boolean);
    log.success(`${parts.join('; ')}.`);
  }
}

/** Areas back to closed polylines: the outer ring keeps the data, holes become plain polylines. */
export class ToPolylineTool extends SelectionActionTool {
  readonly id = 'toPolyline';
  protected readonly label = 'Çizgiye çevir';

  protected run(targets: Entity[]): void {
    const { log, selection } = this.ctx;
    const polys = targets.filter((e) => e.kind === 'polygon');
    if (!polys.length) return log.warn('Çizgiye çevrilecek bir kapalı alan seçin.');
    // The outer ring is the area itself, now a polyline (it keeps its slot, persistent id and data, docs/adr/0014);
    // holes become new, blank polylines from it.
    const changes: EntityEdit[] = [];
    const holes: number[] = [];
    for (const e of polys) {
      if (e.kind !== 'polygon') continue;
      const uid = uidOf(this.ctx, e);
      const rings = polylinesOfPolygon(e);
      rings.forEach((g, i) =>
        changes.push(i === 0 ? { kind: 'replace', uid, geometry: editGeometry(g), keepData: true } : { kind: 'add', from: uid, geometry: editGeometry(g), keepData: false }),
      );
      holes.push(rings.length - 1);
    }
    const out = writeEdit(this.ctx, 'toPolyline', changes);
    if (!out) return;
    const made = createdIds(this.ctx, out);
    let k = 0;
    const created = polys.flatMap((e, i) => [e.id, ...made.slice(k, (k += holes[i]))]);
    selection.set(created);
    log.success(`${polys.length} alan kapalı çoklu çizgiye çevrildi (${created.length} çizgi).`);
  }
}

// ── İçine tıklayarak alan ───────────────────────────────────────────────

/**
 * Click inside a region closed by visible line work: the region becomes an
 * area on the active layer (AutoCAD BOUNDARY, Netcad "alan oluştur").
 * Closed groups inside it become holes unless islands are off. The
 * boundary set is everything visible, or one layer.
 */
export class BoundaryTool implements Tool {
  readonly id = 'boundary';
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  readonly snaps = false;
  private static islands = true;
  private readonly ctx: AppContext;
  private pickingLayer = false;
  private readonly faces: VisibleFaces;
  private hover: { at: Vec2; area: Area | null } | null = null;
  private hoverEntity: Entity | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.faces = new VisibleFaces(ctx);
  }

  activate(): void {
    this.faces.attach();
    this.refresh();
  }

  deactivate(): void {
    this.faces.detach();
    this.ctx.selection.hover.set(null);
  }

  private refresh(): void {
    const layer = this.faces.layer;
    const layerName = layer ? (this.ctx.doc.layers.get(layer)?.name ?? '—') : 'tümü';
    this.prompt.set(
      this.pickingLayer
        ? 'İçine tıklayarak alan: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]'
        : `İçine tıklayarak alan: alanı oluşturulacak bölgenin içine tıklayın [Adalar (A): ${BoundaryTool.islands ? 'delik olur' : 'yok sayılır'} / Sınır katmanı (K): ${layerName}]`,
    );
    this.ctx.view.requestOverlay();
  }

  pointerMove(p: ToolPointer): void {
    if (this.pickingLayer) {
      this.hoverEntity = this.ctx.view.pick(p.screen);
      this.ctx.selection.hover.set(this.hoverEntity?.id ?? null);
      return;
    }
    this.hover = { at: p.raw, area: this.faces.at(p.raw, BoundaryTool.islands) };
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const { ctx } = this;
    if (this.pickingLayer) {
      const e = ctx.view.pick(p.screen);
      if (!e) return ctx.log.warn('Sınır katmanını seçmek için bir nesneye tıklayın.');
      this.faces.setLayer(e.layerId);
      this.pickingLayer = false;
      ctx.selection.hover.set(null);
      return this.refresh();
    }
    const area = this.faces.at(p.raw, BoundaryTool.islands);
    if (!area) return ctx.log.warn('Tıklanan yer kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli; görünüm dışındaki çizgiler sayılmaz.');
    // On the active layer in the current colour, one step “Alan oluştur”; a locked or hidden layer is said by the command.
    const out = writeObjects(ctx, [polygonOfArea(area)], 'boundary');
    if (!out) return;
    ctx.selection.set([out.ids[0]]);
    ctx.log.success(`Alan oluşturuldu: ${ctx.format.area(netArea(area))}${area.holes.length ? `, ${area.holes.length} ada (delik)` : ''}.`);
  }

  input(text: string): boolean {
    const t = text.trim().toLocaleUpperCase('tr-TR');
    if (t === 'A' && !this.pickingLayer) {
      BoundaryTool.islands = !BoundaryTool.islands;
      if (this.hover) this.hover.area = this.faces.at(this.hover.at, BoundaryTool.islands);
      this.refresh();
      return true;
    }
    if (t === 'K') {
      if (this.pickingLayer || this.faces.layer) {
        this.faces.setLayer(null);
        this.pickingLayer = false;
      } else this.pickingLayer = true;
      this.ctx.selection.hover.set(null);
      this.refresh();
      return true;
    }
    return false;
  }

  confirm(): void {
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    if (!this.pickingLayer) return false;
    this.pickingLayer = false;
    this.ctx.selection.hover.set(null);
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const h = this.hover;
    if (this.pickingLayer || !h?.area) return;
    const pal = this.ctx.view.palette;
    drawArea(g, view, h.area, { color: pal.accent, fill: tint(pal.accent, 0.16), width: 2 });
    const lines = [this.ctx.format.area(netArea(h.area))];
    if (h.area.holes.length) lines.push(`${h.area.holes.length} ada`);
    drawTag(g, view.worldToScreen(h.at), lines, pal.accent, pal.labelHalo);
  }
}

