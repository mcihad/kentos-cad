import { foldTurkish } from '../../core/text';
import type { Entity } from '../entities';
import { compare, equals, findFunction, findVariable, measuredOf, toNumber, toText, truthy, type ExprFunction, type ExprScope, type ExprValue, type ExprVariable } from './expressionLib';

/**
 * İfadeler: a small, safe expression language for processing tools
 * (select by expression, field calculator, filters). No eval: the source
 * is tokenized, parsed (precedence climbing) and compiled to closures.
 *
 *   Nitelik = 'Arsa' ve $alan > 500
 *   'P' || doldur($sıra, 5)
 *   yuvarla([Tapu alanı] - $alan, 2)
 *
 * Fields: bare names (Parsel) or in brackets ([Tapu alanı]). Text: '…' or
 * "…" (a doubled quote inside is one quote). Variables start with $ (see
 * expressionLib.ts). Keywords: ve/and, veya/or, değil/not, doğru/true,
 * yanlış/false, boş/null. Operators: = != <> < <= > >= + - * / % and ||
 * (joins text). Errors say what is wrong and where, for the dialog.
 */

export interface CompiledExpression {
  readonly source: string;
  /** Attribute names the expression reads (to warn about missing ones). */
  readonly fields: readonly string[];
  evaluate(scope: ExprScope): ExprValue;
}

export type CompileResult = { ok: true; expr: CompiledExpression } | { ok: false; error: string; at: number };

type Token =
  | { t: 'num'; v: number; at: number }
  | { t: 'str'; v: string; at: number }
  | { t: 'field'; v: string; at: number }
  | { t: 'var'; v: string; at: number }
  | { t: 'word'; v: string; at: number }
  | { t: 'op'; v: string; at: number }
  | { t: 'end'; at: number };

class ExprError extends Error {
  readonly at: number;
  constructor(message: string, at: number) {
    super(message);
    this.at = at;
  }
}

const OPS = ['<=', '>=', '!=', '<>', '==', '||', '=', '<', '>', '+', '-', '*', '/', '%', '(', ')', ','];
const WORD = /[\p{L}_][\p{L}\p{N}_]*/uy;

function tokenize(src: string): Token[] {
  const out: Token[] = [];
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (/\s/.test(c)) {
      i++;
      continue;
    }
    const at = i + 1;
    if (/\d/.test(c) || (c === '.' && /\d/.test(src[i + 1] ?? ''))) {
      const m = /\d*\.?\d+(?:[eE][+-]?\d+)?|\d+\.?/y;
      m.lastIndex = i;
      const s = m.exec(src)![0];
      out.push({ t: 'num', v: Number(s), at });
      i += s.length;
      continue;
    }
    if (c === "'" || c === '"') {
      let s = '';
      let j = i + 1;
      for (;;) {
        if (j >= src.length) throw new ExprError('Tırnak kapanmamış.', at);
        if (src[j] === c) {
          if (src[j + 1] === c) {
            s += c;
            j += 2;
            continue;
          }
          break;
        }
        s += src[j++];
      }
      out.push({ t: 'str', v: s, at });
      i = j + 1;
      continue;
    }
    if (c === '[') {
      const j = src.indexOf(']', i);
      if (j < 0) throw new ExprError('“]” bekleniyordu: alan adı kapanmamış.', at);
      const name = src.slice(i + 1, j).trim();
      if (!name) throw new ExprError('Köşeli parantez içinde alan adı yok.', at);
      out.push({ t: 'field', v: name, at });
      i = j + 1;
      continue;
    }
    if (c === '$') {
      WORD.lastIndex = i + 1;
      const m = WORD.exec(src);
      if (!m) throw new ExprError('“$” işaretinden sonra değişken adı bekleniyordu.', at);
      out.push({ t: 'var', v: m[0], at });
      i += 1 + m[0].length;
      continue;
    }
    WORD.lastIndex = i;
    const w = WORD.exec(src);
    if (w) {
      out.push({ t: 'word', v: w[0], at });
      i += w[0].length;
      continue;
    }
    const op = OPS.find((o) => src.startsWith(o, i));
    if (!op) throw new ExprError(`Anlaşılmayan karakter: “${c}”.`, at);
    out.push({ t: 'op', v: op, at });
    i += op.length;
  }
  out.push({ t: 'end', at: src.length + 1 });
  return out;
}

type Node =
  | { t: 'lit'; v: ExprValue }
  | { t: 'field'; name: string }
  | { t: 'var'; v: ExprVariable }
  | { t: 'call'; f: ExprFunction; args: Node[] }
  | { t: 'not' | 'neg'; a: Node }
  | { t: 'bin'; op: string; a: Node; b: Node };

