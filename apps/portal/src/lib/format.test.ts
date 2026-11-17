import { describe, expect, it } from 'vitest';
import { bytes, daysLeft, daysUntil, initials, slugOf } from './format';

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
});
