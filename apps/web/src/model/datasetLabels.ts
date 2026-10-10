// A NetCDF dataset's slice dimensions as the windows say their values (docs/adr/0243 §11), twin of the formats core's
// `multidim::cube::dim_labels`: a time's as the time slider shows it (with seconds when one has them), another's its
// shortest decimal and units.

import { showTime } from './time';

/** The shortest decimal that reads back to `v`, positional (the formats core's `num::plain`). */
function plain(v: number): string {
  if (v === 0) return '0';
  const s = String(v);
  if (!/e/i.test(s)) return s;
  // JavaScript writes very large and very small numbers with an exponent; the core writes them out.
  const [mantissa, exp] = s.split(/e/i);
  const e = Number(exp);
  const negative = mantissa.startsWith('-');
  const digits = mantissa.replace('-', '').replace('.', '');
  const point = mantissa.replace('-', '').indexOf('.');
  const whole = (point < 0 ? mantissa.replace('-', '').length : point) + e;
  const out = whole <= 0 ? `0.${'0'.repeat(-whole)}${digits}` : whole >= digits.length ? digits + '0'.repeat(whole - digits.length) : `${digits.slice(0, whole)}.${digits.slice(whole)}`;
  return negative ? `-${out}` : out;
}

/** Each value of a slice dimension as the windows say it. */
export function dimLabels(values: readonly number[], time: boolean, units?: string | null): string[] {
  if (time) {
    const seconds = values.some((t) => ((Math.trunc(t) % 60000) + 60000) % 60000 !== 0);
    return values.map((t) => showTime(t, seconds ? 'second' : 'minute'));
  }
  return values.map((v) => (units ? `${plain(v)} ${units}` : plain(v)));
}
