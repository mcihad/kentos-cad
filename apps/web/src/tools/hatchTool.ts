import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import { Signal } from '../core/signal';
import { entityArea, entityBounds, polygonRing, type Entity, type EntityGeometry, type HatchAssoc, type HatchPattern } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import { hatchSegments } from '../model/geom/hatch';
import { insideArea, netArea, type Area } from '../model/geom/region';
import { areasOfEntity } from '../model/ops/areas';
import { hatchCut, hatchCutout, hatchPatternPieces, hatchTooDense, type HatchCut } from '../model/ops/hatchPatterns';
import type { ViewTransform } from '../viewport/Camera';
import { drawArea, strokePath, tint } from './preview';
import { writeObjects } from './createCommand';
import { askingText, chooseHatchOption, currentChoice, hatchAnswer, hatchChoicesOf, hatchOption, hatchPattern, hatchSession, patternOptions, regionOptions, typedChoice, type HatchAsking } from './hatchOptions';
import type { OptionChoice, Tool, ToolPointer } from './Tool';
import { VisibleFaces } from './visibleFaces';

/** The most lines and dots a preview draws (the desktop's `PREVIEW_PIECES`); the hatch itself draws them all. */
const PREVIEW_PIECES = 3000;

/** The objects a region came from: the closed object, the islands and the cutouts that reached in (kapalı nesne). */
export interface Tie {
  outer: number;
  islands: number[];
  cutouts: number[];
}

/** The texts and inserts shown in `r` and their boxes left open (docs/adr/0186 §4). */
export function cutoutsIn(ctx: AppContext, r: Bounds): { ids: number[]; boxes: Vec2[][] } {
  const ids: number[] = [];
  const boxes: Vec2[][] = [];
  const blocks = ctx.doc.blocks.value;
  for (const e of ctx.view.entitiesIn(r)) {
    if (e.kind !== 'text' && e.kind !== 'insert') continue;
    const box = hatchCutout(e, e.kind === 'insert' ? blocks : null, ctx.doc.settings.drawingFont.value);
    if (box) {
      ids.push(e.id);
      boxes.push(box);
    }
  }
  return { ids, boxes };
}

/**
 * The closed objects inside or across the boundary object, smaller than it (a block around a parcel is not an
 * island); every part of a multi-part object is one (docs/adr/0143).
 */
export function islandsIn(ctx: AppContext, boundary: Entity, size: number): { ids: number[]; areas: Area[][] } {
  const ids: number[] = [];
  const areas: Area[][] = [];
  for (const e of ctx.view.entitiesIn(entityBounds(boundary))) {
    if (e.id === boundary.id || e.kind === 'hatch') continue;
    const own = areasOfEntity(e).filter((a) => netArea(a) < size * (1 - 1e-9));
    if (own.length) {
      ids.push(e.id);
      areas.push(own);
    }
  }
  return { ids, areas };
}

/** A tie in the contract's terms: the objects' persistent ids, the seed. */
export function assocOf(ctx: AppContext, tie: Tie, seed: Vec2): HatchAssoc | undefined {
  const uid = (id: number) => ctx.doc.uidOf(id);
  const outer = uid(tie.outer);
  if (!outer) return undefined;
  const islands = tie.islands.flatMap((id) => uid(id) ?? []);
  const cutouts = tie.cutouts.flatMap((id) => uid(id) ?? []);
  return { outer, ...(islands.length && { islands }), ...(cutouts.length && { cutouts }), seed: { x: seed.x, y: seed.y } };
}

/** What a hatch left out, for its message: the islands and the texts that reached in (kapalı nesne), else the holes. */
export function leftOut(tie: Tie | null, holes: number): string {
  const islands = tie ? tie.islands.length : holes;
  const texts = tie ? tie.cutouts.length : 0;
  return `${islands ? `, ${islands} ada taranmadı` : ''}${texts ? `, ${texts} yazı boş bırakıldı` : ''}`;
}

/** Whether a pattern is too dense over a ring: a user-defined one's lines (ADR 0062's), a pattern's families. */
export function tooDense(ring: Vec2[], holes: Vec2[][], pattern: HatchPattern): boolean {
  if (pattern.type === 'lines' || pattern.type === 'cross') {
    if (hatchSegments(ring, pattern.angle, pattern.spacing, holes)[0] === 1) return true;
    if (pattern.type === 'cross' && hatchSegments(ring, pattern.angle + 90, pattern.spacing, holes)[0] === 1) return true;
  }
  return hatchTooDense(ring, pattern);
}

