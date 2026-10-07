import { foldTurkish } from '../../core/text';
import { ENTITY_KIND_LABEL, type EntityKind } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import type { LayerStore } from '../../model/layers';
import type { InputSummary } from '../../processing/features';
import { fieldNames, fileTable, scopesOf } from '../../processing/parameters';
import { sameNamedLayer } from '../../processing/runner';
import type { FeaturesValue, FileValue, LayerValue, ParamDef } from '../../processing/types';
import { DIALOG_TEXTS as T } from './dialogTexts';

/**
 * What the processing dialog's fields show and do, without the page:
 * the features field (scope, layer list, what it reads, kind chips), the
 * output layer, the point, the attribute name and the expression helpers
 * (field chips and tokens, insertion, the line and its icon), and how a
 * number field reads its text. paramFields draws from these; the dialog's
 * view (dialogPlan) and fixtures/processing/v1/dialog.json use them too.
 */

type FeaturesParam = Extract<ParamDef, { type: 'features' }>;
type ListedScope = Exclude<FeaturesValue['scope'], 'ids'>;

/** A number field's text: a decimal point or comma; empty or anything else is NaN, which the check calls invalid. */
export function numberOfText(text: string): number {
  const n = Number(text.replace(',', '.'));
  return text.trim() !== '' && Number.isFinite(n) ? n : NaN;
}

/** Layers as the fields list them: leaves in tree order with their paths, and any layer's or group's name. */
export interface PlanLayers {
  readonly leaves: readonly { id: string; name: string; path: string; locked: boolean }[];
  name(id: string): string | undefined;
}

export function planLayers(layers: LayerStore): PlanLayers {
  return {
    leaves: layers.leaves().map((l) => ({ id: l.id, name: l.name, path: layers.path(l.id), locked: layers.isLocked(l.id) })),
    name: (id) => layers.get(id)?.name,
  };
}

export interface KindChip {
  kind: EntityKind;
  label: string;
  count: number;
  pressed: boolean;
  title: string;
}

/**
 * The kind filter: chips when there is a choice (two or more kinds in
 * scope) or a filter to undo; otherwise the kinds the tool takes, if it
 * names them.
 */
export function kindsView(def: FeaturesParam, value: FeaturesValue, present: InputSummary['byKind']): { chips: KindChip[] } | { note: string } | null {
  if (present.length >= 2 || (value.kinds && present.length)) {
    const on = new Set<EntityKind>(value.kinds ?? present.map((k) => k.kind));
    return {
      chips: present.map((k) => {
        const pressed = on.has(k.kind);
        return { kind: k.kind, label: ENTITY_KIND_LABEL[k.kind], count: k.count, pressed, title: pressed ? T.features.kindOff : T.features.kindOn };
      }),
    };
  }
  if (!def.kinds) return null;
  // A tool that takes nearly every kind says what it leaves out.
  const left = (Object.keys(ENTITY_KIND_LABEL) as EntityKind[]).filter((k) => !def.kinds!.includes(k));
  return { note: left.length < def.kinds.length ? T.features.kindsLeftOut(left) : T.features.kindsNote(def.kinds) };
}

/** A chip clicked: the kind leaves or joins the filter; every kind in scope on again means no filter. */
export function toggleKind(value: FeaturesValue, present: InputSummary['byKind'], kind: EntityKind): FeaturesValue {
  const on = new Set<EntityKind>(value.kinds ?? present.map((k) => k.kind));
  if (on.has(kind)) on.delete(kind);
  else on.add(kind);
  const { kinds: _old, ...rest } = value;
  return present.every((p) => on.has(p.kind)) ? rest : { ...rest, kinds: present.map((p) => p.kind).filter((k) => on.has(k)) };
}

/** A scope chosen on the segmented control; Katman starts on the active layer. Choosing the scope in use changes nothing. */
export function withScope(value: FeaturesValue, scope: ListedScope, activeLayer: string): FeaturesValue {
  if (value.scope === scope) return value;
  const kinds = value.kinds ? { kinds: value.kinds } : {};
  return scope === 'layer' ? { scope, layerId: activeLayer, ...kinds } : { scope, ...kinds };
}

/** A layer chosen for the Katman scope. */
export function withLayer(value: FeaturesValue, layerId: string): FeaturesValue {
  return { scope: 'layer', layerId, ...(value.kinds ? { kinds: value.kinds } : {}) };
}

