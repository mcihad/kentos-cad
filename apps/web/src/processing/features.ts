import type { CadDocument } from '../model/document';
import { ENTITY_KIND_LABEL, type Entity, type EntityKind, type RasterEntity } from '../model/entities';
import type { Bounds } from '../model/geometry';
import { leftOut } from '../model/layerFilter';
import { withObjects, type DocumentGeometry } from './geometry';
import type { FeatureSet, FeaturesParam, FeaturesValue } from './types';

/**
 * Turns a features value (selection, visible, all, a layer, explicit ids)
 * into the objects a tool works on. Selection, the visible area and the
 * drawing's geometry store come from the host, so this module stays free
 * of UI and viewport.
 */

export interface FeatureHost {
  readonly doc: CadDocument;
  selectedIds(): readonly number[];
  /** World box on screen, or null when there is no view (tests, server). */
  visibleBounds(): Bounds | null;
  /**
   * The drawing's geometry store as the host keeps it (the viewport's,
   * docs/adr/0008): it answers the "visible" scope's box test and the
   * dialog's expression previews. Without one (tests, a server) the
   * objects asked about get a store of their own.
   */
  readonly geometry?: DocumentGeometry;
  /** Replaces the selection (tools that select); absent where there is none. */
  select?(ids: readonly number[]): void;
}

export const SCOPE_LABEL: Record<FeaturesValue['scope'], string> = {
  selection: 'Seçili nesneler',
  visible: 'Görünen alandakiler',
  all: 'Tümü (görünen katmanlar)',
  layer: 'Bir katman',
  ids: 'Önceki adımın çıktısı',
};

/**
 * Objects in scope, before the kind filter: a layer's filter leaves out what it does not pass (docs/adr/0211 §1), in
 * every scope but a model's own step outputs.
 */
function inScope(value: FeaturesValue, host: FeatureHost): Entity[] {
  const list = scoped(value, host);
  if (value.scope === 'ids') return list;
  const out = leftOut(host.doc);
  return out.size ? list.filter((e) => !out.has(e.id)) : list;
}

function scoped(value: FeaturesValue, host: FeatureHost): Entity[] {
  const { doc } = host;
  const shown = (e: Entity) => doc.layers.isVisible(e.layerId);
  switch (value.scope) {
    case 'selection':
      return host.selectedIds().flatMap((id) => doc.get(id) ?? []);
    case 'visible': {
      // Construction lines reach everywhere: they are never "in view".
      const kept = (e: Entity) => shown(e) && e.kind !== 'xline' && e.kind !== 'ray';
      const view = host.visibleBounds();
      if (!view) return [...doc.all()].filter(kept);
      // Whose box overlaps the view is the geometry store's answer, in the document's order.
      const ids = host.geometry ? host.geometry.inBox(view) : withObjects(doc.all(), (s) => s.inBox(view));
      return ids.flatMap((id) => {
        const e = doc.get(id);
        return e && kept(e) ? [e] : [];
      });
    }
    case 'all':
      return [...doc.all()].filter(shown);
    case 'layer': {
      // A group means every layer under it.
      const leaves = new Set(doc.layers.leavesOf(value.layerId).map((l) => l.id));
      leaves.add(value.layerId);
      return [...doc.all()].filter((e) => leaves.has(e.layerId));
    }
    case 'ids':
      return value.ids.flatMap((id) => doc.get(id) ?? []);
  }
}

/** "12 kapalı alan" / "7 nesne (kapalı alan, çoklu çizgi)". */
export function describeCount(entities: readonly Entity[]): string {
  if (!entities.length) return 'uygun nesne yok';
  const kinds = [...new Set(entities.map((e) => e.kind))];
  if (kinds.length === 1) return `${entities.length} ${ENTITY_KIND_LABEL[kinds[0]].toLocaleLowerCase('tr-TR')}`;
  return `${entities.length} nesne (${kinds.map((k) => ENTITY_KIND_LABEL[k].toLocaleLowerCase('tr-TR')).join(', ')})`;
}

export function resolveFeatures(value: FeaturesValue, def: Pick<FeaturesParam, 'kinds'>, host: FeatureHost): FeatureSet {
  return narrow(value, host, fitting(value, def, host));
}

/** Objects in scope that the tool can take, before the user's kind filter. */
function fitting(value: FeaturesValue, def: Pick<FeaturesParam, 'kinds'>, host: FeatureHost): Entity[] {
  const kinds = def.kinds ? new Set<EntityKind>(def.kinds) : null;
  return inScope(value, host).filter((e) => !kinds || kinds.has(e.kind));
}

function narrow(value: FeaturesValue, host: FeatureHost, candidates: Entity[]): FeatureSet {
  const only = value.kinds ? new Set<EntityKind>(value.kinds) : null;
  const entities = only ? candidates.filter((e) => only.has(e.kind)) : candidates;
  if (!entities.length) return { entities, description: `${WHERE_IN[value.scope](host, value)} uygun nesne yok` };
  const where = value.scope === 'layer' ? `“${host.doc.layers.get(value.layerId)?.name ?? '?'}” katmanında` : SCOPE_LABEL[value.scope].toLocaleLowerCase('tr-TR');
  return { entities, description: `${describeCount(entities)}; ${where}` };
}

