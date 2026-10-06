import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { ALL_CRITERIA, similarFacts, similarTo, type SimilarCriteria } from '../model/selectSimilar';
import { pickSelectable, selectableIds } from './selectable';
import type { Tool, ToolPointer } from './Tool';

/** The criteria's options in the prompt, the keys that turn them, and their names in the log. */
const CRITERIA: readonly [keyof SimilarCriteria, string, string, string][] = [
  ['kind', 'Tür', 'T', 'tür'],
  ['layer', 'Katman', 'K', 'katman'],
  ['color', 'Renk', 'R', 'renk'],
  ['symbol', 'Sembol', 'S', 'sembol'],
];

/** The criteria that are on as the log says them: “tür, katman ve renk”. */
export function criteriaText(c: SimilarCriteria): string {
  const on = CRITERIA.filter(([k]) => c[k]).map(([, , , word]) => word);
  if (!on.length) return 'ölçütsüz: bütün nesneler';
  return on.length === 1 ? on[0] : `${on.slice(0, -1).join(', ')} ve ${on[on.length - 1]}`;
}

/**
 * Benzerini seç (docs/adr/0187 §4): the selected objects are the examples, or the one clicked; every visible object
 * similar to one of them is selected, the selection filter passing what it holds (§5): it replaces the selection, or
 * joins the selection the tool started with when Shift is held at the click. Tür (T), Katman (K), Renk (R) and Sembol
 * (S) say what must be equal, all on at first and kept for the session. The tool stays: a criterion changed selects
 * again from the same examples, a click on another object makes it the example; Enter, Esc or a right click ends.
 */
export class SelectSimilarTool implements Tool {
  readonly id = 'selectSimilar';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private static criteria: SimilarCriteria = { ...ALL_CRITERIA };
  /** The examples, by id. */
  private examples: number[] = [];
  /** What the selection was when Shift was held at the click: the result joins it. */
  private base: number[] = [];
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    const ids = [...this.ctx.selection.ids.value];
    if (ids.length) {
      this.examples = ids;
      this.select();
    }
    this.refresh();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  pointerMove(p: ToolPointer): void {
    this.ctx.selection.hover.set(pickSelectable(this.ctx, p.screen, true)?.id ?? null);
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const hit = pickSelectable(this.ctx, p.screen);
    if (!hit) return void this.ctx.log.warn('Örnek olacak bir nesneye tıklayın.');
    this.base = p.shift ? [...this.ctx.selection.ids.value] : [];
    this.examples = [hit.id];
    this.select();
    this.refresh();
  }

  input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    const criterion = CRITERIA.find(([, , k]) => k === key)?.[0];
    if (!criterion) return false;
    const c = SelectSimilarTool.criteria;
    SelectSimilarTool.criteria = { ...c, [criterion]: !c[criterion] };
    if (this.examples.length) this.select();
    this.refresh();
    return true;
  }

  confirm(): void {
    this.ctx.tools.exit();
  }

  /** Selects what is similar to the examples still in the drawing. */
  private select(): void {
    const { doc, selection, log } = this.ctx;
    const examples = this.examples.flatMap((id) => {
      const e = doc.get(id);
      return e ? [similarFacts(e)] : [];
    });
    if (!examples.length) return;
    const objects = [...doc.all()].filter((e) => doc.layers.isVisible(e.layerId)).map(similarFacts);
    const ids = selectableIds(this.ctx, similarTo(objects, examples, SelectSimilarTool.criteria));
    selection.set([...this.base, ...ids]);
    log.info(`Benzer ${ids.length} nesne seçildi (${criteriaText(SelectSimilarTool.criteria)}).`);
  }

  private refresh(): void {
    const c = SelectSimilarTool.criteria;
    const toggles = CRITERIA.map(([k, name, key]) => `${name} (${key})${c[k] ? ': açık' : ''}`).join(' / ');
    this.prompt.set(
      this.examples.length
        ? `Benzerini seç: ölçütleri değiştirin ya da başka bir örneğe tıklayın [${toggles} / Bitir (Enter)]`
        : `Benzerini seç: örnek nesneye tıklayın [${toggles}]`,
    );
  }
}
