import type { Vec2 } from '../../model/geometry';
import type { CadDocument } from '../../model/document';
import type { AngleUnit } from '../../model/projectSettings';
import { surveyPolar, surveyStakeout, type PolarPoint, type Stake } from '../../model/geom/surveyCalc';

/**
 * What the Hesap windows read from their fields, without the page: known
 * points, numbers, and the checks of Kutupsal alım and Aplikasyon before and
 * around the core's computation (model/geom/surveyCalc.ts). The windows show
 * the errors; the unit tests read them here (read.test.ts).
 */

/** A known point as typed: a point's name in the drawing, or "Y,X". */
export type Known = { p: Vec2; name: string } | { error: string } | null;

/** A row of a measurements table: its cells by column key. */
export type Row = Record<string, string>;

const COORDS = /^\s*(-?\d+(?:\.\d+)?)\s*[,;\s]\s*(-?\d+(?:\.\d+)?)\s*$/;

/** Reads a known point: "Y,X" (Y east first, as on the command line) or the name of a point object in `doc`. */
export function resolvePointIn(doc: Pick<CadDocument, 'all'>, text: string): Known {
  const t = text.trim();
  if (!t) return null;
  const m = COORDS.exec(t);
  if (m) return { p: { x: Number(m[1]), y: Number(m[2]) }, name: '' };
  const key = t.toLocaleUpperCase('tr-TR');
  const found = [...doc.all()].filter((e) => e.kind === 'point' && (e.label ?? e.attrs.Ad ?? '').toLocaleUpperCase('tr-TR') === key);
  if (!found.length) return { error: `“${t}” adlı nokta çizimde yok. Adını denetleyin ya da Y,X yazın.` };
  const e = found[0];
  if (e.kind !== 'point') return null;
  return { p: e.p, name: e.label ?? t };
}

/** A number as typed: a decimal comma is taken for a point. Empty: null; not a number: NaN. */
export function readNumber(text: string | undefined): number | null {
  const t = (text ?? '').trim().replace(',', '.');
  if (!t) return null;
  return /^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$/i.test(t) ? Number(t) : Number.NaN;
}

/** Whether a row has anything typed in it. */
const filled = (r: Row) => Object.values(r).some((v) => v?.trim());

// ── Kutupsal alım ──────────────────────────────────────────────────────

export interface PolarForm {
  station: string;
  back: string;
  backReading: string;
  stationZ: string;
  instrumentHeight: string;
  rows: Row[];
}

/**
 * The core numbers the shots it is given (“2. noktanın …”), and the table
 * leaves its empty rows out of them: the message is given back with the row
 * the user sees (`rowOf`: shot number → table row number).
 */
export function atTableRow(message: string, rowOf: (shot: number) => number | undefined): string {
  return message.replace(/^(\d+)\. noktanın /, (whole, n: string) => {
    const row = rowOf(Number(n));
    return row === undefined ? whole : `${row}. noktanın `;
  });
}

/** Kutupsal alım's fields read and computed: the points and their names, or why not (the errors in order). */
export function readPolar(form: PolarForm, resolve: (text: string) => Known, unit: AngleUnit): { errors: string[]; points: PolarPoint[] | null; names: string[] } {
  const errors: string[] = [];
  const known = (text: string, label: string): Vec2 | null => {
    const r = resolve(text);
    if (!r) errors.push(`${label} verilmedi.`);
    else if ('error' in r) errors.push(`${label}: ${r.error}`);
    else return r.p;
    return null;
  };
  const station = known(form.station, 'Durulan nokta');
  const back = known(form.back, 'Bakılan nokta');
  const backReading = readNumber(form.backReading) ?? 0;
  const stationZ = readNumber(form.stationZ);
  const ih = readNumber(form.instrumentHeight);
  if (Number.isNaN(backReading)) errors.push('Bakılan noktanın okuması bir sayı değil.');
  if (Number.isNaN(stationZ)) errors.push('İstasyon kotu bir sayı değil.');
  if (Number.isNaN(ih)) errors.push('Alet yüksekliği bir sayı değil.');
  const rows = form.rows.map((r, i) => ({ r, i })).filter(({ r }) => filled(r));
  if (!rows.length) errors.push('Tabloya en az bir nokta yazın.');
  const shots = rows.map(({ r, i }) => {
    const reading = readNumber(r.reading);
    const distance = readNumber(r.distance);
    const zenith = readNumber(r.zenith);
    const target = readNumber(r.target);
    if (reading === null) errors.push(`${i + 1}. satırda yatay açı okuması yok.`);
    else if (Number.isNaN(reading)) errors.push(`${i + 1}. satırda yatay açı okuması bir sayı değil.`);
    if (distance === null) errors.push(`${i + 1}. satırda uzunluk yok.`);
    else if (Number.isNaN(distance)) errors.push(`${i + 1}. satırda uzunluk bir sayı değil.`);
    if (Number.isNaN(zenith ?? 0) || Number.isNaN(target ?? 0)) errors.push(`${i + 1}. satırda bir değer sayı değil.`);
    return { reading: reading ?? Number.NaN, distance: distance ?? Number.NaN, zenith, targetHeight: target };
  });
  const names = rows.map(({ r, i }) => r.name?.trim() || `${i + 1}`);
  if (errors.length || !station || !back) return { errors, points: null, names };
  try {
    const points = surveyPolar({ unit, station, back, backReading, stationZ, instrumentHeight: ih, shots });
    return { errors, points, names };
  } catch (e) {
    errors.push(atTableRow((e as Error).message, (shot) => (rows[shot - 1] ? rows[shot - 1].i + 1 : undefined)));
    return { errors, points: null, names };
  }
}

// ── Aplikasyon ─────────────────────────────────────────────────────────

export interface StakeoutForm {
  station: string;
  back: string;
  rows: Row[];
}

/**
 * Aplikasyon's fields read and computed: the values and the targets' names,
 * or why not. A target at the station has no bearing: its row says so.
 */
export function readStakeout(form: StakeoutForm, resolve: (text: string) => Known, unit: AngleUnit): { errors: string[]; stakes: Stake[] | null; names: string[]; back: boolean } {
  const errors: string[] = [];
  const st = resolve(form.station);
  const bk = resolve(form.back);
  if (!st) errors.push('Durulan nokta verilmedi.');
  else if ('error' in st) errors.push(`Durulan nokta: ${st.error}`);
  if (bk && 'error' in bk) errors.push(`Bakılan nokta: ${bk.error}`);
  const station = st && 'p' in st ? st.p : null;
  const targets: Vec2[] = [];
  const names: string[] = [];
  form.rows.forEach((r, i) => {
    const t = resolve(r.point ?? '');
    if (!t) return;
    if ('error' in t) errors.push(`${i + 1}. satır: ${t.error}`);
    else if (station && t.p.x === station.x && t.p.y === station.y) errors.push(`${i + 1}. satırdaki nokta durulan noktayla aynı yerde; semt tanımsız.`);
    else {
      targets.push(t.p);
      names.push(t.name || (r.point ?? '').trim());
    }
  });
  if (!targets.length && !errors.length) errors.push('Tabloya aplike edilecek en az bir nokta yazın.');
  const back = !!bk && 'p' in bk;
  if (errors.length || !station) return { errors, stakes: null, names, back };
  try {
    const stakes = surveyStakeout({ unit, station, back: bk && 'p' in bk ? bk.p : null, targets });
    return { errors, stakes, names, back };
  } catch (e) {
    errors.push((e as Error).message);
    return { errors, stakes: null, names, back };
  }
}
