import { describe, expect, it } from 'vitest';
import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { fieldsOf, membersOf } from '../../contracts/testContract';
import { SORT_LABEL, STATE_LABEL, TYPE_HINT, TYPE_LABEL, VIEWS } from './catalog';
import {
  ARCHIVED_TEXT,
  CATALOG_LINES,
  EMPTY_SEARCH,
  NO_ORGANIZATION,
  archiveQuestion,
  deniedText,
  detailPlan,
  primaryPlan,
  purgeQuestion,
  rowPlan,
  trashQuestion,
} from './catalogPlan';

/**
 * The catalog's words and rules (fixtures/cloud/v1/catalog.json, format in
 * fixtures/cloud/README.md): the lists, the labels, a row's texts, the
 * selected project's pane, the main button, the lifecycle's questions and
 * lines. The desktop's catalog checks itself against the same file.
 */

const files = import.meta.glob<string>('../../../../../fixtures/cloud/v1/catalog.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  timeZone: string;
  views: { id: string; label: string; sorts: string[]; empty: string; note?: string }[];
  sorts: Record<string, string>;
  states: Record<string, string>;
  types: Record<string, string>;
  typeHint: string;
  emptySearch: string;
  noOrganization: string;
  tips: { denied: string; archived: string };
  project: ProjectSummary;
  details: { id: string; project: Partial<ProjectSummary>; open: boolean; expect: unknown }[];
  primary: { id: string; view: CatalogView; project: Partial<ProjectSummary> | null; expect: unknown }[];
  rows: { id: string; view: CatalogView; sort: CatalogSort; open: boolean; place: string; project: Partial<ProjectSummary>; expect: unknown }[];
  questions: Record<'trash' | 'purge' | 'archive', { id: string; args: { name: string; tenant?: string; days?: number | null; open?: boolean }; expect: unknown }[]>;
  lines: { line: keyof typeof CATALOG_LINES; args: (string | null)[]; text: string }[];
};

// Times are the device's local time; the cases fix the zone (Node reads TZ again when it changes).
(globalThis as unknown as { process: { env: Record<string, string> } }).process.env.TZ = F.timeZone;

/** The file's project with a case's changes over it. */
const project = (over: Partial<ProjectSummary>): ProjectSummary => ({ ...F.project, ...over, access: { ...F.project.access, ...over.access } });

describe('catalog words and rules (fixtures/cloud/v1/catalog.json)', () => {
  it('is a v1 catalog file', () => {
    expect([F.format, F.version]).toEqual(['kentos.catalog', 1]);
  });

  it('names the lists, orders, states and types as the interface does', () => {
    expect(VIEWS.map((v) => ({ id: v.id, label: v.label, sorts: v.sorts, empty: v.empty, ...(v.note ? { note: v.note } : {}) }))).toEqual(F.views);
    expect(SORT_LABEL).toEqual(F.sorts);
    expect(STATE_LABEL).toEqual(F.states);
    expect(TYPE_LABEL).toEqual(F.types);
    expect(TYPE_HINT).toBe(F.typeHint);
    expect([EMPTY_SEARCH, NO_ORGANIZATION]).toEqual([F.emptySearch, F.noOrganization]);
  });

  it('says why an action is off in the templates’ words', () => {
    const fill = (name: string, what: string, permission: string) => F.tips.denied.replace('{name}', name).replace('{what}', what).replace('{permission}', permission);
    expect(deniedText('Ada 101', 'paylaşma', 'project.share' as ProjectPermission)).toBe(fill('Ada 101', 'paylaşma', 'project.share'));
    expect(ARCHIVED_TEXT).toBe(F.tips.archived);
  });

  for (const c of F.details) it(`details: ${c.id}`, () => expect(detailPlan(project(c.project), c.open)).toEqual(c.expect));
  for (const c of F.primary) it(`main button: ${c.id}`, () => expect(primaryPlan(c.view, c.project && project(c.project))).toEqual(c.expect));
  for (const c of F.rows) it(`row: ${c.id}`, () => expect(rowPlan(project(c.project), c.view, c.sort, c.open, c.place)).toEqual(c.expect));

  it('asks the lifecycle questions in their words', () => {
    for (const c of F.questions.trash) expect(trashQuestion(c.args.name, c.args.tenant!, c.args.days ?? undefined, !!c.args.open), c.id).toEqual(c.expect);
    for (const c of F.questions.purge) expect(purgeQuestion(c.args.name), c.id).toEqual(c.expect);
    for (const c of F.questions.archive) expect(archiveQuestion(c.args.name), c.id).toEqual(c.expect);
  });

  it('writes the actions’ lines', () => {
    for (const l of F.lines) expect((CATALOG_LINES[l.line] as (...a: (string | null)[]) => string)(...l.args), l.line).toBe(l.text);
  });

  // The desktop reads the file's projects into the contract's own types, strictly: a value the contract does not
  // have ("via": "share") must fail here too, not only there.
  it('holds every project of the file, the base and each case’s over it, to the generated contract', () => {
    const projects: [string, ProjectSummary][] = [
      ['project', F.project],
      ...F.details.map((c): [string, ProjectSummary] => [`details: ${c.id}`, project(c.project)]),
      ...F.primary.flatMap((c): [string, ProjectSummary][] => (c.project ? [[`primary: ${c.id}`, project(c.project)]] : [])),
      ...F.rows.map((c): [string, ProjectSummary] => [`rows: ${c.id}`, project(c.project)]),
    ];
    const summary = fieldsOf('ProjectSummary');
    const access = fieldsOf('ProjectAccessView');
    const allowed = (type: string) => new Set(membersOf(type));
    const enums: [string, (p: ProjectSummary) => unknown, Set<string>][] = [
      ['tenantKind', (p) => p.tenantKind, allowed('TenantKind')],
      ['projectType', (p) => p.projectType, allowed('ProjectType')],
      ['state', (p) => p.state, allowed('ProjectState')],
      ['storage', (p) => p.storage, allowed('ProjectStorage')],
      ['areaUnit', (p) => p.areaUnit, allowed('AreaUnit')],
      ['access.role', (p) => p.access.role, allowed('ProjectRole')],
      ['access.via', (p) => p.access.via, allowed('AccessSource')],
    ];
    const permissions = allowed('ProjectPermission');
    for (const [where, p] of projects) {
      const keys = (o: object, of: Map<string, boolean>, what: string) => {
        for (const k of Object.keys(o)) expect(of.has(k), `${where}: ${what} has no field “${k}”`).toBe(true);
        for (const [k, optional] of of) if (!optional) expect(k in o, `${where}: ${what} needs “${k}”`).toBe(true);
      };
      keys(p, summary, 'ProjectSummary');
      keys(p.access, access, 'ProjectAccessView');
      for (const [field, get, values] of enums) expect(values.has(get(p) as string), `${where}: ${field} “${String(get(p))}” is not in the contract (${[...values].join(', ')})`).toBe(true);
      for (const x of p.access.permissions) expect(permissions.has(x), `${where}: permission “${x}”`).toBe(true);
    }
  });
});
