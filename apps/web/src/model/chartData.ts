import type { Entity } from './entities';
import { checkValue, displayValue, fieldLabel, inferField, type LayerField } from './layerFields';
import type { ChartCell, ColumnKind, DataColumn } from './ops/charts';
import { naturalOrder } from './ops/pointEditor';

/**
 * A chart's data (docs/adr/0227 §2), read from a layer's objects or from an İşlemler table; the desktop's is
 * `kentos_interaction::chart_data`. A layer's columns are its fields in order (their aliases; a value list sorts by what
 * it shows), then the keys no field names in the natural order (their kind the one Verilerden al gives every object's
 * value), then the geometry's Alan and Uzunluk in the project's unit (`ops/charts.ts` adds them from the store). A
 * cell is its value's canonical text and what it shows, empty, or refused as written. What to chart first is the core's
 * (`ChartData.complete`, `forColumn`, `forTable`).
 */

/** A field's kind as a column's. */
export function kindOf(f: LayerField): ColumnKind {
  if (f.kind === 'integer' || f.kind === 'decimal') return 'number';
  return f.kind;
}

/** A value of `f` as a cell. */
export function cellOf(f: LayerField, raw: string | undefined): ChartCell {
  const r = checkValue(f, raw ?? '');
  if ('error' in r) return raw !== undefined && raw.trim() ? { invalid: raw } : null;
  if (r.value === '') return null;
  // Only a value list, a date and yes or no show otherwise than their canonical text.
  if (!f.values?.length && f.kind !== 'date' && f.kind !== 'boolean') return { v: r.value };
  const shown = displayValue(f, r.value);
  return shown === r.value ? { v: r.value } : { v: r.value, s: shown };
}

/** A layer's columns: its fields, then its objects' other keys (their kind inferred from every object). */
export function layerColumns(fields: readonly LayerField[], entities: readonly Entity[]): DataColumn[] {
  const named = new Set(fields.map((f) => f.name));
  const others = new Map<string, string[]>();
  for (const e of entities)
    for (const [k, v] of Object.entries(e.attrs)) {
      if (named.has(k)) continue;
      const list = others.get(k);
      if (list) list.push(v);
      else others.set(k, [v]);
    }
  const keys = [...others.keys()];
  const inferred = naturalOrder(keys).map((i) => inferField(keys[i], others.get(keys[i])!));
  const column = (f: LayerField, label: string): DataColumn => ({
    key: f.name,
    label,
    kind: kindOf(f),
    labelled: !!f.values?.length,
    cells: entities.map((e) => cellOf(f, Object.hasOwn(e.attrs, f.name) ? e.attrs[f.name] : undefined)),
  });
  return [...fields.map((f) => column(f, fieldLabel(f))), ...inferred.map((f) => column(f, f.name))];
}

/** An İşlemler table's columns: its headers, their kind inferred from every row. */
export function tableColumns(table: { readonly columns: readonly string[]; readonly rows: readonly (readonly string[])[] }): DataColumn[] {
  return table.columns.map((name, j) => {
    const f = inferField(name, table.rows.map((r) => r[j] ?? ''));
    return { key: name, label: name, kind: kindOf(f), labelled: false, cells: table.rows.map((r) => cellOf(f, r[j])) };
  });
}
