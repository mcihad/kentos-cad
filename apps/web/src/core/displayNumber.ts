/**
 * How a number is written for the user (docs/adr/0149, display v1): on screen, in a dimension's value, in a label
 * written into the drawing. The value is first rounded to seven decimals (from the float's exact binary value, an
 * exact half away from zero: `toFixed`), then that decimal to the digits shown, a half away from zero, as on paper.
 * Seven decimals (0.1 µm) are a hundred times the noise of a measure taken from Transverse Mercator coordinates
 * (about 1e-9 m) and ten thousand times finer than a millimetre: a value that is a half at the digits shown (12.125, a
 * typed 5.0005) is written the same way whichever way it was computed. With seven digits or more the value is rounded
 * once, to them. A value that rounds to zero is written without a minus sign.
 *
 * The desktop's and the core's twin is `crates/shared/geometry-core/src/display.rs`; both pass
 * `fixtures/numeric/v1/display.json`, which `scripts/fixtures/numeric_display.py` writes from the rule alone. Display
 * only: a written value is never read back into a computation (CLAUDE.md §23.2). Values from 1e21 up are out of
 * the rule's range (`toFixed` writes them with an exponent).
 */

/** The decimals the noise is rounded away at. */
export const NOISE_DECIMALS = 7;

/** `v` written with `d` decimals by the display rule. */
export function fixed(v: number, d: number): string {
  if (Number.isNaN(v)) return 'NaN';
  if (!Number.isFinite(v)) return v > 0 ? 'Infinity' : '-Infinity';
  const x = Math.abs(v);
  const body = d >= NOISE_DECIMALS ? x.toFixed(d) : roundText(x.toFixed(NOISE_DECIMALS), d);
  return v < 0 && /[1-9]/.test(body) ? `-${body}` : body;
}

/** A decimal text with more than `d` decimals rounded to `d`, a half away from zero: up when the first dropped digit is 5 or more. */
function roundText(text: string, d: number): string {
  const dot = text.indexOf('.');
  if (dot < 0) return text;
  const keep = d === 0 ? dot : dot + 1 + d;
  if (keep >= text.length) return text;
  const dropped = text[d === 0 ? dot + 1 : keep];
  const kept = text.slice(0, keep);
  return dropped >= '5' ? upOne(kept) : kept;
}

/** A decimal text plus one unit in its last place: `9.99` → `10.00`. */
function upOne(digits: string): string {
  const chars = [...digits];
  let i = chars.length;
  for (;;) {
    if (i === 0) {
      chars.unshift('1');
      break;
    }
    i -= 1;
    const c = chars[i];
    if (c === '.') continue;
    if (c === '9') {
      chars[i] = '0';
      continue;
    }
    chars[i] = String.fromCharCode(c.charCodeAt(0) + 1);
    break;
  }
  return chars.join('');
}
