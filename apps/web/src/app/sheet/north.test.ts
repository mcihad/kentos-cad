import { describe, expect, it } from 'vitest';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import { testEngine } from '../../product/sheet/engineTesting';
import { MemoryKeyValue } from '../../product/sheet/store';
import { byHand, declinationText, degreesText } from '../../ui/sheet/inspector/northSection';
import { SheetService } from './service';
import { fakeApp } from './sheetAppTesting';
import { sheetFromTemplate } from './templateActions';

/**
 * Magnetic north on the web (docs/sheet/design.md §8a): the engine's magnetic
 * model against NOAA's own test values, the north arrow's lines as the engine
 * gives them (`northInfo`: its map's centre, the declination and its source,
 * the date it is for and where that comes from) and as the inspector writes
 * them, and the preflight's findings with the core's fixes, which the
 * inspector's Ön denetim tab runs.
 */

const e = testEngine();
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const rows = (file: string) =>
  fs
    .readFileSync(new URL(`../../../../../fixtures/sheet/v1/wmm/${file}`, import.meta.url), 'utf8')
    .split('\n')
    .filter((l) => l.trim() && !l.trim().startsWith('#'))
    .map((l) => l.trim().split(/\s+/).map(Number));

/** The ifraz sheet of the system template on the demo drawing's place, its north arrow made magnetic, its date given. */
async function ifraz(date: string | null, crs?: Record<string, unknown>) {
  const { app } = fakeApp();
  if (crs) (app.doc.crs as unknown as { set(v: unknown): void }).set(crs);
  const s = new SheetService(app, new MemoryKeyValue(), () => Promise.resolve(e));
  await s.ensureEngine();
  const t = e.systemTemplates().find((x) => x.meta.id === 'sys:ifraz-paftasi')!;
  const id = (await sheetFromTemplate(s, t, null))!;
  const sheet = () => s.book()!.book.sheets.find((x) => x.id === id)!;
  const arrow = sheet().items.find((i) => i.kind.type === 'northArrow')!;
  s.apply([{ op: 'setItemProps', id: arrow.id, patch: { kind: { north: 'magnetic' } } }], 'Manyetik');
  if (date) s.apply([{ op: 'saveVariables', sheet: id, variables: [...sheet().variables, { name: 'tarih', label: 'Tarih', kind: 'date', value: date }] }], 'Tarih');
  return { s, id, arrow: arrow.id, sheet };
}

describe('the magnetic model through the engine', () => {
  it('agrees with NOAA’s test values (WMM2025)', () => {
    const page = rows('WMM2025_TEST_VALUES.txt');
    expect(page.length).toBeGreaterThan(10);
    for (const r of page) {
      const f = e.magneticField(r[2], r[3], r[1], r[0]);
      expect(Math.abs(f.declination - r[10])).toBeLessThanOrEqual(0.01);
      expect(Math.abs(f.inclination - r[9])).toBeLessThanOrEqual(0.01);
    }
    expect(e.wmmInfo()).toEqual({ model: 'WMM2025', released: '2024-12-17', validFrom: 2025, validUntil: 2030 });
    expect(e.decimalYear('2026-10-03')).toBeCloseTo(2026 + 275 / 365, 12);
  });
});

