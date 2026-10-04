import { describe, expect, it } from 'vitest';
import { Formatter } from '../app/format';
import type { Vec2 } from '../model/geometry';
import { groundLines, trimmedHeight, UNREACHED } from '../model/groundMeasures';
import { ProjectSettings } from '../model/projectSettings';

/**
 * Mesafe ölç's and Alan hesapla's lines on the ellipsoid and on the ground (docs/adr/0171 §4a), through the WASM core:
 * the ellipsoid's with the plane's scale, the ground's at the project's mean ellipsoidal height with the height's factor;
 * none without a height or in a local project. The desktop's `kentos_interaction::ground` tests the same.
 */
const square = (o: Vec2, side: number) => ({
  pts: [o, { x: o.x + side, y: o.y }, { x: o.x + side, y: o.y + side }, { x: o.x, y: o.y + side }],
  bulges: null,
});

describe('groundLines', () => {
  const tm30 = () => new ProjectSettings({ srid: 5254, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000 });

  it('names the ellipsoid and the ground', () => {
    const s = tm30();
    s.assign({ survey: { groundHeight: 850 } });
    const f = new Formatter(s);
    const ring = square({ x: 414000, y: 4540000 }, 40);
    const said = groundLines(s, [ring], true, f);
    expect(said).toHaveLength(2);
    expect(said[0]).toMatch(/^Elipsoit üstünde: Alan 1599\.\d\d m² {3}Çevre \d+\.\d{3} m {3}Ölçek 1\.0000\d{4}$/);
    expect(said[1]).toMatch(/^Zeminde \(h = 850 m\): Alan \d+\.\d\d m² {3}Çevre \d+\.\d{3} m {3}Yükseklik çarpanı 0\.99986\d{3}$/);
    expect(groundLines(s, [ring], false, f)[0]).toMatch(/^Elipsoit üstünde: Toplam uzunluk 119\.9\d\d m {3}Ölçek /);
  });

  it('says nothing without a height, nor in a local project', () => {
    const s = tm30();
    const f = new Formatter(s);
    expect(groundLines(s, [square({ x: 414000, y: 4540000 }, 40)], true, f)).toEqual([]);
    const local = new ProjectSettings({ srid: 0, angleUnit: 'deg', survey: { groundHeight: 850 } });
    expect(groundLines(local, [square({ x: 0, y: 0 }, 40)], true, new Formatter(local))).toEqual([]);
    expect(UNREACHED).toContain('değer yazılmadı');
  });

  it('writes a height as the form does', () => {
    expect([850, 850.25, -27.123456, -0].map(trimmedHeight)).toEqual(['850', '850.25', '-27.1235', '0']);
  });
});
