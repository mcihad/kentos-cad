import { MAX_LINE_WEIGHT, TEXT_ALIGNS, type TextAlign } from './entities';
import type { LineType } from './layers';

/**
 * Nesne şablonları (docs/adr/0176): a template is a drawing recipe for one kind of object (a parcel's boundary, a building, a
 * traverse point): the tool it is drawn with, the layer it goes on (found by name, opened when the drawing lacks it),
 * its colour, line weight, symbol, attributes and label. Templates are the style library's third kind of item, beside
 * symbols and assets (`LibraryTemplate`, model/style.ts): shipped, the user's own or the project's, shared as .kstil files.
 * The desktop's rules are kentos_native_style's `template`; both pass fixtures/style/v1/object-templates.json.
 */

/** The tools a template draws with. */
export const TEMPLATE_TOOLS = ['point', 'line', 'polyline', 'polygon', 'rectangle', 'rectangle3', 'circle', 'text', 'blockInsert'] as const;
export type TemplateTool = (typeof TEMPLATE_TOOLS)[number];

/** The tools' names, as the interface says them. */
export const TEMPLATE_TOOL_LABEL: Record<TemplateTool, string> = {
  point: 'Nokta',
  line: 'Çizgi',
  polyline: 'Çoklu çizgi',
  polygon: 'Kapalı alan',
  rectangle: 'Dikdörtgen',
  rectangle3: 'Döndürülmüş dikdörtgen',
  circle: 'Daire',
  text: 'Yazı',
  blockInsert: 'Blok ekle',
};

/** The methods a template may name, by tool (their options in the tool catalog); a tool not here has none to choose. */
export const TEMPLATE_METHODS: Partial<Record<TemplateTool, readonly string[]>> = { circle: ['2N', '3N', 'TTY', 'TTT'] };

/** The layer a template's objects go on: found by its name, opened under its path with this look when the drawing lacks it. */
export interface TemplateLayer {
  /** The groups above it, from the top (“Kadastro”); empty: at the top. */
  readonly path: readonly string[];
  readonly name: string;
  /** Its look when it is opened; absent parts are a new layer's. */
  readonly color?: string;
  readonly lineType?: LineType;
  readonly lineWeight?: number;
}

export interface ObjectTemplate {
  readonly tool: TemplateTool;
  /** The tool's method (Dikdörtgen's “3 nokta”, Daire's “2 nokta”); absent: the tool's first. */
  readonly method?: string;
  readonly layer: TemplateLayer;
  /** The objects' own colour and line weight; absent: their layer's. */
  readonly color?: string;
  readonly lineWeight?: number;
  /** A library symbol's id, drawn instead of the layer's style. */
  readonly symbol?: string;
  /** Attributes written with each object, by name. */
  readonly attrs?: Readonly<Record<string, string>>;
  /** The text shown beside each object. */
  readonly label?: string;
  /** A point template's first name (it goes up with each point, as Nokta's Ad) and code (docs/adr/0152). */
  readonly point?: { readonly name?: string; readonly code?: string };
  /** A text template's height (metres on the ground), alignment and mask. */
  readonly text?: { readonly height: number; readonly align?: TextAlign; readonly mask?: boolean };
  /** A block template's block, by name. */
  readonly block?: string;
}

