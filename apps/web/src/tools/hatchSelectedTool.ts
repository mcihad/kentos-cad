import type { AppContext } from '../app/context';
import { entityBounds, polygonRing, type EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Area } from '../model/geom/region';
import { areasOfEntity } from '../model/ops/areas';
import { hatchCut, hatchPick, hatchSeed } from '../model/ops/hatchPatterns';
import type { ViewTransform } from '../viewport/Camera';
import { writeObjects } from './createCommand';
import { assocOf, cutoutsIn, islandsIn, leftOut, tooDense, type Tie } from './hatchTool';
import { askingText, chooseHatchOption, currentChoice, hatchAnswer, hatchChoicesOf, hatchOption, hatchPattern, hatchSession, patternOptions, regionOptions, typedChoice, type HatchAsking } from './hatchOptions';
import { SelectionFirstTool } from './modifyTools';
import { drawArea, tint } from './preview';
import type { OptionChoice } from './Tool';
import { netArea } from '../model/geom/region';

const LABEL = 'Çoklu tara';

/** A region to hatch: its part's area cut, the seed it was picked by and what it came from. */
interface Planned {
  area: Area;
  seed: Vec2;
  tie: Tie;
}

/**
 * Çoklu tara (docs/adr/0186 §4; Netcad's Çoklu Tara): selection first; each closed object of it (an area and each
 * part of a multi-part one, a circle, a full ellipse, a closed curve) is hatched by itself with Tarama's options, from
 * a point inside it (its seed), its islands left out when Adalar says so, its texts and inserts when Yazılar does,
 * following its objects when İlişkili is on. Enter writes them all in one step, “Tarama”; the regions show outlined
 * before. The desktop's is kentos_interaction's hatch_selected.rs.
 */
export class HatchSelectedTool extends SelectionFirstTool {
  readonly id = 'hatchSelected';
  protected readonly label = LABEL;
  // Nothing is placed by the cursor.
  override readonly snaps = false;
  private ids: number[] = [];
  private asking: HatchAsking | null = null;
  private plan: { key: string; regions: Planned[] } | null = null;

  constructor(ctx: AppContext) {
    super(ctx);
  }

  protected begin(): void {
    this.asking = null;
    this.plan = null;
    this.ids = [...this.ctx.selection.ids.value]
      .filter((id) => {
        const e = this.ctx.doc.get(id);
        return !!e && areasOfEntity(e).length > 0;
      })
      .sort((a, b) => a - b);
    if (this.ids.length) return;
    this.ctx.log.warn('Çoklu tara: seçimde kapalı nesne yok; kapalı alan, daire, tam elips ya da kapalı eğri seçin.');
    // Leaving from inside activate() would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }

  /** Every closed object's regions, each part by itself from a point inside it; kept until the drawing or the options change. */
  private regions(): Planned[] {
    const S = hatchSession;
    const key = `${this.ctx.doc.revision}|${S.islands}|${S.texts}|${this.ctx.doc.settings.drawingFont.value}`;
    if (this.plan?.key === key) return this.plan.regions;
    const regions: Planned[] = [];
    for (const id of this.ids) {
      const e = this.ctx.doc.get(id);
      if (!e) continue;
      for (const part of areasOfEntity(e)) {
        const seed = hatchSeed(part);
        if (!seed) continue;
        const islands = S.islands ? islandsIn(this.ctx, e, netArea(part)) : { ids: [], areas: [] };
        const cutouts = S.texts ? cutoutsIn(this.ctx, entityBounds(e)) : { ids: [], boxes: [] };
        const cut = hatchCut(part, islands.areas, cutouts.boxes);
        const k = hatchPick(cut.parts, seed);
        if (k === null) continue;
        regions.push({ area: cut.parts[k], seed, tie: { outer: id, islands: cut.islands.map((i) => islands.ids[i]), cutouts: cut.cutouts.map((i) => cutouts.ids[i]) } });
      }
    }
    this.plan = { key, regions };
    return regions;
  }

  protected override pickHint(): string {
    return `[${[...patternOptions(), ...regionOptions(true)].join(' / ')}]`;
  }

  protected stagePrompt(): string {
    return `${this.regions().length} bölge taranacak; Enter ya da sağ tıkla tarayın [${[...patternOptions(), ...regionOptions(true)].join(' / ')}]`;
  }

  protected override refresh(): void {
    if (!this.asking) return super.refresh();
    this.prompt.set(`${LABEL}: ${askingText(this.asking)}`);
    this.ctx.view.requestOverlay();
  }

  /** A click places nothing: Enter or a quick right click writes. */
  protected point(): void {}

  override input(text: string): boolean {
    if (this.asking) {
      if (hatchAnswer(this.ctx, this.asking, text)) this.asking = null;
      this.refresh();
      return true;
    }
    const o = hatchOption(text.trim().toLocaleUpperCase('tr-TR'), true);
    if (o.taken) this.asking = o.asking ?? null;
    else if (!typedChoice(text)) return false;
    this.refresh();
    return true;
  }

  override acceptPoint(): boolean {
    return false;
  }

  optionChoices(key: string): readonly OptionChoice[] | null {
    return hatchChoicesOf(key);
  }

  chooseOption(key: string, typed: string): boolean {
    const taken = chooseHatchOption(key, typed);
    if (taken && key === 'R') this.asking = null;
    this.refresh();
    return taken;
  }

  /** Esc while a value is asked keeps the old one; otherwise the tool leaves. */
  cancel(): boolean {
    if (!this.asking) return false;
    this.asking = null;
    this.refresh();
    return true;
  }

  /** Enter: the hatches written in one step and the tool leaves; while a value is asked, it keeps the old one. */
  override confirm(): void {
    if (this.asking) {
      this.asking = null;
      return this.refresh();
    }
    if (this.picking) return super.confirm();
    const { ctx } = this;
    const regions = this.regions();
    if (!regions.length) {
      ctx.log.warn('Çoklu tara: taranacak bölge kalmadı; adalar ya da yazılar alanların hepsini kaplıyor.');
      return ctx.tools.exit();
    }
    const pattern = hatchPattern(ctx);
    const objects: EntityGeometry[] = [];
    let islands = 0;
    let texts = 0;
    for (const r of regions) {
      const ring = polygonRing(r.area.outer);
      const holes = r.area.holes.map(polygonRing);
      if (tooDense(ring, holes, pattern)) return ctx.log.warn('Desen bu alanlar için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin.');
      islands += r.tie.islands.length;
      texts += r.tie.cutouts.length;
      const assoc = hatchSession.assoc ? assocOf(ctx, r.tie, r.seed) : undefined;
      objects.push({ kind: 'hatch', ring: ring.map((q) => ({ ...q })), ...(holes.length && { holes: holes.map((h) => h.map((q) => ({ ...q }))) }), pattern, ...(assoc && { assoc }) } as EntityGeometry);
    }
    if (!writeObjects(ctx, objects, 'hatch')) return;
    ctx.log.success(`${LABEL}: ${objects.length} ${currentChoice().name} tarama eklendi${leftOut({ outer: 0, islands: Array(islands).fill(0), cutouts: Array(texts).fill(0) }, 0)}`);
    ctx.tools.exit();
  }

  /** The regions to hatch, outlined dashed. */
  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    for (const r of this.regions()) drawArea(g, view, r.area, { color: pal.accent, dash: [4, 3], width: 1.5, fill: tint(pal.accent, 0.12) });
  }
}
