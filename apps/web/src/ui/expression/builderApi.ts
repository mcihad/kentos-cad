import type { ExprField } from '../../model/expression/builder';
import type { BuilderObjects } from '../../model/expression/builderObjects';
import { h } from '../dom';
import { icon } from '../icons';

/**
 * Opening the expression builder (DESIGN.md §7.16, docs/adr/0100 §5) from
 * any expression field: the dialog itself loads on first use. An expression
 * field puts `builderButton` beside it; the builder edits that field's text,
 * Tamam writes it back, Vazgeç leaves it as it was.
 */

export interface ExpressionBuilderOptions {
  /** The expression to start from. */
  readonly value: string;
  /** What the expression is for, beside the title: “İfadeyle seç · Koşul”. */
  readonly context?: string;
  /** The fields of the objects it runs on: completion, the tree, the warnings. */
  readonly fields: readonly ExprField[];
  /** The objects it runs on, for the preview and a field's values. */
  readonly objects?: BuilderObjects;
  /** Tamam: the text to write back. */
  readonly onOk: (value: string) => void;
}

export function openExpressionBuilder(opts: ExpressionBuilderOptions): void {
  void import('./ExpressionBuilder').then((m) => m.openBuilder(opts));
}

/**
 * The ε button beside an expression field. It does not take the focus, so
 * the field's caret stays where it was until the builder opens.
 */
export function builderButton(opts: {
  readonly get: () => string;
  readonly set: (value: string) => void;
  readonly fields: () => readonly ExprField[];
  readonly objects?: () => BuilderObjects | undefined;
  readonly context?: string;
}): HTMLButtonElement {
  const b = h('button', { class: 'ibtn exprb-open', type: 'button', title: 'İfade oluşturucu…', 'aria-label': 'İfade oluşturucu' }, icon('expression', 16));
  b.addEventListener('mousedown', (e) => e.preventDefault());
  b.addEventListener('click', () =>
    openExpressionBuilder({ value: opts.get(), context: opts.context, fields: opts.fields(), objects: opts.objects?.(), onOk: opts.set }),
  );
  return b;
}