const LINE_TYPES: readonly LineType[] = ['continuous', 'dashed', 'dashdot', 'dotted'];
/** A colour: hex with or without alpha, or a theme token (the style file's rule). */
const COLOR = /^(#[0-9a-f]{6}([0-9a-f]{2})?|ink|paper|fg|fg-dim)$/i;
/** Empty or only white space, as Unicode's White_Space has it (the product commands' `isBlank`). */
const blank = (text: string) => /^[\t-\r \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]*$/.test(text);

type O = Record<string, unknown>;
const isObj = (v: unknown): v is O => !!v && typeof v === 'object' && !Array.isArray(v);
const weightOk = (v: unknown) => typeof v === 'number' && Number.isFinite(v) && v >= 0 && v <= MAX_LINE_WEIGHT;

/**
 * What is wrong with a template, each problem with where it is (`where`: “öğe 3 (Parsel sınırı)”), in the order the fields
 * are read; none for a template the app can draw with. A template read from a file or a library is untrusted data.
 */
export function templateIssues(template: unknown, where = 'şablon'): string[] {
  if (!isObj(template)) return [`${where}: şablon tanımı yok`];
  const out: string[] = [];
  const say = (text: string) => out.push(`${where}: ${text}`);
  const tool = template.tool as TemplateTool;
  const known = (TEMPLATE_TOOLS as readonly unknown[]).includes(tool);
  if (!known) say(`bilinmeyen araç “${String(template.tool)}”`);
  if (template.method !== undefined && known) {
    const methods = TEMPLATE_METHODS[tool] ?? [];
    if (!methods.length) say(`${TEMPLATE_TOOL_LABEL[tool]} aracının seçilecek yöntemi yok`);
    else if (!methods.includes(template.method as string)) say(`“${String(template.method)}” yöntemi ${TEMPLATE_TOOL_LABEL[tool]} aracında yok`);
  }
  const layer = template.layer;
  if (!isObj(layer)) say('katmanı yok');
  else {
    if (typeof layer.name !== 'string' || blank(layer.name)) say('katmanın adı boş');
    if (!Array.isArray(layer.path) || layer.path.some((p) => typeof p !== 'string')) say('katmanın yolu metin listesi olmalı');
    else if (layer.path.some((p) => blank(p as string))) say('katmanın yolunda boş ad var');
    if (layer.color !== undefined && (typeof layer.color !== 'string' || !COLOR.test(layer.color))) say(`katmanın rengi geçersiz “${String(layer.color)}”`);
    if (layer.lineType !== undefined && !LINE_TYPES.includes(layer.lineType as LineType)) say(`katmanın çizgi tipi bilinmiyor “${String(layer.lineType)}”`);
    if (layer.lineWeight !== undefined && !weightOk(layer.lineWeight)) say(`katmanın kalınlığı 0 ile ${MAX_LINE_WEIGHT} mm arasında olmalı`);
  }
  if (template.color !== undefined && (typeof template.color !== 'string' || !COLOR.test(template.color))) say(`renk geçersiz “${String(template.color)}”`);
  if (template.lineWeight !== undefined && !weightOk(template.lineWeight)) say(`kalınlık 0 ile ${MAX_LINE_WEIGHT} mm arasında olmalı`);
  if (template.symbol !== undefined && (typeof template.symbol !== 'string' || blank(template.symbol))) say('sembolün kimliği boş');
  if (template.attrs !== undefined) {
    if (!isObj(template.attrs)) say('öznitelikler ad ve metin değer olmalı');
    else
      // By name, as the desktop's JSON map holds them, so both say them in the same order.
      for (const [name, value] of Object.entries(template.attrs).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
        if (blank(name)) say('öznitelik adı boş');
        if (typeof value !== 'string') say(`“${name}” özniteliğinin değeri metin olmalı`);
      }
  }
  if (template.label !== undefined && typeof template.label !== 'string') say('etiket metin olmalı');
  if (template.point !== undefined) {
    if (tool !== 'point') say('ad ve kod yalnız nokta şablonunda olur');
    else if (!isObj(template.point) || (template.point.name !== undefined && typeof template.point.name !== 'string') || (template.point.code !== undefined && typeof template.point.code !== 'string'))
      say('noktanın adı ve kodu metin olmalı');
  }
  if (template.text !== undefined) {
    if (tool !== 'text') say('yazı bilgisi yalnız yazı şablonunda olur');
    else if (!isObj(template.text)) say('yazı bilgisi yok');
    else {
      const h = template.text.height;
      if (typeof h !== 'number' || !Number.isFinite(h) || !(h > 0)) say('yazının yüksekliği sıfırdan büyük olmalı');
      if (template.text.align !== undefined && !TEXT_ALIGNS.includes(template.text.align as TextAlign)) say(`bilinmeyen hiza “${String(template.text.align)}”`);
      if (template.text.mask !== undefined && typeof template.text.mask !== 'boolean') say('zemin açık ya da kapalı olmalı');
    }
  }
  if (tool === 'blockInsert') {
    if (typeof template.block !== 'string' || blank(template.block)) say('blok şablonunun bloğu yok');
  } else if (template.block !== undefined) say('blok yalnız blok şablonunda olur');
  return out;
}
