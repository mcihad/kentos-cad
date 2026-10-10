import { describe, expect, it } from 'vitest';
import type { ProjectVariable } from '../contracts/generated/ProjectVariable';
import type { VariableKind } from '../contracts/generated/VariableKind';
import { ProjectSettings } from './projectSettings';
import { documentSource, expressionVariables } from './projectVariables';
import { newVariableRow, readVariables, VARIABLE_TEXTS, variableRows, variablesBlocked, withKind, type VariableRow } from './variableForm';

/**
 * Proje ayarları › Değişkenler's form and the built-in `@` values (docs/adr/0214 §2.3, §4) against the shared cases
 * (fixtures/project/v1/variable-form.json, written by scripts/fixtures/variable_form_cases.py from the rules alone).
 * The desktop reads the same file (crates/native/project/src/variable_form.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Value = string | number | boolean | null;
interface File {
  format: string;
  messages: Omit<typeof VARIABLE_TEXTS, 'textLong'>;
  reads: { case: string; rows: VariableRow[]; variables: ProjectVariable[]; problems: { name: string | null; value: string | null }[]; list: string | null }[];
  rows: { case: string; variables: ProjectVariable[]; rows: VariableRow[] }[];
  newRows: { case: string; rows: VariableRow[]; row: VariableRow }[];
  kinds: { case: string; row: VariableRow; kind: VariableKind; result: VariableRow }[];
  builtins: {
    case: string;
    name: string;
    user: string;
    now: string;
    settings: { srid: number; plotScale: number; customCrs?: { name: string }; variables: ProjectVariable[] };
    variables: { name: string; value: Value; description: string }[];
  }[];
  tooMany: string;
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/project/v1/variable-form.json', import.meta.url), 'utf8')) as File;

/** A local wall moment `YYYY-AA-GGTSS:DD:ss.mmm` as this device's Date. */
function local(iso: string): Date {
  const [y, mo, d, h, mi, s, ms] = iso.split(/[-T:.]/).map(Number);
  return new Date(y, mo - 1, d, h, mi, s, ms);
}

describe('Proje ayarları › Değişkenler (docs/adr/0214 §4)', () => {
  it('reads and writes the rows as the shared cases say', () => {
    expect(file.format).toBe('kentos.variable-form');
    const { textLong: _long, ...messages } = VARIABLE_TEXTS;
    expect(file.messages).toEqual(messages);
    let rows = 0;
    for (const c of file.reads) {
      const r = readVariables(c.rows);
      expect(r.variables, c.case).toEqual(c.variables);
      expect(r.problems, c.case).toEqual(c.problems);
      expect(r.list).toBe(c.list);
      rows += c.rows.length;
    }
    expect(rows).toBeGreaterThanOrEqual(30);
    for (const c of file.rows) {
      expect(variableRows(c.variables), c.case).toEqual(c.rows);
      // Written and read back, the variables are the same (-0 as 0).
      const back = readVariables(variableRows(c.variables)).variables;
      expect(back, c.case).toEqual(c.variables.map((v) => (Object.is(v.value, -0) ? { ...v, value: 0 } : v)));
    }
    for (const c of file.newRows) expect(newVariableRow(c.rows), c.case).toEqual(c.row);
    for (const c of file.kinds) expect(withKind(c.row, c.kind), c.case).toEqual(c.result);
    const many = readVariables(Array.from({ length: 201 }, (_, i) => ({ name: `v${i}`, label: '', kind: 'text' as const, value: '' })));
    expect(many.list).toBe(file.tooMany);
    expect(variablesBlocked(many)).toBe(true);
  });

  it('gives the built-in values the shared cases say', () => {
    for (const c of file.builtins) {
      const s = c.settings;
      // A definition of the project's own: the registry's TM30 under another name (as the desktop's case).
      const customCrs = s.customCrs
        ? { name: s.customCrs.name, system: { kind: 'tm' as const, datum: 'TUREF' as const, centralMeridian: 30, scaleFactor: 1, falseEasting: 500000, falseNorthing: 0 } }
        : undefined;
      const settings = new ProjectSettings({ srid: s.srid, plotScale: s.plotScale, variables: s.variables, ...(customCrs ? { customCrs } : {}) });
      const got = expressionVariables(documentSource(c.name, settings), local(c.now), c.user);
      expect(
        got.map((v) => ({ name: v.name, value: v.value, description: v.description })),
        c.case,
      ).toEqual(c.variables);
    }
  });
});