export interface FeaturesView {
  /** The checked segment: a step's output (ids) shows as the first scope. */
  scope: ListedScope;
  /** The Katman scope's layer list. */
  layer: { text: string; items: { id: string; label: string; checked: boolean }[] } | null;
  /** What the input reads now; a warning when nothing. */
  count: { text: string; empty: boolean };
  kinds: { chips: KindChip[] } | { note: string } | null;
}

export function featuresView(def: FeaturesParam, value: FeaturesValue, found: InputSummary | undefined, layers: PlanLayers): FeaturesView {
  return {
    scope: value.scope === 'ids' ? scopesOf(def)[0] : value.scope,
    layer:
      value.scope === 'layer'
        ? { text: layers.name(value.layerId) ?? T.features.noLayer, items: layers.leaves.map((l) => ({ id: l.id, label: l.path, checked: l.id === value.layerId })) }
        : null,
    count: { text: found?.description ?? '', empty: !found?.count },
    kinds: kindsView(def, value, found?.byKind ?? []),
  };
}

export type LayerItem = { header: string } | { label: string; value: LayerValue; checked: boolean; disabled: boolean; hint?: string; icon?: string };

export interface LayerFieldView {
  /** The dropdown: the chosen layer, or the new layer's name and whether a layer of that name exists. */
  text: string;
  /** The new layer's name field; null when an existing layer is chosen. */
  name: string | null;
  items: LayerItem[];
}

/** The name "Yeni: …" offers when an existing layer is chosen: the tool's default new layer, else "Yeni katman". */
export function suggestedLayerName(def: Extract<ParamDef, { type: 'layer' }>): string {
  const d = typeof def.default === 'function' ? null : def.default;
  return d && 'newName' in d ? d.newName : T.layer.suggested;
}

/**
 * An output layer: a new one by name, or an existing one; locked layers
 * cannot be chosen. A new name that an existing layer has (trimmed,
 * Turkish-folded: the runner's rule) writes to that layer, "(mevcut)".
 */
export function layerFieldView(def: Extract<ParamDef, { type: 'layer' }>, value: LayerValue, layers: PlanLayers): LayerFieldView {
  const isNew = 'newName' in value;
  const offered = isNew ? value.newName : suggestedLayerName(def);
  const items: LayerItem[] = [
    { header: T.layer.newHeader },
    { label: T.layer.newItem(offered.trim()), value: { newName: offered }, checked: isNew, disabled: false, icon: 'layerAdd' },
    { header: T.layer.existingHeader },
    ...layers.leaves.map((l): LayerItem => ({ label: l.path, value: { layerId: l.id }, checked: !isNew && value.layerId === l.id, disabled: l.locked, ...(l.locked ? { hint: T.layer.locked } : {}) })),
  ];
  if (!isNew) return { text: layers.name(value.layerId) ?? T.features.noLayer, name: null, items };
  const same = sameNamedLayer(layers.leaves, value.newName);
  const text = same ? `${same.name} ${T.layer.existing}` : [value.newName.trim(), T.layer.fresh].filter(Boolean).join(' ');
  return { text, name: value.newName, items };
}

export interface PointView {
  text: string;
  button: string;
  /** Whether a point is given (the coordinates are numbers). */
  shown: boolean;
}

export function pointView(value: Vec2 | null, format: (p: Vec2) => string): PointView {
  return value ? { text: format(value), button: T.point.again, shown: true } : { text: T.point.none, button: T.point.show, shown: false };
}

export type AttrItem = { label: string; hint: string; checked: boolean } | { label: string; disabled: true };

export interface AttrFieldView {
  /** The combo's text, or the dropdown's (Alan seçin when empty). */
  text: string;
  /** Under the field: whether the name is on the objects, new, or missing. */
  note: string;
  items: AttrItem[];
}

/**
 * The note under an attribute name: on n objects (a field written to: its value changes), on n rows of a file, a new
 * field (when new ones are allowed) or not on these objects.
 */
export function fieldNote(fields: InputSummary['fields'], name: string, allowNew: boolean, rows = false): string {
  const key = name.trim();
  if (!key) return '';
  const known = fields.find((f) => f.name === key);
  if (known) return rows ? T.field.presentRows(known.count) : allowNew ? T.field.has(known.count) : T.field.present(known.count);
  return allowNew ? T.field.fresh : T.field.missing;
}

