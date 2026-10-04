import { describe, expect, it } from 'vitest';
import { Formatter } from '../../app/format';
import { Signal } from '../../core/signal';
import type { Entity, NewEntity } from '../../model/entities';
import { elevatedPaths } from '../../product/elevation';
import { toolHarness } from '../../tools/toolHarness';
import { cellText, removeVertices, vertexRows, writeVertexCell, writeVertexDraft, type Editable, type VertexColumn } from './vertexEdit';

/**
 * Köşe tablosu's writes against fixtures/vertex-table/v1/edits.json (scripts/fixtures/vertex_edit_cases.py, worked out
 * from the rules with no KentOS code): every case's drawing after a cell, a draft or a removal is written, the messages
 * said, the undo step and whether the cell stays open, as the desktop's `vertices/edit.rs` gives them too.
 * Coordinates and elevations bit for bit, bulges within 1e-12 (relative), a missing field null.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  name: string;
  objects: (NewEntity & { id: number })[];
  workspace: 'gis' | 'cad';
  cell?: { id: number; path: number; index: number; column: VertexColumn; text: string };
  draft?: { id: number; path: number; after: number; east: string; north: string; z: string };
  remove?: { id: number; at: [number, number][] };
  expected: { said: string[]; step: string | null; stay: boolean; next?: [number, number] | null; objects: unknown[] };
}

function differ(a: unknown, e: unknown, path: string): string | null {
  if (typeof a === 'number' && typeof e === 'number') {
    const close = path.includes('.bulges') ? Math.abs(a - e) <= 1e-12 * Math.max(Math.abs(e), 1) : a === e;
    return close ? null : `${path}: ${a} ≠ ${e}`;
  }
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

/** An object as the cases compare it: line work's paths with their elevations and bulges (null when straight). */
function view(e: Entity): unknown {
  if (e.kind !== 'line' && e.kind !== 'polyline' && e.kind !== 'polygon') return { kind: e.kind };
  return {
    kind: e.kind,
    paths: elevatedPaths(e).map((p) => {
      const b = p.pts.map((_, i) => p.bulges?.[i] ?? 0);
      return { pts: p.pts.map((q) => [q.x, q.y]), zs: p.zs, bulges: b.some((x) => Math.abs(x) > 1e-12) ? b : null };
    }),
  };
}

describe('Köşe tablosu: yazma', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/vertex-table/v1/edits.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };

  it('is the reference’s file', () => {
    expect([file.format, file.version]).toEqual(['kentos.vertex-edits', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(50);
  });

  it('writes every case as the reference does', () => {
    const off = file.cases.flatMap((c) => {
      const h = toolHarness();
      for (const { id: _id, ...o } of c.objects) h.add(o as never);
      h.doc.settings.workspace.set(c.workspace);
      const f = new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2'), angleUnit: new Signal('grad'), workspace: h.doc.settings.workspace });
      const length = (v: number) => f.length(v);
      const id = (c.cell ?? c.draft ?? c.remove)!.id;
      const e = h.doc.get(id) as Editable;
      const before = h.doc.revision;
      let got: { said: string[]; step: string | null; stay: boolean; next?: [number, number] | null };
      if (c.cell) {
        got = writeVertexCell(h.doc, e, { path: c.cell.path, index: c.cell.index }, c.cell.column, c.cell.text, length);
      } else if (c.draft) {
        const out = writeVertexDraft(h.doc, e, { path: c.draft.path, index: c.draft.after }, c.draft, length);
        got = { ...out, next: out.next ? [out.next.path, out.next.index] : null };
      } else {
        got = removeVertices(h.doc, e, c.remove!.at.map(([path, index]) => ({ path, index })), length);
      }
      const objects = [...h.doc.all()].map(view);
      const step = got.step === null ? (h.doc.revision === before ? null : 'yazıldı') : h.doc.undo();
      const seen = { said: got.said, step, stay: got.stay, ...(c.draft && { next: got.next ?? null }), objects };
      const d = differ(seen, c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });

  it('opens a cell with the value whole, in a local project’s unit', () => {
    const h = toolHarness();
    const e = h.add({ kind: 'polyline', pts: [{ x: 1.25, y: 2 }, { x: 3, y: 4.5 }, { x: 6, y: 4.5 }], bulges: [0.5, 0, 0], zs: [10, null, null] } as never) as Editable;
    const [first, second] = vertexRows(e);
    expect([cellText(first, 'east'), cellText(first, 'north'), cellText(first, 'z'), cellText(second, 'z')]).toEqual(['1.25', '2', '10', '']);
    expect(Number(cellText(first, 'radius'))).toBeCloseTo(first.radius!, 12);
    expect(cellText(second, 'radius')).toBe('');
    expect(cellText(first, 'east', 100)).toBe('125');
  });
});
