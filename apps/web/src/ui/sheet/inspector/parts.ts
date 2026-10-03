import type { AppContext } from '../../../app/context';
import type { Item } from '../../../contracts/generated/sheet/Item';
import type { ItemKind } from '../../../contracts/generated/sheet/ItemKind';
import type { Op } from '../../../contracts/generated/sheet/Op';
import type { Sheet } from '../../../contracts/generated/sheet/Sheet';
import type { DisposableStore } from '../../../core/disposable';
import type { ItemView } from '../../../product/sheet/view';
import { h, type Child } from '../../dom';
import { toggleSwitch } from '../../widgets/controls';
import { tooltip } from '../../widgets/tooltip';
import type { SheetHost } from '../host';
import { common } from '../inspectorPlan';
import { numberField } from '../widgets/fields';
import { choice, colorInput, field, longText, textInput, type ChoiceOption } from '../widgets/form';

/**
 * What the inspector's sections share (docs/sheet/design.md §11): the chosen
 * items, the sheet they are on, why they cannot be edited now, and the one
 * way an edit leaves: a patch per item (`SetItemProps`, a JSON merge the
 * engine checks against the kind's schema: an unknown field or a wrong type
 * is refused, never dropped), all of them one undo step. Several items of
 * one kind show what they share and “—” where they differ; a value given
 * goes to all of them.
 */

export interface SectionCtx {
  readonly ctx: AppContext;
  readonly host: SheetHost;
  readonly items: readonly ItemView[];
  readonly sheet: Sheet;
  /** Why the items cannot be edited now (the engine is away, one is locked); null when they can. */
  readonly readOnly: string | null;
  readonly d: DisposableStore;
}

type KindOf<T extends ItemKind['type']> = Extract<ItemKind, { type: T }>;

/** The kind's content of each chosen item (all are of kind `T` when a kind's section is shown). */
export const kinds = <T extends ItemKind['type']>(c: SectionCtx, _t: T): KindOf<T>[] => c.items.map((i) => i.source.kind as KindOf<T>);

/** A value every chosen item's kind has, or null when they differ. */
export function shared<T extends ItemKind['type'], V>(c: SectionCtx, t: T, of: (k: KindOf<T>) => V): V | null {
  const ks = kinds(c, t);
  if (!ks.length) return null;
  const first = JSON.stringify(of(ks[0]));
  return ks.every((k) => JSON.stringify(of(k)) === first) ? of(ks[0]) : null;
}

/** A patch of each chosen item, one undo step named `label`. */
export function patchEach(c: SectionCtx, label: string, patch: (i: Item) => Record<string, unknown>): boolean {
  const ops: Op[] = c.items.map((i) => ({ op: 'setItemProps', id: i.id, patch: patch(i.source) }));
  return c.host.apply(ops, label);
}

/** The same patch of every chosen item's kind content. */
export const patchKind = (c: SectionCtx, label: string, kind: Record<string, unknown>): boolean => patchEach(c, label, () => ({ kind }));

/** A patch of one chosen item's kind content worked out from that item's own (a list changed in place). */
export const patchKindEach = <T extends ItemKind['type']>(c: SectionCtx, _t: T, label: string, kind: (k: KindOf<T>) => Record<string, unknown>): boolean =>
  patchEach(c, label, (i) => ({ kind: kind(i.kind as KindOf<T>) }));

/** The maps of the sheet, for a scale bar's, a legend's, a north arrow's or a text's link. */
export function mapOptions(c: SectionCtx, none: string): ChoiceOption<string>[] {
  return [{ value: '', label: none }, ...c.sheet.items.filter((i) => i.kind.type === 'map' && !c.items.some((x) => x.id === i.id)).map((i) => ({ value: i.id, label: i.name }))];
}

/** A yes/no field in a section (“—” beside it when the items differ). */
export function flag(c: SectionCtx, label: string, value: boolean | null, take: (v: boolean) => void, hint?: string): HTMLElement {
  const sw = toggleSwitch({ label, checked: value === true, disabled: c.readOnly !== null, onChange: (v) => take(v) });
  const row = h('div', { class: 'sheet-insp__row' }, h('span', { class: 'sheet-insp__label' }, label, value === null ? h('span', { class: 'sheet-insp__hint' }, ' — farklı') : null), sw);
  if (hint || c.readOnly) c.d.add(tooltip(row, () => ({ title: label, description: hint, note: c.readOnly ?? undefined }), 'top'));
  return row;
}

/** Millimetres of a µm value, as a field (the engine keeps µm). */
export function mmField(c: SectionCtx, label: string, key: string, um: number | null, take: (um: number) => void, o: { min?: number; max?: number; decimals?: number; unit?: string } = {}): HTMLElement {
  return numberField({ label, key, value: um === null ? null : um / 1000, unit: o.unit ?? 'mm', decimals: o.decimals ?? 1, min: o.min, max: o.max, readOnly: c.readOnly, onCommit: (v) => take(Math.round(v * 1000)) }, c.d);
}

export function numField(c: SectionCtx, label: string, key: string, value: number | null, take: (v: number) => void, o: { min?: number; max?: number; decimals?: number; unit?: string } = {}): HTMLElement {
  return numberField({ label, key, value, unit: o.unit ?? '', decimals: o.decimals ?? 0, min: o.min, max: o.max, readOnly: c.readOnly, onCommit: take }, c.d);
}

export const pick = <T>(c: SectionCtx, label: string, key: string, value: T | null, options: readonly ChoiceOption<T>[], take: (v: T) => void): HTMLElement =>
  choice({ label, key, value, options, readOnly: c.readOnly, onChange: take }, c.d);

export const text = (c: SectionCtx, label: string, key: string, value: string | null, take: (v: string) => void, placeholder?: string): HTMLElement =>
  textInput({ label, key, value, readOnly: c.readOnly, placeholder, onCommit: take }, c.d);

export const area = (c: SectionCtx, label: string, key: string, value: string | null, take: (v: string) => void, rows = 3, extra?: Child): HTMLElement =>
  longText({ label, key, value, readOnly: c.readOnly, rows, onCommit: take, extra }, c.d);

export const color = (c: SectionCtx, label: string, key: string, value: string | null, take: (v: string) => void): HTMLElement => colorInput({ label, key, value, readOnly: c.readOnly, onCommit: take }, c.d);

/** Fields side by side (two columns in the inspector). */
export const pair = (...fields: Child[]): HTMLElement => h('div', { class: 'sheet-fields' }, fields);

/** A field's label alone over something of its own. */
export const labelled = (label: string, body: Child, hint?: string): HTMLElement => field(label, body, hint);

/** The common value of chosen items as the inspector shows several items (re-exported for the sections). */
export { common };
