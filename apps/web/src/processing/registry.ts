import type { Disposable } from '../core/disposable';
import { Signal } from '../core/signal';
import { foldTurkish } from '../core/text';
import { PROCESSING_CATEGORIES, type ProcessingCategory } from './categories';
import type { ProcessingTool } from './types';

export interface CategoryNode {
  category: ProcessingCategory;
  tools: ProcessingTool[];
  children: CategoryNode[];
}

/**
 * Every processing tool, by id and by category. Built-in tools are
 * registered at start-up (app/createApp); plug-ins will add theirs through
 * the same call. `version` bumps on every change for cheap UI refresh.
 */
export class ProcessingRegistry {
  readonly version = new Signal(0);
  private readonly tools = new Map<string, ProcessingTool>();
  private readonly categories = new Map<string, ProcessingCategory>(PROCESSING_CATEGORIES.map((c) => [c.id, c]));

  register(tool: ProcessingTool): Disposable {
    if (this.tools.has(tool.id)) throw new Error(`İşlem aracı iki kez kaydedildi: ${tool.id}`);
    if (!this.categories.has(tool.category)) throw new Error(`Bilinmeyen işlem kategorisi: ${tool.category} (${tool.id})`);
    const names = new Set<string>();
    for (const p of tool.parameters) {
      if (names.has(p.name)) throw new Error(`${tool.id}: “${p.name}” parametresi iki kez tanımlı`);
      names.add(p.name);
    }
    this.tools.set(tool.id, tool);
    this.bump();
    return () => {
      this.tools.delete(tool.id);
      this.bump();
    };
  }

  registerCategory(c: ProcessingCategory): Disposable {
    if (c.parent && !this.categories.has(c.parent)) throw new Error(`Bilinmeyen üst kategori: ${c.parent}`);
    this.categories.set(c.id, c);
    this.bump();
    return () => {
      this.categories.delete(c.id);
      this.bump();
    };
  }

  get(id: string): ProcessingTool | undefined {
    return this.tools.get(id);
  }

  list(): ProcessingTool[] {
    return [...this.tools.values()];
  }

  category(id: string): ProcessingCategory | undefined {
    return this.categories.get(id);
  }

  /** Category path for display: "Kadastro › Numaralandırma". */
  categoryPath(id: string): string {
    const parts: string[] = [];
    for (let c = this.categories.get(id); c; c = c.parent ? this.categories.get(c.parent) : undefined) parts.unshift(c.label);
    return parts.join(' › ');
  }

  /** Categories with their tools, empty branches left out, in declaration order. */
  tree(filter?: (t: ProcessingTool) => boolean): CategoryNode[] {
    const build = (parent: string | undefined): CategoryNode[] =>
      [...this.categories.values()]
        .filter((c) => c.parent === parent)
        .map((category) => ({
          category,
          tools: this.list()
            .filter((t) => t.category === category.id && (!filter || filter(t)))
            .sort((a, b) => a.label.localeCompare(b.label, 'tr')),
          children: build(category.id),
        }))
        .filter((n) => n.tools.length || n.children.length);
    return build(undefined);
  }

  /** Tools matching every word of the query in name, keywords, description or category. */
  search(query: string): ProcessingTool[] {
    const words = foldTurkish(query).split(/\s+/).filter(Boolean);
    if (!words.length) return this.list();
    return this.list().filter((t) => {
      const hay = foldTurkish([t.label, t.description, ...(t.keywords ?? []), this.categoryPath(t.category)].join(' '));
      return words.every((w) => hay.includes(w));
    });
  }

  private bump(): void {
    this.version.set(this.version.value + 1);
  }
}
