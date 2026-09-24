/**
 * Equality of plain data as JSON writes it, without writing it. The
 * document tells geometry edits from attribute edits this way
 * (`geometryChanged` in ./document); comparing JSON.stringify strings cost
 * two full serialisations of every edited object, a large polyline's
 * points on each attribute change.
 */

/** Values JSON leaves out as a property and writes as null in an array. */
const omitted = (v: unknown) => v === undefined || typeof v === 'function' || typeof v === 'symbol';

/** Values JSON writes as null: null itself and non-finite numbers. */
const writtenAsNull = (v: unknown) => v === null || (typeof v === 'number' && !Number.isFinite(v));

/**
 * Whether `JSON.stringify(a) === JSON.stringify(b)` for plain data (no
 * `toJSON`, no cycles): the same keys in the same order once undefined
 * properties are dropped, undefined array items and non-finite numbers
 * equal to null, -0 equal to 0. `skipKey` leaves one key of the outer
 * object out of the comparison.
 */
export function sameJson(a: unknown, b: unknown, skipKey?: string): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return writtenAsNull(a) && writtenAsNull(b);
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      const x: unknown = a[i];
      const y: unknown = b[i];
      if (x !== y && !sameJson(omitted(x) ? null : x, omitted(y) ? null : y)) return false;
    }
    return true;
  }
  const ra = a as Record<string, unknown>;
  const rb = b as Record<string, unknown>;
  const ka = Object.keys(ra);
  const kb = Object.keys(rb);
  // Walk both key lists in step: JSON writes keys in this order, so it matters.
  for (let i = 0, j = 0; ; i++, j++) {
    while (i < ka.length && (ka[i] === skipKey || omitted(ra[ka[i]]))) i++;
    while (j < kb.length && (kb[j] === skipKey || omitted(rb[kb[j]]))) j++;
    if (i === ka.length || j === kb.length) return i === ka.length && j === kb.length;
    if (ka[i] !== kb[j] || !sameJson(ra[ka[i]], rb[kb[j]])) return false;
  }
}