/** An area's box. */
function areaBounds(a: Area): Bounds {
  const pts = polygonRing(a.outer);
  return { minX: Math.min(...pts.map((p) => p.x)), minY: Math.min(...pts.map((p) => p.y)), maxX: Math.max(...pts.map((p) => p.x)), maxY: Math.max(...pts.map((p) => p.y)) };
}

/**
 * Tarama (docs/adr/0062, 0186): click inside, the region is filled. Two ways to find the region:
 *   kapalı nesne (default, Netcad): the smallest closed object around the click (of a multi-part area, the part the
 *     click is in, docs/adr/0143); closed objects inside it or across its edge (buildings in a parcel) become islands
 *     left unhatched; with İlişkili on, the hatch follows its objects (`assoc`, docs/adr/0186 §6);
 *   çizgiler (AutoCAD): the face closed by the visible line work, groups inside it as islands; the boundary set can
 *     be one layer. It follows nothing.
 * Yazılar leaves the texts and inserts in the region open. The pattern's options are hatchOptions.ts' (Desen with its
 * menu, Ölçek, Açı, İkinci renk, Ters). The desktop's is kentos_interaction's hatch.rs.
 */
export class HatchTool implements Tool {
  readonly id = 'hatch';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private readonly faces: VisibleFaces;
  private pickingLayer = false;
  private asking: HatchAsking | null = null;
  /** Kapalı nesne: the object's part cut by what reaches in, kept while the drawing and the options stay as they were. */
  private cache: { key: string; cut: HatchCut; islands: number[]; cutouts: number[] } | null = null;
  private sub: Disposable | null = null;
  private hover: Area | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.faces = new VisibleFaces(ctx);
  }

  activate(): void {
    this.faces.attach();
    this.sub = this.ctx.doc.events.on('changed', () => (this.cache = null));
    this.refresh();
  }

  deactivate(): void {
    this.faces.detach();
    this.sub?.();
    this.sub = null;
    this.cache = null;
    this.ctx.selection.hover.set(null);
  }

  private refresh(): void {
    const S = hatchSession;
    if (this.pickingLayer) {
      this.prompt.set('Tarama: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]');
    } else if (this.asking) {
      this.prompt.set(`Tarama: ${askingText(this.asking)}`);
    } else {
      const layer = this.faces.layer ? (this.ctx.doc.layers.get(this.faces.layer)?.name ?? '—') : 'tümü';
      const opts = [...patternOptions(), `Sınır (B): ${S.byLines ? 'çizgiler' : 'kapalı nesne'}`, ...regionOptions(!S.byLines), ...(S.byLines ? [`Sınır katmanı (K): ${layer}`] : [])];
      this.prompt.set(`Tarama: taranacak yerin içine tıklayın [${opts.join(' / ')}]`);
    }
    this.ctx.view.requestOverlay();
  }

  /** The region to fill around p and, kapalı nesne, the objects it came from; null when there is none. */
  private region(p: Vec2): { area: Area; tie: Tie | null } | null {
    const S = hatchSession;
    const font = this.ctx.doc.settings.drawingFont.value;
    if (S.byLines) {
      const face = this.faces.at(p, S.islands);
      if (!face || !S.texts) return face ? { area: face, tie: null } : null;
      const { boxes } = cutoutsIn(this.ctx, areaBounds(face));
      const area = hatchCut(face, [], boxes).parts.find((a) => insideArea(a, p));
      return area ? { area, tie: null } : null;
    }
    const r = this.ctx.view.enclosingRing(p);
    if (!r) return null;
    const areas = areasOfEntity(r.entity);
    const part = areas.length === 1 ? 0 : areas.findIndex((a) => insideArea(a, p));
    if (part < 0) return null;
    const base = areas[part];
    const key = `${this.ctx.doc.revision}|${r.entity.id}|${part}|${S.islands}|${S.texts}|${font}`;
    if (this.cache?.key !== key) {
      const islands = S.islands ? islandsIn(this.ctx, r.entity, netArea(base)) : { ids: [], areas: [] };
      const cutouts = S.texts ? cutoutsIn(this.ctx, entityBounds(r.entity)) : { ids: [], boxes: [] };
      this.cache = { key, cut: hatchCut(base, islands.areas, cutouts.boxes), islands: islands.ids, cutouts: cutouts.ids };
    }
    const c = this.cache;
    const area = c.cut.parts.find((a) => insideArea(a, p));
    if (!area) return null;
    return { area, tie: { outer: r.entity.id, islands: c.cut.islands.map((k) => c.islands[k]), cutouts: c.cut.cutouts.map((k) => c.cutouts[k]) } };
  }

  pointerMove(p: ToolPointer): void {
    if (this.pickingLayer) {
      this.ctx.selection.hover.set(this.ctx.view.pick(p.screen)?.id ?? null);
      return;
    }
    this.hover = this.region(p.raw)?.area ?? null;
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
    this.asking = null;
    const found = this.region(p.raw);
    if (!found) {
      this.refresh();
      return ctx.log.warn(
        hatchSession.byLines
          ? 'Tıklanan yer çizgilerle kapalı bir bölgenin içinde değil; görünüm dışındaki çizgiler sayılmaz.'
          : 'Tıklanan noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için “Sınır: çizgiler” seçin.',
      );
    }
    const ring = polygonRing(found.area.outer);
    const holes = found.area.holes.map(polygonRing);
    const pattern = hatchPattern(ctx);
    if (tooDense(ring, holes, pattern)) {
      this.refresh();
      return ctx.log.warn('Desen bu alan için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin.');
    }
    const assoc = found.tie && hatchSession.assoc ? assocOf(ctx, found.tie, p.raw) : undefined;
    // Through `cad.entities.create` (docs/adr/0062): the active layer and the current colour, one step named
    // “Tarama”; the locked and hidden layer answers are the command's (the tools' own words).
    const geometry = { kind: 'hatch', ring: ring.map((q) => ({ ...q })), ...(holes.length && { holes: holes.map((h) => h.map((q) => ({ ...q }))) }), pattern, ...(assoc && { assoc }) } as EntityGeometry;
    const name = currentChoice().name;
    const out = writeObjects(ctx, [geometry], 'hatch');
    const hatch = out && ctx.doc.get(out.ids[0]);
    this.cache = null;
    this.refresh();
    if (!hatch) return;
    ctx.log.success(`${name} tarama eklendi: ${ctx.format.area(entityArea(hatch) ?? 0)}${leftOut(found.tie, holes.length)}`);
  }

  input(text: string): boolean {
    if (this.asking) {
      const taken = hatchAnswer(this.ctx, this.asking, text);
      if (taken) this.asking = null;
      this.hover = null;
      this.refresh();
      return true;
    }
    const t = text.trim().toLocaleUpperCase('tr-TR');
    const S = hatchSession;
    if (t === 'B') {
      S.byLines = !S.byLines;
      this.pickingLayer = false;
    } else if (t === 'K' && (S.byLines || this.pickingLayer)) {
      if (this.pickingLayer || this.faces.layer) {
        this.faces.setLayer(null);
        this.pickingLayer = false;
      } else this.pickingLayer = true;
      this.ctx.selection.hover.set(null);
    } else {
      const o = hatchOption(t, !S.byLines);
      if (o.taken) {
        this.asking = o.asking ?? null;
        this.cache = null;
      } else if (!typedChoice(text)) return false;
    }
    this.hover = null;
    this.refresh();
    return true;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    return hatchChoicesOf(key);
  }

  chooseOption(key: string, typed: string): boolean {
    const taken = chooseHatchOption(key, typed);
    if (taken && key === 'R') this.asking = null;
    this.hover = null;
    this.refresh();
    return taken;
  }

  /** Enter leaves the tool; while a value is asked, it keeps the old one. */
  confirm(): void {
    if (this.asking) {
      this.asking = null;
      return this.refresh();
    }
    this.ctx.tools.exit();
  }

  /** Esc leaves what is asked first, then the layer picking. */
  cancel(): boolean {
    if (this.asking) {
      this.asking = null;
      this.refresh();
      return true;
    }
    if (!this.pickingLayer) return false;
    this.pickingLayer = false;
    this.ctx.selection.hover.set(null);
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.hover || this.pickingLayer) return;
    const pal = this.ctx.view.palette;
    const kind = currentChoice().kind;
    const filled = kind === 'solid' || kind === 'gradient';
    drawArea(g, view, this.hover, { color: pal.accent, dash: [4, 3], width: 1.5, fill: filled ? tint(pal.accent, 0.25) : undefined });
    if (filled) return;
    const pieces = hatchPatternPieces(polygonRing(this.hover.outer), this.hover.holes.map(polygonRing), hatchPattern(this.ctx), PREVIEW_PIECES);
    if (pieces.capped) return; // preview only; the real hatch is drawn on the GPU
    g.save();
    g.globalAlpha = 0.6;
    for (const [a, b] of pieces.segments) strokePath(g, view, [a, b], { color: pal.accent });
    g.fillStyle = pal.accent;
    for (const q of pieces.dots) {
      const s = view.worldToScreen(q);
      g.fillRect(s.x - 0.75, s.y - 0.75, 1.5, 1.5);
    }
    g.restore();
  }
}
