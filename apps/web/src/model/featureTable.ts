import { ENTITY_KIND_LABEL, type Entity } from './entities';
import { checkValue, displayValue, fieldLabel, type LayerField } from './layerFields';
import type { ColumnOrder, TableColumn, TableRow } from './ops/featureTable';
import { naturalOrder } from './ops/pointEditor';

/**
 * The attribute table's columns and rows (docs/adr/0199 §4), read from a layer's objects and fields for the core's
 * `featureTable` (./ops/featureTable.ts), which shows and orders them. Columns: Tür, the layer's fields (their aliases,
 * in order), then the keys no field names (the natural order). A field's cell shows its value as the field displays
 * it (a code its label, Evet/Hayır, GG.AA.YYYY) and sorts by its canonical text (a value list by its label); a value
 * that does not keep the field's rules shows as written, with the reason, and sorts last. The desktop's is
 * `apps/desktop/src/features/model.rs`.
 */

/** A column of the table: the attribute's key (none: Tür), its header, its field, how it sorts. */
export interface FeatureColumn {
  key: string | null;
  label: string;
  field: LayerField | null;
  order: ColumnOrder;
}

/** The table read from a layer: its columns, objects (the drawing's order), the core's rows, the cells' problems. */
export interface FeatureTableModel {
  columns: FeatureColumn[];
  entities: Entity[];
  rows: TableRow[];
  /** Why a cell's value does not keep its field's rules, by `row:column`. */
  problems: Map<string, string>;
}

/** How a field's values are ordered: a value list by its labels (what is seen), else by its kind. */
export function orderOf(f: LayerField): ColumnOrder {
  if (f.values?.length) return 'text';
  if (f.kind === 'integer' || f.kind === 'decimal') return 'number';
  if (f.kind === 'date' || f.kind === 'boolean') return f.kind;
  return 'text';
}

/** A field's cell: what it shows, its sort key, and why its value does not keep the field's rules (when it does not). */
export function fieldCell(f: LayerField, raw: string | undefined): { shown: string; key: string | null; problem: string | null } {
  const r = checkValue(f, raw ?? '');
  if ('error' in r) return { shown: raw ?? '', key: null, problem: r.message };
  if (r.value === '') return { shown: '', key: null, problem: null };
  const shown = displayValue(f, r.value);
  return { shown, key: f.values?.length ? shown : r.value, problem: null };
}

/** The columns a layer's table has: Tür, its fields, then the other keys its objects carry (the natural order). */
export function featureColumns(fields: readonly LayerField[], entities: readonly Entity[]): FeatureColumn[] {
  const named = new Set(fields.map((f) => f.name));
  const others = new Set<string>();
  for (const e of entities) for (const k of Object.keys(e.attrs)) if (!named.has(k)) others.add(k);
  const keys = [...others];
  return [
    { key: null, label: 'Tür', field: null, order: 'text' },
    ...fields.map((f) => ({ key: f.name, label: fieldLabel(f), field: f, order: orderOf(f) })),
    ...naturalOrder(keys).map((i) => ({ key: keys[i], label: keys[i], field: null, order: 'text' as const })),
  ];
}

/**
 * The table of `entities` (a layer's objects in the drawing's order) under `fields`: each object's cells, whether it
 * is selected, in the view, and kept by the expression filter (`passes`, none: every one).
 */
export function featureTableModel(
  fields: readonly LayerField[],
  entities: readonly Entity[],
  selected: (id: number) => boolean,
  inView: (e: Entity) => boolean,
  passes: readonly boolean[] | null,
): FeatureTableModel {
  const columns = featureColumns(fields, entities);
  const problems = new Map<string, string>();
  const rows = entities.map((e, i): TableRow => {
    const cells = columns.map((c, j) => {
      if (c.key === null) {
        const kind = ENTITY_KIND_LABEL[e.kind];
        return { shown: kind, key: kind };
      }
      const raw = Object.hasOwn(e.attrs, c.key) ? e.attrs[c.key] : undefined;
      if (c.field) {
        const cell = fieldCell(c.field, raw);
        if (cell.problem) problems.set(`${i}:${j}`, cell.problem);
        return { shown: cell.shown, key: cell.key };
      }
      const value = raw?.trim();
      return { shown: raw ?? '', key: value ? value : null };
    });
    return { cells, selected: selected(e.id), inView: inView(e), passes: passes ? passes[i] : true };
  });
  return { columns, entities: [...entities], rows, problems };
}

/** The column headers as the table and the data's search see them: the key, its alias for a field. */
export const columnTable = (columns: readonly FeatureColumn[]): TableColumn[] => columns.map((c) => ({ order: c.order, searched: c.key !== null }));
