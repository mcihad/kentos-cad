import { nameKey } from '../model/blocks';
import { TEMPLATE_TOOL_LABEL } from '../model/objectTemplate';
import type { LibrarySource, LibraryTemplate, Sourced } from '../model/style';
import type { StyleLibrary } from './library';

/**
 * Şablonlar panelinin listesi (docs/adr/0176 §4): the library's object templates from every source (not its symbols
 * nor its assets), grouped by their category paths. The groups go by their paths in Turkish order, part by part, the
 * templates without a category last; in a group the templates go by their names in Turkish order, one name's
 * templates project first, then Kitaplığım, then the system's. A search, its white space around left out and folded
 * the Turkish way (`nameKey`), is found in a template's name, description, category path, tool's name and layer's
 * name; an empty one keeps every template, and a group left without one is not listed. The desktop's is
 * `kentos_native_style::object_template::listed`; both pass fixtures/style/v1/template-list.json.
 */

export interface TemplateGroup {
  /** The category path; empty: the templates without a category. */
  readonly path: readonly string[];
  readonly items: readonly Sourced<LibraryTemplate>[];
}

const SOURCE_ORDER: Record<LibrarySource, number> = { project: 0, user: 1, system: 2 };

/** Two category paths in Turkish order, part by part; a shorter path before the longer one it begins; the empty one last. */
function comparePaths(a: readonly string[], b: readonly string[]): number {
  if (!a.length || !b.length) return Number(!a.length) - Number(!b.length);
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    const c = a[i].localeCompare(b[i], 'tr');
    if (c) return c;
  }
  return a.length - b.length;
}

/** The words a search is looked for in. */
function searched(t: Sourced<LibraryTemplate>): string[] {
  return [t.name, t.description ?? '', t.path.join(' / '), TEMPLATE_TOOL_LABEL[t.template.tool] ?? '', t.template.layer?.name ?? ''];
}

/** The library's templates as the panel lists them, for `query`. */
export function listTemplates(lib: StyleLibrary, query = ''): TemplateGroup[] {
  const q = nameKey(query.trim());
  const found = lib
    .items()
    .filter((i): i is Sourced<LibraryTemplate> => i.kind === 'template')
    .filter((t) => !q || searched(t).some((s) => nameKey(s).includes(q)));
  const groups = new Map<string, Sourced<LibraryTemplate>[]>();
  for (const t of found) {
    const key = t.path.join('\u0000');
    const list = groups.get(key);
    if (list) list.push(t);
    else groups.set(key, [t]);
  }
  return [...groups.values()]
    .map((items) => ({
      path: items[0].path,
      items: items.sort((x, y) => x.name.localeCompare(y.name, 'tr') || SOURCE_ORDER[x.source] - SOURCE_ORDER[y.source]),
    }))
    .sort((x, y) => comparePaths(x.path, y.path));
}
