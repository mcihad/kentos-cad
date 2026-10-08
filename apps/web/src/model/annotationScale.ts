/**
 * The project's annotation heights and how annotations follow the scale (docs/adr/0205). A project keeps seven heights
 * on paper (`settings.annotation`); a new annotation's height is its kind's at the plot scale (`paperHeight`). When
 * the plot scale or a kind's height changes, the annotations still at the old height take the new one
 * (`followAnnotationScale`); what was changed by hand stays. The rules are the contract's
 * (crates/shared/contracts/src/annotation_scale.rs) word for word, with the same expressions so the heights compare
 * exactly; both hold to fixtures/text/v1/scale.json (scripts/fixtures/annotation_scale_cases.py).
 */
import type { AnnotationHeights } from '../contracts/generated/AnnotationHeights';
import type { DimensionStyleDef, TextStyleDef } from './annotationStyles';
import type { Entity } from './entities';

export type { AnnotationHeights };

/** A kind of annotation with a height of its own, by its key in the settings. */
export type AnnotationKind = keyof AnnotationHeights & ('text' | 'leader' | 'dimension' | 'table' | 'coordinate' | 'station' | 'measure');

/** Every kind, in the settings' order. */
export const ANNOTATION_KINDS: readonly AnnotationKind[] = ['text', 'leader', 'dimension', 'table', 'coordinate', 'station', 'measure'];

/** Each kind's name as the interface says it. */
export const ANNOTATION_LABEL: Readonly<Record<AnnotationKind, string>> = {
  text: 'Yazı',
  leader: 'Kılavuz',
  dimension: 'Ölçü',
  table: 'Tablo',
  coordinate: 'Koordinat yazısı',
  station: 'Km yazısı',
  measure: 'Kenar ve köşe yazıları',
};

/** What each kind writes, for the settings' rows. */
export const ANNOTATION_NOTE: Readonly<Record<AnnotationKind, string>> = {
  text: 'Yazı, çok satırlı yazı, eğri boyunca yazı, metin dosyası, blok öznitelikleri',
  leader: 'Kılavuzun notu ve oku',
  dimension: 'Bütün ölçüler ve Hızlı ölçü; Standart ölçü stili',
  table: 'Tablo ekle',
  coordinate: 'Koordinat yaz ve çizelgesi',
  station: 'Km yaz',
  measure: 'İşlemler: kenar uzunlukları, köşe numaraları',
};

/** Each kind's height when a project names none (mm): the tools' own before the project had heights. */
export const ANNOTATION_DEFAULT_MM: Readonly<Record<AnnotationKind, number>> = {
  text: 2.5,
  leader: 2.5,
  dimension: 2.5,
  table: 2.5,
  coordinate: 2,
  station: 2,
  measure: 2,
};

/** The tallest an annotation's height may be on paper, mm. */
export const MAX_ANNOTATION_MM = 100;

/** İşlemler's edge lengths and corner numbers (their `Tür`): they follow Kenar ve köşe yazıları, any other text Yazı. */
export const MEASURE_TEXT_KINDS: readonly string[] = ['Kenar ölçüsü', 'Köşe noktası'];

/** An annotation `mm` high on paper at 1:`scale`, metres: the expression every tool writes with. */
export const paperHeight = (mm: number, scale: number): number => (mm / 1000) * scale;

/** Whether `mm` may be an annotation's height: finite, over 0, at most MAX_ANNOTATION_MM. */
export const annotationMmHolds = (mm: number): boolean => Number.isFinite(mm) && mm > 0 && mm <= MAX_ANNOTATION_MM;

/** A kind's height, mm: the one given, else its default. */
export const annotationMm = (heights: AnnotationHeights | undefined, kind: AnnotationKind): number => heights?.[kind] ?? ANNOTATION_DEFAULT_MM[kind];

/** What is wrong with the heights: the key and the refusal's words; null when they hold. */
export function annotationHeightsProblem(h: AnnotationHeights): [AnnotationKind, string] | null {
  for (const kind of ANNOTATION_KINDS) {
    const mm = h[kind];
    if (mm !== undefined && !annotationMmHolds(mm)) return [kind, `${ANNOTATION_LABEL[kind]} yüksekliği kâğıtta sıfırdan büyük, en çok ${MAX_ANNOTATION_MM} mm olmalı; ${mm} verildi.`];
  }
  return null;
}

