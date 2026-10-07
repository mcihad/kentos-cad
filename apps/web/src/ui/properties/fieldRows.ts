import type { AppContext } from '../../app/context';
import type { Entity } from '../../model/entities';
import { checkValue, displayValue, fieldLabel, isNumberKind, type LayerField } from '../../model/layerFields';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import { setProperties, uidsOf } from './write';

/**
 * Öznitelikler's form of a layer's fields (docs/adr/0199 §5): a row per field, in order, its alias and a mark when it
 * is required; a value list and yes or no in a list, a number, a date or a text in a field showing the value as the
 * field displays it; a value the field refuses as written, with why under it. Written through `cad.entities.set`
 * (“Değiştir”), which checks the value by the same rules; a blank one takes the attribute away. The desktop's is
 * `apps/desktop/src/properties/field_rows.rs`.
 */

/** The text a list shows for no value. */
const NONE = '—';

/** The rows of `e`'s fields (`fields`, its layer's); `locked`: shown, not edited. */
export function fieldRows(ctx: AppContext, e: Entity, fields: readonly LayerField[], locked: boolean): PropRow[] {
  const write = (name: string, value: string | null) => setProperties(ctx, { uids: uidsOf(ctx, [e.id]), attrs: { [name]: value }, operation: 'attributes' });
  return fields.map((f) => {
    const raw = Object.hasOwn(e.attrs, f.name) ? e.attrs[f.name] : '';
    const r = checkValue(f, raw);
    const value = 'value' in r ? r.value : raw;
    const shown = 'value' in r ? displayValue(f, r.value) : raw;
    const row: PropRow = {
      label: `${fieldLabel(f)}${f.required ? ' *' : ''}`,
      value: shown,
      numeric: isNumberKind(f.kind) && !f.values?.length,
      ...('error' in r && { note: r.message, warn: true }),
    };
    if (locked) return row;
    if (f.values?.length || f.kind === 'boolean') {
      const choices = f.values?.length ? f.values.map((c) => ({ code: c.code, label: c.label })) : [{ code: 'true', label: 'Evet' }, { code: 'false', label: 'Hayır' }];
      const items = (): MenuItem[] => [
        { label: NONE, radio: true, checked: value === '', run: () => void write(f.name, null) },
        ...choices.map((c) => ({ label: c.label, radio: true, checked: c.code === value, run: () => void write(f.name, c.code) })),
      ];
      return { ...row, editor: { type: 'select', items, display: () => ({ text: shown || NONE }) } };
    }
    return { ...row, editor: { type: isNumberKind(f.kind) ? 'number' : 'text', commit: (v: string) => void write(f.name, v.trim() ? v : null) } };
  });
}
