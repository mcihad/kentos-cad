import { describe, expect, it } from 'vitest';
import { isUuid } from '../core/uuid';
import { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import { LayerStore } from '../model/layers';
import { readSnapshot } from '../model/snapshot';
import type { ProductCommand } from './command';
import { findProductCommand, WEB_COMMANDS } from './registry';

/**
 * The shared product command cases (fixtures/commands/v1/*.json, TODOS.md
 * CMD-04..07, docs/adr/0022, 0027) run against the web's handlers over
 * CadDocument. The desktop runs the same files against its own
 * (crates/native/application/tests/fixtures.rs). The format is in
 * fixtures/commands/README.md.
 *
 * JSON carries no NaN or ±∞: a step's `nonFinite` puts them into the input
 * after it is read, at the paths the errors name.
 */

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

interface Expect {
  ids?: number[];
  entities?: Record<string, Json>;
  canUndo?: boolean;
  canRedo?: boolean;
  dirty?: boolean;
  revision?: 'same' | 'changed';
  /** Object → the name of a persistent id taken with `captureUid`, or "new": one taken by none. */
  uids?: Record<string, string>;
}

interface Step {
  op: string;
  input?: Json;
  nonFinite?: Record<string, 'NaN' | 'Infinity' | '-Infinity'>;
  result?: Json;
  returns?: Json;
  as?: string;
  id?: number;
  expect?: Expect;
  note?: string;
}

interface Case {
  name: string;
  setup?: Json;
  steps: Step[];
}

interface Fixture {
  format: string;
  version: number;
  command: string;
  commandVersion: number;
  title: string;
  setup: Json;
  cases: Case[];
}

const STEP_KEYS: readonly string[] = ['op', 'input', 'nonFinite', 'result', 'returns', 'as', 'id', 'expect', 'note'];
const EXPECT_KEYS: readonly string[] = ['ids', 'entities', 'canUndo', 'canRedo', 'dirty', 'revision', 'uids'];
const NON_FINITE = { NaN: Number.NaN, Infinity: Number.POSITIVE_INFINITY, '-Infinity': Number.NEGATIVE_INFINITY } as const;

const files = import.meta.glob<string>('../../../../fixtures/commands/v1/*.json', { query: '?raw', import: 'default', eager: true });

/** Persistent ids are random (UUIDv7): objects are compared without them, and ids only with each other. */
const withoutUid = (e: Entity | undefined) => {
  if (!e) return e;
  const { uid: _uid, ...rest } = e;
  return rest;
};

/** `path` (`pts[1].y`, `holes[0].bulges[2]`) as keys and indices. */
const segments = (path: string): (string | number)[] => path.split(/[.[\]]+/).filter(Boolean).map((s) => (/^\d+$/.test(s) ? Number(s) : s));

class Run {
  readonly doc: CadDocument;
  private readonly command: ProductCommand<unknown, unknown, unknown>;
  private readonly revisions = new Map<string, string>();
  private readonly uids = new Map<string, string>();

  constructor(setup: Json, command: ProductCommand<unknown, unknown, unknown>) {
    this.command = command;
    const read = readSnapshot(JSON.stringify(setup));
    if (!read.ok) throw new Error(`Kurulum dosyası okunamadı: ${read.error}`);
    this.doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
    this.doc.replaceWith(read.content);
  }

  /**
   * `value` with its `$…` placeholders filled in: `$current` is the revision now, `$name` one taken by
   * `captureRevision`; `$uid:name`, anywhere in a text (an id list, a message), the persistent id taken by `captureUid`.
   */
  private fill(value: Json, where: string): Json {
    if (typeof value === 'string' && value.includes('$uid:'))
      return value.replace(/\$uid:([A-Za-z0-9_-]+)/g, (_, name: string) => {
        const uid = this.uids.get(name);
        if (uid === undefined) throw new Error(`${where}: “${name}” kimliği alınmadı`);
        return uid;
      });
    if (typeof value === 'string' && value.startsWith('$')) {
      const name = value.slice(1);
      if (name === 'current') return String(this.doc.revision);
      const taken = this.revisions.get(name);
      if (taken === undefined) throw new Error(`${where}: “${name}” sürümü alınmadı`);
      return taken;
    }
    if (Array.isArray(value)) return value.map((v) => this.fill(v, where));
    if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, this.fill(v, where)]));
    return value;
  }

  step(s: Step, where: string): void {
    expect(
      Object.keys(s).filter((k) => !STEP_KEYS.includes(k)),
      `${where}: bilinmeyen alan`,
    ).toEqual([]);
    const before = this.doc.revision;
    switch (s.op) {
      case 'validate':
      case 'plan':
      case 'execute':
        this.run(s, where);
        break;
      case 'undo':
      case 'redo': {
        const label = s.op === 'undo' ? this.doc.undo() : this.doc.redo();
        if ('returns' in s) expect(label, `${where}: dönen değer`).toEqual(s.returns);
        break;
      }
      case 'captureRevision':
        this.revisions.set(s.as!, String(this.doc.revision));
        break;
      case 'captureUid': {
        const uid = this.doc.uidOf(s.id!);
        if (uid === undefined) throw new Error(`${where}: ${s.id} nesnesi yok`);
        this.uids.set(s.as!, uid);
        break;
      }
      default:
        throw new Error(`${where}: bilinmeyen işlem “${s.op}”`);
    }
    this.check(s.expect, before, where);
  }

  /** Runs one command step and compares its whole result, as the wire carries it. */
  private run(s: Step, where: string): void {
    const input = this.fill(s.input ?? null, where) as Record<string, unknown>;
    for (const [path, name] of Object.entries(s.nonFinite ?? {})) {
      const keys = segments(path);
      let at: Record<string | number, unknown> = input;
      for (const key of keys.slice(0, -1)) {
        at = at?.[key] as Record<string | number, unknown>;
        if (at === null || typeof at !== 'object') throw new Error(`${where}: girdide böyle bir yer yok: ${path}`);
      }
      const last = keys.at(-1)!;
      if (typeof at[last] !== 'number') throw new Error(`${where}: girdide böyle bir yer yok: ${path}`);
      at[last] = NON_FINITE[name];
    }
    if (s.result === undefined) throw new Error(`${where}: komut adımında “result” yok`);
    const cx = { doc: this.doc };
    const result = s.op === 'validate' ? this.command.validate(cx, input) : s.op === 'plan' ? this.command.plan(cx, input) : this.command.execute(cx, input);
    const got = JSON.parse(JSON.stringify(result)) as { output?: { uid?: string; id?: number } };
    const want = structuredClone(s.result) as { output?: { uid?: string } };
    // “$uid”: the persistent id of the object the command wrote, lowercase with hyphens.
    if (want.output?.uid === '$uid') {
      const uid = got.output?.uid;
      expect(isUuid(uid), `${where}: output.uid bir UUID olmalı: ${uid}`).toBe(true);
      expect(uid, `${where}: output.uid yuvadaki nesnenin kimliği olmalı`).toBe(this.doc.uidOf(got.output?.id ?? -1));
      want.output.uid = uid;
    }
    expect(got, `${where}: sonuç`).toEqual(this.fill(want as Json, where));
  }

  private check(e: Expect | undefined, before: number, where: string): void {
    if (!e) return;
    // A misspelt expectation would otherwise pass unchecked.
    expect(
      Object.keys(e).filter((k) => !EXPECT_KEYS.includes(k)),
      `${where}: bilinmeyen beklenti`,
    ).toEqual([]);
    const doc = this.doc;
    if (e.ids) expect([...doc.all()].map((x) => x.id), `${where}: nesneler`).toEqual(e.ids);
    for (const [id, want] of Object.entries(e.entities ?? {})) expect(withoutUid(doc.get(Number(id))), `${where}: nesne ${id}`).toEqual(want);
    if (e.canUndo !== undefined) expect(doc.canUndo.value, `${where}: canUndo`).toBe(e.canUndo);
    if (e.canRedo !== undefined) expect(doc.canRedo.value, `${where}: canRedo`).toBe(e.canRedo);
    if (e.dirty !== undefined) expect(doc.dirty.value, `${where}: dirty`).toBe(e.dirty);
    if (e.revision) expect(doc.revision === before ? 'same' : 'changed', `${where}: sürüm`).toBe(e.revision);
    for (const [id, name] of Object.entries(e.uids ?? {})) {
      const uid = doc.uidOf(Number(id));
      expect(isUuid(uid), `${where}: ${id} nesnesinin kalıcı kimliği`).toBe(true);
      if (name === 'new') expect([...this.uids.values()], `${where}: ${id} nesnesinin kimliği yeni olmalı`).not.toContain(uid);
      else {
        expect(this.uids.has(name), `${where}: “${name}” kimliği alınmadı`).toBe(true);
        expect(uid, `${where}: ${id} nesnesinin kimliği “${name}” olmalı`).toBe(this.uids.get(name));
      }
    }
  }
}

