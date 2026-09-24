import { Emitter } from '../core/emitter';
import { Signal } from '../core/signal';
import { foldTurkish } from '../core/text';
import type { LibraryAsset, LibraryCategory, LibraryItem, LibrarySource, LibrarySymbol, Sourced, Symbol } from '../model/style';

/**
 * The style library: symbols and assets from three sources. System items
 * ship with KentOS and can be copied but never changed or deleted; the
 * user's items follow the user across projects; project items travel in
 * the project file so everyone who opens it sees the same symbols.
 * Items sit in a category tree of any depth ("MPYY › Uygulama İmar Planı ›
 * Sınırlar"). The library is data only; the app persists the user and
 * project parts when `changed` fires.
 */

export type EditableSource = Exclude<LibrarySource, 'system'>;

export interface CategoryNode {
  readonly name: string;
  readonly path: readonly string[];
  readonly items: readonly Sourced[];
  readonly children: readonly CategoryNode[];
  readonly description?: string;
}

interface LibraryEvents {
  changed: { source: EditableSource };
}

/** A fresh id for a user or project item. */
export const newItemId = (prefix = 'u') => `${prefix}-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

/** Asset ids a symbol draws with (SVG and raster markers, image fills), nested markers included. */
export function assetsOfSymbol(sym: Symbol): string[] {
  const out = new Set<string>();
  const walk = (s: Symbol) => {
    for (const l of s.layers as readonly { type: string; asset?: string; marker?: Symbol }[]) {
      if (l.asset) out.add(l.asset);
      if (l.marker) walk(l.marker);
    }
  };
  walk(sym);
  return [...out];
}

const pathKey = (p: readonly string[]) => p.join('\u0000');

export class StyleLibrary {
  readonly events = new Emitter<LibraryEvents>();
  /** Bumped on every change, for cheap UI subscriptions. */
  readonly version = new Signal(0);
  private readonly stores: Record<LibrarySource, Map<string, LibraryItem>> = { system: new Map(), user: new Map(), project: new Map() };
  private readonly cats: Record<LibrarySource, LibraryCategory[]> = { system: [], user: [], project: [] };

  constructor(system: { items: readonly LibraryItem[]; categories?: readonly LibraryCategory[] } = { items: [] }) {
    for (const it of system.items) {
      if (this.stores.system.has(it.id)) throw new Error(`Sistem kitaplığında iki kez tanımlı: ${it.id}`);
      this.stores.system.set(it.id, it);
    }
    this.cats.system = [...(system.categories ?? [])];
  }

  /** Replaces a source's content (loading from storage or from a project). */
  load(source: EditableSource, items: readonly LibraryItem[], categories: readonly LibraryCategory[] = []): void {
    this.stores[source] = new Map(items.filter((i) => !this.stores.system.has(i.id)).map((i) => [i.id, clone(i)]));
    this.cats[source] = clone([...categories]);
    this.version.set(this.version.value + 1);
  }

  /** A source's items and categories as saved (JSON-safe). */
  dump(source: EditableSource): { items: LibraryItem[]; categories: LibraryCategory[] } {
    return { items: clone([...this.stores[source].values()]), categories: clone(this.cats[source]) };
  }

  // ── Reading ──────────────────────────────────────────────────────────

  get(id: string): Sourced | undefined {
    for (const source of ['project', 'user', 'system'] as const) {
      const it = this.stores[source].get(id);
      if (it) return { ...it, source } as Sourced;
    }
    return undefined;
  }

  symbol(id: string): Symbol | undefined {
    const it = this.get(id);
    return it?.kind === 'symbol' ? it.symbol : undefined;
  }

  asset(id: string): LibraryAsset | undefined {
    const it = this.get(id);
    return it?.kind === 'asset' ? it : undefined;
  }

  items(source?: LibrarySource): Sourced[] {
    const sources: LibrarySource[] = source ? [source] : ['system', 'user', 'project'];
    return sources.flatMap((s) => [...this.stores[s].values()].map((it) => ({ ...it, source: s }) as Sourced));
  }

  canEdit(id: string): boolean {
    const s = this.get(id)?.source;
    return s === 'user' || s === 'project';
  }

  /** Symbols that draw with an asset (to warn before removing it). */
  usersOf(assetId: string): Sourced<LibrarySymbol>[] {
    return this.items().filter((i): i is Sourced<LibrarySymbol> => i.kind === 'symbol' && assetsOfSymbol(i.symbol).includes(assetId));
  }

  /**
   * The category tree of the chosen items (all by default). Siblings are
   * ordered by their category's `order`, then by name in Turkish order.
   */
  tree(filter: { source?: LibrarySource; query?: string; kind?: LibraryItem['kind'] } = {}): CategoryNode[] {
    const words = filter.query ? foldTurkish(filter.query).split(/\s+/).filter(Boolean) : [];
    const items = this.items(filter.source).filter((i) => {
      if (filter.kind && i.kind !== filter.kind) return false;
      if (!words.length) return true;
      const hay = foldTurkish([i.name, ...i.path, ...(i.tags ?? []), i.kind === 'symbol' ? (i.description ?? '') : ''].join(' '));
      return words.every((w) => hay.includes(w));
    });
    const cats = (filter.source ? this.cats[filter.source] : [...this.cats.system, ...this.cats.user, ...this.cats.project]).filter(() => !words.length);
    const meta = new Map<string, LibraryCategory>();
    for (const c of [...this.cats.system, ...this.cats.user, ...this.cats.project]) meta.set(pathKey(c.path), c);

    interface Build {
      name: string;
      path: string[];
      items: Sourced[];
      children: Map<string, Build>;
    }
    const root: Build = { name: '', path: [], items: [], children: new Map() };
    const ensure = (path: readonly string[]) => {
      let node = root;
      for (let i = 0; i < path.length; i++) {
        let next = node.children.get(path[i]);
        if (!next) node.children.set(path[i], (next = { name: path[i], path: path.slice(0, i + 1), items: [], children: new Map() }));
        node = next;
      }
      return node;
    };
    for (const c of cats) ensure(c.path);
    for (const i of items) ensure(i.path).items.push(i);
    const finish = (b: Build): CategoryNode => ({
      name: b.name,
      path: b.path,
      description: meta.get(pathKey(b.path))?.description,
      items: b.items.sort((x, y) => x.name.localeCompare(y.name, 'tr')),
      children: [...b.children.values()]
        .sort((x, y) => (meta.get(pathKey(x.path))?.order ?? 1e9) - (meta.get(pathKey(y.path))?.order ?? 1e9) || x.name.localeCompare(y.name, 'tr'))
        .map(finish),
    });
    return finish(root).children as CategoryNode[];
  }

  // ── Changing ─────────────────────────────────────────────────────────

  private editable(id: string): { source: EditableSource; item: LibraryItem } {
    const it = this.get(id);
    if (!it) throw new Error(`Kitaplıkta yok: ${id}`);
    if (it.source === 'system') throw new Error(`“${it.name}” bir sistem öğesidir; değiştirilemez ya da silinemez. Kopyasını alıp onu düzenleyin.`);
    return { source: it.source, item: this.stores[it.source].get(id)! };
  }

  private touched(source: EditableSource): void {
    this.version.set(this.version.value + 1);
    this.events.emit('changed', { source });
  }

  add(source: EditableSource, item: LibraryItem): LibraryItem {
    if (this.get(item.id)) throw new Error(`Bu kimlikle bir öğe zaten var: ${item.id}`);
    const copy = clone(item);
    this.stores[source].set(copy.id, copy);
    this.touched(source);
    return copy;
  }

  /** Changes an item's content, name, path, tags … (never its id or kind). */
  update(id: string, patch: Partial<Omit<LibrarySymbol, 'id' | 'kind'>> | Partial<Omit<LibraryAsset, 'id' | 'kind'>>): void {
    const { source, item } = this.editable(id);
    this.stores[source].set(id, { ...item, ...clone(patch), id, kind: item.kind } as LibraryItem);
    this.touched(source);
  }

  rename(id: string, name: string): void {
    this.update(id, { name: name.trim() || 'Adsız' });
  }

  move(id: string, path: readonly string[]): void {
    this.update(id, { path: [...path] });
  }

  /** Removes a user or project item. Symbols that use a removed asset keep their reference (and draw nothing for it). */
  remove(id: string): void {
    const { source } = this.editable(id);
    this.stores[source].delete(id);
    this.touched(source);
  }

  /**
   * Copies any item (a system one included) into the user's or the
   * project's library under a new id. A symbol copied into a project takes
   * along the user assets it draws with, so the project stays complete for
   * colleagues (system assets are always there).
   */
  copy(id: string, to: EditableSource, opts: { name?: string; path?: readonly string[] } = {}): LibraryItem {
    const src = this.get(id);
    if (!src) throw new Error(`Kitaplıkta yok: ${id}`);
    const { source: _s, ...plain } = src;
    const item = { ...clone(plain), id: newItemId(to === 'project' ? 'p' : 'u'), name: opts.name ?? `${src.name} (kopya)`, path: [...(opts.path ?? src.path)] } as LibraryItem;
    if (to === 'project' && item.kind === 'symbol') {
      for (const a of assetsOfSymbol(item.symbol)) {
        const asset = this.get(a);
        if (asset && asset.source === 'user' && !this.stores.project.has(a)) {
          const { source: _x, ...rest } = asset;
          this.stores.project.set(a, clone(rest) as LibraryItem);
        }
      }
    }
    this.stores[to].set(item.id, item);
    this.touched(to);
    return item;
  }

  /** Adds an empty category (or sets its description/order). */
  addCategory(source: EditableSource, category: LibraryCategory): void {
    const key = pathKey(category.path);
    this.cats[source] = [...this.cats[source].filter((c) => pathKey(c.path) !== key), clone(category)];
    this.touched(source);
  }

  /** Renames a category and moves everything under it (user and project items only). */
  renameCategory(source: EditableSource, path: readonly string[], name: string): void {
    const depth = path.length - 1;
    const under = (p: readonly string[]) => p.length >= path.length && path.every((s, i) => p[i] === s);
    const renamed = (p: readonly string[]) => p.map((s, i) => (i === depth ? name : s));
    for (const [id, it] of this.stores[source]) if (under(it.path)) this.stores[source].set(id, { ...it, path: renamed(it.path) });
    this.cats[source] = this.cats[source].map((c) => (under(c.path) ? { ...c, path: renamed(c.path) } : c));
    this.touched(source);
  }
}
