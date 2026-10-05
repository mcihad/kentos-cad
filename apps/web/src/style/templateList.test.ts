import { describe, expect, it } from 'vitest';
import type { LibraryItem } from '../model/style';
import { StyleLibrary } from './library';
import { listTemplates } from './templateList';

/**
 * The Şablonlar panel's list (docs/adr/0176 §4) as fixtures/style/v1/template-list.json has it, written by hand from
 * the rule: the groups by their category paths in Turkish order, the templates by their names, a search in the name,
 * description, category, tool and layer (the desktop's `object_template::listed` runs the same cases).
 */

interface ListCase {
  name: string;
  query: string;
  groups: { path: string[]; ids: string[] }[];
}

const files = import.meta.glob<string>('../../../../fixtures/style/v1/template-list.json', { query: '?raw', import: 'default', eager: true });
const fixture = JSON.parse(Object.values(files)[0]) as { format: string; version: number; library: Record<'system' | 'user' | 'project', LibraryItem[]>; cases: ListCase[] };

describe('the templates panel’s list (fixtures/style/v1/template-list.json)', () => {
  const lib = new StyleLibrary({ items: fixture.library.system });
  lib.load('user', fixture.library.user);
  lib.load('project', fixture.library.project);
  it('is a template-list-cases v1 file', () => {
    expect([fixture.format, fixture.version]).toEqual(['kentos.template-list-cases', 1]);
    expect(fixture.cases.length).toBeGreaterThanOrEqual(8);
  });
  for (const c of fixture.cases) {
    it(c.name, () => {
      expect(listTemplates(lib, c.query).map((g) => ({ path: [...g.path], ids: g.items.map((t) => t.id) }))).toEqual(c.groups);
    });
  }
});