describe('a north arrow’s declination in the inspector', () => {
  it('writes an angle as the paper does (the minute an ASCII mark)', () => {
    expect(degreesText(6 + 19 / 60)).toBe("6°19' D");
    expect(degreesText(-(2 + 10 / 60))).toBe("2°10' B");
    // A book of before the model: by hand when a declination was typed.
    expect(byHand({ declination: 2500 } as never)).toBe(true);
    expect(byHand({ declination: 0 } as never)).toBe(false);
    expect(byHand({ declination: 2500, declinationHand: false } as never)).toBe(false);
  });

  it('reads the declination, its place and its date from the engine', async () => {
    const { s, id, arrow } = await ifraz('2026-10-03');
    const info = s.northInfo(id, arrow)!;
    expect(info).toMatchObject({ source: 'model', model: 'WMM2025', validFrom: 2025, validUntil: 2030, date: '2026-10-03', dateSource: 'sheet', inModel: true });
    expect(info.declination).toBeCloseTo(6 + 19 / 60, 1);
    // The place: the map's centre on the ground, as latitude and longitude; the model at it gives the same.
    expect(info.lat).toBeCloseTo(39.91, 1);
    expect(info.lon).toBeCloseTo(35.84, 1);
    expect(e.magneticField(info.lat!, info.lon!, 0, info.year!).declination).toBeCloseTo(info.declination!, 9);
    expect(declinationText(info)).toBe("6°19' D · WMM2025 · 2026-10");
    // Typed by hand: the paper writes the hand's, and so does the inspector.
    s.apply([{ op: 'setItemProps', id: arrow, patch: { kind: { declinationHand: true, declination: -2167, declinationYear: 2024 } } }], 'Elle');
    const hand = s.northInfo(id, arrow)!;
    expect(hand).toMatchObject({ source: 'hand', handYear: 2024 });
    expect(declinationText(hand)).toBe("2°10' B · elle · 2024");
    const texts = s.plan(s.book()!, id)!.prims.flatMap((p) => (p.type === 'text' && p.item === arrow ? [p.text] : []));
    expect(texts).toContain("Manyetik sapma 2°10' B (elle, 2024)");
    s.dispose();
  });

  it('knows where the date comes from: the sheet’s “tarih”, else the project’s, else today', async () => {
    const { s, id, arrow, sheet } = await ifraz('2027-01-15');
    expect(s.northInfo(id, arrow)).toMatchObject({ date: '2027-01-15', dateSource: 'sheet' });
    s.apply([{ op: 'saveVariables', sheet: id, variables: sheet().variables.filter((v) => v.name !== 'tarih') }], 'Tarihsiz');
    expect(s.northInfo(id, arrow)?.dateSource).toBe('today');
    s.dispose();
  });
});

describe('the magnetic findings and their fixes', () => {
  it('warns of a date the model does not cover, with a way to change it and one to type the value', async () => {
    const { s, id, arrow } = await ifraz('2031-03-01');
    const f = s.preflight(id).find((x) => x.code === 'magnetic_out_of_model');
    expect(f?.severity).toBe('warning');
    expect(f?.item).toBe(arrow);
    // The core's own fixes: open the values to give another date, or type the declination by hand.
    const fixes = f!.fixes;
    expect(fixes.map((x) => x.label)).toEqual(['Değişkenleri aç', 'Sapmayı elle gir']);
    expect(fixes[0].action).toBe('sheet.variables');
    expect(s.northInfo(id, arrow)?.inModel).toBe(false);
    s.apply(fixes[1].ops, fixes[1].label);
    expect(s.preflight(id).some((x) => x.code === 'magnetic_out_of_model')).toBe(false);
    s.dispose();
  });

  it('says a map with no place has no declination: choose a system, or type it', async () => {
    const geographic = { srid: 5252, name: 'TUREF', kind: 'geographic', datum: 'TUREF', ellipsoid: 'GRS80', unit: 'degree', area: '' };
    const { s, id, arrow } = await ifraz('2026-10-03', geographic);
    const f = s.preflight(id).find((x) => x.code === 'magnetic_no_place');
    expect(f?.severity).toBe('error');
    const fixes = f!.fixes;
    expect(fixes.map((x) => [x.label, x.action ?? null])).toEqual([
      ['Koordinat sistemi seç', 'project.crs'],
      ['Sapmayı elle gir', null],
    ]);
    expect(s.northInfo(id, arrow)?.missing).toBe('noPlace');
    s.apply(fixes[1].ops, fixes[1].label);
    expect(s.preflight(id).some((x: Finding) => x.code === 'magnetic_no_place')).toBe(false);
    s.dispose();
  });
});
