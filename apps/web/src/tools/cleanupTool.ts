import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { entityGeometry, type Entity } from '../model/entities';
import { cleanupFindings, type CleanupFindings } from '../model/ops/split';
import type { ViewTransform } from '../viewport/Camera';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { MAX_GHOSTS, SelectionFirstTool } from './modifyTools';
import { strokeGeometry } from './preview';
import { markVertices, outlinesOf, pathRings } from './reshapePreview';

/**
 * Çizimi temizle (docs/adr/0140): finds what a drawing carries twice or
 * empty, shows it, and Enter cleans it in one step.
 *
 * - Scope: the selection, or the whole drawing when nothing is selected
 *   (clicks and windows still change the selection while the tool runs).
 * - Found: objects repeated exactly on the same layer (to 1e-9 m; the first
 *   stays), objects with no length or no area, vertices repeated in a row.
 * - Objects on locked layers are left alone.
 *
 * The finding is the core's (`cleanupFindings`); it is written through
 * `cad.entities.edit` (operation `cleanup`): the repeats and empty objects
 * removed, the cleaned ones updated. Nothing is snapped and no tolerance is
 * widened (CLAUDE.md §23.3).
 */
export class CleanupTool extends SelectionFirstTool {
  readonly id = 'cleanup';
  protected readonly label = 'Çizimi temizle';
  private found: { scope: Entity[]; findings: CleanupFindings; locked: number; whole: boolean } | null = null;
  /** The finding last put in the log, so a pick that changes nothing does not repeat it. */
  private said = '';

  // The tool is always picking: what is selected is the scope, and Enter cleans.
  protected begin(): void {
    this.picking = true;
  }
  protected stagePrompt(): string {
    return '';
  }
  protected point(): void {}

  private scan(): void {
    const { doc, selection } = this.ctx;
    const whole = selection.size === 0;
    const all = whole ? [...doc.all()] : this.targets();
    const scope = all.filter((e) => !doc.layers.isLocked(e.layerId));
    const findings = scope.length ? cleanupFindings(scope) : { repeats: [], empty: [], cleaned: [], vertices: 0 };
    // An object goes or is cleaned, never both.
    const gone = new Set([...findings.repeats, ...findings.empty]);
    findings.cleaned = findings.cleaned.filter((e) => !gone.has(e.id));
    this.found = { scope, findings, locked: all.length - scope.length, whole };
  }

  /** The finding in words: "3 yinelenen, 1 boş nesne, 5 tekrarlanan köşe", or null when there is none. */
  private summary(): string | null {
    const f = this.found?.findings;
    if (!f) return null;
    const parts = [
      ...(f.repeats.length ? [`${f.repeats.length} yinelenen`] : []),
      ...(f.empty.length ? [`${f.empty.length} boş nesne`] : []),
      ...(f.vertices ? [`${f.vertices} tekrarlanan köşe`] : []),
    ];
    return parts.length ? parts.join(', ') : null;
  }

  override activate(): void {
    this.said = '';
    super.activate();
  }

  protected override refresh(): void {
    this.scan();
    const { log } = this.ctx;
    const f = this.found!;
    const summary = this.summary();
    const scope = f.whole ? 'bütün çizim' : `${this.ctx.selection.size} seçili nesne`;
    const locked = f.locked ? ` (${f.locked} nesne kilitli katmanda, atlandı)` : '';
    const key = `${scope}|${summary ?? ''}|${f.locked}`;
    if (key !== this.said) {
      this.said = key;
      const line = `Çizimi temizle: ${summary ? `${summary}. Enter ile temizleyin.` : `temizlenecek bir şey yok; yinelenen ya da boş nesne, tekrarlanan köşe bulunamadı${locked}.`}`;
      if (summary) log.info(line);
      else log.warn(line);
    }
    this.prompt.set(
      summary
        ? `Çizimi temizle: ${scope}: ${summary} [Temizle (Enter)]; nesnelere tıklayarak kapsamı daraltın`
        : `Çizimi temizle: ${scope}: temizlenecek bir şey yok; nesnelere tıklayarak kapsamı değiştirin`,
    );
    this.ctx.view.requestOverlay();
  }

  override confirm(): void {
    this.scan();
    const f = this.found!;
    if (!this.summary()) {
      this.ctx.log.warn('Çizimi temizle: temizlenecek bir şey yok; hiçbir şey değişmedi.');
      return this.ctx.tools.exit();
    }
    const { repeats, empty, cleaned, vertices } = f.findings;
    const { doc, log, selection } = this.ctx;
    const changes: EntityEdit[] = [
      ...[...repeats, ...empty].map((id): EntityEdit => ({ kind: 'remove', uid: uidOf(this.ctx, id) })),
      ...cleaned.map((e): EntityEdit => ({ kind: 'update', uid: uidOf(this.ctx, e), geometry: editGeometry(entityGeometry(e)) })),
    ];
    if (!writeEdit(this.ctx, 'cleanup', changes)) return;
    selection.retain((id) => !!doc.get(id));
    log.success(`Çizimi temizle: ${[...(repeats.length ? [`${repeats.length} yinelenen nesne silindi`] : []), ...(empty.length ? [`${empty.length} boş nesne silindi`] : []), ...(vertices ? [`${vertices} tekrarlanan köşe atıldı`] : [])].join(', ')}.`);
    this.ctx.tools.exit();
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    super.draw(g, view);
    if (!this.found) return;
    const { doc, view: v } = this.ctx;
    const pal = v.palette;
    const { repeats, empty, cleaned } = this.found.findings;
    // What goes: struck through in the warning colour; the empty ones by a ring where they lie.
    for (const id of repeats.slice(0, MAX_GHOSTS)) {
      const e = doc.get(id);
      if (e) strokeGeometry(g, view, entityGeometry(e), { color: pal.danger, dash: [6, 4], width: 3 });
    }
    for (const id of empty.slice(0, MAX_GHOSTS)) {
      const e = doc.get(id);
      const at = e && anchorOf(e);
      if (at) markVertices(g, view, [at], pal.danger, true);
    }
    for (const c of cleaned.slice(0, MAX_GHOSTS)) {
      const before = doc.get(c.id);
      if (!before) continue;
      strokeGeometry(g, view, entityGeometry(c), { color: pal.accent, width: 2.5 });
      markVertices(g, view, repeatedIn(before), pal.danger, true);
    }
  }
}

/** Where an object with no length or area lies, to mark it. */
function anchorOf(e: Entity) {
  const g = entityGeometry(e);
  const o = outlinesOf(g)[0]?.pts[0];
  return o ?? ('p' in g ? g.p : null);
}

/** The vertices of a path, an area's every part included, that repeat the one before them (the ones cleaning drops). */
export function repeatedIn(e: Entity) {
  return pathRings(e).flatMap((pts) => pts.filter((p, i) => i > 0 && Math.hypot(p.x - pts[i - 1].x, p.y - pts[i - 1].y) <= 1e-9));
}
