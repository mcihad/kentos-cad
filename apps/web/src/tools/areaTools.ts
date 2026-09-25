import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity, type NewEntity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { entityFaceIndex, intersectAreas, netArea, splitArea, subtractAreas, unionAreas, type Area, type Source } from '../model/geom/region';
import { areaOfEntity, lineSource, polygonOfArea, polylinesOfPolygon } from '../model/ops/areas';
import type { ViewTransform } from '../viewport/Camera';
import { SelectionActionTool } from './editTools';
import { SelectionFirstTool } from './modifyTools';
import { drawArea, drawTag, strokeGeometry, strokePath, tint } from './preview';
import { writableLayer } from './targetLayer';
import type { Tool, ToolPointer } from './Tool';
import { drawTracking } from './tracking';
import { VisibleFaces } from './visibleFaces';

/**
 * Area operations (Netcad "Alan işlemleri"): union, intersection,
 * difference and splitting, closed objects to areas, an area by clicking
 * inside line work, and areas back to lines. The geometry is exact and
 * lives in model/geom/region (arcs stay arcs, holes are kept, input
 * corners keep their coordinates); these tools pick, preview and record
 * one undo step.
 */

const AREA_KINDS = 'kapalı alan, daire, elips ya da kapalı eğri';

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

/** A polygon for `a` on the layer of `from`, with its colour and (when `keepData`) its data and label. */
function areaEntity(a: Area, from: Entity, keepData = true): NewEntity {
  return { ...polygonOfArea(a), layerId: from.layerId, color: from.color, attrs: keepData ? { ...from.attrs } : {}, label: keepData ? from.label : undefined } as NewEntity;
}

/** Adds polygons for `areas` (`areaEntity`). */
function addAreas(ctx: AppContext, areas: readonly Area[], from: Entity, keepData = true): number[] {
  return areas.map((a) => ctx.doc.add(areaEntity(a, from, keepData)).id);
}

/** Straight cut line through the clicked points. */
function pathSource(pts: readonly Vec2[]): Source {
  const edges = pts.slice(1).map((b, i) => ({ kind: 'seg' as const, a: pts[i], b }));
  return { edges, points: [...pts], cut: true };
}

// ── Birleştir, kesiştir ─────────────────────────────────────────────────

export class AreaUnionTool extends SelectionActionTool {
  readonly id = 'areaUnion';
  protected readonly label = 'Alan birleştir';

  protected run(targets: Entity[]): void {
    const { doc, log, selection, format } = this.ctx;
    const list = areasOf(targets);
    if (list.length < 2) return log.warn(`Birleştirmek için en az iki alan seçin (${AREA_KINDS}).`);
    const result = unionAreas(list.map((x) => x.a));
    let created: number[] = [];
    doc.transact(this.label, () => {
      doc.remove(list.map((x) => x.e.id));
      // The first picked area lends its layer, colour and data (tevhit: the parcel kept).
      created = addAreas(this.ctx, result, list[0].e);
    });
    selection.set(created);
    const parts = result.length === 1 ? 'tek alan' : `${result.length} ayrı alan (birbirine değmeyenler ayrı kalır)`;
    log.success(`${list.length} alan birleştirildi: ${parts}, toplam ${format.area(totalArea(result))}.`);
  }
}

export class AreaIntersectTool extends SelectionActionTool {
  readonly id = 'areaIntersect';
  protected readonly label = 'Alan kesiştir';
  private static erase = false;

  protected override pickHint(): string {
    return `[Kaynakları sil (S): ${AreaIntersectTool.erase ? 'evet' : 'hayır'}]`;
  }

  override input(text: string): boolean {
    if (this.picking && text.trim().toLocaleUpperCase('tr-TR') === 'S') {
      AreaIntersectTool.erase = !AreaIntersectTool.erase;
      this.refresh();
      return true;
    }
    return super.input(text);
  }

  protected run(targets: Entity[]): void {
    const { doc, log, selection, format } = this.ctx;
    const list = areasOf(targets);
    if (list.length < 2) return log.warn(`Kesiştirmek için en az iki alan seçin (${AREA_KINDS}).`);
    const result = intersectAreas(list.map((x) => x.a));
    if (!result.length) return log.warn('Seçili alanların ortak bir parçası yok.');
    const erase = AreaIntersectTool.erase;
    let created: number[] = [];
    doc.transact(this.label, () => {
      if (erase) doc.remove(list.map((x) => x.e.id));
      // Kept sources keep their data; the overlap is a new, blank area.
      created = addAreas(this.ctx, result, list[0].e, erase);
    });
    selection.set(created);
    log.success(`Ortak alan: ${format.area(totalArea(result))}${result.length > 1 ? ` (${result.length} parça)` : ''}${erase ? '; kaynaklar silindi' : ''}.`);
  }
}

// ── Çıkar ──────────────────────────────────────────────────────────────

/** Two selections: the areas to cut from, then the areas to take away. */
export class AreaSubtractTool extends SelectionFirstTool {
  readonly id = 'areaSubtract';
  protected readonly label = 'Alan çıkar';
  private static eraseCutters = false;
  private from: Picked[] | null = null;

  override activate(): void {
    this.from = null;
    super.activate();
  }

