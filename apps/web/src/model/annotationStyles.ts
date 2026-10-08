/**
 * Named text and dimension styles (docs/adr/0183): a style is a preset the project keeps (`settings.textStyles`,
 * `settings.dimensionStyles`); a text or a dimension follows one by its id and carries the style's values in its own
 * fields (a text's face, a dimension's look), so whatever draws, picks or writes it reads the object alone. The rules
 * are the contract's (crates/shared/contracts/src/annotation.rs) word for word; both hold to the shared cases
 * fixtures/text/v1/styles.json (scripts/fixtures/annotation_style_cases.py).
 */
import type { DimensionArrow } from '../contracts/generated/DimensionArrow';
import type { DimensionLook } from '../contracts/generated/DimensionLook';
import type { DimensionStyleDef } from '../contracts/generated/DimensionStyleDef';
import type { DimensionTextPlace } from '../contracts/generated/DimensionTextPlace';
import type { TextFace } from '../contracts/generated/TextFace';
import type { TextStyleDef } from '../contracts/generated/TextStyleDef';
import { widthFactorOk } from './entities';

/**
 * A text's face (its style, typeface, bold, italic and slant) and a dimension's look (its style, arrowheads, sizes
 * times its value's height, the value's place, decimals, unit, affixes and typeface): the contract's own types.
 */
export type { DimensionArrow, DimensionLook, DimensionStyleDef, DimensionTextPlace, TextFace, TextStyleDef };

export const MAX_OBLIQUE = 85;
export const DEFAULT_TICK = 0.6;
export const DEFAULT_ARROW = 1;
export const DEFAULT_EXT_OFFSET = 0.5;
export const DEFAULT_EXT_BEYOND = 0.5;
export const DEFAULT_TEXT_GAP = 0.35;
export const MAX_DIMENSION_RATIO = 100;
export const STANDARD_DIMENSION_HEIGHT_MM = 2.5;
export const MAX_DIMENSION_DECIMALS = 8;
export const MAX_AFFIX = 32;
export const MAX_STYLE_NAME = 64;
export const MAX_STYLE_MM = 1000;
/** The name the styleless look goes by; no style takes it. */
export const STANDARD_STYLE = 'Standart';

/** Every arrowhead a dimension may name, in the contract's order (the typed columns number them so); absent: ticks. */
export const DIMENSION_ARROWS: readonly DimensionArrow[] = ['closed', 'open', 'dot', 'none'];

/** The face's fields, in the contract's order. */
export const FACE_FIELDS = ['textStyle', 'font', 'bold', 'italic', 'oblique'] as const;
/** The look's fields, in the contract's order; its lines last (docs/adr/0205 §6). */
export const LOOK_FIELDS = [
  'dimStyle',
  'arrow',
  'arrowSize',
  'extOffset',
  'extBeyond',
  'textGap',
  'textPlace',
  'decimals',
  'unit',
  'prefix',
  'suffix',
  'font',
  'dimLineColor',
  'dimLineWeight',
  'dimLineType',
  'extColor',
  'extWeight',
  'extLineType',
  'textColor',
] as const;
/** The look's line fields (docs/adr/0205 §6): `.kcad` schema 30. */
export const LINE_FIELDS = ['dimLineColor', 'dimLineWeight', 'dimLineType', 'extColor', 'extWeight', 'extLineType', 'textColor'] as const;
/** The heaviest an object's (and a dimension line's) weight may be, mm (the contract's `MAX_LINE_WEIGHT`). */
const MAX_LINE_WEIGHT = 100;
const HEX = /^#[0-9A-Fa-f]{6}$/;

/** Whether a dimension line's weight may be written (the contract's `line_weight_holds`). */
export const lineWeightHolds = (w: number): boolean => Number.isFinite(w) && w >= 0 && w <= MAX_LINE_WEIGHT;

/** Whether a look or a dimension style has a line field (`.kcad` schema 30). */
export const hasLines = (l: DimensionLook | DimensionStyleDef): boolean => LINE_FIELDS.some((k) => l[k] !== undefined);

