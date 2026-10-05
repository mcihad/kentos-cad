import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import type { MemberSide } from '../objectTemplate';
import { templateMemberCentroid, templateMemberOffsets } from './templateMembers';

/**
 * A group template's members (docs/adr/0176 §5) through the WASM core, against the independent reference in
 * fixtures/template-members/v1/cases.json (scripts/fixtures/template_member_cases.py, exact fractions, no KentOS code),
 * the cases the core runs natively in crates/shared/geometry-core/tests/all/template_members.rs: numbers within
 * 1e-9 m, a missing field null.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  version: number;
  offsets: { name: string; entity: Entity; distance: number; side: MemberSide; expected: unknown }[];
  centroids: { name: string; entity: Entity; expected: unknown }[];
}

function differ(a: unknown, e: unknown, path: string): string | null {
  if (typeof a === 'number' && typeof e === 'number') return Math.abs(a - e) <= 1e-9 ? null : `${path}: ${a} ≠ ${e}`;
  if (Array.isArray(a) || Array.isArray(e)) {
    if (!Array.isArray(a) || !Array.isArray(e) || a.length !== e.length) return `${path}: ${JSON.stringify(a)?.slice(0, 80)} ≠ ${JSON.stringify(e)?.slice(0, 80)}`;
    for (let i = 0; i < a.length; i++) {
      const d = differ(a[i], e[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (a && e && typeof a === 'object' && typeof e === 'object') {
    for (const k of new Set([...Object.keys(a), ...Object.keys(e)])) {
      const d = differ((a as Record<string, unknown>)[k] ?? null, (e as Record<string, unknown>)[k] ?? null, `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return (a ?? null) === (e ?? null) ? null : `${path}: ${String(a)} ≠ ${String(e)}`;
}

describe('a group template’s members', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/template-members/v1/cases.json', import.meta.url), 'utf8')) as File;
  it('is a template-member-cases v1 file', () => {
    expect([file.format, file.version]).toEqual(['kentos.template-member-cases', 1]);
    expect(file.offsets.length).toBeGreaterThanOrEqual(20);
    expect(file.centroids.length).toBeGreaterThanOrEqual(6);
  });
  it('makes every offset as the reference makes it', () => {
    const off = file.offsets.flatMap((c) => {
      const d = differ(templateMemberOffsets(c.entity, c.distance, c.side), c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });
  it('puts every centroid where the reference puts it', () => {
    const off = file.centroids.flatMap((c) => {
      const d = differ(templateMemberCentroid(c.entity), c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });
});
