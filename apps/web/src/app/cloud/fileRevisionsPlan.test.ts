import { describe, expect, it } from 'vitest';
import type { FileRevisions } from '../../contracts/generated/FileRevisions';
import { fieldProblems, memberProblem } from '../../contracts/testContract';
import { fileTip, saveCellAction, saveCellView, type FileSave } from '../../ui/statusbar/cellsPlan';
import {
  HISTORY_MARKS,
  REVISION_TEXTS,
  RESYNC_RETRY_MS,
  cellState,
  newerLine,
  newestOf,
  offer,
  readEvents,
  resyncStep,
  revisionMarks,
  revisionWho,
  saveStep,
  step,
  type NewerRevision,
  type Offer,
  type ProjectAnswer,
  type RevisionEvent,
  type RevisionInput,
  type RevisionState,
} from './fileRevisionsPlan';

/**
 * An open file project's revisions (fixtures/cloud/v1/file-revisions.json,
 * format in fixtures/cloud/README.md; docs/specs/file-revisions.md): the
 * state and what changes it, the save cell, what Kaydet does first, the
 * events, the newest revision, the resync, the questions and their answers,
 * the words, and whole sequences. The file's answers are worked out apart
 * from this code (scripts/fixtures/file_revisions_cases.py); the desktop
 * reads the same file.
 */

type TraceStep =
  | { input: RevisionInput; say: boolean; cell: unknown }
  | { save: string; cell: unknown }
  | { offer: 'newest' | 'conflict'; busy: boolean; expect: unknown };

const files = import.meta.glob<string>('../../../../../fixtures/cloud/v1/file-revisions.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  timeZone: string;
  resyncRetryMs: number;
  texts: Record<'busy' | 'unreadableDeleted' | 'unreadableRevoked' | 'resyncFailed' | 'detached' | 'openFailed', { sample: string[]; text: string }>;
  marks: unknown;
  who: { newer: NewerRevision; text: string }[];
  cellStates: { title: string; state: RevisionState; expect: string }[];
  steps: { title: string; from: RevisionState; input: RevisionInput; expect: unknown }[];
  saveSteps: { title: string; state: RevisionState; expect: string }[];
  events: { title: string; own: string[]; events: RevisionEvent[]; expect: unknown }[];
  newest: { title: string; revisions: FileRevisions; expect: unknown }[];
  resync: { title: string; answer: ProjectAnswer; expect: unknown }[];
  offers: { title: string; input: { name: string; state: RevisionState; busy: boolean; via: 'newest' | 'conflict' }; expect: unknown }[];
  lines: { title: string; name: string; newer: NewerRevision; base: string; dirty: boolean; text: string }[];
  tips: { input: Parameters<typeof fileTip>[0]; expect: unknown }[];
  cells: { title: string; state: RevisionState; expect: unknown }[];
  historyMarks: { title: string; revision: string; current: string; openBase: string | null; expect: string[] }[];
  traces: { title: string; start: RevisionState; steps: TraceStep[]; end: RevisionState }[];
};

// Times are the device's local time; the cases fix the zone (Node reads TZ again when it changes).
(globalThis as unknown as { process: { env: Record<string, string> } }).process.env.TZ = F.timeZone;

/** The save cell as the status bar reads it from a state (no upload under way: progress 0). */
function cellOf(s: RevisionState): { state: string; text: string; action: string | null } {
  const input: FileSave = {
    kind: 'file',
    state: cellState(s),
    base: s.base,
    progress: 0,
    conflictActual: s.conflict?.actual ?? null,
    newerRevision: s.newer?.revision ?? null,
    dirty: s.dirty,
  };
  const view = saveCellView(input);
  return { state: view.state!, text: view.text, action: saveCellAction(input) };
}

/** An offer as a trace names it: its kind, and the question's id and answers, or the line. */
function offerOf(o: Offer): unknown {
  if (o.kind === 'ask') return { kind: 'ask', id: o.question.id, answers: o.question.answers.map((a) => a.value) };
  if (o.kind === 'say') return { kind: 'say', line: o.line };
  return { kind: 'none' };
}

