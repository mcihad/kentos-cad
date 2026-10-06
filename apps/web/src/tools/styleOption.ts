/**
 * Yazı ve ölçü stilleri in the tools (docs/adr/0183 §4): Yazı, Çok satırlı yazı, Metin dosyası yerleştir and every
 * dimension tool write in the style the session chose, Stil (S) in their prompt, its menu the project's styles and the
 * styles' window. A CAD project's interface only: a CBS project's tools write as they always did. Zincir ölçü and Baz
 * ölçü write in their base's style. The desktop's twin is `kentos_interaction::styles`.
 */
import type { AppContext } from '../app/context';
import { faceOf, lookOf, STANDARD_DIMENSION_HEIGHT_MM, STANDARD_STYLE, type DimensionLook, type DimensionStyleDef, type TextFace, type TextStyleDef } from '../model/annotationStyles';
import type { OptionChoice } from './Tool';

/** The styles the session chose, by their ids; null: Standart. A style the project no longer has is Standart. */
export const chosenStyles: { text: string | null; dimension: string | null } = { text: null, dimension: null };

/** The styles' windows' commands and their menu entries. */
export const TEXT_STYLES = 'style.textStyles';
export const DIMENSION_STYLES = 'style.dimensionStyles';
export const TEXT_STYLES_ENTRY = 'Yazı stilleri…';
export const DIMENSION_STYLES_ENTRY = 'Ölçü stilleri…';

/** Whether the tools show the styles: a CAD project's (docs/adr/0183 §4). */
export const stylesShown = (ctx: AppContext): boolean => ctx.doc.settings.workspace.value === 'cad';

/** The text style the session chose, when the project has it. */
export const textStyleNow = (ctx: AppContext): TextStyleDef | null => ctx.doc.settings.textStyle(chosenStyles.text ?? undefined);

/** The dimension style the session chose, when the project has it. */
export const dimensionStyleNow = (ctx: AppContext): DimensionStyleDef | null => ctx.doc.settings.dimensionStyle(chosenStyles.dimension ?? undefined);

/** The chosen style's name as the prompt writes it: Standart for none. */
export const textStyleName = (ctx: AppContext): string => textStyleNow(ctx)?.name ?? STANDARD_STYLE;
export const dimensionStyleName = (ctx: AppContext): string => dimensionStyleNow(ctx)?.name ?? STANDARD_STYLE;

/** Stil's menu: Standart and the styles, then the window. */
function choices(chosen: string | null, styles: readonly { id: string; name: string }[], entry: string, command: string, icon: string): OptionChoice[] {
  const known = styles.some((s) => s.id === chosen);
  return [
    { label: STANDARD_STYLE, typed: STANDARD_STYLE, checked: !known },
    ...styles.map((s) => ({ label: s.name, typed: s.name, checked: s.id === chosen })),
    { label: entry, typed: entry, icon, checked: false, command },
  ];
}

export const textStyleChoices = (ctx: AppContext): OptionChoice[] => choices(chosenStyles.text, ctx.doc.settings.textStyles.value, TEXT_STYLES_ENTRY, TEXT_STYLES, 'textStyle');

export const dimensionStyleChoices = (ctx: AppContext): OptionChoice[] =>
  choices(chosenStyles.dimension, ctx.doc.settings.dimensionStyles.value, DIMENSION_STYLES_ENTRY, DIMENSION_STYLES, 'dimensionStyle');

/** A typed name as a style's: trimmed, letters' case aside. */
const sameName = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

/**
 * A typed or chosen text style: the style (null for Standart), kept as the session's; undefined (said) for a name the
 * project has none of. The caller takes its height and width factor.
 */
export function takeTextStyle(ctx: AppContext, typed: string): TextStyleDef | null | undefined {
  if (sameName(typed, STANDARD_STYLE)) {
    chosenStyles.text = null;
    return null;
  }
  const s = ctx.doc.settings.textStyles.value.find((x) => sameName(x.name, typed));
  if (!s) {
    ctx.log.warn(`“${typed.trim()}” adlı yazı stili yok. Stili menüden seçin ya da adını yazın; yeni stil Yazı stilleri penceresinde tanımlanır.`);
    return undefined;
  }
  chosenStyles.text = s.id;
  return s;
}

/** A typed or chosen dimension style, as `takeTextStyle`: true when taken. */
export function takeDimensionStyle(ctx: AppContext, typed: string): boolean {
  if (sameName(typed, STANDARD_STYLE)) {
    chosenStyles.dimension = null;
    return true;
  }
  const s = ctx.doc.settings.dimensionStyles.value.find((x) => sameName(x.name, typed));
  if (!s) {
    ctx.log.warn(`“${typed.trim()}” adlı ölçü stili yok. Stili menüden seçin ya da adını yazın; yeni stil Ölçü stilleri penceresinde tanımlanır.`);
    return false;
  }
  chosenStyles.dimension = s.id;
  return true;
}

/** The face a new text has: its style's in a CAD project, else none. */
export function textFaceNow(ctx: AppContext): TextFace {
  const s = stylesShown(ctx) ? textStyleNow(ctx) : null;
  return s ? faceOf(s) : {};
}

/** A new multi-line text's width factor: its style's in a CAD project (Çok satırlı yazı has no Genişlik), else none. */
export function textWidthFactorNow(ctx: AppContext): number | undefined {
  return stylesShown(ctx) ? textStyleNow(ctx)?.widthFactor : undefined;
}

/** A new dimension's look and value height: its style's in a CAD project, else none and `standard` (metres). */
export function dimensionLookNow(ctx: AppContext, standard: number): { look: DimensionLook; height: number } {
  const s = stylesShown(ctx) ? dimensionStyleNow(ctx) : null;
  return s ? { look: lookOf(s), height: (s.height / 1000) * ctx.doc.settings.plotScale.value } : { look: {}, height: standard };
}

/** Standart's value height at the project's scale, metres (the tools' 2.5 mm). */
export const standardDimensionHeight = (ctx: AppContext): number => (STANDARD_DIMENSION_HEIGHT_MM / 1000) * ctx.doc.settings.plotScale.value;
