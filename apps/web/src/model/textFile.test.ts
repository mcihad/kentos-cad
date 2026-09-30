import { describe, expect, it } from 'vitest';
import file from '../../../../fixtures/text/v1/file.json?raw';
import { MAX_TEXT_FILE_BYTES, textFileLines } from './textFile';

/**
 * Metin dosyası yerleştir's file rule (docs/adr/0145 §6) against the shared cases (fixtures/text/v1/file.json,
 * written from the rule by scripts/fixtures/text_cases.py); natively crates/native/interaction/tests/text_file.rs.
 */
interface Case {
  name: string;
  text?: string;
  hex?: string;
  expect: { lines?: string[]; count?: number; first?: string; last?: string; error?: string };
}

const bytesOf = (c: Case): Uint8Array =>
  c.hex !== undefined ? new Uint8Array((c.hex.match(/../g) ?? []).map((b) => parseInt(b, 16))) : new TextEncoder().encode(c.text ?? '');

describe('Metin dosyası yerleştir: the file’s lines', () => {
  it('reads every shared case as the rule says', () => {
    const cases = (JSON.parse(file) as { cases: Case[] }).cases;
    expect(cases.length).toBeGreaterThanOrEqual(10);
    for (const c of cases) {
      const got = textFileLines('satirlar.txt', bytesOf(c));
      if (c.expect.error) {
        expect(got.ok ? null : got.kind, c.name).toBe(c.expect.error);
        continue;
      }
      if (!got.ok) throw new Error(`${c.name}: ${got.error}`);
      if (c.expect.lines) expect(got.lines, c.name).toEqual(c.expect.lines);
      else expect([got.lines.length, got.lines[0], got.lines.at(-1)], c.name).toEqual([c.expect.count, c.expect.first, c.expect.last]);
    }
  });

  it('refuses more than 1 MB and says so with its size', () => {
    const got = textFileLines('büyük.txt', new Uint8Array(MAX_TEXT_FILE_BYTES + 1).fill(0x61));
    expect(got).toEqual({ ok: false, kind: 'tooBig', error: "“büyük.txt” 1 MB'tan büyük (1.0 MB); en çok 1 MB okunur. Dosyayı bölüp yeniden deneyin." });
    expect(textFileLines('tam.txt', new Uint8Array(MAX_TEXT_FILE_BYTES).fill(0x61)).ok).toBe(true);
  });

  it('says why in words', () => {
    const say = (bytes: Uint8Array) => {
      const got = textFileLines('a.txt', bytes);
      return got.ok ? null : got.error;
    };
    expect(say(new Uint8Array([0xfe, 0x41]))).toBe('“a.txt” UTF-8 değil. Dosyayı UTF-8 olarak kaydedip yeniden deneyin.');
    expect(say(new TextEncoder().encode('\n \n'))).toBe('“a.txt” boş: yerleştirilecek satır yok.');
    expect(say(new TextEncoder().encode('a\n'.repeat(10_001)))).toBe("“a.txt” 10001 satır; en çok 10000 satır yerleştirilir. Dosyayı bölüp yeniden deneyin.");
  });
});
