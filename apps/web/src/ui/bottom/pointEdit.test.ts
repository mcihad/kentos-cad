import { describe, expect, it } from 'vitest';
import type { Entity, NewEntity, PointEntity } from '../../model/entities';
import { elevatedPaths } from '../../product/elevation';
import { toolHarness } from '../../tools/toolHarness';
import { emptyDraft, nextCell, writeCell, writeDraft, type Draft, type EditColumn } from './pointEdit';

/**
 * Nokta editörü's writes against fixtures/point-editor/v1/edits.json (scripts/fixtures/point_edit_cases.py, worked
 * out from the rules with no KentOS code): every case's drawing after a cell or a draft is written, the messages said
 * and the undo step, as the desktop's `points/edit.rs` gives them too. Numbers within 1e-9 m, a missing field null.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  name: string;
  objects: (NewEntity & { id: number })[];
  active: string;
  cell?: { id: number; column: EditColumn; text: string; follow: boolean };
  draft?: Draft;
  expected: { said: string[]; step: string | null; next?: string | null; objects: unknown[] };
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

/** An object as the cases compare it: its kind, label, attributes, and a point's place and elevation or line work's paths. */
function view(e: Entity): unknown {
  const head = { kind: e.kind, label: e.label ?? null, attrs: e.attrs };
  if (e.kind === 'point') return { ...head, at: [e.p.x, e.p.y], z: e.z ?? null };
  return { ...head, paths: elevatedPaths(e).map((p) => ({ pts: p.pts.map((q) => [q.x, q.y]), zs: p.zs ?? p.pts.map(() => null) })) };
}

describe('Nokta editörü: düzenleme', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/point-editor/v1/edits.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };

  it('is the reference’s file', () => {
    expect([file.format, file.version]).toEqual(['kentos.point-editor-edits', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(30);
  });

  it('writes every case as the reference does', () => {
    const off = file.cases.flatMap((c) => {
      const h = toolHarness();
      for (const { id: _id, ...o } of c.objects) h.add(o as never);
      const before = h.doc.revision;
      let got: { said: string[]; step: string | null; next?: string | null };
      if (c.cell) {
        const e = h.doc.get(c.cell.id) as PointEntity;
        got = writeCell(h.doc, e, c.cell.column, c.cell.text, c.cell.follow);
      } else {
        const out = writeDraft(h.doc, c.draft ?? emptyDraft(), c.active, null);
        got = { said: out.said, step: out.step, next: out.next };
      }
      const objects = [...h.doc.all()].map(view);
      const step = got.step === null ? (h.doc.revision === before ? null : 'yazıldı') : h.doc.undo();
      const seen = { said: got.said, step, ...(c.draft && { next: got.next ?? null }), objects };
      const d = differ(seen, c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });
});

describe('Nokta editörü: hücreler arasında', () => {
  it('goes down the column, right and left, as the desktop’s editor does', () => {
    const ids = [11, 12, 13];
    expect(nextCell(ids, 12, 'east', 'down')).toEqual({ id: 13, col: 'east' });
    expect(nextCell(ids, 13, 'east', 'down')).toBeNull();
    expect(nextCell(ids, 11, 'name', 'right')).toEqual({ id: 11, col: 'east' });
    expect(nextCell(ids, 11, 'code', 'right')).toEqual({ id: 12, col: 'name' });
    expect(nextCell(ids, 13, 'code', 'right')).toBeNull();
    expect(nextCell(ids, 12, 'name', 'left')).toEqual({ id: 11, col: 'code' });
    expect(nextCell(ids, 11, 'name', 'left')).toBeNull();
    expect(nextCell(ids, 99, 'name', 'down')).toBeNull();
  });
});