/** A features parameter as the dialog shows it before running. */
export interface InputSummary {
  count: number;
  description: string;
  /** Kinds in scope the tool can take, with counts (for the kind filter), before that filter. */
  byKind: { kind: EntityKind; count: number }[];
  /** Attribute names on the objects, most common first (field pickers, expressions). */
  fields: { name: string; count: number }[];
  /** A chosen file's table (docs/adr/0200 §7): the counts are its rows, not objects. */
  rows?: true;
  /** The rasters' names in the run's order (docs/adr/0233 §2, §3): a value or a comparison a raster's rows (docs/adr/0237 §9). */
  rasters?: string[];
}

export function summarizeFeatures(value: FeaturesValue, def: Pick<FeaturesParam, 'kinds'>, host: FeatureHost): InputSummary {
  const candidates = fitting(value, def, host);
  const set = narrow(value, host, candidates);
  const kinds = new Map<EntityKind, number>();
  for (const e of candidates) kinds.set(e.kind, (kinds.get(e.kind) ?? 0) + 1);
  const fields = new Map<string, number>();
  for (const e of set.entities.slice(0, 20000)) for (const k of Object.keys(e.attrs)) fields.set(k, (fields.get(k) ?? 0) + 1);
  // A raster's bands are what an expression names it by (Raster hesaplayıcı, docs/adr/0233 §3).
  const rasters = set.entities.filter((e): e is RasterEntity => e.kind === 'raster');
  const leaves = rasters.length ? new Map(host.doc.layers.leaves().map((l, k) => [l.id, k])) : new Map<string, number>();
  const doc: RasterRunDoc = {
    layerIndex: (id) => leaves.get(id) ?? Number.MAX_SAFE_INTEGER,
    byLayer: (id) => host.doc.byLayer(id),
    layerName: (id) => host.doc.layers.get(id)?.name ?? id,
  };
  const run = rasterRun(rasters, doc);
  for (const { raster, name } of run) {
    fields.set(name, 1);
    for (let b = 2; b <= raster.bands; b++) fields.set(`${name}@${b}`, 1);
  }
  return {
    count: set.entities.length,
    description: set.description,
    byKind: [...kinds].map(([kind, count]) => ({ kind, count })).sort((a, b) => b.count - a.count),
    fields: [...fields].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count || a.name.localeCompare(b.name, 'tr')),
    ...(run.length ? { rasters: run.map((r) => r.name) } : {}),
  };
}

/** What a raster operation's order and names read of the drawing: the layers' places, a layer's objects, its name. */
export interface RasterRunDoc {
  /** A layer's place among the layers, the top of the panel first. */
  layerIndex(id: string): number;
  /** The objects on a layer, in document order. */
  byLayer(layerId: string): readonly Entity[];
  layerName(id: string): string;
}

/**
 * Rasters in a raster operation's order (docs/adr/0233 §2): the layers from the top of the panel down, on a layer the
 * later first; and the name an expression reads each by (§3): its layer's, a second raster on a layer “Ad (2)”, and so
 * on. Only rasters sharing a layer ask for the layer's order of objects.
 */
export function rasterRun<T extends RasterEntity>(list: readonly T[], d: RasterRunDoc): { raster: T; name: string }[] {
  if (!list.length) return [];
  const on = new Map<string, number>();
  for (const r of list) on.set(r.layerId, (on.get(r.layerId) ?? 0) + 1);
  const place = new Map<number, number>();
  for (const [layerId, n] of on) if (n > 1) d.byLayer(layerId).forEach((e, k) => place.set(e.id, k));
  const sorted = [...list].sort((a, b) => d.layerIndex(a.layerId) - d.layerIndex(b.layerId) || (place.get(b.id) ?? 0) - (place.get(a.id) ?? 0));
  const seen = new Map<string, number>();
  return sorted.map((raster) => {
    const name = d.layerName(raster.layerId);
    const n = (seen.get(name) ?? 0) + 1;
    seen.set(name, n);
    return { raster, name: n === 1 ? name : `${name} (${n})` };
  });
}

/** "Seçili nesneler arasında uygun nesne yok" — where the tool looked, as a sentence start. */
const WHERE_IN: Record<FeaturesValue['scope'], (host: FeatureHost, v: FeaturesValue) => string> = {
  selection: () => 'Seçili nesneler arasında',
  visible: () => 'Görünen alanda',
  all: () => 'Görünen katmanlarda',
  layer: (host, v) => `“${v.scope === 'layer' ? (host.doc.layers.get(v.layerId)?.name ?? '?') : '?'}” katmanında`,
  ids: () => 'Önceki adımın çıktısında',
};
