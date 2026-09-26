import { describe, expect, it } from 'vitest';
import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { ProjectPage } from '../../contracts/generated/ProjectPage';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { catalogQuery, type CatalogRequest } from './api';
import { CatalogPager, PROJECT_TYPES, SORT_LABEL, TYPE_LABEL, VIEWS, catalogEnvelope, parseTags, purgeText, viewDef } from './catalog';

const summary = (id: string, extra: Partial<ProjectSummary> = {}): ProjectSummary => ({
  id,
  name: id,
  srid: 5256,
  dataRevision: '1',
  updatedAt: '2026-09-26T10:00:00Z',
  tenantId: 't',
  tenantName: 'Büro',
  tenantKind: 'organization',
  ownerName: 'Ayşe',
  access: { role: 'owner', via: 'owner', permissions: ['project.read'] },
  projectType: 'cad',
  description: '',
  tags: [],
  state: 'active',
  catalogVersion: '1',
  createdAt: '2026-09-26T09:00:00Z',
  creatorName: 'Ayşe',
  areaUnit: 'm2',
  storage: 'database',
  favorite: false,
  ...extra,
});

describe('the catalog’s words', () => {
  it('names every type, view and order the server knows', () => {
    const types: Record<(typeof PROJECT_TYPES)[number], true> = { cad: true, gis: true, landReadjustment: true, zoningPlan: true, subdivision: true, road: true, architecture: true };
    expect(PROJECT_TYPES).toEqual(Object.keys(types));
    for (const t of PROJECT_TYPES) expect(TYPE_LABEL[t]).toBeTruthy();
    const views: CatalogView[] = ['recent', 'favorites', 'mine', 'organization', 'shared', 'archived', 'trash'];
    expect(VIEWS.map((v) => v.id)).toEqual(views);
    for (const v of VIEWS) {
      expect(v.sorts.length).toBeGreaterThan(0);
      // Only the recent list orders by opening, only the trash by when it was moved there (the server refuses the rest).
      expect(v.sorts.includes('opened')).toBe(v.id === 'recent');
      expect(v.sorts.includes('trashed')).toBe(v.id === 'trash');
    }
    expect(viewDef('trash').sorts[0]).toBe('trashed');
    const sorts: CatalogSort[] = ['updated', 'name', 'created', 'opened', 'trashed'];
    for (const s of sorts) expect(SORT_LABEL[s]).toBeTruthy();
  });

  it('reads tags from one field and says when the trash lets go', () => {
    expect(parseTags(' Kadıköy,  DOP   %40 ,, ')).toEqual(['Kadıköy', 'DOP %40']);
    expect(parseTags('')).toEqual([]);
    expect(purgeText(summary('a', { purgeAfter: '2026-10-26T10:00:00Z' }))).toMatch(/^26\.10\.2026 tarihinde kalıcı olarak silinir$/);
    expect(purgeText(summary('a'))).toMatch(/Elle silinene kadar/);
  });

  it('asks for a list without its empty parts', () => {
    expect(catalogQuery({ view: 'mine' })).toBe('view=mine');
    const full: CatalogRequest = { view: 'organization', tenant: 't1', q: '  ada 101 ', type: 'gis', sort: 'name', limit: 20, after: '["x","y"]' };
    expect(new URLSearchParams(catalogQuery(full)).get('q')).toBe('ada 101');
    expect(Object.fromEntries(new URLSearchParams(catalogQuery(full)))).toEqual({ view: 'organization', tenant: 't1', q: 'ada 101', type: 'gis', sort: 'name', limit: '20', after: '["x","y"]' });
    expect(catalogQuery({ view: 'shared', q: '   ' })).toBe('view=shared');
  });

  it('makes each command with its own keys', () => {
    const a = catalogEnvelope('project.archive', 't', 'p', {});
    const b = catalogEnvelope('project.archive', 't', 'p', {});
    expect(a.idempotencyKey).not.toBe(b.idempotencyKey);
    expect(a.requestId).toMatch(/^web-/);
    expect(catalogEnvelope('project.metadata.update', 't', 'p', { tags: ['a'] }, { '@catalog': '3' }).expectedVersions).toEqual({ '@catalog': '3' });
  });
});

describe('paging through a list', () => {
  /** A server of `n` projects in pages, whose answers the test releases one by one. */
  function server(n: number) {
    const all = Array.from({ length: n }, (_, i) => summary(`p${i}`));
    const waiting: (() => void)[] = [];
    const asked: CatalogRequest[] = [];
    return {
      asked,
      release: () => waiting.shift()?.(),
      catalog: (r: CatalogRequest): Promise<ProjectPage> => {
        asked.push(r);
        const from = r.after ? Number(r.after) : 0;
        const size = r.limit ?? 2;
        const page: ProjectPage = { projects: all.slice(from, from + size), total: n, trashRetentionDays: 30, ...(from + size < n ? { next: String(from + size) } : {}) };
        return new Promise((resolve) => waiting.push(() => resolve(page)));
      },
    };
  }

  it('adds each page after the one before and stops at the last', async () => {
    const s = server(5);
    const pager = new CatalogPager(s);
    const first = pager.reset({ view: 'mine', limit: 2 });
    s.release();
    expect(await first).toBe(true);
    expect(pager.projects.map((p) => p.id)).toEqual(['p0', 'p1']);
    expect([pager.total, pager.hasMore, pager.retentionDays]).toEqual([5, true, 30]);
    for (const want of [['p0', 'p1', 'p2', 'p3'], ['p0', 'p1', 'p2', 'p3', 'p4']]) {
      const more = pager.more();
      s.release();
      await more;
      expect(pager.projects.map((p) => p.id)).toEqual(want);
    }
    expect(pager.hasMore).toBe(false);
    // The next page asks from where the last ended, with the same view.
    expect(s.asked.map((r) => [r.view, r.after])).toEqual([
      ['mine', undefined],
      ['mine', '2'],
      ['mine', '4'],
    ]);
  });

  it('throws away an answer a newer request overtook', async () => {
    const s = server(3);
    const pager = new CatalogPager(s);
    const slow = pager.reset({ view: 'mine', q: 'a', limit: 2 });
    const fast = pager.reset({ view: 'mine', q: 'ab', limit: 1 });
    s.release();
    s.release();
    expect(await slow).toBe(false);
    expect(await fast).toBe(true);
    expect(pager.projects.map((p) => p.id)).toEqual(['p0']);
    expect(pager.hasMore).toBe(true);
  });
});
