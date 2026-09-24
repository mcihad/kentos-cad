import ts from 'typescript';
import { describe, expect, it } from 'vitest';

/**
 * One source of computation (CLAUDE.md §14, docs/adr/0008 S3c): the
 * geometry is written once, in the Rust core, and the TypeScript left where
 * the algorithms were only passes calls through (`op('name')`) or reads
 * the core's flat answers. This test parses those files and fails on
 * arithmetic or `Math.` in them, so an algorithm cannot come back into
 * TypeScript unnoticed: write it in crates/shared/geometry-core and call it.
 *
 * Counting and indexing into a flat buffer is not geometry and passes when
 * every operand is an index: an integer, `i`/`j`/`k`/`n`/`at`, an
 * UPPER_CASE stride or a `.length`. Unary `+` reads a number from typed
 * text. The few other exceptions are listed below with their reason.
 *
 * The expression language is the style core's (crates/shared/style-core,
 * docs/adr/0008 “İfade dili”): its TypeScript only builds the table of what
 * an expression reads and reads the answer back. The style engine and the
 * SVG editor keep their own geometry for now (the next style-core slices,
 * docs/DEVIR.md), so `src/style` is not checked; screen-space drawing
 * (pixel offsets on the overlay) is presentation, not geometry.
 */

/**
 * The facades: every TypeScript file in model/geom and model/ops (tests
 * aside; goldenCases.ts is test support that compares results with the
 * golden file), and the readers elsewhere where the S1–S5 slices moved a
 * calculation to the core. A new file in those folders is checked too.
 */
const SOURCES = import.meta.glob<string>(
  [
    './geom/*.ts',
    './ops/*.ts',
    './expression/*.ts',
    '!./**/*.test.ts',
    '!./geom/goldenCases.ts',
    '!./expression/cases.ts',
    './geometry.ts',
    './entities.ts',
    '../render/triangulate.ts',
    '../tools/constructions.ts',
    '../tools/coordinateInput.ts',
    '../viewport/objectTracking.ts',
    '../viewport/picking.ts',
    '../viewport/storeRecords.ts',
    '../processing/geometry.ts',
  ],
  { query: '?raw', import: 'default', eager: true },
);

/** A glob key as a repository path (src/…). */
const repoPath = (key: string) => new URL(key, 'file:///src/model/').pathname.slice(1);

/** Allowed arithmetic, by file and function, with the reason. */
const EXCEPTIONS: Record<string, Record<string, string>> = {
  'src/model/geometry.ts': {
    extendBounds: 'Growing a box by a point is bookkeeping (min, max and the padding the caller asks for), not geometry.',
  },
  'src/model/expression/expression.ts': {
    put: 'Joining the table’s texts into the one text the core reads (text, not arithmetic).',
    textSpans: 'Where each text starts in the core’s joined answer: a running sum of the lengths it gives (reading a flat buffer).',
    value: 'Slicing an object’s text out of the core’s joined answer at the offset and length found once (reading a flat buffer).',
  },
};

const ARITHMETIC = new Set<ts.SyntaxKind>([
  ts.SyntaxKind.PlusToken,
  ts.SyntaxKind.MinusToken,
  ts.SyntaxKind.AsteriskToken,
  ts.SyntaxKind.SlashToken,
  ts.SyntaxKind.PercentToken,
  ts.SyntaxKind.AsteriskAsteriskToken,
  ts.SyntaxKind.PlusEqualsToken,
  ts.SyntaxKind.MinusEqualsToken,
  ts.SyntaxKind.AsteriskEqualsToken,
  ts.SyntaxKind.SlashEqualsToken,
  ts.SyntaxKind.PercentEqualsToken,
  ts.SyntaxKind.AsteriskAsteriskEqualsToken,
]);

const INDEX_NAMES = new Set(['i', 'j', 'k', 'n', 'at']);

/** Whether an expression only counts or indexes. */
function isIndex(e: ts.Expression): boolean {
  if (ts.isParenthesizedExpression(e)) return isIndex(e.expression);
  if (ts.isNumericLiteral(e)) return Number.isInteger(Number(e.text));
  if (ts.isIdentifier(e)) return INDEX_NAMES.has(e.text) || /^[A-Z][A-Z0-9_]*$/.test(e.text);
  if (ts.isPropertyAccessExpression(e)) return e.name.text === 'length' || (e.expression.kind === ts.SyntaxKind.ThisKeyword && INDEX_NAMES.has(e.name.text));
  if (ts.isBinaryExpression(e) && ARITHMETIC.has(e.operatorToken.kind)) return isIndex(e.left) && isIndex(e.right);
  return false;
}