/**
 * An attribute name, or several (`multiple`: the list checks each one written, the note speaks of the first the
 * objects lack, or of none).
 */
export function attrFieldView(def: Extract<ParamDef, { type: 'field' }>, value: string, source: InputSummary | undefined): AttrFieldView {
  const fields = source?.fields ?? [];
  const rows = !!source?.rows;
  const names = fieldNames(def, value);
  const lacking = def.multiple ? names.find((n) => !fields.some((f) => f.name === n)) : undefined;
  return {
    text: def.allowNew ? value : value || T.field.choose,
    note: def.multiple ? (lacking ? T.field.lacking(lacking) : '') : fieldNote(fields, value, !!def.allowNew, rows),
    items: fields.length
      ? fields.map((f) => ({ label: f.name, hint: rows ? T.field.rows(f.count) : T.field.count(f.count), checked: names.includes(f.name) }))
      : [{ label: T.field.none, disabled: true }],
  };
}

/** A name checked in a `multiple` field's list: added after the names written, or taken out. */
export function toggledName(value: string, name: string): string {
  const names = fieldNames({ multiple: true }, value);
  return (names.includes(name) ? names.filter((n) => n !== name) : [...names, name]).join(', ');
}

export interface FileView {
  /** The file's name, or that none is chosen. */
  text: string;
  /** Under it: its rows and columns, or that it must be chosen again (the last values keep only its name). */
  note: string;
  button: string;
  /** Whether a file with its rows is there. */
  chosen: boolean;
}

export function fileView(value: FileValue | null): FileView {
  if (!value) return { text: T.file.none, note: '', button: T.file.choose, chosen: false };
  const t = fileTable(value);
  if (!t) return { text: value.name, note: value.rows ? T.file.empty : T.file.again, button: T.file.choose, chosen: false };
  return { text: value.name, note: T.file.size({ rows: t.rows.length, columns: t.header.filter(Boolean).length }), button: T.file.other, chosen: true };
}

const PLAIN_NAME = /^[\p{L}_][\p{L}\p{N}_]*$/u;
const RESERVED = new Set(['VE', 'VEYA', 'DEGIL', 'AND', 'OR', 'NOT', 'DOGRU', 'YANLIS', 'TRUE', 'FALSE', 'BOS', 'NULL']);

/** How a field is written in an expression: bare when it can be, in brackets otherwise. */
export const fieldToken = (name: string): string => (PLAIN_NAME.test(name) && !RESERVED.has(foldTurkish(name)) ? name : `[${name}]`);

/** Fields shown as chips; the rest sit behind a "+n" menu. */
export const CHIP_FIELDS = 6;

/** The expression line's icon: information when the line says something is missing or empty, a tick otherwise. */
export const previewIcon = (text: string): 'info' | 'check' => (/ yok\.| boş\./.test(text) ? 'info' : 'check');

/**
 * Text put in at the caret (or over the selection): a space goes first
 * unless the text before already ends in a space, "(" or ",". The caret
 * ends after what was put in.
 */
export function insertText(text: string, start: number, end: number, insert: string): { text: string; caret: number } {
  const before = text.slice(0, start);
  const pad = before && !/[\s(,]$/.test(before) ? ' ' : '';
  return { text: before + pad + insert + text.slice(end), caret: start + pad.length + insert.length };
}

export interface ExpressionView {
  /** The input's fields as chips, most common first. */
  chips: { name: string; token: string; title: string }[];
  /** The rest behind "+n". */
  more: { label: string; items: { name: string; hint: string; token: string }[] } | null;
  /** How the expression works out on the input's objects now; null when empty or wrong (the field says so). */
  preview: { icon: 'info' | 'check'; text: string } | null;
}

export function expressionView(fields: InputSummary['fields'], preview: string | null): ExpressionView {
  const extra = fields.slice(CHIP_FIELDS);
  return {
    chips: fields.slice(0, CHIP_FIELDS).map((f) => ({ name: f.name, token: fieldToken(f.name), title: T.expression.chipTitle(f.count) })),
    more: extra.length ? { label: T.expression.more(extra.length), items: extra.map((f) => ({ name: f.name, hint: String(f.count), token: fieldToken(f.name) })) } : null,
    preview: preview ? { icon: previewIcon(preview), text: preview } : null,
  };
}
