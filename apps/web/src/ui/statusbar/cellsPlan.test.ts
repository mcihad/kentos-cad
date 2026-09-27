import { describe, expect, it } from 'vitest';
import type { SaveState } from '../../app/cloud/syncCore';
import type { ServerState } from '../../app/server';
import type { Health } from '../../contracts/generated/Health';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import { fieldProblems, memberProblem } from '../../contracts/testContract';
import {
  LINK_TEXT,
  PROJECT_ACTIONS,
  SAVE_TEXT,
  SERVER_TEXT,
  accountRows,
  ago,
  databaseTip,
  fileTip,
  saveCellAction,
  saveCellView,
  serverTip,
  type SaveCellInput,
} from './cellsPlan';

/**
 * The status bar's cloud cells (fixtures/cloud/v1/cells.json, format in
 * fixtures/cloud/README.md): the save cell's words, state and click, how long
 * ago, both tips, the server cell's words and tip, and the account menu's
 * rows. The file's answers are worked out apart from this code
 * (scripts/fixtures/cells_cases.py); the desktop's status bar checks itself
 * against the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/cloud/v1/cells.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  saveTexts: { state: SaveState; sample: number; text: string }[];
  linkTexts: unknown;
  views: { input: SaveCellInput; view: unknown; action: string | null }[];
  ago: { at: number | null; now: number; text: string }[];
  databaseTips: { input: Parameters<typeof databaseTip>[0]; expect: unknown }[];
  fileTips: { input: Parameters<typeof fileTip>[0]; expect: unknown }[];
  serverTexts: unknown;
  serverTips: { input: { state: ServerState; health: Health | null; detail: string; dev: boolean }; expect: unknown }[];
  projectActions: { command: string; permission: ProjectPermission }[];
  accountRows: { title: string; input: { user: string | null; project: { tenantName: string } | null; disabled: string[]; denied: ProjectPermission[] }; rows: unknown }[];
};

describe('status bar cloud cells (fixtures/cloud/v1/cells.json)', () => {
  it('is a v1 cells file with the cells’ words', () => {
    expect([F.format, F.version]).toEqual(['kentos.cells', 1]);
    for (const c of F.saveTexts) expect(SAVE_TEXT[c.state](c.sample), c.state).toBe(c.text);
    expect(F.saveTexts.map((c) => c.state)).toEqual(Object.keys(SAVE_TEXT));
    expect(LINK_TEXT).toEqual(F.linkTexts);
    expect(SERVER_TEXT).toEqual(F.serverTexts);
    expect(PROJECT_ACTIONS).toEqual(F.projectActions);
  });

  it('says the save state and does the next useful thing on a click', () => {
    for (const c of F.views) {
      expect(saveCellView(c.input), JSON.stringify(c.input)).toEqual(c.view);
      expect(saveCellAction(c.input), JSON.stringify(c.input)).toBe(c.action);
    }
  });

  it('says how long ago', () => {
    for (const c of F.ago) expect(ago(c.at, c.now), String(c.at)).toBe(c.text);
  });

  it('writes both save tips and the server’s', () => {
    for (const c of F.databaseTips) expect(databaseTip(c.input), c.input.state).toEqual(c.expect);
    for (const c of F.fileTips) expect(fileTip(c.input), JSON.stringify(c.input)).toEqual(c.expect);
    for (const c of F.serverTips) expect(serverTip(c.input), c.input.state).toEqual(c.expect);
  });

  it('lists the account menu’s rows, and says which right an off action needs', () => {
    for (const c of F.accountRows)
      expect(accountRows({ user: c.input.user, project: c.input.project, disabled: (id) => c.input.disabled.includes(id), may: (p) => !c.input.denied.includes(p) }), c.title).toEqual(c.rows);
  });

  // The desktop reads the file's server answers and rights into the contract's own types, strictly: a field or
  // a value the contract does not have must fail here too, not only there.
  it('holds the file’s contract values to the generated contract', () => {
    for (const c of F.serverTips) {
      if (!c.input.health) continue;
      expect(fieldProblems(c.input.health, 'Health'), c.input.state).toEqual([]);
      expect(c.input.health.status, c.input.state).toBe('ok');
      expect(typeof c.input.health.commit === 'string' || !('commit' in c.input.health), `${c.input.state}: commit is a text or left out`).toBe(true);
    }
    for (const a of F.projectActions) expect(memberProblem(a.permission, 'ProjectPermission'), a.command).toBeNull();
    for (const c of F.accountRows) for (const p of c.input.denied) expect(memberProblem(p, 'ProjectPermission'), c.title).toBeNull();
  });
});
