import { describe, expect, it } from 'vitest';
import type { EntityKind } from '../../model/entities';
import { BUILTIN_TOOLS } from '../../processing/builtin';
import { BUILTIN_MODELS } from '../../processing/builtin/models';
import { modelAsTool } from '../../processing/modelRunner';
import type { ValidationIssue } from '../../processing/parameters';
import { ProcessingRunner, type TargetChoice } from '../../processing/runner';
import type { ExecutionTarget, FeaturesValue } from '../../processing/types';
import { applyDelta, builtinForms, fixtureTexts, loadDrawing, playSession, type PickRecord, type SessionSpec } from './dialogFixture';
import { footerOf, startState, statusLine, targetsView, type DialogStatus, type DialogView } from './dialogPlan';
import { SCOPE_SHORT } from './dialogTexts';
import { fieldToken, insertText, kindsView, numberOfText, previewIcon, toggleKind } from './fieldPlan';
import { TARGET_LABEL, TARGET_SHORT } from './targets';

/**
 * The processing dialog as fixtures/processing/v1/dialog.json holds it
 * (scripts/fixtures/record-processing-dialog.test.ts; format in
 * fixtures/processing/README.md): the forms, the sessions on parcels.kcad
 * step by step, and the pure rules' tables. The desktop's tool window
 * plays the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/processing/v1/*', { query: '?raw', import: 'default', eager: true });
const file = (name: string): string => {
  const text = files[`../../../../../fixtures/processing/v1/${name}`];
  if (text === undefined) throw new Error(`fixtures/processing/v1/${name} yok`);
  return text;
};

type Step = { do: SessionSpec['steps'][number]['do']; expect: Record<string, unknown> } & Partial<PickRecord>;
type Session = Omit<SessionSpec, 'steps'> & { opened: DialogView; steps: Step[] };
const F = JSON.parse(file('dialog.json')) as {
  format: string;
  version: number;
  document: string;
  texts: unknown;
  scopes: unknown;
  targetLabels: unknown;
  targetShort: unknown;
  forms: unknown;
  sessions: Session[];
  status: { title: string; status: DialogStatus; attempted: boolean; issues: ValidationIssue[]; expect: unknown }[];
  targets: { title: string; declared: ExecutionTarget[]; available: ExecutionTarget[]; auto: ExecutionTarget | null; model: boolean; choice: TargetChoice; expect: unknown }[];
  restore: { title: string; tool: string; stored: Record<string, unknown>; expect: unknown }[];
  kinds: { title: string; tool: string; value: FeaturesValue; present: { kind: EntityKind; count: number }[]; click: EntityKind; expect: unknown }[];
  tokens: { name: string; token: string }[];
  inserts: { text: string; start: number; end: number; insert: string; expect: unknown }[];
  numbers: { text: string; value: number | null }[];
  icons: { text: string; icon: string }[];
};
const plain = (v: unknown): unknown => JSON.parse(JSON.stringify(v));

const TOOLS = new Map([
  ...BUILTIN_TOOLS.map((t) => [t.id, t] as const),
  ...BUILTIN_MODELS.map((m) => [`model:${m.id}`, modelAsTool(m, (id) => BUILTIN_TOOLS.find((t) => t.id === id))] as const),
]);

describe('processing dialog (fixtures/processing/v1/dialog.json)', () => {
  it('is a v1 dialog file with the dialog’s texts and names', () => {
    expect([F.format, F.version, F.document]).toEqual(['kentos.processing-dialog', 1, 'parcels.kcad']);
    expect(fixtureTexts()).toEqual(F.texts);
    expect([SCOPE_SHORT, TARGET_LABEL, TARGET_SHORT]).toEqual([F.scopes, F.targetLabels, F.targetShort]);
  });

  it('builds every built-in tool’s and model’s form', () => {
    expect(plain(builtinForms())).toEqual(F.forms);
  });

  for (const s of F.sessions)
    it(`plays the session ${s.id}`, async () => {
      const { views, picks } = await playSession(s, file(F.document));
      expect(views.length).toBe(s.steps.length + 1);
      let want = s.opened;
      expect(views[0], `${s.id}: açılış`).toEqual(want);
      s.steps.forEach((step, i) => {
        want = applyDelta(want, step.expect);
        expect(views[i + 1], `${s.id}: ${i + 1}. adım ${JSON.stringify(step.do)}`).toEqual(want);
        // Sahneden seç: the selection and the command line while picking, and after.
        expect(picks[i], `${s.id}: ${i + 1}. adımda çizimden seçim`).toEqual(step.picking ? { picking: step.picking, after: step.after } : null);
      });
    });

  it('says what the status line and the footer show', () => {
    for (const c of F.status) expect(plain({ status: statusLine(c.status, c.attempted, c.issues), footer: footerOf(c.status) }), c.title).toEqual(c.expect);
  });

  it('lists where a tool runs', () => {
    for (const c of F.targets) expect(plain(targetsView({ declared: c.declared, available: new Set(c.available), auto: c.auto, model: c.model }, c.choice)), c.title).toEqual(c.expect);
  });

  it('restores stored values over the defaults', () => {
    const runner = new ProcessingRunner({ doc: loadDrawing(file(F.document)), selectedIds: () => [], visibleBounds: () => null });
    for (const c of F.restore) {
      const tool = TOOLS.get(c.tool)!;
      const state = startState(tool, undefined, c.stored, runner.defaults(), 'auto');
      expect(plain({ values: state.values, advancedOpen: state.advancedOpen, issues: runner.validate(tool, state.values) }), c.title).toEqual(c.expect);
    }
  });

  it('shows and toggles kind chips', () => {
    for (const c of F.kinds) {
      const def = TOOLS.get(c.tool)!.parameters.find((p) => p.type === 'features');
      if (def?.type !== 'features') throw new Error(c.tool);
      const after = toggleKind(c.value, c.present, c.click);
      expect(plain({ before: kindsView(def, c.value, c.present), after, afterView: kindsView(def, after, c.present) }), c.title).toEqual(c.expect);
    }
  });

  it('writes field names in expressions, inserts at the caret, reads number text and picks the line’s icon', () => {
    for (const t of F.tokens) expect(fieldToken(t.name), t.name).toBe(t.token);
    for (const c of F.inserts) expect(insertText(c.text, c.start, c.end, c.insert), JSON.stringify(c)).toEqual(c.expect);
    for (const n of F.numbers) expect(plain(numberOfText(n.text)), JSON.stringify(n.text)).toEqual(n.value);
    for (const l of F.icons) expect(previewIcon(l.text), l.text).toBe(l.icon);
  });
});