describe('product command cases (fixtures/commands/v1)', () => {
  it('finds a case file for every command the web runs', () => {
    const covered = Object.values(files).map((text) => (JSON.parse(text) as Fixture).command);
    for (const c of WEB_COMMANDS) expect(covered, `${c.id}: fixtures/commands/v1'de dosyası yok`).toContain(c.id);
  });
  for (const [path, text] of Object.entries(files)) {
    const fixture = JSON.parse(text) as Fixture;
    const file = path.split('/').pop();
    describe(`${file}: ${fixture.title}`, () => {
      const command = findProductCommand(fixture.command, fixture.commandVersion) as ProductCommand<unknown, unknown, unknown> | undefined;
      it('is a command-cases v1 file of a command the web runs', () => {
        expect(fixture.format).toBe('kentos.command-cases');
        expect(fixture.version).toBe(1);
        expect(command, `${fixture.command} v${fixture.commandVersion} web'de çalışmıyor`).toBeDefined();
        expect(fixture.cases.length).toBeGreaterThanOrEqual(20);
      });
      if (!command) return;
      for (const c of fixture.cases)
        it(c.name, () => {
          expect(c.steps.length, 'adım yok').toBeGreaterThan(0);
          const run = new Run(c.setup ?? fixture.setup, command);
          expect(run.doc.dirty.value || run.doc.canUndo.value, 'kurulumdan sonra belge temiz değil').toBe(false);
          c.steps.forEach((s, i) => run.step(s, `${c.name} › ${i + 1} ${s.op}`));
        });
    });
  }
});
