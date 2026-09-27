import { exprTokens, type ExprItemKind, type ExprTokenClass } from '../../model/expression/builder';
import { h } from '../dom';

/**
 * Expression text in the syntax colours (styles/expression.css), for the
 * help's signatures and examples; the editor paints its own text the same way.
 */

export const TOKEN_STYLE: Record<ExprTokenClass, string> = {
  number: 'x-literal',
  text: 'x-text',
  field: 'x-field',
  variable: 'x-variable',
  function: 'x-function',
  keyword: 'x-keyword',
  constant: 'x-literal',
  operator: 'x-operator',
  paren: 'x-operator',
  comma: 'x-operator',
  unknown: 'x-unknown',
};

/** The colour of an entry's name in the lists. */
export const KIND_STYLE: Record<ExprItemKind, string> = {
  field: 'x-field',
  variable: 'x-variable',
  function: 'x-function',
  operator: 'x-operator',
  keyword: 'x-keyword',
};

/** The text as nodes: each token in its colour, what is between as it is. */
export function highlighted(src: string): (Node | string)[] {
  const out: (Node | string)[] = [];
  let at = 0;
  for (const t of exprTokens(src)) {
    if (t.start > at) out.push(src.slice(at, t.start));
    out.push(h('span', { class: TOKEN_STYLE[t.class] }, src.slice(t.start, t.end)));
    at = t.end;
  }
  if (at < src.length) out.push(src.slice(at));
  return out;
}