interface Finding {
  line: number;
  owner: string;
  text: string;
}

/** Arithmetic and `Math.` in a source text, with the enclosing function's name. */
function arithmeticIn(name: string, text: string): Finding[] {
  const src = ts.createSourceFile(name, text, ts.ScriptTarget.ES2023, true);
  const out: Finding[] = [];
  const found = (n: ts.Node, owner: string): void => {
    out.push({ line: src.getLineAndCharacterOfPosition(n.getStart()).line + 1, owner, text: n.getText().slice(0, 100) });
  };
  const walk = (n: ts.Node, owner: string): void => {
    let o = owner;
    if ((ts.isFunctionDeclaration(n) || ts.isMethodDeclaration(n)) && n.name) o = n.name.getText();
    else if (ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) && n.initializer && (ts.isArrowFunction(n.initializer) || ts.isFunctionExpression(n.initializer))) o = n.name.text;
    // A reported expression is not walked further: its operands are in the same finding.
    if (ts.isBinaryExpression(n) && ARITHMETIC.has(n.operatorToken.kind)) {
      if (!isIndex(n)) return found(n, o);
    } else if (ts.isPrefixUnaryExpression(n) && n.operator === ts.SyntaxKind.MinusToken) {
      if (!isIndex(n.operand) && !(ts.isIdentifier(n.operand) && n.operand.text === 'Infinity')) return found(n, o);
    } else if ((ts.isPrefixUnaryExpression(n) || ts.isPostfixUnaryExpression(n)) && (n.operator === ts.SyntaxKind.PlusPlusToken || n.operator === ts.SyntaxKind.MinusMinusToken)) {
      if (!isIndex(n.operand)) found(n, o);
    } else if (ts.isPropertyAccessExpression(n) && ts.isIdentifier(n.expression) && n.expression.text === 'Math') found(n, o);
    ts.forEachChild(n, (c) => walk(c, o));
  };
  walk(src, '(modül)');
  return out;
}

describe('one source of computation (CLAUDE.md §14)', () => {
  it('finds arithmetic and Math, and lets counting and indexing pass', () => {
    const found = arithmeticIn(
      'sample.ts',
      [
        'export const mid = (a: Vec2, b: Vec2) => ({ x: (a.x + b.x) / 2, y: a.y });',
        'export function far(p: Vec2) { return Math.hypot(p.x, p.y); }',
        'export function flip(p: Vec2) { return { x: -p.x, y: p.y }; }',
        'export function read(xy: Float64Array, n: number) { const out = []; for (let i = 0; i < n; i++) out.push({ x: xy[2 * i], y: xy[2 * i + 1] }); return [out, xy.length * 2, -1, -Infinity]; }',
      ].join('\n'),
    );
    expect(found.map((f) => `${f.line} ${f.owner} ${f.text}`)).toEqual(['1 mid (a.x + b.x) / 2', '2 far Math.hypot', '3 flip -p.x']);
  });

  it('the facades only pass calls through', () => {
    const files = Object.entries(SOURCES).map(([key, text]) => [repoPath(key), text] as const);
    expect(files.length).toBeGreaterThan(40);
    const failures: string[] = [];
    const used = new Set<string>();
    for (const [f, text] of files)
      for (const x of arithmeticIn(f, text)) {
        if (EXCEPTIONS[f]?.[x.owner]) used.add(`${f} ${x.owner}`);
        else failures.push(`${f}:${x.line} [${x.owner}] ${x.text}`);
      }
    expect(failures, 'Hesabı crates/shared/geometry-core içinde yazıp op() ile çağırın (CLAUDE.md §14).').toEqual([]);
    // An exception nothing needs any more is removed rather than left to excuse a new algorithm.
    const stale = Object.entries(EXCEPTIONS).flatMap(([f, fns]) => Object.keys(fns).filter((fn) => !used.has(`${f} ${fn}`)).map((fn) => `${f} ${fn}`));
    expect(stale).toEqual([]);
  });
});