const KEYWORDS: Record<string, string> = { VE: 'and', AND: 'and', VEYA: 'or', OR: 'or', DEGIL: 'not', NOT: 'not', DOGRU: 'true', TRUE: 'true', YANLIS: 'false', FALSE: 'false', BOS: 'null', NULL: 'null' };
const keyword = (tok: Token) => (tok.t === 'word' ? KEYWORDS[foldTurkish(tok.v)] : undefined);
const describe = (tok: Token) => (tok.t === 'end' ? 'ifadenin sonu' : tok.t === 'num' || tok.t === 'str' || tok.t === 'field' || tok.t === 'word' || tok.t === 'op' ? `“${tok.v}”` : `“$${tok.v}”`);

/** Binary operators by precedence, loosest first. */
const LEVELS: readonly (readonly string[])[] = [['or'], ['and'], ['=', '==', '!=', '<>', '<', '<=', '>', '>='], ['+', '-', '||'], ['*', '/', '%']];

class Parser {
  private readonly toks: Token[];
  private i = 0;
  readonly fields = new Set<string>();

  constructor(toks: Token[]) {
    this.toks = toks;
  }

  parse(): Node {
    if (this.peek().t === 'end') throw new ExprError('İfade boş.', 1);
    const n = this.binary(0);
    const rest = this.peek();
    if (rest.t !== 'end') throw new ExprError(`Beklenmeyen ${describe(rest)}; iki değer arasında işleç eksik olabilir.`, rest.at);
    return n;
  }

  private peek(): Token {
    return this.toks[this.i];
  }

  private next(): Token {
    return this.toks[this.i++];
  }

  /** The operator at the cursor, words folded to and/or. */
  private opAt(): string | undefined {
    const tok = this.peek();
    if (tok.t === 'op') return tok.v;
    const k = keyword(tok);
    return k === 'and' || k === 'or' ? k : undefined;
  }

  private binary(level: number): Node {
    if (level === LEVELS.length) return this.unary();
    let a = this.binary(level + 1);
    for (let op = this.opAt(); op && LEVELS[level].includes(op); op = this.opAt()) {
      this.next();
      a = { t: 'bin', op, a, b: this.binary(level + 1) };
    }
    return a;
  }

  private unary(): Node {
    const tok = this.peek();
    if (keyword(tok) === 'not') {
      this.next();
      return { t: 'not', a: this.unary() };
    }
    if (tok.t === 'op' && (tok.v === '-' || tok.v === '+')) {
      this.next();
      const a = this.unary();
      return tok.v === '-' ? { t: 'neg', a } : a;
    }
    return this.primary();
  }

  private primary(): Node {
    const tok = this.next();
    switch (tok.t) {
      case 'num':
      case 'str':
        return { t: 'lit', v: tok.v };
      case 'field':
        this.fields.add(tok.v);
        return { t: 'field', name: tok.v };
      case 'var': {
        const v = findVariable(tok.v);
        if (!v) throw new ExprError(`Bilinmeyen değişken: $${tok.v}.`, tok.at);
        return { t: 'var', v };
      }
      case 'word': {
        const after = this.peek();
        if (after.t === 'op' && after.v === '(') return this.call(tok);
        const k = keyword(tok);
        if (k === 'true' || k === 'false') return { t: 'lit', v: k === 'true' };
        if (k === 'null') return { t: 'lit', v: null };
        if (k) throw new ExprError(`${describe(tok)} burada kullanılamaz; önünde bir değer olmalı.`, tok.at);
        this.fields.add(tok.v);
        return { t: 'field', name: tok.v };
      }
      case 'op':
        if (tok.v === '(') {
          const n = this.binary(0);
          this.expect(')', 'Parantez kapanmamış: “)” bekleniyordu.');
          return n;
        }
        throw new ExprError(`Beklenmeyen ${describe(tok)}; burada bir değer, alan ya da işlev olmalı.`, tok.at);
      case 'end':
        throw new ExprError('İfade yarım kalmış: sonunda bir değer eksik.', tok.at);
    }
  }

  private call(name: Extract<Token, { t: 'word' }>): Node {
    const f = findFunction(name.v);
    if (!f) throw new ExprError(`Bilinmeyen işlev: ${name.v}().`, name.at);
    this.next(); // (
    const args: Node[] = [];
    const close = this.peek();
    if (!(close.t === 'op' && close.v === ')')) {
      for (;;) {
        args.push(this.binary(0));
        const sep = this.peek();
        if (sep.t === 'op' && sep.v === ',') {
          this.next();
          continue;
        }
        break;
      }
    }
    this.expect(')', `${f.name}(…) kapanmamış: “)” bekleniyordu.`);
    const [min, max] = f.arity;
    if (args.length < min || args.length > max) {
      const want = min === max ? `${min}` : max === Infinity ? `en az ${min}` : `${min} ya da ${max}`;
      throw new ExprError(`${f.name}() ${want} değer alır; ${args.length} verildi. Kullanım: ${f.signature}`, name.at);
    }
    return { t: 'call', f, args };
  }