/** What is wrong with the line colours and weights (the contract's `line_problem`): the field and the refusal's words. */
function lineProblem(l: DimensionLook | DimensionStyleDef): [string, string] | null {
  for (const [field, what, c] of [
    ['dimLineColor', 'ölçü çizgisinin rengi', l.dimLineColor],
    ['extColor', 'uzatma çizgilerinin rengi', l.extColor],
    ['textColor', 'değerinin rengi', l.textColor],
  ] as const)
    if (c !== undefined && !HEX.test(c)) return [field, `Ölçünün ${what} #RRGGBB biçiminde olmalı; “${c}” verildi. Rengi #RRGGBB olarak verin ya da alanı kaldırın (nesnenin rengi).`];
  for (const [field, what, w] of [
    ['dimLineWeight', 'ölçü çizgisinin kalınlığı', l.dimLineWeight],
    ['extWeight', 'uzatma çizgilerinin kalınlığı', l.extWeight],
  ] as const)
    if (w !== undefined && !lineWeightHolds(w)) return [field, `Ölçünün ${what} kâğıtta 0 ile ${MAX_LINE_WEIGHT} mm arasında olmalı; ${w} verildi. Bu aralıkta verin ya da alanı kaldırın (kılcal).`];
  return null;
}

const control = (s: string): boolean => [...s].some((c) => c.charCodeAt(0) < 32 || (c.charCodeAt(0) >= 0x7f && c.charCodeAt(0) <= 0x9f));

/** Whether `o` may be a text's slant: finite, not 0, less than MAX_OBLIQUE either way. */
export const obliqueHolds = (o: number): boolean => Number.isFinite(o) && o !== 0 && Math.abs(o) < MAX_OBLIQUE;

/** What is wrong with a text's face: the field and the refusal's words; null when it may be written. */
export function faceProblem(f: TextFace): [string, string] | null {
  if (f.textStyle === '') return ['textStyle', 'Yazının stil kimliği boş. Stilin kimliğini verin ya da alanı kaldırın (Standart).'];
  if (f.font === undefined) {
    const field = f.bold ? 'bold' : f.italic ? 'italic' : f.oblique !== undefined ? 'oblique' : null;
    if (field)
      return [
        field,
        'Kalın, eğik ve yatık yazı bir yazı tipiyle olur; yazının yazı tipi yok. Yazıya bir yazı tipi verin ya da bu alanları kaldırın (yazı projenin yazı tipiyle çizilir).',
      ];
  }
  if (f.oblique !== undefined && !obliqueHolds(f.oblique))
    return ['oblique', `Yazının eğikliği −${MAX_OBLIQUE} ile ${MAX_OBLIQUE} derece arasında ve sıfırdan farklı olmalı; ${f.oblique} verildi. Eğikliği bu aralıkta verin ya da alanı kaldırın (dik).`];
  return null;
}

/** Why a prefix or a suffix may not be written; null when it may. */
function affixProblem(s: string): string | null {
  const n = [...s].length;
  if (n === 0) return 'boş';
  if (n > MAX_AFFIX) return `${n} harf; en çok ${MAX_AFFIX}`;
  return control(s) ? 'satır sonu ya da denetim karakteri var' : null;
}

/** What is wrong with a dimension's look: the field and the refusal's words; null when it may be written. */
export function lookProblem(l: DimensionLook): [string, string] | null {
  if (l.dimStyle === '') return ['dimStyle', 'Ölçünün stil kimliği boş. Stilin kimliğini verin ya da alanı kaldırın (Standart).'];
  const sizes: [string, string, number | undefined, boolean][] = [
    ['arrowSize', 'ok boyu', l.arrowSize, true],
    ['extOffset', 'uzatma çizgisinin boşluğu', l.extOffset, false],
    ['extBeyond', 'uzatma çizgisinin aşması', l.extBeyond, false],
    ['textGap', 'değerin çizgiden yüksekliği', l.textGap, false],
  ];
  for (const [field, what, v, positive] of sizes)
    if (v !== undefined && !(Number.isFinite(v) && v <= MAX_DIMENSION_RATIO && (positive ? v > 0 : v >= 0)))
      return [
        field,
        `Ölçünün ${what} değer yüksekliğinin katıdır: ${positive ? 'sıfırdan büyük' : '0 ya da büyük'}, en çok ${MAX_DIMENSION_RATIO} olmalı; ${v} verildi. Bu aralıkta verin ya da alanı kaldırın (Standart'ınki).`,
      ];
  if (l.decimals !== undefined && l.decimals > MAX_DIMENSION_DECIMALS)
    return ['decimals', `Ölçünün basamak sayısı en çok ${MAX_DIMENSION_DECIMALS}; ${l.decimals} verildi. Daha az basamak verin ya da alanı kaldırın (projenin basamakları).`];
  for (const [field, what, v] of [
    ['prefix', 'öneki', l.prefix],
    ['suffix', 'soneki', l.suffix],
  ] as const) {
    const why = v === undefined ? null : affixProblem(v);
    if (why) return [field, `Ölçünün ${what} yazılamaz: ${why}. Tek satır, en çok ${MAX_AFFIX} harf verin ya da alanı kaldırın.`];
  }
  return lineProblem(l);
}

