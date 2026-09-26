import type { AppContext } from '../app/context';
import type { Disposable } from '../core/disposable';
import { Signal } from '../core/signal';
import { entityArea, entityBounds, HATCH_PATTERN_LABEL, polygonRing, type Entity, type EntityGeometry, type HatchPattern } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { hatchSegments } from '../model/geom/hatch';
import { insideArea, netArea, subtractAreas, type Area } from '../model/geom/region';
import { areaOfEntity } from '../model/ops/areas';
import type { ViewTransform } from '../viewport/Camera';
import { drawArea, strokePath, tint } from './preview';
import { writeObjects } from './createCommand';
import type { Tool, ToolPointer } from './Tool';
import { VisibleFaces } from './visibleFaces';

const PRESETS: { name: string; type: HatchPattern['type']; angle: number; mm: number }[] = [
  { name: 'Çizgili 45°', type: 'lines', angle: 45, mm: 3 },
  { name: 'Çapraz 45°', type: 'cross', angle: 45, mm: 3 },
  { name: 'Yatay çizgili', type: 'lines', angle: 0, mm: 2 },
  { name: 'Dolu', type: 'solid', angle: 0, mm: 3 },
];

/** Paper sizes (mm) converted to world metres at the project's plot scale. */
const paper = (ctx: AppContext, mm: number) => (mm / 1000) * ctx.doc.settings.plotScale.value;

/**
 * Tarama: click inside, the region is filled (not associative). Two ways
 * to find the region:
 *   kapalı nesne (default, Netcad): the smallest closed object around the
 *     click; closed objects inside it or across its edge (buildings in a
 *     parcel) become islands left unhatched;
 *   çizgiler (AutoCAD): the face closed by the visible line work, groups
 *     inside it as islands; the boundary set can be one layer.
 * Islands can be switched off (A).
 */
