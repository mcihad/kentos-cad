import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Elevated } from '../model/ops/elevation';
import { vertexPoints, type VertexPoints } from '../model/ops/vertexPoints';
import { elevatedPaths } from '../product/elevation';
import { entitiesCreate } from '../product/entitiesCreate';
import type { ViewTransform } from '../viewport/Camera';
import { indexMark, ringMark } from './constructPreview';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { drawTag } from './preview';
import { askPointField, pointOptions, SurveyPointTool } from './surveyPointTool';

const LABEL = 'Köşelere nokta';
/** What is said when the selection has no vertices to take. */
const NOTHING = `${LABEL}: seçimde çizgi, çoklu çizgi ya da alan yok.`;

/** The kinds whose vertices it takes (docs/adr/0152 §5). */
const isSource = (e: Entity): boolean => e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon';

/**
 * Köşelere nokta (docs/adr/0152 §5; Netcad's Otomatik Nokta Üret, QGIS' Extract vertices): a named point at every
 * vertex of the selected lines, polylines and areas. The finding is the shared core's (`vertexPoints`): objects in the
 * drawing's order, an area's outer ring, its holes, then its other parts; a place two vertices share (1 µm) one point,
 * a place a point already holds passed over and counted. Names run from Nokta's Ad by Yazı's Artır, the code is Nokta's
 * Kod, the elevation the vertex's (docs/adr/0142). Selection first, as the modify tools; the points are shown with
 * their names, Ad (A) and Kod (K) are Nokta's; Enter, Uygula or a quick right click writes them in one step “Köşelere
 * nokta” through `cad.entities.create` on the active layer, and Nokta's Ad moves on past the last. The desktop's tool
 * is `kentos_interaction::vertex_points`; both play `fixtures/interaction/v1/vertex-points.json`.
 */
export class VertexPointsTool extends SelectionFirstTool {
  readonly id = 'vertexPoints';
  protected readonly label = LABEL;
  // Nothing is placed by the cursor.
  override readonly snaps = false;
  /** The objects it reads, taken when the selection is confirmed. */
  private ids = new Set<number>();
  /** A or K: its value is being typed in the field. */
  private typing = false;
  private plan: { key: string; result: VertexPoints } | null = null;

  protected begin(): void {
    this.typing = false;
    this.plan = null;
    this.ids = new Set(this.targets().filter(isSource).map((e) => e.id));
    if (this.ids.size) return;
    this.ctx.log.warn(NOTHING);
    // Leaving from inside activate() would race the manager; defer it.
    queueMicrotask(() => this.ctx.tools.exit());
  }

  /** The points for the drawing as it is, kept until the drawing or the first name change. */
  private current(): VertexPoints {
    const { doc } = this.ctx;
    const first = SurveyPointTool.next;
    const key = `${doc.revision}|${first}`;
    if (this.plan?.key === key) return this.plan.result;
    // In the drawing's order: the objects' vertices, and every point's place (hidden layers too: a place is taken).
    const objects: Elevated[][] = [];
    const existing: Vec2[] = [];
    for (const e of doc.all()) {
      if (e.kind === 'point') existing.push(e.p);
      else if (this.ids.has(e.id)) objects.push(elevatedPaths(e));
    }
    const result = vertexPoints(objects, existing, first || null);
    this.plan = { key, result };
    return result;
  }

  protected stagePrompt(): string {
    if (this.typing) return 'değeri yazın';
    return `${finding(this.current())} [${pointOptions()} / Uygula (Enter)]`;
  }

  /** A click places nothing: Enter, Uygula or a quick right click writes. */
  protected point(): void {}

  override input(text: string): boolean {
    if (this.picking || this.typing) return false;
    const key = text.trim().toLocaleUpperCase('tr-TR');
    if (key !== 'A' && key !== 'K') return false;
    this.typing = true;
    this.refresh();
    askPointField(this.ctx, key, this.hover, () => {
      this.typing = false;
      this.refresh();
    });
    return true;
  }

  override acceptPoint(): boolean {
    return false;
  }

  /** Enter: while picking, the selection is taken; then the points are written in one step and the tool leaves. */
  override confirm(): void {
    if (this.picking) return super.confirm();
    if (this.typing) return;
    const { doc, log, settings } = this.ctx;
    const { points, skipped, next } = this.current();
    if (!points.length) {
      log.warn(`${LABEL}: yazılacak nokta yok${skipped ? `; ${skipped} köşede zaten nokta var` : ''}.`);
      return this.ctx.tools.exit();
    }
    const color = settings.color.value;
    const code = SurveyPointTool.code;
    const objects = points.map((pt) => ({
      geometry: { kind: 'point', p: pt.p, ...(pt.z != null && { z: pt.z }) } as NewGeometry,
      ...(color !== null && { color }),
      ...(pt.name && { label: pt.name }),
      ...(code && { attrs: { Kod: code } }),
    }));
    const result = entitiesCreate.execute({ doc }, { layerId: doc.layers.active.value, objects, operation: 'vertexPoints' });
    if (result.status !== 'completed') {
      if ('error' in result) log.warn(result.error.message);
      return;
    }
    for (const w of result.warnings) log.warn(w.message);
    // Nokta goes on past the last name (docs/adr/0152 §5).
    if (SurveyPointTool.next && next) SurveyPointTool.next = next;
    const first = points[0].name;
    const last = points.at(-1)!.name;
    const names = first ? ` (${first === last ? first : `${first} – ${last}`})` : '';
    log.success(`${LABEL}: ${points.length} nokta eklendi${names}${skipped ? `; ${skipped} köşede zaten nokta vardı` : ''}.`);
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    const result = this.current();
    // As the modify tools' ghosts: a very large selection previews its first ones.
    for (const pt of result.points.slice(0, MAX_GHOSTS)) {
      // Round the selection's grip at the corner: 6 px clears its square.
      ringMark(g, view, pt.p, pal.accent, 6);
      if (pt.name) indexMark(g, view, pt.p, pt.name, pal.accent, pal.labelHalo);
    }
    if (this.hover) drawTag(g, view.worldToScreen(this.hover), tagLines(result), pal.accent, pal.labelHalo);
  }
}

/** `12 nokta (101 – 112); 2 köşede nokta var`: what would be written, then what is passed over. */
function finding(r: VertexPoints): string {
  if (!r.points.length) return r.skipped ? `yazılacak nokta yok; ${r.skipped} köşede zaten nokta var` : 'yazılacak nokta yok';
  const head = `${r.points.length} nokta`;
  return r.skipped ? `${head}; ${r.skipped} köşede zaten nokta var` : head;
}

/** The finding beside the cursor, a part a line, and what writes it. */
function tagLines(r: VertexPoints): string[] {
  const lines = finding(r).split('; ');
  return r.points.length ? [...lines, 'Enter: uygula'] : lines;
}
