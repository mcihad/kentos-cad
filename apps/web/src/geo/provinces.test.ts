import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/crs/v1/provinces.json?raw';
import { turefZoneFor } from './crs';
import { PROVINCES, provinceByCode, provincesFixture, searchProvinces } from './provinces';

describe('provinces (docs/adr/0165 §3)', () => {
  it('are the file shared with the desktop (re-record after a deliberate change)', () => {
    expect(JSON.parse(text)).toEqual(JSON.parse(JSON.stringify(provincesFixture())));
  });

  it('are the 81, by their plate codes, every centre inside Türkiye', () => {
    expect(PROVINCES.map((x) => x.code)).toEqual(Array.from({ length: 81 }, (_, i) => i + 1));
    expect(new Set(PROVINCES.map((x) => x.name)).size).toBe(81);
    for (const x of PROVINCES) {
      expect(x.lat, x.name).toBeGreaterThan(35.8);
      expect(x.lat, x.name).toBeLessThan(42.2);
      expect(x.lon, x.name).toBeGreaterThan(25.6);
      expect(x.lon, x.name).toBeLessThan(44.9);
    }
  });

  it('are found by name, Turkish case folded, or by plate code', () => {
    expect(searchProvinces('iz').map((x) => x.name)).toEqual(['İzmir', 'Denizli', 'Rize']);
    expect(searchProvinces('IĞ').map((x) => x.name)).toEqual(['Iğdır', 'Elazığ']);
    expect(searchProvinces('06').map((x) => x.name)).toEqual(['Ankara']);
    expect(searchProvinces('3').slice(0, 3).map((x) => x.code)).toEqual([3, 30, 31]);
    expect(searchProvinces('  ').length).toBe(81);
    expect(provinceByCode(34)?.name).toBe('İstanbul');
  });

  it('suggest the TUREF zone of their longitude', () => {
    const zone = (code: number) => turefZoneFor(provinceByCode(code)!.lon)?.name;
    expect([zone(34), zone(6), zone(35), zone(61), zone(65), zone(76)]).toEqual(['TUREF / TM30', 'TUREF / TM33', 'TUREF / TM27', 'TUREF / TM39', 'TUREF / TM42', 'TUREF / TM45']);
  });
});