export class HatchTool implements Tool {
  readonly id = 'hatch';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private static preset = 0;
  private static byLines = false;
  private static islands = true;
  private readonly ctx: AppContext;
  private readonly faces: VisibleFaces;
  private pickingLayer = false;
  /** Object mode: the enclosing object's region with islands cut out, kept while the cursor stays in it. */
  private cache: { id: number; parts: Area[] } | null = null;
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
    const S = HatchTool;
    if (this.pickingLayer) {
      this.prompt.set('Tarama: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]');
    } else {
      const layer = this.faces.layer ? (this.ctx.doc.layers.get(this.faces.layer)?.name ?? '—') : 'tümü';
      const opts = [
        `Desen (D): ${PRESETS[S.preset].name}`,
        `Sınır (B): ${S.byLines ? 'çizgiler' : 'kapalı nesne'}`,
        `Adalar (A): ${S.islands ? 'taranmaz' : 'taranır'}`,
        ...(S.byLines ? [`Sınır katmanı (K): ${layer}`] : []),
      ];
      this.prompt.set(`Tarama: taranacak yerin içine tıklayın [${opts.join(' / ')}]`);
    }
    this.ctx.view.requestOverlay();
  }

  private pattern(): HatchPattern {
    const p = PRESETS[HatchTool.preset];
    return { type: p.type, angle: p.angle, spacing: paper(this.ctx, p.mm) };
  }

  /** The region to fill around p, or null. */
  private region(p: Vec2): Area | null {
    const S = HatchTool;
    if (S.byLines) return this.faces.at(p, S.islands);
    const r = this.ctx.view.enclosingRing(p);
    const base = r && areaOfEntity(r.entity);
    if (!r || !base) return null;
    if (!S.islands) return base;
    if (this.cache?.id !== r.entity.id) this.cache = { id: r.entity.id, parts: subtractAreas([base], this.islandsOf(r.entity, netArea(base))) };
    return this.cache.parts.find((a) => insideArea(a, p)) ?? null;
  }

  /** Closed objects inside or across the boundary object, smaller than it (a block around a parcel is not an island). */
  private islandsOf(boundary: Entity, size: number): Area[] {
    return this.ctx.view.entitiesIn(entityBounds(boundary)).flatMap((e) => {
      if (e.id === boundary.id || e.kind === 'hatch') return [];
      const a = areaOfEntity(e);
      return a && netArea(a) < size * (1 - 1e-9) ? [a] : [];
    });
  }

  pointerMove(p: ToolPointer): void {
    if (this.pickingLayer) {
      this.ctx.selection.hover.set(this.ctx.view.pick(p.screen)?.id ?? null);
      return;
    }
    this.hover = this.region(p.raw);
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
    const area = this.region(p.raw);
    if (!area) {
      return ctx.log.warn(
        HatchTool.byLines
          ? 'Tıklanan yer çizgilerle kapalı bir bölgenin içinde değil; görünüm dışındaki çizgiler sayılmaz.'
          : 'Tıklanan noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için “Sınır: çizgiler” seçin.',
      );
    }
    const ring = polygonRing(area.outer);
    const holes = area.holes.map(polygonRing);
    const pattern = this.pattern();
    if (pattern.type !== 'solid' && hatchSegments(ring, pattern.angle, pattern.spacing, holes)[0] === 1) {
      return ctx.log.warn('Desen bu alan için çok sık; çizim ölçeğini büyütün ya da başka bir desen seçin.');
    }
    // Through `cad.entities.create` (docs/adr/0062): the active layer and the current colour, one step
    // named “Tarama”; the locked and hidden layer answers are the command's (the tools' own words).
    const geometry = { kind: 'hatch', ring: ring.map((q) => ({ ...q })), ...(holes.length && { holes: holes.map((h) => h.map((q) => ({ ...q }))) }), pattern } as EntityGeometry;
    const out = writeObjects(ctx, [geometry], 'hatch');
    const hatch = out && ctx.doc.get(out.ids[0]);
    if (!hatch) return;
    ctx.log.success(`${HATCH_PATTERN_LABEL[pattern.type]} tarama eklendi: ${ctx.format.area(entityArea(hatch) ?? 0)}${holes.length ? `, ${holes.length} ada taranmadı` : ''}`);
  }

  input(text: string): boolean {
    const t = text.trim().toLocaleUpperCase('tr-TR');
    const S = HatchTool;
    if (t === 'D') S.preset = (S.preset + 1) % PRESETS.length;
    else if (t === 'B') {
      S.byLines = !S.byLines;
      this.pickingLayer = false;
    } else if (t === 'A') {
      S.islands = !S.islands;
      this.cache = null;
    } else if (t === 'K' && (S.byLines || this.pickingLayer)) {
      if (this.pickingLayer || this.faces.layer) {
        this.faces.setLayer(null);
        this.pickingLayer = false;
      } else this.pickingLayer = true;
      this.ctx.selection.hover.set(null);
    } else return false;
    this.hover = null;
    this.refresh();
    return true;
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
    if (!this.hover || this.pickingLayer) return;
    const pal = this.ctx.view.palette;
    const pat = this.pattern();
    drawArea(g, view, this.hover, { color: pal.accent, dash: [4, 3], width: 1.5, fill: pat.type === 'solid' ? tint(pal.accent, 0.25) : undefined });
    if (pat.type === 'solid') return;
    // [capped, ax, ay, bx, by, …]: no points are made for a preview that is skipped.
    const xy = hatchSegments(polygonRing(this.hover.outer), pat.angle, pat.spacing, this.hover.holes.map(polygonRing));
    if (xy.length > 1 + 4 * 3000) return; // preview only; the real hatch is drawn on the GPU
    g.save();
    g.globalAlpha = 0.6;
    for (let k = 1; k + 3 < xy.length; k += 4)
      strokePath(g, view, [{ x: xy[k], y: xy[k + 1] }, { x: xy[k + 2], y: xy[k + 3] }], { color: pal.accent });
    g.restore();
  }
}
