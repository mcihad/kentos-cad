// How the project page says a record's values (docs/adr/9194), apart from the DOM: a listed code by its label, a yes/no
// in words, a value never written (NULL) apart from an empty text; the fields the caller's role does not read named in
// the desktop table's words; a page's records by layer and by their labels.
import type { FeatureRecord, LayerField } from '../api/types';

/** A value as the table says it; `null` for a value never written (NULL), `''` for an empty text. */
export function valueText(field: LayerField, raw: string | undefined): string | null {
  if (raw === undefined) return null;
  if (raw === '') return '';
  if (field.kind === 'boolean') return raw === 'true' ? 'Evet' : raw === 'false' ? 'Hayır' : raw;
  const label = (code: string) => field.values?.find((c) => c.code === code)?.label ?? code;
  return field.multiple ? raw.split('; ').map(label).join(', ') : label(raw);
}

/** The words for the fields withheld from the caller. */
export function withheldWords(fields: string[]): string {
  return `Rolünüz bu alanları okumuyor, gösterilmiyor: ${fields.join(', ')}`;
}

/** What a record's attributes are read through: the part of every kind of object the page needs. */
export type RecordBase = { layerId: string; attrs: Record<string, string>; label?: string };

export const recordBase = (r: FeatureRecord): RecordBase => r.entity as unknown as RecordBase;

/** A page's records by their layer, in the page's order. */
export function byLayer(records: FeatureRecord[]): Map<string, FeatureRecord[]> {
  const out = new Map<string, FeatureRecord[]>();
  for (const r of records) {
    const id = recordBase(r).layerId;
    const list = out.get(id);
    if (list) list.push(r);
    else out.set(id, [r]);
  }
  return out;
}

const COLLATOR = new Intl.Collator('tr', { numeric: true });

/** Records by their label in natural order (D-2 before D-10); those without one after them, as they came. */
export function byLabel(records: FeatureRecord[]): FeatureRecord[] {
  return records
    .map((r, i) => ({ r, i, label: recordBase(r).label }))
    .sort((a, b) => {
      if (a.label === undefined || b.label === undefined) {
        return a.label === b.label ? a.i - b.i : a.label === undefined ? 1 : -1;
      }
      return COLLATOR.compare(a.label, b.label) || a.i - b.i;
    })
    .map((x) => x.r);
}
