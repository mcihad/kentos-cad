import type { ExprField } from '../../model/expression/builder';
import type { ExprVariable } from '../../model/expression/expression';
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
  /** The `@` values (docs/adr/0214 §2.3): completion, the tree, the warnings. */
  readonly variables?: readonly ExprVariable[];
  /** Whether the functions that look at other layers can be used (İşlemler); elsewhere refused and not offered. */
  readonly world?: boolean;
  /** The objects it runs on, for the preview and a field's values. */
  readonly objects?: BuilderObjects;
  /** Tamam: the text to write back. */
  readonly onOk: (value: string) => void;
}

/** Opens the builder; the promise fails when its code cannot be loaded (the caller says so). */
export function openExpressionBuilder(opts: ExpressionBuilderOptions): Promise<void> {
  return import('./ExpressionBuilder').then((m) => m.showBuilder(opts));
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
  /** The `@` values, asked when the builder opens. */
  readonly variables?: () => readonly ExprVariable[];
  /** Whether the functions that look at other layers can be used. */
  readonly world?: boolean;
  /** Says that the builder could not be loaded (the app's log). */
  readonly fail: (message: string) => void;
}): HTMLButtonElement {
  const b = h('button', { class: 'ibtn exprb-open', type: 'button', title: 'İfade oluşturucu…', 'aria-label': 'İfade oluşturucu' }, icon('expression', 16));
  b.addEventListener('mousedown', (e) => e.preventDefault());
  b.addEventListener('click', () =>
    openExpressionBuilder({
      value: opts.get(),
      context: opts.context,
      fields: opts.fields(),
      variables: opts.variables?.(),
      world: opts.world,
      objects: opts.objects?.(),
      onOk: opts.set,
    }).catch((e: Error) =>
      opts.fail(`İfade oluşturucu yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`),
    ),
  );
  return b;
}