/** As a project keeps them: heights that do not hold and heights equal to their default left out; undefined when none is left. */
export function sanitizedAnnotationHeights(h: AnnotationHeights | undefined): AnnotationHeights | undefined {
  if (!h) return undefined;
  const out: AnnotationHeights = {};
  for (const kind of ANNOTATION_KINDS) {
    const mm = h[kind];
    if (mm !== undefined && annotationMmHolds(mm) && mm !== ANNOTATION_DEFAULT_MM[kind]) out[kind] = mm;
  }
  return Object.keys(out).length ? out : undefined;
}

/** A change of the plot scale or of the heights (docs/adr/0205 §3): what they were and what they are. */
export interface ScaleChange {
  readonly fromScale: number;
  readonly toScale: number;
  readonly from: AnnotationHeights | undefined;
  readonly to: AnnotationHeights | undefined;
}

/** Whether nothing an annotation's height depends on changed. */
export const scaleChangeIsNone = (c: ScaleChange): boolean => c.fromScale === c.toScale && ANNOTATION_KINDS.every((k) => annotationMm(c.from, k) === annotationMm(c.to, k));

type Span = readonly [number, number];
const kindSpan = (c: ScaleChange, kind: AnnotationKind): Span => [paperHeight(annotationMm(c.from, kind), c.fromScale), paperHeight(annotationMm(c.to, kind), c.toScale)];
const styleSpan = (c: ScaleChange, mm: number): Span => [paperHeight(mm, c.fromScale), paperHeight(mm, c.toScale)];

/** The new height of one at `height` that was at `was` and follows to `now`; null when it stays. */
const moved = (height: number, [was, now]: Span): number | null => (height === was && now !== height ? now : null);

/**
 * `e` after the scale or the heights changed (docs/adr/0205 §3); null when it stays. A text follows its style's fixed
 * height when its style has one, else Yazı (İşlemler's edge lengths and corner numbers, by their `Tür`, Kenar ve köşe
 * yazıları); a multi-line text's box grows with it. A leader follows Kılavuz, a dimension its style's height
 * (Standart: Ölçü), a table its text style's fixed height or Tablo, its rows, columns and frame growing with it about
 * its corner. A linked text (docs/adr/0175) and any other kind stay.
 */
export function followAnnotationScale(e: Entity, change: ScaleChange, textStyles: readonly TextStyleDef[], dimensionStyles: readonly DimensionStyleDef[]): Entity | null {
  const fixed = (id: string | undefined): number | undefined => (id === undefined ? undefined : textStyles.find((s) => s.id === id)?.height);
  switch (e.kind) {
    case 'text': {
      if (e.labelOf !== undefined) return null;
      const mm = fixed(e.textStyle);
      const measure = MEASURE_TEXT_KINDS.includes(e.attrs['Tür'] ?? '');
      const now = moved(e.height, mm !== undefined ? styleSpan(change, mm) : kindSpan(change, measure ? 'measure' : 'text'));
      if (now === null) return null;
      const factor = now / e.height;
      return { ...e, height: now, ...(e.boxWidth !== undefined && { boxWidth: e.boxWidth * factor }) };
    }
    case 'leader': {
      const now = moved(e.height, kindSpan(change, 'leader'));
      return now === null ? null : { ...e, height: now };
    }
    case 'dimension': {
      const style = e.dimStyle === undefined ? undefined : dimensionStyles.find((s) => s.id === e.dimStyle);
      const now = moved(e.height, style ? styleSpan(change, style.height) : kindSpan(change, 'dimension'));
      return now === null ? null : { ...e, height: now };
    }
    case 'table': {
      const mm = fixed(e.textStyle);
      const now = moved(e.height, mm !== undefined ? styleSpan(change, mm) : kindSpan(change, 'table'));
      if (now === null) return null;
      const factor = now / e.height;
      return {
        ...e,
        height: now,
        rows: e.rows.map((r) => r * factor),
        columns: e.columns.map((c) => c * factor),
        ...(e.frame !== undefined && { frame: e.frame * factor }),
      };
    }
    default:
      return null;
  }
}
