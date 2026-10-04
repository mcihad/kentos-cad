import { describe, expect, it } from 'vitest';
import { fixed } from './displayNumber';

/**
 * The display rule (docs/adr/0149) against the independent reference in fixtures/numeric/v1/display.json (Python
 * decimal, not KentOS code; scripts/fixtures/numeric_display.py), the cases the core's
 * crates/shared/geometry-core/tests/all/display.rs reads too.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  readonly v: string;
  readonly d: number;
  readonly expected: string;
  readonly why?: string;
}

describe('display rule', () => {
  it('writes every case as the reference writes it', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/numeric/v1/display.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.display-fixtures', 1]);
    expect(file.cases.length).toBeGreaterThan(1000);
    // Number() parses the text to the very float the reference wrote.
    const differ = file.cases.filter((c) => fixed(Number(c.v), c.d) !== c.expected).map((c) => `${c.v} with ${c.d}: ${fixed(Number(c.v), c.d)}, expected ${c.expected}`);
    expect(differ).toEqual([]);
  });

  it('writes a half at the digits shown the same way however the value was computed', () => {
    expect([fixed(12.125, 2), fixed(12.1249999997, 2), fixed(12.1250000003, 2)]).toEqual(['12.13', '12.13', '12.13']);
    expect([fixed(5.0005, 3), fixed(1.005, 2), fixed(-2.5, 0), fixed(-0.0001, 3)]).toEqual(['5.001', '1.01', '-3', '0.000']);
  });
});
