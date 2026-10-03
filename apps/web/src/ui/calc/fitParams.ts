import type { AppContext } from '../../app/context';
import type { Transform } from '../../contracts/generated/Transform';
import type { Vec2 } from '../../model/geometry';
import { fitScaleTurn } from '../../model/ops/fit';
import { readNumber, resolvePoint, summaryLine } from './common';

/**
 * Vektör oturtma's Parametrelerle (docs/adr/0156 §7): the transform given by its numbers instead of control points: a
 * base point, the Y (east) and X (north) scales, a counter-clockwise turn and a shift, an affine about the base whose
 * linear part is the core's `fitScaleTurn`. An empty scale counts as 1, an empty turn or shift as 0; a scale cannot be
 * 0 (below 0 it mirrors). With equal scales the transform is a similarity and every kind keeps its shape. The
 * desktop's is `apps/desktop/src/calc/fit/params.rs`.
 */

/** How the transform is given. */
export type Method = 'points' | 'parameters';

export const METHODS: { value: Method; label: string }[] = [
  { value: 'points', label: 'Kontrol noktaları' },
  { value: 'parameters', label: 'Parametrelerle' },
];

export const METHOD_HINT: Record<Method, string> = {
  points: 'Ortak noktalardan en küçük kareler ile; artıklar ve m0 görünür.',
  parameters: "Taban noktasına göre Y ve X ölçeği, dönüklük ve öteleme; Netcad'in XY Yönünde Ölçekle'si gibi.",
};

/** Parametrelerle's numbers, by their state keys. */
export type Param = 'scaleY' | 'scaleX' | 'rotation' | 'shiftY' | 'shiftX';

/** The numbers in their fields' order: their names, and what an empty one counts as. */
export const PARAMS: { key: Param; label: string; empty: number; scale?: boolean }[] = [
  { key: 'scaleY', label: 'Y ölçeği', empty: 1, scale: true },
  { key: 'scaleX', label: 'X ölçeği', empty: 1, scale: true },
  { key: 'rotation', label: 'Dönüklük', empty: 0 },
  { key: 'shiftY', label: 'Öteleme ΔY', empty: 0 },
  { key: 'shiftX', label: 'Öteleme ΔX', empty: 0 },
];

/** The line under the fields: in which order the transform works. */
export const PARAMS_HINT =
  'Önce Y (sağa) ve X (yukarı) yönünde ölçeklenir, sonra saat yönünün tersine döner, sonra ötelenir. Eksi ölçek aynalar; ölçekler eşitse her nesne biçimini korur.';

/** What is typed: the base point (a name or Y,X) and the numbers. */
export type TypedParams = { base: { text: string } } & Record<Param, string>;

/** Parametrelerle read: the base, the scales, the turn (radians) and the shift. */
export interface Params {
  base: Vec2;
  scaleY: number;
  scaleX: number;
  rotation: number;
  shift: Vec2;
}

/** A number's field label, with its unit. */
export function paramLabel(ctx: AppContext, p: (typeof PARAMS)[number]): string {
  const unit = ctx.format.angleUnitLabel === '°' ? '°' : 'g';
  return p.key === 'rotation' ? `${p.label} (${unit})` : p.scale ? p.label : `${p.label} (m)`;
}

/** The numbers read and the base resolved (the turn in radians), or what is wrong, in the fields' order. */
export function readParams(ctx: AppContext, typed: TypedParams): Params | HTMLElement[] {
  const problems: HTMLElement[] = [];
  const known = resolvePoint(ctx, typed.base.text);
  if (!known) problems.push(summaryLine('info', 'Taban noktasını yazın ya da çizimden seçin.'));
  else if ('error' in known) problems.push(summaryLine('warn', `Taban noktası: ${known.error}`));
  const values = { scaleY: 1, scaleX: 1, rotation: 0, shiftY: 0, shiftX: 0 };
  for (const p of PARAMS) {
    const v = readNumber(typed[p.key]);
    if (v !== null && !Number.isFinite(v)) problems.push(summaryLine('warn', `${p.label} sayı olmalı.`));
    else if (p.scale && v === 0) problems.push(summaryLine('warn', `${p.label} sıfır olamaz; eksi ölçek aynalar.`));
    else values[p.key] = v ?? p.empty;
  }
  if (problems.length || !known || 'error' in known) return problems;
  return {
    base: { x: known.p.x, y: known.p.y },
    scaleY: values.scaleY,
    scaleX: values.scaleX,
    rotation: ctx.format.angleFromTyped(values.rotation),
    shift: { x: values.shiftY, y: values.shiftX },
  };
}

/** Where the base lands. */
function landing(p: Params): Vec2 {
  return { x: p.base.x + p.shift.x, y: p.base.y + p.shift.y };
}

/** The summary: the numbers, where the base lands, and what the objects become. */
export function paramsSummary(ctx: AppContext, p: Params): HTMLElement[] {
  // The Hesap windows are in metres, as typed (docs/adr/0165 §2).
  const format = ctx.format.metric();
  const mirrored = p.scaleY * p.scaleX < 0 ? ' Ölçeklerden biri eksi: nesneler aynalanır.' : '';
  return [
    summaryLine('ok', `Y ölçeği ${String(p.scaleY)}, X ölçeği ${String(p.scaleX)}, dönüklük ${format.angle(p.rotation)}, öteleme ΔY ${format.length(p.shift.x)}, ΔX ${format.length(p.shift.y)}.`),
    summaryLine('info', `Taban noktası ${format.point(p.base)} → ${format.point(landing(p))}.`),
    summaryLine(
      'info',
      p.scaleY === p.scaleX
        ? 'Ölçekler eşit: benzerlik; her nesne biçimini korur.'
        : `Ölçekler farklı: afin dönüşüm; daireler ve yaylar elips olur, yazılar, bloklar ve ölçüler yerinde biçimini korur.${mirrored}`,
    ),
  ];
}

/** `cad.entities.transform`'s affine about the base (its centred form). */
export function paramsTransform(p: Params): Transform {
  return { kind: 'affine', from: p.base, to: landing(p), m: fitScaleTurn(p.scaleY, p.scaleX, p.rotation) };
}

/** The report: the base, the numbers as read and the linear part, tab-separated. */
export function paramsReport(ctx: AppContext, title: string, p: Params, typed: TypedParams): string[][] {
  const unit = ctx.format.angleUnitLabel === '°' ? '°' : 'g';
  return [
    [title, 'Parametrelerle'],
    ['Taban Y', String(p.base.x), 'Taban X', String(p.base.y)],
    ['Y ölçeği', String(p.scaleY)],
    ['X ölçeği', String(p.scaleX)],
    [`Dönüklük (${unit})`, String(readNumber(typed.rotation) ?? 0)],
    ['Öteleme ΔY (m)', String(p.shift.x)],
    ['Öteleme ΔX (m)', String(p.shift.y)],
    ['Merkezli sayılar', ...fitScaleTurn(p.scaleY, p.scaleX, p.rotation).map(String)],
  ];
}