/** The tables' name rule, in the words of `kind` (“yazı stili”); null when the names hold. */
function namesProblem(kind: string, styles: readonly { id: string; name: string }[]): string | null {
  const ids: string[] = [];
  const names: string[] = [];
  for (const { id, name } of styles) {
    if (id === '') return `${kind} kimliği boş`;
    if (ids.includes(id)) return `“${id}” kimlikli ${kind} iki kez var`;
    const trimmed = name.trim();
    if (trimmed === '') return `${kind} adı boş`;
    if (trimmed !== name) return `“${name}” ${kind} adının başında ya da sonunda boşluk var`;
    if ([...trimmed].length > MAX_STYLE_NAME) return `“${trimmed}” ${kind} adı ${MAX_STYLE_NAME} harften uzun`;
    if (control(trimmed)) return `“${trimmed}” ${kind} adında satır sonu ya da denetim karakteri var`;
    const folded = trimmed.toLowerCase();
    if (folded === STANDARD_STYLE.toLowerCase()) return `“${trimmed}” adı Standart'ındır; ${kind} başka bir ad almalı`;
    if (names.includes(folded)) return `“${trimmed}” adlı ${kind} iki kez var`;
    ids.push(id);
    names.push(folded);
  }
  return null;
}

const mmHolds = (mm: number, positive: boolean): boolean => Number.isFinite(mm) && mm <= MAX_STYLE_MM && (positive ? mm > 0 : mm >= 0);

/** What is wrong with a text style's values (its name aside); null when they hold. */
export function textStyleProblem(s: TextStyleDef): string | null {
  const name = s.name;
  if (s.oblique !== undefined && !obliqueHolds(s.oblique))
    return `“${name}” yazı stilinin eğikliği ${s.oblique}; −${MAX_OBLIQUE} ile ${MAX_OBLIQUE} arasında ve sıfırdan farklı olmalı`;
  if (s.height !== undefined && !mmHolds(s.height, true)) return `“${name}” yazı stilinin yüksekliği ${s.height} mm; sıfırdan büyük, en çok ${MAX_STYLE_MM} olmalı`;
  if (s.widthFactor !== undefined && !(widthFactorOk(s.widthFactor) && s.widthFactor !== 1))
    return `“${name}” yazı stilinin genişlik çarpanı ${s.widthFactor}; sıfırdan büyük, en çok 100 ve 1'den farklı olmalı (1 yazılmaz)`;
  if (s.fontFile === '') return `“${name}” yazı stilinin yazı tipi dosyası boş`;
  return null;
}

/** What is wrong with a dimension style's values (its name aside); null when they hold. */
export function dimensionStyleProblem(s: DimensionStyleDef): string | null {
  const name = s.name;
  if (!mmHolds(s.height, true)) return `“${name}” ölçü stilinin değer yüksekliği ${s.height} mm; sıfırdan büyük, en çok ${MAX_STYLE_MM} olmalı`;
  const sizes: [string, number | undefined, boolean][] = [
    ['ok boyu', s.arrowSize, true],
    ['uzatma çizgisinin boşluğu', s.extOffset, false],
    ['uzatma çizgisinin aşması', s.extBeyond, false],
    ['değerin çizgiden yüksekliği', s.textGap, false],
  ];
  for (const [what, v, positive] of sizes)
    if (v !== undefined && !(mmHolds(v, positive) && v / s.height <= MAX_DIMENSION_RATIO))
      return `“${name}” ölçü stilinin ${what} ${v} mm; ${positive ? 'sıfırdan büyük' : '0 ya da büyük'}, en çok ${MAX_STYLE_MM} ve değer yüksekliğinin ${MAX_DIMENSION_RATIO} katı olmalı`;
  if (s.decimals !== undefined && s.decimals > MAX_DIMENSION_DECIMALS) return `“${name}” ölçü stilinin basamak sayısı ${s.decimals}; en çok ${MAX_DIMENSION_DECIMALS} olmalı`;
  for (const [what, v] of [
    ['öneki', s.prefix],
    ['soneki', s.suffix],
  ] as const) {
    const why = v === undefined ? null : affixProblem(v);
    if (why) return `“${name}” ölçü stilinin ${what} yazılamaz: ${why}`;
  }
  const line = lineProblem(s);
  return line ? `“${name}” ölçü stili: ${line[1]}` : null;
}

