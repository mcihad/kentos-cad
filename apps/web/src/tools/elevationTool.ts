import type { EntityEdit } from '../contracts/generated/EntityEdit';
import { hasVertexElevation, mapElevations, takesElevation } from '../product/elevationValues';
import { parseNumber } from './coordinateInput';
import { uidOf, writeEdit } from './editCommand';
import { SelectionFirstTool } from './modifyTools';

/**
 * Kot ver (docs/adr/0142): gives the vertices of the selected lines, polylines, areas (holes too) and points
 * their elevations, in metres. Objects are picked before or after (a selection skips the step); then a number
 * typed writes:
 *
 * - Sabit: every vertex gets that value, and a point its `z`;
 * - Artır (A): every vertex that has an elevation gets it plus the difference; one without stays without, a
 *   point with a `z` is raised;
 * - Sıfırla (S) writes at once: every vertex without an elevation (not 0), a point without `z`.
 *
 * One `cad.entities.edit` (operation `elevation`), one undo step, one update per object with its own geometry
 * and the elevations explicit; the command refuses objects on locked layers as a whole. After writing the tool is
 * back at picking with nothing selected. The steps and messages are the words the desktop says too.
 */

/** The message when nothing selected can hold an elevation. */
const NONE_TAKES = 'Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.';

/** What a number typed does to each vertex. */
type Change = { kind: 'set'; z: number | null } | { kind: 'raise'; by: number };

/** A difference as the messages say it: with its sign, the minus a real one (U+2212). */
const signed = (by: number, length: (m: number) => string) => `${by < 0 ? '−' : '+'}${length(Math.abs(by))}`;

export class ElevationTool extends SelectionFirstTool {
  readonly id = 'setElevation';
  protected readonly label = 'Kot ver';
  /**
   * Artır: the number typed is a difference to add, not the elevation. Kept for this run only, so the ribbon's
   * methods (Sabit, Artır, Sıfırla) each start the tool the way they say.
   */
  private raise = false;

  // The selection is confirmed: it must hold something that takes an elevation.
  protected begin(): void {
    if (!this.targets().some(takesElevation)) {
      this.ctx.log.warn(NONE_TAKES);
      this.picking = true;
    }
  }

  protected stagePrompt(): string {
    return this.raise ? 'eklenecek farkı yazın (m)' : 'kotu yazın (m)';
  }

  protected point(): void {}

  protected override refresh(): void {
    const chips = `[Artır (A)${this.raise ? ': açık' : ''} / Sıfırla (S)]`;
    this.prompt.set(`${this.label}: ${this.picking ? 'kot verilecek nesneleri seçin' : this.stagePrompt()} ${chips}`);
    this.ctx.view.requestOverlay();
  }

  override input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    if (key === 'A') {
      this.raise = !this.raise;
      this.refresh();
      return true;
    }
    if (key === 'S') {
      this.reset();
      return true;
    }
    if (this.picking) return false;
    const n = parseNumber(text);
    if (n === null || /[,;@<]/.test(text)) return false;
    this.write(this.raise ? { kind: 'raise', by: n } : { kind: 'set', z: n });
    return true;
  }

  // A point from the point calculator means nothing here: only a number does.
  override acceptPoint(): boolean {
    return false;
  }

  /** Sıfırla: with objects picked and not yet confirmed, they are the ones. */
  private reset(): void {
    if (this.picking) {
      if (!this.ctx.selection.size) return void this.ctx.log.warn('Önce kotu silinecek nesneleri seçin.');
      this.picking = false;
      this.ctx.selection.hover.set(null);
    }
    this.write({ kind: 'set', z: null });
  }

  /** One step back: from the number to the picking of objects (the selection stays); from picking out of the tool. */
  cancel(): boolean {
    if (this.picking) return false;
    this.picking = true;
    this.hover = null;
    this.refresh();
    return true;
  }

  private write(change: Change): void {
    const { doc, log, selection, format } = this.ctx;
    const all = this.targets();
    const takers = all.filter(takesElevation);
    if (!takers.length) return void log.warn(NONE_TAKES);
    const open = takers.filter((e) => !doc.layers.isLocked(e.layerId));
    // Only locked objects: the command refuses them with its own message, and nothing is written.
    const chosen = !open.length ? takers : change.kind === 'raise' ? open.filter(hasVertexElevation) : open;
    if (!chosen.length) return void log.warn('Seçili nesnelerin hiçbir köşesinde kot yok; fark eklenecek bir şey bulunamadı.');
    const f = change.kind === 'raise' ? (z: number | null) => (z === null ? null : z + change.by) : () => change.z;
    const changes = chosen.map((e): EntityEdit => ({ kind: 'update', uid: uidOf(this.ctx, e), geometry: mapElevations(e, f)! }));
    if (!writeEdit(this.ctx, 'elevation', changes)) return;
    const n = chosen.length;
    log.info(change.kind === 'raise' ? `Kotlar ${signed(change.by, (m) => format.length(m))} değişti: ${n} nesne.` : change.z === null ? `Kot silindi: ${n} nesne.` : `Kot verildi: ${n} nesne.`);
    if (all.length > takers.length) log.warn(`${all.length - takers.length} nesne kot almaz (yalnız çizgi, çoklu çizgi, alan ve nokta).`);
    if (open.length < takers.length) log.warn(`${takers.length - open.length} nesne kilitli katmanda olduğu için atlandı.`);
    // Done: back at the first step with nothing selected, for the next objects.
    selection.clear();
    this.picking = true;
    this.hover = null;
    this.refresh();
  }
}
