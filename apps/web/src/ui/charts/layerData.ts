import type { AppContext } from '../../app/context';
import { layerColumns } from '../../model/chartData';
import { ChartData } from '../../model/ops/charts';
import type { TableShow } from '../../model/ops/featureTable';

/**
 * A layer's data for the İstatistik and Grafik tabs (docs/adr/0227 §2): its objects in the drawing's order (those its
 * filter keeps, as the Tablo tab shows them), their columns and their geometry's measures, in the core once. Rows are
 * the objects' places; the tabs ask by rows and select by ids.
 */
export class LayerData {
  readonly layer: string;
  readonly ids: readonly number[];
  readonly data: ChartData;
  private readonly at: Map<number, number>;

  private constructor(layer: string, ids: number[], data: ChartData) {
    this.layer = layer;
    this.ids = ids;
    this.data = data;
    this.at = new Map(ids.map((id, i) => [id, i]));
  }

  static of(ctx: AppContext, layer: string): LayerData {
    const { doc, view } = ctx;
    const entities = doc.byLayer(layer).filter((e) => view.geometry.filterShown(e.id));
    const ids = entities.map((e) => e.id);
    const columns = layerColumns(doc.layers.get(layer)?.fields ?? [], entities);
    return new LayerData(layer, ids, new ChartData(ids.length, columns, view.geometry.measures(ids)));
  }

  /** Göster's rows: every object, the selected ones or the ones in the view. */
  rows(ctx: AppContext, show: TableShow): Uint32Array {
    if (show === 'all') return Uint32Array.from(this.ids.keys());
    if (show === 'selected') return this.selectedRows(ctx);
    const visible = new Set(ctx.view.inBox(ctx.view.camera.visibleBounds()));
    const out: number[] = [];
    this.ids.forEach((id, i) => visible.has(id) && out.push(i));
    return Uint32Array.from(out);
  }

  /** The rows of the drawing's selected objects (in the rows' order). */
  selectedRows(ctx: AppContext): Uint32Array {
    const out: number[] = [];
    for (const id of ctx.selection.ids.value) {
      const r = this.at.get(id);
      if (r !== undefined) out.push(r);
    }
    return Uint32Array.from(out.sort((a, b) => a - b));
  }

  idsOf(rows: ArrayLike<number>): number[] {
    return Array.from(rows, (r) => this.ids[r]).filter((id) => id !== undefined);
  }

  free(): void {
    this.data.free();
  }
}

/** A click's way with the drawing's selection: replace, add (Shift) or take away (Ctrl, ⌘). */
export function applyPick(ctx: AppContext, ids: readonly number[], e: { shiftKey: boolean; ctrlKey: boolean; metaKey: boolean }): void {
  const current = ctx.selection.ids.value;
  if (e.ctrlKey || e.metaKey) {
    const drop = new Set(ids);
    ctx.selection.set([...current].filter((id) => !drop.has(id)));
  } else if (e.shiftKey) ctx.selection.set([...new Set([...current, ...ids])]);
  else ctx.selection.set([...ids]);
}

/** The layer the tabs show: the one chosen while it is a layer, else the first selected object's, else the active one. */
export function layerAtHand(ctx: AppContext, kept: string | null): string | null {
  const { doc, selection } = ctx;
  const isLayer = (id: string | null | undefined): id is string => !!id && doc.layers.get(id)?.type === 'layer';
  if (isLayer(kept)) return kept;
  const first = [...selection.ids.value].map((id) => doc.get(id)).find((e) => e);
  if (isLayer(first?.layerId)) return first.layerId;
  if (isLayer(doc.layers.active.value)) return doc.layers.active.value;
  return doc.layers.leaves()[0]?.id ?? null;
}
