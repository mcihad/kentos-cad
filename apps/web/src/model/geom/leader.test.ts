import { describe, expect, it } from 'vitest';
import layout from '../../../../../fixtures/leader/v1/layout.json?raw';
import type { LeaderEntity } from '../entities';
import type { Vec2 } from '../geometry';
import { leaderLayout } from './leader';

/**
 * A leader's layout (docs/adr/0146 §2) through WASM against the shared cases (fixtures/leader/v1/layout.json,
 * written from the rule alone by scripts/fixtures/leader_cases.py); natively
 * crates/shared/geometry-core/tests/all/leader.rs.
 */

interface Case {
  name: string;
  leader: Omit<LeaderEntity, 'kind' | 'id' | 'layerId' | 'attrs'>;
  want: {
    head: { fills: Vec2[][]; lines: { pts: Vec2[]; closed: boolean }[]; back: number };
    start: Vec2;
    first: number;
    side: number;
    landing?: Vec2[];
    notePoint?: Vec2;
    noteAlign?: string;
  };
}

const near = (a: number, e: number) => Math.abs(a - e) <= 1e-9 + 1e-15 * Math.abs(e);
const at = (p: Vec2 | undefined, e: Vec2 | undefined) => !!p && !!e && near(p.x, e.x) && near(p.y, e.y);
const all = (ps: readonly Vec2[] | undefined, es: readonly Vec2[] | undefined) => !!ps && !!es && ps.length === es.length && ps.every((p, i) => at(p, es[i]));

describe('leader layout (fixtures/leader/v1)', () => {
  it('lays every shared case out as the independent reference does', () => {
    const file = JSON.parse(layout) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.leader-cases', 2]);
    expect(file.cases).toHaveLength(22);
    for (const c of file.cases) {
      const got = leaderLayout({ ...c.leader, kind: 'leader', id: 1, layerId: '0', attrs: {} });
      expect(got, c.name).not.toBeNull();
      if (!got) continue;
      const { head, start, first, side, landing, notePoint, noteAlign } = c.want;
      // Its arrowhead (docs/adr/0205 §7): its areas, its lines, where its line starts.
      expect(got.head.fills.length === head.fills.length && got.head.fills.every((r, i) => all(r, head.fills[i])), c.name).toBe(true);
      expect(got.head.lines.length === head.lines.length && got.head.lines.every((l, i) => all(l.pts, head.lines[i].pts) && l.closed === head.lines[i].closed), c.name).toBe(true);
      expect(near(got.head.back, head.back) && at(got.start, start) && got.first === first, c.name).toBe(true);
      expect(got.side, c.name).toBe(side);
      expect(got.landing === undefined, c.name).toBe(landing === undefined);
      if (landing) expect(all(got.landing, landing) && at(got.notePoint, notePoint), c.name).toBe(true);
      expect(got.noteAlign, c.name).toBe(noteAlign);
    }
  });
});
