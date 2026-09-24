import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import { compileExpression, expressionError, previewExpression, type ExprValue } from './expression';

const parcel: Entity = {
  id: 7,
  kind: 'polygon',
  layerId: 'parsel',
  pts: [
    { x: 0, y: 0 },
    { x: 20, y: 0 },
    { x: 20, y: 30 },
    { x: 0, y: 30 },
  ],
  label: '12',
  attrs: { Parsel: '12', Ada: '1245', Nitelik: 'Arsa', 'Tapu alanı': '598.50', Boş: '' },
};
const line: Entity = { id: 8, kind: 'line', layerId: 'yol', a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, attrs: {} };
const layerName = (id: string) => ({ parsel: 'Parsel sınırı', yol: 'Yol ekseni' })[id] ?? id;

/** The value for `entity` at 1-based position `index` of a run (the run repeats it). */
function run(src: string, entity: Entity = parcel, index = 1): ExprValue {
  const r = compileExpression(src);
  if (!r.ok) throw new Error(expressionError(r));
  return r.expr.evaluateAll({ entities: Array.from({ length: index }, () => entity), layerName }).value(index - 1);
}
const error = (src: string) => {
  const r = compileExpression(src);
  return r.ok ? null : expressionError(r);
};

describe('expressions', () => {
  it('reads fields, bracketed names and geometry variables', () => {
    expect(run('Parsel')).toBe('12');
    expect(run('[Tapu alanı]')).toBe('598.50');
    expect(run('$alan')).toBe(600);
    expect(run('$uzunluk')).toBe(100);
    expect(run('$çevre')).toBe(100);
    expect(run('$CEVRE')).toBe(100);
    expect(run('$köşe')).toBe(4);
    expect(run('$tür')).toBe('Kapalı alan');
    expect(run('$katman')).toBe('Parsel sınırı');
    expect(run('$etiket')).toBe('12');
    expect(run('$sıra', parcel, 5)).toBe(5);
    expect(run('$uzunluk', line)).toBe(5);
    expect(run('$alan', line)).toBeNull();
    expect(run('Yok')).toBeNull();
  });
  it('does arithmetic on text numbers and joins text', () => {
    expect(run('[Tapu alanı] - $alan')).toBeCloseTo(-1.5, 12);
    expect(run('Parsel + 1')).toBe(13);
    expect(run('Ada || "/" || Parsel')).toBe('1245/12');
    expect(run('Nitelik + " parsel"')).toBe('Arsa parsel');
    expect(run('2 + 3 * 4 - (1 + 1) / 2')).toBe(13);
    expect(run('-$alan')).toBe(-600);
    expect(run('7 % 4')).toBe(3);
    expect(run('1 / 0')).toBeNull();
    expect(run('Nitelik * 2')).toBeNull();
    expect(run('Yok + 1')).toBeNull();
    expect(run("0.1 + 0.2 || ''")).toBe('0.3');
  });
  it('compares and combines conditions in Turkish and English', () => {
    expect(run("Nitelik = 'Arsa' ve $alan > 500")).toBe(true);
    expect(run("Nitelik = 'arsa'")).toBe(false);
    expect(run("Nitelik = 'Tarla' veya Parsel = 12")).toBe(true);
    expect(run('değil $alan > 500')).toBe(false);
    expect(run('not ($alan < 500) and true')).toBe(true);
    expect(run('Parsel = 12.0')).toBe(true);
    expect(run('Parsel <> 12')).toBe(false);
    expect(run("Nitelik < 'Bahçe'")).toBe(true);
    expect(run('Yok = boş')).toBe(true);
    expect(run('[Boş] = boş')).toBe(true);
    expect(run('Parsel = boş')).toBe(false);
    expect(run('Yok > 5')).toBe(false);
    expect(run('doğru ve yanlış')).toBe(false);
  });
  it('calls functions by Turkish or English name', () => {
    expect(run('yuvarla(12.345, 2)')).toBe(12.35);
    expect(run('YUVARLA(1.005, 2)')).toBe(1.01);
    expect(run('round($alan / 7)')).toBe(86);
    expect(run('metin(452.1, 2)')).toBe('452.10');
    expect(run("'P' || doldur($sıra, 5)", parcel, 12)).toBe('P00012');
    expect(run("doldur(Parsel, 4, '_')")).toBe('__12');
    expect(run('parça("P00012", 2, 3)')).toBe('000');
    expect(run('büyük("kadıköy")')).toBe('KADIKÖY');
    expect(run('küçük("IŞIK")')).toBe('ışık');
    expect(run("içerir(Nitelik, 'ARS')")).toBe(true);
    expect(run("başlar('Çınar', 'cin')")).toBe(true);
    expect(run("eğer($alan > 1000, 'büyük', 'küçük')")).toBe('küçük');
    expect(run('boş(Yok)')).toBe(true);
    expect(run("varsayılan(Yok, [Boş], 'yok')")).toBe('yok');
    expect(run('max(1, Parsel, 3)')).toBe(12);
    expect(run('tamsayı(-2.7)')).toBe(-2);
    expect(run("değiştir('1245/12', '/', '-')")).toBe('1245-12');
    expect(run("sayı('abc')")).toBeNull();
  });
  it('explains mistakes with a position', () => {
    expect(error('')).toBe('İfade boş.');
    expect(error("Nitelik = 'Arsa")).toBe('11. karakterde: Tırnak kapanmamış.');
    expect(error('(1 + 2')).toContain('Parantez kapanmamış');
    expect(error('1 2')).toContain('3. karakterde: Beklenmeyen “2”');
    expect(error('foo(1)')).toBe('Bilinmeyen işlev: foo().');
    expect(error('yuvarla()')).toContain('1 ya da 2 değer alır; 0 verildi');
    expect(error('$nope')).toBe('Bilinmeyen değişken: $nope.');
    expect(error('1 +')).toContain('yarım kalmış');
    expect(error('ve 1')).toContain('burada kullanılamaz');
    expect(error('[Tapu')).toContain('“]” bekleniyordu');
    expect(error('1 # 2')).toContain('Anlaşılmayan karakter: “#”');
  });
  it('lists the fields it reads and previews on objects', () => {
    const r = compileExpression("Nitelik = 'Arsa' ve [Tapu alanı] > 0 ve Kat > 1");
    expect(r.ok && r.expr.fields).toEqual(['Nitelik', 'Tapu alanı', 'Kat']);
    if (!r.ok) return;
    expect(previewExpression(r.expr, [parcel, line], 'condition', layerName)).toBe('0 / 2 nesne koşulu sağlıyor. “Kat” alanı bu nesnelerde yok.');
    const v = compileExpression('yuvarla($alan, 1)');
    expect(v.ok && previewExpression(v.expr, [parcel, line], 'value', layerName)).toBe('İlk nesnede (12): “600”. 1 nesnede sonuç boş.');
  });
});