export const textStylesProblem = (styles: readonly TextStyleDef[]): string | null =>
  namesProblem('yazı stili', styles) ?? styles.map(textStyleProblem).find((p) => p !== null) ?? null;

export const dimensionStylesProblem = (styles: readonly DimensionStyleDef[]): string | null =>
  namesProblem('ölçü stili', styles) ?? styles.map(dimensionStyleProblem).find((p) => p !== null) ?? null;

/** The text styles as a project keeps them: of those breaking the name rule against the ones before (or alone) none, and none whose values do not hold. */
export function sanitizedTextStyles(styles: readonly TextStyleDef[]): TextStyleDef[] {
  const out: TextStyleDef[] = [];
  for (const s of styles) if (textStylesProblem([...out, s]) === null) out.push(s);
  return out;
}

/** The dimension styles as a project keeps them, as sanitizedTextStyles. */
export function sanitizedDimensionStyles(styles: readonly DimensionStyleDef[]): DimensionStyleDef[] {
  const out: DimensionStyleDef[] = [];
  for (const s of styles) if (dimensionStylesProblem([...out, s]) === null) out.push(s);
  return out;
}

/** A text's fields a style sets: its face, width factor and height. */
export type TextLook = TextFace & { widthFactor?: number; height: number };

/** A text style's face. */
export function faceOf(s: TextStyleDef): TextFace {
  return { textStyle: s.id, font: s.font, ...(s.bold && { bold: true }), ...(s.italic && { italic: true }), ...(s.oblique !== undefined && { oblique: s.oblique }) };
}

const heightAt = (mm: number, plotScale: number): number => (mm / 1000) * plotScale;

/** A text's face fields and width factor left out: what a style replaces. */
function bare<T extends TextLook>(t: T): TextLook {
  const out: Record<string, unknown> = { ...t };
  for (const k of [...FACE_FIELDS, 'widthFactor']) delete out[k];
  return out as TextLook;
}

/**
 * A text after `style` is applied (null: Standart), at 1:plotScale (docs/adr/0183 §2): the style's face and width
 * factor; its height when the style fixes one. Standart takes the face and the width factor away, keeps the height.
 */
export function applyTextStyle(style: TextStyleDef | null, look: TextLook, plotScale: number): TextLook {
  const out = bare(look);
  if (!style) return out;
  return {
    ...out,
    ...faceOf(style),
    ...(style.widthFactor !== undefined && { widthFactor: style.widthFactor }),
    ...(style.height !== undefined && { height: heightAt(style.height, plotScale) }),
  };
}

/**
 * A text of `old` after the style became `next` (docs/adr/0183 §1): each field still holding the old style's value
 * takes the new one; a field of its own stays. The height moves when it is the old style's and the new one fixes one.
 */
export function followTextStyle(old: TextStyleDef, next: TextStyleDef, look: TextLook, plotScale: number): TextLook {
  const out: TextLook = { ...look };
  const was = faceOf(old);
  const now = faceOf(next);
  for (const k of ['font', 'oblique'] as const)
    if (look[k] === was[k]) {
      delete out[k];
      if (now[k] !== undefined) (out as Record<string, unknown>)[k] = now[k];
    }
  for (const k of ['bold', 'italic'] as const)
    if ((look[k] ?? false) === (was[k] ?? false)) {
      delete out[k];
      if (now[k]) out[k] = true;
    }
  if ((look.widthFactor ?? 1) === (old.widthFactor ?? 1)) {
    delete out.widthFactor;
    if (next.widthFactor !== undefined) out.widthFactor = next.widthFactor;
  }
  if (old.height !== undefined && next.height !== undefined && look.height === heightAt(old.height, plotScale)) out.height = heightAt(next.height, plotScale);
  return out;
}

