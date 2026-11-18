import { describe, expect, it } from 'vitest';
import { bytes, daysLeft, daysUntil, decimal, initials, money, parseDecimal, slugOf } from './format';

describe('the portal writes', () => {
  it('a deadline in words, never as a negative number', () => {
    expect(daysLeft(5)).toBe('5 gün kaldı');
    expect(daysLeft(0)).toBe('bugün son gün');
    expect(daysLeft(-2)).toBe('2 gün geçti');
    expect(daysUntil('2026-11-20', new Date(2026, 10, 17, 9))).toBe(3);
  });

  it('sizes with a dot before decimals and names as their initials', () => {
    expect(bytes(512)).toBe('512 B');
    expect(bytes(1536)).toBe('2 KB');
    expect(bytes(1.4 * 1024 * 1024)).toBe('1.4 MB');
    expect(initials('Ayşe Yılmaz')).toBe('AY');
    expect(initials('ismet inönü')).toBe('İİ');
  });

  it("an organisation's short name without Turkish letters", () => {
    expect(slugOf('Örnek Belediyesi')).toBe('ornek-belediyesi');
    expect(slugOf('Çağlayan Şişli İlçe Belediyesi')).toBe('caglayan-sisli-ilce-belediyesi');
    expect(slugOf('  Pınarlı / Merkez  ')).toBe('pinarli-merkez');
  });

  it('money in kuruş as lira, and numbers as a Turkish hand types them', () => {
    expect(money(751063)).toBe('7.510,63 TL');
    expect(money(5)).toBe('0,05 TL');
    expect(decimal(3.75)).toBe('3,75');
    expect(parseDecimal('152,5')).toBe(152.5);
    expect(parseDecimal('1.250')).toBe(1250);
    expect(parseDecimal('1.234,5')).toBe(1234.5);
    expect(parseDecimal('152.5')).toBe(152.5);
    expect(parseDecimal(' ')).toBeNull();
    expect(parseDecimal('on iki')).toBeNull();
  });
});
