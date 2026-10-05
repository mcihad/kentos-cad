import type { TextAlign } from './entities';
import type { LineType } from './layers';
import { TEMPLATE_METHODS, type ObjectTemplate, type TemplateTool } from './objectTemplate';

/**
 * The Şablon düzenleyici's form (docs/adr/0176 §4): its texts and choices, the library item's fields and the
 * template made from them, and back. The rules (fixtures/style/v1/template-form.json, written from them by
 * scripts/fixtures/template_form_cases.py): Ad is trimmed and needed; Kategori and the layer's Gruplar are split on
 * “/”, each part trimmed, the empty ones left out; Açıklama, Etiket and the point's Ad and Kod are trimmed and, empty,
 * not written; Yöntem only for a tool that has methods; Katman (its name) is trimmed and needed; an empty choice is the
 * default; a weight is a number with a point or a comma, 0 to 100 mm; attribute rows with neither a name nor a value
 * are left out, a value without a name is said by its row, a name written twice by its name; a text template needs a
 * Yükseklik above zero, a block template its Blok; the other tools' fields are not written. Every problem in the
 * order of the fields. The desktop's is `kentos_native_style::template_form`.
 */

export interface TemplateForm {
  name: string;
  /** The category path, “Kadastro / Sınırlar”. */
  category: string;
  description: string;
  tool: TemplateTool;
  /** The tool's method; empty: its first. */
  method: string;
  /** The groups above the layer, “Kadastro / Tapu”, and its name. */
  layerGroups: string;
  layerName: string;
  /** The opened layer's look; empty: a new layer's. */
  layerColor: string;
  layerLineType: '' | LineType;
  layerWeight: string;
  /** The objects' own colour, weight and symbol; empty: their layer's. */
  color: string;
  weight: string;
  symbol: string;
  /** Attribute rows, name and value, as typed. */
  attrs: [string, string][];
  label: string;
  pointName: string;
  pointCode: string;
  textHeight: string;
  textAlign: '' | TextAlign;
  textMask: boolean;
  block: string;
}

/** What a form makes: the library item's fields and its template, or what is wrong. */
export type TemplateFormResult =
  | { readonly name: string; readonly path: readonly string[]; readonly description?: string; readonly template: ObjectTemplate }
  | { readonly issues: readonly string[] };

/** A new template's form: Kapalı alan, nothing else chosen. */
export const EMPTY_TEMPLATE_FORM: TemplateForm = {
  name: '',
  category: '',
  description: '',
  tool: 'polygon',
  method: '',
  layerGroups: '',
  layerName: '',
  layerColor: '',
  layerLineType: '',
  layerWeight: '',
  color: '',
  weight: '',
  symbol: '',
  attrs: [],
  label: '',
  pointName: '',
  pointCode: '',
  textHeight: '',
  textAlign: '',
  textMask: false,
  block: '',
};

const parts = (text: string) =>
  text
    .split('/')
    .map((p) => p.trim())
    .filter(Boolean);

const NUMBER = /^\d+(?:[.,]\d+)?$/;

/** A non-negative number with a point or a comma; null for anything else. */
function number(text: string): number | null {
  const t = text.trim();
  return NUMBER.test(t) ? Number(t.replace(',', '.')) : null;
}

/** The library item's fields and the template a form makes, or what is wrong with it. */
export function templateFromForm(f: TemplateForm): TemplateFormResult {
  const issues: string[] = [];
  const name = f.name.trim();
  if (!name) issues.push('Şablonun adı boş; bir ad yazın.');
  const layerName = f.layerName.trim();
  if (!layerName) issues.push('Katmanın adı boş; şablonun çizeceği katmanı yazın ya da seçin.');
  let layerWeight: number | undefined;
  if (f.layerWeight.trim()) {
    const w = number(f.layerWeight);
    if (w === null || w > 100) issues.push('Katmanın kalınlığı 0 ile 100 mm arasında bir sayı olmalı.');
    else layerWeight = w;
  }
  let weight: number | undefined;
  if (f.weight.trim()) {
    const w = number(f.weight);
    if (w === null || w > 100) issues.push('Kalınlık 0 ile 100 mm arasında bir sayı olmalı.');
    else weight = w;
  }
  const attrs: Record<string, string> = {};
  f.attrs.forEach(([rawKey, value], i) => {
    const key = rawKey.trim();
    if (!key && !value.trim()) return;
    if (!key) issues.push(`${i + 1}. öznitelik satırının adı boş.`);
    else if (key in attrs) issues.push(`“${key}” özniteliği iki kez yazılmış.`);
    else attrs[key] = value;
  });
  let text: ObjectTemplate['text'];
  if (f.tool === 'text') {
    const h = f.textHeight.trim() ? number(f.textHeight) : null;
    if (h === null || !(h > 0)) issues.push('Yazının yüksekliği sıfırdan büyük bir sayı olmalı.');
    else text = { height: h, ...(f.textAlign && { align: f.textAlign }), ...(f.textMask && { mask: true }) };
  }
  const block = f.block.trim();
  if (f.tool === 'blockInsert' && !block) issues.push('Blok şablonunun bloğu yok; yerleştirilecek bloğu seçin.');
  if (issues.length) return { issues };
  const point = f.tool === 'point' ? { ...(f.pointName.trim() && { name: f.pointName.trim() }), ...(f.pointCode.trim() && { code: f.pointCode.trim() }) } : {};
  const sorted = Object.fromEntries(Object.entries(attrs).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  const template: ObjectTemplate = {
    tool: f.tool,
    ...(TEMPLATE_METHODS[f.tool] && f.method && { method: f.method }),
    layer: {
      path: parts(f.layerGroups),
      name: layerName,
      ...(f.layerColor && { color: f.layerColor }),
      ...(f.layerLineType && { lineType: f.layerLineType }),
      ...(layerWeight !== undefined && { lineWeight: layerWeight }),
    },
    ...(f.color && { color: f.color }),
    ...(weight !== undefined && { lineWeight: weight }),
    ...(f.symbol && { symbol: f.symbol }),
    ...(Object.keys(sorted).length > 0 && { attrs: sorted }),
    ...(f.label.trim() && { label: f.label.trim() }),
    ...(Object.keys(point).length > 0 && { point }),
    ...(text && { text }),
    ...(f.tool === 'blockInsert' && { block }),
  };
  const description = f.description.trim();
  return { name, path: parts(f.category), ...(description && { description }), template };
}

/** A library item's fields and template as its form shows them. */
export function formOfTemplate(item: { readonly name: string; readonly path: readonly string[]; readonly description?: string; readonly template: ObjectTemplate }): TemplateForm {
  const t = item.template;
  const num = (n: number | undefined) => (n === undefined ? '' : String(n));
  return {
    name: item.name,
    category: item.path.join(' / '),
    description: item.description ?? '',
    tool: t.tool,
    method: t.method ?? '',
    layerGroups: t.layer.path.join(' / '),
    layerName: t.layer.name,
    layerColor: t.layer.color ?? '',
    layerLineType: t.layer.lineType ?? '',
    layerWeight: num(t.layer.lineWeight),
    color: t.color ?? '',
    weight: num(t.lineWeight),
    symbol: t.symbol ?? '',
    attrs: Object.entries(t.attrs ?? {})
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(([k, v]) => [k, v]),
    label: t.label ?? '',
    pointName: t.point?.name ?? '',
    pointCode: t.point?.code ?? '',
    textHeight: num(t.text?.height),
    textAlign: t.text?.align ?? '',
    textMask: t.text?.mask === true,
    block: t.block ?? '',
  };
}
