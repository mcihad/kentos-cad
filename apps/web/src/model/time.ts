// Time values and the time slider's arithmetic (docs/adr/0210 §3–§5): the core's `time` module
// (crates/shared/geometry-core/src/time.rs) through the call table; a layer's objects' times in one typed call.

import { coreTimeLayer, coreTimeLayerMask, op } from '../wasm/core';

/** A step's unit, as the core names it. */
export type TimeUnit = 'second' | 'minute' | 'hour' | 'day' | 'week' | 'month' | 'year';
/** The units in the core's order (`Unit::ALL`). */
export const TIME_UNITS: readonly TimeUnit[] = ['second', 'minute', 'hour', 'day', 'week', 'month', 'year'];
/** A unit's Turkish word (“1 ay”, “2 yıl”). */
export const UNIT_WORD: Record<TimeUnit, string> = { second: 'saniye', minute: 'dakika', hour: 'saat', day: 'gün', week: 'hafta', month: 'ay', year: 'yıl' };

/** A step of the slider: `n` (1–999) units. */
export interface TimeStep {
  n: number;
  unit: TimeUnit;
}

/** What a time text reads as: nothing, a moment (milliseconds since 1970, UTC), or not a time. */
export type TimeRead = { kind: 'empty' } | { kind: 'moment'; t: number } | { kind: 'unreadable' };

/** The slider's window: a moment, or from `a` up to `b`. */
export type TimeWindow = { kind: 'instant'; a: number } | { kind: 'range'; a: number; b: number };

/** What a temporal layer's values give (Zaman ayarları' preview). */
export interface TimeSummary {
  timed: number;
  timeless: number;
  unreadable: number;
  extent: [number, number] | null;
}

const coreRead = op<(text: string) => [number, number]>('timeRead');
const coreAutoStep = op<(start: number, end: number) => [number, number]>('timeAutoStep');
const corePositions = op<(start: number, end: number, n: number, unit: TimeUnit) => [number, number] | null>('timePositions');
const corePosition = op<(anchor: number, n: number, unit: TimeUnit, k: number) => number>('timePosition');

export function readTime(text: string): TimeRead {
  const [kind, t] = coreRead(text);
  return kind === 1 ? { kind: 'moment', t } : kind === 0 ? { kind: 'empty' } : { kind: 'unreadable' };
}

/** A moment as it is written into an attribute: `YYYY-AA-GG` at midnight (or `dateOnly`), else with its time. */
export const writeTime = op<(t: number, dateOnly: boolean) => string>('timeWrite');
/** A moment as the interface shows it: `GG.AA.YYYY`, with the hour for a step under a day. */
export const showTime = op<(t: number, unit: TimeUnit) => string>('timeShow');
const coreShowWindow = op<(a: number, b: number, unit: TimeUnit) => string>('timeShowWindow');
const coreShowEnds = op<(first: number, last: number, unit: TimeUnit) => [string, string]>('timeShowEnds');
/** What the slider's position shows: an instant's moment, or a period's two (its date once when inside one day). */
export function showWindow(w: TimeWindow, unit: TimeUnit): string {
  return w.kind === 'instant' ? coreShowWindow(w.a, Number.NaN, unit) : coreShowWindow(w.a, w.b, unit);
}
/** The slider's ends: under a day's step their clocks within one day, else their dates; a day or more as shown. */
export const showEnds = coreShowEnds;
/** A moment rounded down to its unit. */
export const floorTime = op<(t: number, unit: TimeUnit) => number>('timeFloor');

/** The step the slider takes by itself: the first of year, month, day, hour, minute and second giving five steps or more. */
export function autoStep(extent: readonly [number, number]): TimeStep {
  const [n, unit] = coreAutoStep(extent[0], extent[1]);
  return { n, unit: TIME_UNITS[unit] ?? 'day' };
}

/** The slider's positions: the anchor and the last position's number (0 … `last`); null past 100 000 positions. */
export function timePositions(extent: readonly [number, number], step: TimeStep): { anchor: number; last: number } | null {
  const r = corePositions(extent[0], extent[1], step.n, step.unit);
  return r ? { anchor: r[0], last: r[1] } : null;
}

/** The `k`-th position from the anchor, months and years by the calendar. */
export function timePosition(anchor: number, step: TimeStep, k: number): number {
  return corePosition(anchor, step.n, step.unit, k);
}

/** The window at position `k`: its moment, or up to the next position. */
export function windowAt(anchor: number, step: TimeStep, k: number, ranged: boolean): TimeWindow {
  const a = timePosition(anchor, step, k);
  return ranged ? { kind: 'range', a, b: timePosition(anchor, step, k + 1) } : { kind: 'instant', a };
}

/** Each object's start and end values as one text and their lengths (−1: the object lacks it). */
function packed(values: readonly (readonly [string | undefined, string | undefined])[]): { texts: string; lens: Int32Array } {
  const texts: string[] = [];
  const lens = new Int32Array(values.length * 2);
  values.forEach(([s, e], i) => {
    lens[2 * i] = s === undefined ? -1 : s.length;
    lens[2 * i + 1] = e === undefined ? -1 : e.length;
    if (s !== undefined) texts.push(s);
    if (e !== undefined) texts.push(e);
  });
  return { texts: texts.join(''), lens };
}

/** A layer's objects' start and end values as its time setting names them. */
export function timeValues(rule: { start: string; end?: string | null }, attrs: readonly Readonly<Record<string, string>>[]): [string | undefined, string | undefined][] {
  return attrs.map((a) => [a[rule.start], rule.end != null ? a[rule.end] : undefined]);
}

/** Which of a temporal layer's objects show in `w`: per object, a timeless one always. */
export function shownIn(ranged: boolean, cumulative: boolean, values: readonly (readonly [string | undefined, string | undefined])[], w: TimeWindow): Uint8Array {
  const { texts, lens } = packed(values);
  return w.kind === 'instant' ? coreTimeLayerMask(ranged, cumulative, texts, lens, 1, w.a, w.a) : coreTimeLayerMask(ranged, cumulative, texts, lens, 2, w.a, w.b);
}

/**
 * The times of a temporal layer's objects from their start and end values (`undefined`: the object lacks it), in one
 * call: per object `s, e, mode` (mode −1: timeless), and what they give.
 */
export function layerTimes(ranged: boolean, cumulative: boolean, values: readonly (readonly [string | undefined, string | undefined])[]): { times: Float64Array; summary: TimeSummary } {
  const { texts, lens } = packed(values);
  const out = coreTimeLayer(ranged, cumulative, texts, lens);
  const n = values.length * 3;
  const [timed, timeless, unreadable, lo, hi] = out.subarray(n);
  return { times: out.subarray(0, n), summary: { timed, timeless, unreadable, extent: Number.isNaN(lo) ? null : [lo, hi] } };
}