  protected override refresh(): void {
    const n = this.ctx.selection.size;
    const step = this.from
      ? `çıkarılacak alanları seçin, bitince sağ tıklayın (${n} seçili) [Çıkarılanları sil (S): ${AreaSubtractTool.eraseCutters ? 'evet' : 'hayır'}]`
      : `kesilecek alanları seçin, bitince sağ tıklayın (${n} seçili)`;
    this.prompt.set(`${this.label}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  protected begin(): void {
    const { doc, log, selection } = this.ctx;
    const picked = areasOf(this.targets());
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

  private apply(from: Picked[], cutters: Picked[]): void {
    const { doc, log, selection, format } = this.ctx;
    const cut = cutters.map((x) => x.a);
    const created: number[] = [];
    let changed = 0;
    let gone = 0;
    const erase = AreaSubtractTool.eraseCutters;
    doc.transact(this.label, () => {
      for (const t of from) {
        const rest = subtractAreas([t.a], cut);
        // Untouched areas stay as they are (a circle is not turned into a polygon for nothing).
        if (rest.length === 1 && Math.abs(totalArea(rest) - netArea(t.a)) <= 1e-9 * Math.max(1, netArea(t.a))) continue;
        doc.remove([t.e.id]);
        created.push(...addAreas(this.ctx, rest, t.e));
        changed++;
        if (!rest.length) gone++;
      }
      if (erase && changed) doc.remove(cutters.filter((x) => !doc.layers.isLocked(x.e.layerId)).map((x) => x.e.id));
    });
    if (!changed) return log.warn('Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.');
    selection.set(created);
    const left = format.area(totalArea(created.map((id) => areaOfEntity(doc.get(id)!)!)));
    log.success(`${changed} alandan çıkarıldı; kalan ${left}${gone ? `, ${gone} alan tamamen silindi` : ''}${erase ? '; çıkarılan alanlar silindi' : ''}.`);
  }

  override input(text: string): boolean {
    if (this.picking && this.from && text.trim().toLocaleUpperCase('tr-TR') === 'S') {
      AreaSubtractTool.eraseCutters = !AreaSubtractTool.eraseCutters;
      this.refresh();
      return true;
    }
    return super.input(text);
  }

  protected stagePrompt(): string {
    return '';
  }
  protected point(): void {}

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    super.draw(g, view);
    const pal = this.ctx.view.palette;
    for (const f of this.from ?? []) drawArea(g, view, f.a, { color: pal.accent, width: 2 });
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
    const { doc, log, selection, format } = this.ctx;
    const created: number[] = [];
    const sizes: number[] = [];
    let changed = 0;
    doc.transact(this.label, () => {
      for (const t of this.list) {
        if (t.e.id === cutterId) continue;
        const pieces = splitArea(t.a, cut);
        if (pieces.length < 2) continue;
        // The first piece is the area itself, split: it keeps its slot and persistent id; the rest are new (ADR 0014).
        doc.replace(t.e.id, areaEntity(pieces[0], t.e));
        created.push(t.e.id, ...addAreas(this.ctx, pieces.slice(1), t.e));
        sizes.push(...pieces.map(netArea));
        changed++;
      }
    });
    if (!changed) {
      this.pts = [];
      this.refresh();
      return log.warn('Kesme çizgisi alanı baştan başa geçmiyor; çizgi alanın sınırını iki yerden kesmeli.');
    }
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
    const { doc, log, selection, format } = this.ctx;
    const closed = areasOf(targets.filter((e) => e.kind !== 'polygon'));
    const lines = targets.filter((e) => !areaOfEntity(e) && (e.kind === 'line' || e.kind === 'arc' || e.kind === 'polyline' || e.kind === 'spline' || e.kind === 'ellipse'));
    const faces = lines.length ? facesOf(lines) : [];
    if (!closed.length && !faces.length) {
      const already = targets.some((e) => e.kind === 'polygon');
      return log.warn(already ? 'Seçili nesneler zaten alan.' : 'Alana çevrilecek kapalı nesne ya da kapalı bölge oluşturan çizgi bulunamadı. Çizgilerin uçları birleşmeli ya da kesişmeli.');
    }
    const created: number[] = [];
    doc.transact(this.label, () => {
      for (const x of closed) {
        doc.remove([x.e.id]);
        created.push(...addAreas(this.ctx, [x.a], x.e));
      }
      // Line work stays; the regions it closes become new areas on its layer.
      if (faces.length) created.push(...addAreas(this.ctx, faces, lines[0], false));
    });
    selection.set(created);
    const parts = [closed.length ? `${closed.length} nesne alana çevrildi` : '', faces.length ? `çizgilerden ${faces.length} alan oluştu (${format.area(totalArea(faces))})` : ''].filter(Boolean);
    log.success(`${parts.join('; ')}.`);
  }
}

/** Areas back to closed polylines: the outer ring keeps the data, holes become plain polylines. */
export class ToPolylineTool extends SelectionActionTool {
  readonly id = 'toPolyline';
  protected readonly label = 'Çizgiye çevir';

  protected run(targets: Entity[]): void {
    const { doc, log, selection } = this.ctx;
    const polys = targets.filter((e) => e.kind === 'polygon');
    if (!polys.length) return log.warn('Çizgiye çevrilecek bir kapalı alan seçin.');
    const created: number[] = [];
    doc.transact(this.label, () => {
      for (const e of polys) {
        if (e.kind !== 'polygon') continue;
        doc.remove([e.id]);
        polylinesOfPolygon(e).forEach((g, i) =>
          created.push(doc.add({ ...g, layerId: e.layerId, color: e.color, attrs: i === 0 ? { ...e.attrs } : {}, label: i === 0 ? e.label : undefined } as NewEntity).id),
        );
      }
    });
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
    const layerId = writableLayer(ctx);
    if (!layerId) return;
    let id = 0;
    ctx.doc.transact('Alan oluştur', () => {
      id = ctx.doc.add({ ...polygonOfArea(area), layerId, color: ctx.settings.color.value ?? undefined, attrs: {} } as NewEntity).id;
    });
    ctx.selection.set([id]);
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

