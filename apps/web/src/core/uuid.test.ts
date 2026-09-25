import { describe, expect, it } from 'vitest';
import { isUuid, uuidv7, uuidv7Source } from './uuid';

/** The 48-bit millisecond timestamp at the start of a UUIDv7. */
const time = (id: string) => parseInt(id.slice(0, 8) + id.slice(9, 13), 16);

describe('UUIDv7 (RFC 9562)', () => {
  it('is lowercase text with hyphens, version 7 and variant 10', () => {
    for (let i = 0; i < 200; i++) {
      const id = uuidv7();
      expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
      expect(isUuid(id)).toBe(true);
    }
  });

  it('starts with the Unix time in milliseconds', () => {
    const at = Date.UTC(2026, 8, 25, 12, 30, 0, 123);
    expect(time(uuidv7Source(() => at)())).toBe(at);
    // 48 bits hold every millisecond until the year 10889.
    expect(time(uuidv7Source(() => 2 ** 48 - 1)())).toBe(2 ** 48 - 1);
    expect(uuidv7Source(() => 0)().slice(0, 13)).toBe('00000000-0000');
  });

  it('sorts in creation order, within a millisecond, across milliseconds and when the clock steps back', () => {
    let now = 1_758_800_000_000;
    const next = uuidv7Source(() => now);
    const ids: string[] = [];
    // More than the 4096 counter values of one millisecond: it borrows the next ones.
    for (let i = 0; i < 10_000; i++) ids.push(next());
    now += 5;
    ids.push(next());
    now -= 1000;
    for (let i = 0; i < 10; i++) ids.push(next());
    expect(ids.every((id, i) => i === 0 || id > ids[i - 1])).toBe(true);
    expect(new Set(ids).size).toBe(ids.length);
    // The borrowed milliseconds stay close to the clock: at most one per 2048 ids.
    expect(time(ids[9_999]) - time(ids[0])).toBeLessThanOrEqual(5);
    // After the clock stepped back, the ids go on from the last millisecond instead of going back.
    expect(time(ids.at(-1)!)).toBeGreaterThanOrEqual(time(ids[10_000]));
  });

  it('draws the rest at random: two makers on the same clock never agree', () => {
    const a = uuidv7Source(() => 1_000);
    const b = uuidv7Source(() => 1_000);
    const seen = new Set<string>();
    for (let i = 0; i < 1000; i++) {
      seen.add(a());
      seen.add(b());
    }
    expect(seen.size).toBe(2000);
  });

  it('accepts any version as a UUID, only in the lowercase hyphenated form', () => {
    expect(isUuid('4a5259a1-97f7-4742-88ce-b747287025ed')).toBe(true);
    expect(isUuid('2ed6657d-e927-568b-95e1-2665a8aea6a2')).toBe(true);
    expect(isUuid('2ED6657D-E927-568B-95E1-2665A8AEA6A2')).toBe(false);
    expect(isUuid('2ed6657de927568b95e12665a8aea6a2')).toBe(false);
    expect(isUuid('2ed6657d-e927-568b-95e1-2665a8aea6a')).toBe(false);
    expect(isUuid(7)).toBe(false);
  });
});