/** A dimension style's look: its sizes as ratios of its height, a default ratio left out. */
export function lookOf(s: DimensionStyleDef): DimensionLook {
  const tick = s.arrow === undefined;
  const ratio = (mm: number | undefined, standard: number): number | undefined => {
    if (mm === undefined) return undefined;
    const r = mm / s.height;
    return r === standard ? undefined : r;
  };
  const out: DimensionLook = { dimStyle: s.id };
  if (s.arrow !== undefined) out.arrow = s.arrow;
  const sizes: [keyof DimensionLook, number | undefined][] = [
    ['arrowSize', ratio(s.arrowSize, tick ? DEFAULT_TICK : DEFAULT_ARROW)],
    ['extOffset', ratio(s.extOffset, DEFAULT_EXT_OFFSET)],
    ['extBeyond', ratio(s.extBeyond, DEFAULT_EXT_BEYOND)],
    ['textGap', ratio(s.textGap, DEFAULT_TEXT_GAP)],
  ];
  for (const [k, v] of sizes) if (v !== undefined) (out as Record<string, unknown>)[k] = v;
  if (s.textPlace !== undefined) out.textPlace = s.textPlace;
  if (s.decimals !== undefined) out.decimals = s.decimals;
  if (s.unit !== undefined) out.unit = s.unit;
  if (s.prefix !== undefined) out.prefix = s.prefix;
  if (s.suffix !== undefined) out.suffix = s.suffix;
  if (s.font !== undefined) out.font = s.font;
  for (const k of LINE_FIELDS) if (s[k] !== undefined) (out as Record<string, unknown>)[k] = s[k];
  return out;
}

/**
 * A dimension after `style` is applied (null: Standart), at 1:plotScale (docs/adr/0183 §3): its look and height.
 * Standart's height is the project's dimension height `standardMm` (docs/adr/0205 §1).
 */
export function applyDimensionStyle(style: DimensionStyleDef | null, plotScale: number, standardMm: number = STANDARD_DIMENSION_HEIGHT_MM): { look: DimensionLook; height: number } {
  return style ? { look: lookOf(style), height: heightAt(style.height, plotScale) } : { look: {}, height: heightAt(standardMm, plotScale) };
}

/** A dimension of `old` after the style became `next`, as followTextStyle: its look's fields and its height. */
export function followDimensionStyle(old: DimensionStyleDef, next: DimensionStyleDef, look: DimensionLook, height: number, plotScale: number): { look: DimensionLook; height: number } {
  const was = lookOf(old);
  const now = lookOf(next);
  const out: DimensionLook = { ...(look.dimStyle !== undefined && { dimStyle: look.dimStyle }) };
  for (const k of LOOK_FIELDS) {
    if (k === 'dimStyle') continue;
    const v = look[k] === was[k] ? now[k] : look[k];
    if (v !== undefined) (out as Record<string, unknown>)[k] = v;
  }
  return { look: out, height: height === heightAt(old.height, plotScale) ? heightAt(next.height, plotScale) : height };
}

/** A text's own face: its face fields as an object (absent ones left out). */
export function faceOfText(t: TextFace): TextFace {
  const out: TextFace = {};
  for (const k of FACE_FIELDS) if (t[k] !== undefined) (out as Record<string, unknown>)[k] = t[k];
  return out;
}

/** A dimension's own look: its look fields as an object (absent ones left out). */
export function lookOfDimension(d: DimensionLook): DimensionLook {
  const out: DimensionLook = {};
  for (const k of LOOK_FIELDS) if (d[k] !== undefined) (out as Record<string, unknown>)[k] = d[k];
  return out;
}

/** The tangent of a text's slant, what its letters' x moves per unit of y: 0 without a typeface or a slant. */
export const leanOf = (f: TextFace): number => (f.font !== undefined && f.oblique !== undefined ? Math.tan((f.oblique * Math.PI) / 180) : 0);
