import { describe, expect, it } from 'vitest';
import { fixed } from '../core/displayNumber';
import { callNamed } from './core';

/**
 * The measures against the independent reference in fixtures/measure/v1/cases.json (docs/adr/0149 §3; 50-digit
 * mpmath, scripts/fixtures/measure_cases.py, not KentOS code), through the WASM core the app calls: every operation
 * must give the exact value within the case's bound and be written, with 0 to 6 decimals, as the exact value is
 * written by the display rule. The core runs the same cases natively (crates/shared/geometry-core/tests/measure.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  readonly what: string;
  readonly op: string;
  readonly args: unknown[];
  readonly pick?: string;
  readonly unit: string;
  readonly exact: string;
  readonly abs: number;
  readonly rel: number;
  readonly shown: string[];
}

describe('measures', () => {
  it('are exact within their bounds and shown as the exact values', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/measure/v1/cases.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.measure-fixtures', 1]);
    const off: string[] = [];
    for (const c of file.cases) {
      const out = callNamed(c.op, c.args) as Record<string, unknown> | number;
      const got = (c.pick ? (out as Record<string, unknown>)[c.pick] : out) as number;
      const exact = Number(c.exact);
      const bound = c.abs + c.rel * Math.abs(exact);
      if (!(Math.abs(got - exact) <= bound)) off.push(`${c.what}: ${got} ≠ ${c.exact}`);
      // What the app writes: an angle in grads, as `(rad * 200) / Math.PI`.
      const shownValue = c.unit === 'angle' ? (got * 200) / Math.PI : got;
      c.shown.forEach((want, d) => {
        const text = fixed(shownValue, d);
        if (text !== want) off.push(`${c.what}: ${d} basamakta ${text}, kesin değerinki ${want}`);
      });
    }
    expect(off).toEqual([]);
  });
});
