import { fixed } from '../../core/displayNumber';

/**
 * The millimetre rulers' marks (docs/sheet/design.md §11): numbers at a
 * 1-2-5 step that leaves them room at the zoom shown, and the smaller marks
 * between them as fine as the screen allows. Paper millimetres from the
 * paper's top left corner (negative beyond it, on the desk). Pure: the ruler
 * painter draws what this lists.
 */

export interface Tick {
  /** Millimetres of paper. */
  readonly at: number;
  /** 2: a numbered mark, 1: half way between two, 0: the finest. */
  readonly level: 0 | 1 | 2;
  /** The number written beside a numbered mark. */
  readonly label?: string;
}

/** The smallest 1-2-5 step (mm) whose numbers stand at least `minPx` apart at `pxPerMm`. */
export function labelStep(pxPerMm: number, minPx = 56): number {
  const want = minPx / pxPerMm;
  let p = Math.pow(10, Math.floor(Math.log10(want)));
  for (;;) {
    for (const m of [1, 2, 5]) if (m * p >= want - 1e-9) return m * p;
    p *= 10;
  }
}

/** How a step's number is written: whole millimetres, or as many decimals as the step needs below one. */
export function stepText(mm: number, step: number): string {
  const decimals = step >= 1 ? 0 : Math.min(3, Math.ceil(-Math.log10(step) - 1e-9));
  return fixed(Math.abs(mm) < step / 1000 ? 0 : mm, decimals);
}

/**
 * The marks from `from` to `to` mm: numbers every `labelStep`, and between
 * them tenths, fifths or halves of it, the finest that stay `minFinePx`
 * apart; a half step is drawn longer when it is a mark of its own.
 */
export function rulerTicks(from: number, to: number, pxPerMm: number, minLabelPx = 56, minFinePx = 5): Tick[] {
  if (!(to > from) || !(pxPerMm > 0)) return [];
  const step = labelStep(pxPerMm, minLabelPx);
  const fine = [10, 5, 2].map((n) => step / n).find((s) => s * pxPerMm >= minFinePx) ?? step;
  const per = Math.round(step / fine);
  const start = Math.floor(from / fine);
  const end = Math.ceil(to / fine);
  const out: Tick[] = [];
  // At most a few thousand marks: a huge range at a fine step is the caller's mistake, not a reason to hang.
  if (end - start > 20000) return out;
  for (let i = start; i <= end; i++) {
    const at = i * fine;
    const k = ((i % per) + per) % per;
    if (k === 0) out.push({ at, level: 2, label: stepText(at, step) });
    else out.push({ at, level: per % 2 === 0 && k === per / 2 ? 1 : 0 });
  }
  return out;
}