  private expect(op: string, message: string): void {
    const tok = this.peek();
    if (tok.t === 'op' && tok.v === op) this.next();
    else throw new ExprError(message, tok.at);
  }
}

type Eval = (s: ExprScope) => ExprValue;

function binaryOp(op: string, a: ExprValue, b: ExprValue): ExprValue {
  switch (op) {
    case 'or':
      return truthy(a) || truthy(b);
    case 'and':
      return truthy(a) && truthy(b);
    case '=':
    case '==':
      return equals(a, b);
    case '!=':
    case '<>':
      return !equals(a, b);
    case '<':
    case '<=':
    case '>':
    case '>=': {
      const c = compare(a, b);
      if (c === null) return false;
      return op === '<' ? c < 0 : op === '<=' ? c <= 0 : op === '>' ? c > 0 : c >= 0;
    }
    case '||':
      return toText(a) + toText(b);
    case '+': {
      if (a === null || b === null) return null;
      const na = toNumber(a);
      const nb = toNumber(b);
      return na !== null && nb !== null ? na + nb : toText(a) + toText(b);
    }
    default: {
      const na = toNumber(a);
      const nb = toNumber(b);
      if (na === null || nb === null) return null;
      if ((op === '/' || op === '%') && nb === 0) return null;
      return op === '-' ? na - nb : op === '*' ? na * nb : op === '/' ? na / nb : na % nb;
    }
  }
}

function compileNode(n: Node): Eval {
  switch (n.t) {
    case 'lit': {
      const v = n.v;
      return () => v;
    }
    case 'field': {
      const name = n.name;
      return (s) => {
        const v = s.entity.attrs[name];
        return v === undefined ? null : v;
      };
    }
    case 'var': {
      const v = n.v;
      return (s) => v.get(s);
    }
    case 'call': {
      const f = n.f;
      const args = n.args.map(compileNode);
      return (s) => f.call(args.map((a) => a(s)));
    }
    case 'not': {
      const a = compileNode(n.a);
      return (s) => !truthy(a(s));
    }
    case 'neg': {
      const a = compileNode(n.a);
      return (s) => {
        const v = toNumber(a(s));
        return v === null ? null : -v;
      };
    }
    case 'bin': {
      const a = compileNode(n.a);
      const b = compileNode(n.b);
      const op = n.op;
      return (s) => binaryOp(op, a(s), b(s));
    }
  }
}

export function compileExpression(source: string): CompileResult {
  try {
    const parser = new Parser(tokenize(source));
    const fn = compileNode(parser.parse());
    const expr: CompiledExpression = {
      source,
      fields: [...parser.fields],
      evaluate: (s) => {
        try {
          const v = fn(s);
          return typeof v === 'number' && !Number.isFinite(v) ? null : v;
        } catch {
          return null;
        }
      },
    };
    return { ok: true, expr };
  } catch (e) {
    if (e instanceof ExprError) return { ok: false, error: e.message, at: e.at };
    throw e;
  }
}

/** Message for the dialog: "12. karakterde: …". */
export const expressionError = (r: Extract<CompileResult, { ok: false }>) => (r.at > 1 ? `${r.at}. karakterde: ${r.error}` : r.error);

/**
 * One line for the dialog: how the expression works out on the objects it
 * will read. `measures`: the geometry store's values of objects
 * (`measuredAt` records), asked once for all the previewed objects when the
 * expression first needs `$alan`, `$uzunluk`, `$y` or `$x`.
 */
export function previewExpression(expr: CompiledExpression, entities: readonly Entity[], kind: 'condition' | 'value', layerName: (id: string) => string, measures?: (entities: readonly Entity[]) => Float64Array): string {
  if (!entities.length) return 'Önizleme için uygun nesne yok.';
  const missing = expr.fields.filter((f) => !entities.some((e) => f in e.attrs));
  const note = missing.length ? ` ${missing.map((f) => `“${f}”`).join(', ')} alanı bu nesnelerde yok.` : '';
  const limit = Math.min(entities.length, 20000);
  const measured = measures && measuredOf(() => measures(entities.slice(0, limit)));
  const scope = (entity: Entity, index: number): ExprScope => ({ entity, index, layerName, measured: measured && (() => measured(index - 1)) });
  if (kind === 'condition') {
    let hits = 0;
    for (let i = 0; i < limit; i++) if (truthy(expr.evaluate(scope(entities[i], i + 1)))) hits++;
    return `${hits} / ${limit} nesne koşulu sağlıyor.${note}`;
  }
  const first = expr.evaluate(scope(entities[0], 1));
  let empty = 0;
  for (let i = 0; i < limit; i++) if (expr.evaluate(scope(entities[i], i + 1)) === null) empty++;
  const who = entities[0].label ? ` (${entities[0].label})` : '';
  return `İlk nesnede${who}: ${first === null ? 'boş' : `“${toText(first)}”`}.${empty ? ` ${empty} nesnede sonuç boş.` : ''}${note}`;
}