describe('file project revisions (fixtures/cloud/v1/file-revisions.json)', () => {
  it('is a v1 file with the words', () => {
    expect([F.format, F.version, F.resyncRetryMs]).toEqual(['kentos.fileRevisions', 1, RESYNC_RETRY_MS]);
    const [name, why] = F.texts.openFailed.sample;
    expect(REVISION_TEXTS.openFailed(name, why)).toBe(F.texts.openFailed.text);
    expect(REVISION_TEXTS.busy(F.texts.busy.sample[0])).toBe(F.texts.busy.text);
    expect(REVISION_TEXTS.unreadable.deleted(F.texts.unreadableDeleted.sample[0])).toBe(F.texts.unreadableDeleted.text);
    expect(REVISION_TEXTS.unreadable.revoked(F.texts.unreadableRevoked.sample[0])).toBe(F.texts.unreadableRevoked.text);
    expect(REVISION_TEXTS.resyncFailed(F.texts.resyncFailed.sample[0])).toBe(F.texts.resyncFailed.text);
    expect(REVISION_TEXTS.detached(F.texts.detached.sample[0])).toBe(F.texts.detached.text);
    expect(HISTORY_MARKS).toEqual(F.marks);
    for (const c of F.who) expect(revisionWho(c.newer), JSON.stringify(c.newer)).toBe(c.text);
  });

  it('holds values the contract holds (a strict reader refuses others)', () => {
    for (const c of F.newest) for (const r of c.revisions.revisions) expect(fieldProblems(r, 'FileRevision'), c.title).toEqual([]);
    for (const c of F.newest) expect(fieldProblems(c.revisions, 'FileRevisions'), c.title).toEqual([]);
    for (const c of F.resync) if (c.answer.kind === 'project') expect(memberProblem(c.answer.state, 'ProjectState'), c.title).toBeNull();
  });

  it('says the cell’s state from what the project knows', () => {
    for (const c of F.cellStates) expect(cellState(c.state), c.title).toBe(c.expect);
    for (const c of F.cells) expect(cellOf(c.state), c.title).toEqual(c.expect);
  });

  it('changes what the project knows one input at a time', () => {
    for (const c of F.steps) expect(step(c.from, c.input), c.title).toEqual(c.expect);
  });

  it('decides what Kaydet does first', () => {
    for (const c of F.saveSteps) expect(saveStep(c.state), c.title).toBe(c.expect);
  });

  it('reads the events, the newest revision and the resync’s answer', () => {
    for (const c of F.events) expect(readEvents(c.events, (id) => c.own.includes(id)), c.title).toEqual(c.expect);
    for (const c of F.newest) expect(newestOf(c.revisions), c.title).toEqual(c.expect);
    for (const c of F.resync) expect(resyncStep(c.answer), c.title).toEqual(c.expect);
  });

  it('asks the question the state calls for, with its words and what each answer does', () => {
    for (const c of F.offers) expect(offer({ name: c.input.name, s: c.input.state, busy: c.input.busy, via: c.input.via }), c.title).toEqual(c.expect);
  });

  it('says a newer revision in the log and the tip, and marks the history', () => {
    for (const c of F.lines) expect(newerLine(c.name, c.newer, c.base, c.dirty), c.title).toBe(c.text);
    for (const c of F.tips) expect(fileTip(c.input), JSON.stringify(c.input)).toEqual(c.expect);
    for (const c of F.historyMarks) expect(revisionMarks(c.revision, c.current, c.openBase), c.title).toEqual(c.expect);
  });

  it('keeps what it knows consistent through any inputs: a conflict is with the newer revision, which is above the base; an end stays', () => {
    // A fixed pseudo-random walk (the same every run).
    let seed = 20260928;
    const rand = (n: number) => ((seed = (seed * 1103515245 + 12345) % 2147483648) % n);
    const newest = (): NewerRevision | null => (rand(8) === 0 ? null : { revision: String(rand(9)), by: ['', 'Mehmet Demir', 'Zeynep Kaya'][rand(3)], at: rand(2) ? '2026-09-27T11:32:00Z' : null });
    const inputs = (): RevisionInput => {
      switch (rand(9)) {
        case 0:
          return { kind: 'dirty', dirty: rand(2) === 0 };
        case 1:
        case 2:
          return { kind: 'newest', newest: newest() };
        case 3:
          return { kind: 'stage', stage: (['encoding', 'uploading', 'verifying'] as const)[rand(3)] };
        case 4:
          return { kind: 'committed', revision: String(rand(9)), dirty: rand(2) === 0 };
        case 5:
          return { kind: 'refused', actual: String(rand(9)) };
        case 6:
          return rand(2) ? { kind: 'failed' } : { kind: 'unchanged' };
        case 7:
          return { kind: 'access', writable: rand(3) !== 0 };
        default:
          return rand(6) ? { kind: 'dirty', dirty: rand(2) === 0 } : { kind: 'ended', why: (['deleted', 'revoked', 'archived'] as const)[rand(3)] };
      }
    };
    for (let run = 0; run < 200; run++) {
      let s: RevisionState = { base: String(rand(4)), newer: null, conflict: null, dirty: false, stage: 'idle', failed: false, writable: true, ended: null };
      for (let i = 0; i < 40; i++) {
        const input = inputs();
        // As the server does: Kaydet commits only without a conflict, and commits or refuses on a revision above the drawing's base.
        if (input.kind === 'committed' && (s.conflict || Number(input.revision) <= Number(s.base))) continue;
        if (input.kind === 'refused' && Number(input.actual) <= Number(s.base)) continue;
        const before = s;
        s = step(s, input).state;
        const where = `${run}/${i}: ${JSON.stringify(input)} → ${JSON.stringify(s)}`;
        if (s.conflict) expect(s.newer?.revision, where).toBe(s.conflict.actual);
        if (s.newer) expect(Number(s.newer.revision) > Number(s.base), where).toBe(true);
        if (before.ended) expect([s.ended, s.base, s.newer, s.conflict], where).toEqual([before.ended, before.base, before.newer, before.conflict]);
      }
    }
  });

  it('goes through whole sequences as the file does', () => {
    for (const t of F.traces) {
      let s = t.start;
      for (const [i, x] of t.steps.entries()) {
        const where = `${t.title}: ${i + 1}`;
        if ('input' in x) {
          const r = step(s, x.input);
          s = r.state;
          expect([r.say, cellOf(s)], where).toEqual([x.say, x.cell]);
        } else if ('save' in x) {
          const first = saveStep(s);
          if (first === 'behind') s = step(s, { kind: 'refused', actual: s.newer!.revision }).state;
          else if (first === 'unchanged') s = step(s, { kind: 'unchanged' }).state;
          expect([first, cellOf(s)], where).toEqual([x.save, x.cell]);
        } else expect(offerOf(offer({ name: 'Kadastro paftası 2026', s, busy: x.busy, via: x.offer })), where).toEqual(x.expect);
      }
      expect(s, t.title).toEqual(t.end);
    }
  });
});
