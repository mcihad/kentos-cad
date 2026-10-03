import { describe, expect, it } from 'vitest';
import type { Template } from '../../contracts/generated/sheet/Template';
import { testEngine } from './engineTesting';
import { arrangeTemplates, cardOf, categoryLabel, facetsOf, hiddenByMode, metasOf, sectionOf, type GalleryQuery, type TemplateCard } from './templates';

/**
 * The gallery's list (docs/sheet/design.md §11, §11a): the engine's order for
 * the project (`rankTemplates`), kept; another mode's templates only with
 * “Bütün türlerin şablonları”, with the engine's badge; the query's
 * section, paper, kind and words; the facets the filters offer.
 */

const e = testEngine();
const system = e.systemTemplates().map((t) => cardOf(t, 'system', ['system']));
/** A user's own copy of a system template, as this device keeps it. */
const mine = (from: string, id: string, name: string): TemplateCard => {
  const t = e.systemTemplates().find((x) => x.meta.id === from)!;
  return cardOf({ ...t, meta: { ...t.meta, id, name } } as Template, 'device', ['device', 'unsynced']);
};
const cards = [...system, mine('sys:gis-tematik', 'dev-1', 'Belediye tematik'), mine('sys:ifraz-paftasi', 'dev-2', 'Belediye ifraz')];
const q = (over: Partial<GalleryQuery> = {}): GalleryQuery => ({ section: 'all', text: '', paper: null, kind: null, allModes: false, ...over });

describe('template gallery', () => {
  it('keeps the engine’s order: the project’s type, its mode, then the common ones', () => {
    const ranked = e.rankTemplates(metasOf(cards), 'cad', 'subdivision');
    const list = arrangeTemplates(cards, ranked, q());
    const fits = list.map((a) => a.fit);
    expect(fits.indexOf('workspace')).toBeGreaterThan(fits.lastIndexOf('projectType'));
    expect(fits.indexOf('common')).toBeGreaterThan(fits.lastIndexOf('workspace'));
    expect(list[0].card.projectTypes).toContain('subdivision');
    expect(list.map((a) => a.card.id)).toEqual(ranked.filter((r) => r.matches).map((r) => r.id));
  });

  it('shows another mode’s templates only when asked, last and badged', () => {
    const ranked = e.rankTemplates(metasOf(cards), 'cad', null);
    const plain = arrangeTemplates(cards, ranked, q());
    expect(plain.some((a) => a.card.id === 'sys:gis-tematik')).toBe(false);
    const all = arrangeTemplates(cards, ranked, q({ allModes: true }));
    const gis = all.find((a) => a.card.id === 'sys:gis-tematik')!;
    expect(gis.fit).toBe('other');
    expect(gis.badge).toBe('CBS şablonu');
    expect(all.findIndex((a) => a.fit === 'other')).toBeGreaterThan(all.findLastIndex((a) => a.fit !== 'other'));
    expect(hiddenByMode(cards, ranked, 'all')).toBe(all.length - plain.length);
    expect(hiddenByMode(cards, ranked, 'mine')).toBe(1);
  });

  it('filters by section, paper, kind and words (Turkish letters folded)', () => {
    const ranked = e.rankTemplates(metasOf(cards), null, null);
    expect(arrangeTemplates(cards, ranked, q({ section: 'mine' })).map((a) => a.card.id).sort()).toEqual(['dev-1', 'dev-2']);
    expect(arrangeTemplates(cards, ranked, q({ paper: 'a1' })).every((a) => a.card.papers.some((p) => p.paper === 'a1'))).toBe(true);
    expect(arrangeTemplates(cards, ranked, q({ kind: 'kadastro' })).every((a) => a.card.category === 'kadastro')).toBe(true);
    expect(arrangeTemplates(cards, ranked, q({ text: 'IMAR' })).map((a) => a.card.id)).toContain('sys:imar-plani');
    expect(arrangeTemplates(cards, ranked, q({ text: 'belediye ifraz' })).map((a) => a.card.id)).toEqual(['dev-2']);
  });

  it('offers the papers and kinds the cards use, in the series’ order', () => {
    const f = facetsOf(system);
    expect(f.papers).toEqual([...f.papers].sort((a, b) => (a[0] === b[0] ? Number(a.slice(1)) - Number(b.slice(1)) : a < b ? -1 : 1)));
    expect(f.papers[0]).toBe('a0');
    expect(f.kinds).toContain('kadastro');
    expect(categoryLabel('kadastro')).toBe('Kadastro');
    expect(sectionOf('device')).toBe('mine');
    expect(sectionOf('shared')).toBe('shared');
  });
});
