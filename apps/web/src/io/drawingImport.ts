import type { Bounds } from '../contracts/generated/Bounds';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { readEntityList } from '../model/snapshot';
import { makeLayers, prepareImport, unusable, type ImportPlan, type Prepared } from './apply';
import type { ImportedDrawing } from './client';
import { ColumnsReader } from './columns';

/**
 * A drawing a reader imported (DXF, Netcad NCZ; docs/adr/0138) going into
 * the drawing. A small one goes in at once (`importedEntities`, then
 * `applyImport`); a large one a slice of time at a time (`ProgressiveImport`),
 * so the page keeps drawing and the objects appear as they go in, yet as ONE
 * undo step: its layers and every object are made inside a document group,
 * which `stop` reverts whole. The desktop does the same with the same
 * numbers (apps/desktop/src/exchange/apply.rs `Progressive`).
 */

/** Up to this many objects go in at once; more go in a slice of time at a time. */
export const AT_ONCE = 20_000;

/** Objects checked and added between two looks at the clock. */
const BATCH = 2048;

/** The objects of the chosen layers, all of them (a small import). */
export function importedEntities(d: ImportedDrawing, chosen: ReadonlySet<string>): ContractEntity[] {
  const r = new ColumnsReader(d.columns);
  const out: ContractEntity[] = [];
  for (let i = 0; i < r.count; i++) {
    // Not in a drawing yet: the persistent id the columns carry is a placeholder.
    const { uid: _placeholder, ...e } = r.next();
    if (chosen.has(e.layerId)) out.push(e as ContractEntity);
  }
  return out;
}

/**
 * Where the view shows the chosen layers (their boxes, `ImportLayer.bounds`):
 * their extent inside the import's view, so a stray on a chosen layer is left
 * out as it is from the whole; their whole extent when the chosen layers hold
 * only what lies beyond it (`kentos_formats::import::view_of`, on four numbers).
 */
export function viewOf(view: Bounds | undefined, chosen: Iterable<Bounds>): Bounds | null {
  let all: Bounds | null = null;
  for (const b of chosen)
    all = all ? { minX: Math.min(all.minX, b.minX), minY: Math.min(all.minY, b.minY), maxX: Math.max(all.maxX, b.maxX), maxY: Math.max(all.maxY, b.maxY) } : { ...b };
  if (!all || !view) return all;
  const inside = { minX: Math.max(all.minX, view.minX), minY: Math.max(all.minY, view.minY), maxX: Math.min(all.maxX, view.maxX), maxY: Math.min(all.maxY, view.maxY) };
  return inside.minX <= inside.maxX && inside.minY <= inside.maxY ? inside : all;
}

/** A large import going into the drawing a slice at a time, as one undo step. */
export class ProgressiveImport {
  /** The objects the chosen layers hold. */
  readonly total: number;
  /** The layers made for the import, by name. */
  readonly created: readonly string[];
  private readonly doc: CadDocument;
  private readonly label: string;
  private readonly prepared: Prepared;
  private readonly reader: ColumnsReader;
  private readonly group: { end(): void; cancel(): void };
  private read = 0;
  private written = 0;
  private over = false;

  private constructor(doc: CadDocument, d: ImportedDrawing, plan: ImportPlan, prepared: Prepared) {
    this.doc = doc;
    this.label = plan.label;
    this.prepared = prepared;
    this.reader = new ColumnsReader(d.columns);
    this.total = d.result.layers.reduce((n, l) => n + (prepared.targets.has(l.name) ? l.count : 0), 0);
    this.group = doc.beginGroup(plan.label);
    try {
      this.created = doc.transact(plan.label, () => makeLayers(doc, plan, prepared));
    } catch (e) {
      this.group.cancel();
      throw e;
    }
  }

  /** Opens the group and makes the layers; the reason in words when the import cannot go in. */
  static start(doc: CadDocument, d: ImportedDrawing, plan: ImportPlan): ProgressiveImport | { error: string } {
    const prepared = prepareImport(doc, plan);
    if ('error' in prepared) return prepared;
    try {
      return new ProgressiveImport(doc, d, plan, prepared);
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) };
    }
  }

  /** Objects written into the drawing so far. */
  get done(): number {
    return this.written;
  }

  /** How far the import is (0–1). */
  get share(): number {
    return this.total ? this.written / this.total : 1;
  }

  /**
   * Writes objects for about `budgetMs`: `more` while some are left, `done`
   * once every one is in (the group ends: one undo step), or the reason the
   * import stopped, with everything it made taken back.
   */
  step(budgetMs: number): 'more' | 'done' | { error: string } {
    if (this.over) return 'done';
    const start = performance.now();
    const { targets, valid } = this.prepared;
    const r = this.reader;
    try {
      while (this.read < r.count) {
        const chunk: unknown[] = [];
        while (chunk.length < BATCH && this.read < r.count) {
          const { uid: _placeholder, ...e } = r.next();
          this.read++;
          const layerId = targets.get(e.layerId);
          if (layerId) chunk.push({ ...e, layerId, id: chunk.length + 1 });
        }
        if (chunk.length) {
          const checked = readEntityList(chunk, valid, 'İçe aktarılan nesne', this.written);
          if (!checked.ok) return this.fail(unusable(checked.error));
          this.written += this.doc.transact(this.label, () => this.doc.addMany(checked.entities as unknown as NewEntity[], this.label)).length;
        }
        if (performance.now() - start >= budgetMs) return 'more';
      }
      if (!r.done) return this.fail(unusable('nesnelerden sonra fazladan değer var'));
    } catch (e) {
      return this.fail(unusable(e instanceof Error ? e.message : String(e)));
    }
    this.over = true;
    this.group.end();
    return 'done';
  }

  /** Durdur: what the import made goes, and nothing is recorded. */
  stop(): void {
    if (this.over) return;
    this.over = true;
    this.group.cancel();
  }

  private fail(error: string): { error: string } {
    this.stop();
    return { error };
  }
}
